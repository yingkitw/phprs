//! Post-compile optimization passes.
//!
//! Two passes run once per compiled op array (script body, named functions,
//! closures, and class methods — the class walk recurses so classes defined
//! inside functions are covered too):
//!
//! 1. **Constant folding** — pure ops whose operands are entirely literal are
//!    evaluated at compile time by running the *real* dispatch handler
//!    (`dispatch_opcode`) on a scratch [`ExecuteData`], then rewriting the op
//!    to `QmAssign <literal> -> <same result slot>`. Reusing the handlers
//!    guarantees the folded value matches runtime semantics by construction
//!    (bool construction, int/float preservation, overflow wrapping).
//!    Folded results for a temp slot are remembered and substituted into
//!    later foldable ops reading that slot, so nested expressions such as
//!    `2 + 3 * 4` collapse to a single literal. Knowledge is dropped at
//!    every jump target (basic-block boundary). Operands that are temp refs,
//!    var refs, or `ConstantAst` (bare-identifier constant lookups such as
//!    `PHP_INT_MAX`) are never folded — a `ConstantAst` may be `define()`d
//!    at runtime before the op executes. Division/modulo by zero return
//!    `Err` from the handler and are left untouched to preserve exact
//!    runtime error semantics. Op count never changes, so no jump
//!    renumbering is needed for this pass.
//!
//! 2. **Dead code elimination** — ops unreachable from index 0 or from any
//!    absolute jump target are removed, and the `extended_value` targets of
//!    `Jmp`, `JmpZ`, `JmpNZ`, `JmpNullZ`, `FeFetch`, and `TryCatchBegin` are
//!    renumbered. (Catch/finally body addresses are derived at runtime from
//!    the *position* of `CatchBegin`/`FinallyBegin` markers, so they stay
//!    consistent as long as the markers themselves are kept.) Exception
//!    structural markers are therefore never removed: an unreached
//!    `TryCatchBegin` never triggers a handler scan, so keeping dead markers
//!    is harmless, while removing one could confuse `collect_handlers`.

use crate::engine::compile::function_table::FunctionTable;
use crate::engine::types::{PhpType, PhpValue, Val};
use crate::engine::vm::dispatch_handlers::dispatch_opcode;
use crate::engine::vm::execute_data::{clone_val, is_temp_ref, is_var_ref, result_slot, ExecuteData};
use crate::engine::vm::opcodes::{Op, OpArray, Opcode};
use std::collections::HashMap;

/// Opcodes whose `extended_value` holds an absolute op index (a jump or
/// handler-dispatch target). These are the only operands that must be
/// renumbered when ops are removed. Other opcodes use `extended_value` for
/// non-index data (e.g. `AddArrayElement` keys) and must not be touched.
fn has_jump_target(op: &Op) -> bool {
    matches!(
        op.opcode,
        Opcode::Jmp
            | Opcode::JmpZ
            | Opcode::JmpNZ
            | Opcode::JmpNullZ
            | Opcode::FeFetch
            | Opcode::TryCatchBegin
    )
}

/// Opcodes that never fall through to the next op.
fn is_terminator(op: &Op) -> bool {
    matches!(op.opcode, Opcode::Jmp | Opcode::Return | Opcode::Throw)
}

/// Exception structural markers. Always kept by DCE: the runtime derives
/// catch/finally body positions from their relative layout, and an
/// unreachable marker is inert.
fn is_exception_marker(op: &Op) -> bool {
    matches!(
        op.opcode,
        Opcode::TryCatchBegin
            | Opcode::TryCatchEnd
            | Opcode::CatchBegin
            | Opcode::CatchEnd
            | Opcode::FinallyBegin
            | Opcode::FinallyEnd
    )
}

/// Pure, side-effect-free opcodes eligible for constant folding. All of them
/// only *read* op1/op2 and write their result slot, so substituting literals
/// into their operands is always equivalent.
fn foldable(opcode: Opcode) -> bool {
    matches!(
        opcode,
        Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Mod
            | Opcode::Pow
            | Opcode::Sl
            | Opcode::Sr
            | Opcode::Concat
            | Opcode::BwOr
            | Opcode::BwAnd
            | Opcode::BwXor
            | Opcode::BwNot
            | Opcode::BoolNot
            | Opcode::BoolAnd
            | Opcode::BoolOr
            | Opcode::BoolXor
            | Opcode::IsIdentical
            | Opcode::IsNotIdentical
            | Opcode::IsEqual
            | Opcode::IsNotEqual
            | Opcode::IsSmaller
            | Opcode::IsSmallerOrEqual
            | Opcode::Spaceship
            | Opcode::Coalesce
    )
}

/// A literal operand: not a temp-slot ref, not a named-var ref, and not a
/// bare-identifier constant reference (`ConstantAst`).
fn is_literal(v: &Val) -> bool {
    !is_temp_ref(v) && !is_var_ref(v) && v.get_type() != PhpType::ConstantAst
}

fn null_val() -> Val {
    Val::new(PhpValue::Long(0), PhpType::Null)
}

/// Optimize a whole script: the top-level op array (which carries the class
/// table) plus every function/closure body in the compile-time function
/// table.
pub fn optimize_script(op_array: &mut OpArray, functions: &mut FunctionTable) {
    optimize_op_array(op_array);
    functions.for_each_op_array_mut(optimize_op_array);
}

/// Optimize one op array and, recursively, every class method nested in its
/// class table.
pub fn optimize_op_array(op_array: &mut OpArray) {
    fold_constants(&mut op_array.ops);
    let ops = std::mem::take(&mut op_array.ops);
    op_array.ops = eliminate_dead_code(ops);
    for class in op_array.class_table.values_mut() {
        for method in class.methods.values_mut() {
            optimize_op_array(&mut method.op_array);
        }
    }
}

/// Pass 1: fold pure ops over literal operands, with local constant
/// propagation through temp slots. Op count is preserved.
fn fold_constants(ops: &mut [Op]) {
    let n = ops.len();
    // Jump targets are basic-block joins; knowledge must not cross them.
    let mut is_target = vec![false; n];
    for op in ops.iter() {
        if has_jump_target(op) && (op.extended_value as usize) < n {
            is_target[op.extended_value as usize] = true;
        }
    }

    let mut scratch = ExecuteData::new();
    // slot -> folded literal known to be stored there (only within the
    // current basic block).
    let mut known: HashMap<usize, Val> = HashMap::new();

    for i in 0..n {
        if is_target[i] {
            known.clear();
        }
        if !foldable(ops[i].opcode) {
            // A non-foldable write to a slot invalidates any known value.
            if let Some(slot) = result_slot(&ops[i]) {
                known.remove(&slot);
            }
            continue;
        }
        let unary = matches!(ops[i].opcode, Opcode::BwNot | Opcode::BoolNot);
        substitute_known_literals(&mut ops[i], &known, unary);

        let operands_literal =
            is_literal(&ops[i].op1) && (unary || is_literal(&ops[i].op2));
        if !operands_literal {
            continue;
        }
        let Some(slot) = result_slot(&ops[i]) else {
            continue;
        };
        scratch.temp_vars.clear();
        // Errors (division/modulo by zero) leave the op for runtime.
        if dispatch_opcode(&ops[i], &mut scratch).is_err() {
            continue;
        }
        if slot >= scratch.temp_vars.len() {
            continue;
        }
        let folded =
            std::mem::replace(&mut scratch.temp_vars[slot], null_val());
        ops[i].opcode = Opcode::QmAssign;
        ops[i].op1 = clone_val(&folded);
        known.insert(slot, folded);
    }
}

/// Replace temp-slot operand references whose contents are known literals
/// with the literals themselves. Only applied to foldable (pure-read) ops.
fn substitute_known_literals(op: &mut Op, known: &HashMap<usize, Val>, unary: bool) {
    substitute_one(&mut op.op1, known);
    if !unary {
        substitute_one(&mut op.op2, known);
    }
}

fn substitute_one(operand: &mut Val, known: &HashMap<usize, Val>) {
    if is_temp_ref(operand)
        && let PhpValue::Long(slot) = &operand.value
        && let Some(lit) = known.get(&(*slot as usize))
    {
        *operand = clone_val(lit);
    }
}

/// Pass 2: remove ops that cannot execute (unreachable from entry or any
/// jump target) and renumber the absolute targets of the survivors.
fn eliminate_dead_code(ops: Vec<Op>) -> Vec<Op> {
    let n = ops.len();
    if n == 0 {
        return ops;
    }

    // 1. Seed jump targets.
    let mut reachable = vec![false; n];
    for op in &ops {
        if has_jump_target(op) && (op.extended_value as usize) < n {
            reachable[op.extended_value as usize] = true;
        }
    }
    reachable[0] = true;

    // 2. Linear reachability: execution only ever falls through, so a single
    //    forward pass in index order is a complete fixpoint.
    for i in 0..n {
        if reachable[i] && !is_terminator(&ops[i]) && i + 1 < n {
            reachable[i + 1] = true;
        }
    }

    // 3. Keep reachable ops plus exception markers.
    let mut new_index: Vec<Option<usize>> = vec![None; n];
    let mut kept = Vec::with_capacity(n);
    for (i, op) in ops.into_iter().enumerate() {
        if reachable[i] || is_exception_marker(&op) {
            new_index[i] = Some(kept.len());
            kept.push(op);
        }
    }
    if kept.len() == n {
        return kept; // nothing removed, no renumbering needed
    }

    // 4. Renumber absolute targets. Every reachable op's target is reachable
    //    by construction, so the lookup succeeds; fall back to the old value
    //    defensively.
    for op in &mut kept {
        if has_jump_target(op)
            && let Some(idx) = new_index
                .get(op.extended_value as usize)
                .copied()
                .flatten()
        {
            op.extended_value = idx as u32;
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::compile::compile_string_with_functions;
    use crate::engine::types::PhpResult;
    use crate::engine::vm::{execute_ex, ExecuteData};
    use std::sync::Arc;

    fn compile(code: &str) -> OpArray {
        let (op_array, _) = compile_string_with_functions(code, "opt_test.php")
            .expect("compile");
        op_array
    }

    fn run_php(code: &str) -> Result<(PhpResult, String), String> {
        let (op_array, ft) = compile_string_with_functions(code, "opt_test.php")?;
        crate::php::output::php_output_start().map_err(|e| e.to_string())?;
        let mut ed = ExecuteData::new();
        ed.function_table = Some(Arc::new(ft));
        let result = execute_ex(&mut ed, &op_array);
        let output = crate::php::output::php_output_end().map_err(|e| e.to_string())?;
        Ok((result, output))
    }

    fn assert_success(r: &PhpResult, out: &str) {
        assert!(
            matches!(r, PhpResult::Success),
            "vm result: {r:?}, output: {out:?}"
        );
    }

    #[test]
    fn test_folds_literal_arithmetic_to_qm_assign() {
        let op_array = compile("<?php $x = 1 + 2;");
        assert!(
            op_array.ops.iter().any(|op| op.opcode == Opcode::QmAssign),
            "expected a folded QmAssign op"
        );
        assert!(
            !op_array.ops.iter().any(|op| op.opcode == Opcode::Add),
            "Add should have been folded away"
        );
    }

    #[test]
    fn test_folds_nested_expression_completely() {
        // $x = 2 + 3 * 4  →  Mul folds to 12, propagation folds Add to 14.
        let op_array = compile("<?php $x = 2 + 3 * 4;");
        assert!(
            !op_array
                .ops
                .iter()
                .any(|op| matches!(op.opcode, Opcode::Add | Opcode::Mul)),
            "nested literal expression should fold completely"
        );
        op_array
            .ops
            .iter()
            .find(|op| op.opcode == Opcode::QmAssign && matches!(op.op1.value, PhpValue::Long(14)))
            .expect("expected folded literal 14");
    }

    #[test]
    fn test_folds_string_concat() {
        let op_array = compile("<?php echo \"a\" . \"b\" . \"c\";");
        assert!(
            !op_array.ops.iter().any(|op| op.opcode == Opcode::Concat),
            "literal concat should fold"
        );
    }

    #[test]
    fn test_keeps_variable_operands() {
        let op_array = compile("<?php $a = 1; $x = $a + 1;");
        assert!(
            op_array.ops.iter().any(|op| op.opcode == Opcode::Add),
            "op with a variable operand must not fold"
        );
    }

    #[test]
    fn test_does_not_fold_constant_ast_operand() {
        // FOO is an undefined ConstantAst at compile time; it may be
        // define()d before this op runs, so folding is forbidden.
        let op_array = compile("<?php echo FOO + 0;");
        assert!(
            op_array.ops.iter().any(|op| op.opcode == Opcode::Add),
            "ConstantAst operand must not fold"
        );
    }

    #[test]
    fn test_division_by_zero_left_for_runtime() {
        let op_array = compile("<?php $x = 1 / 0;");
        assert!(
            op_array.ops.iter().any(|op| op.opcode == Opcode::Div),
            "division by zero must not fold"
        );
        // But an even division folds.
        let op_array = compile("<?php $x = 10 / 2;");
        assert!(op_array
            .ops
            .iter()
            .any(|op| op.opcode == Opcode::QmAssign
                && matches!(op.op1.value, PhpValue::Long(5))));
    }

    #[test]
    fn test_dead_code_after_return_removed() {
        let op_array = compile("<?php echo 'live'; return 1; echo 'dead';");
        // Only the live echo survives; the return is intact.
        assert_eq!(
            op_array
                .ops
                .iter()
                .filter(|op| op.opcode == Opcode::Echo)
                .count(),
            1,
            "dead echo should be eliminated"
        );
        assert!(op_array.ops.iter().any(|op| op.opcode == Opcode::Return));
    }

    #[test]
    fn test_optimization_preserves_loop_semantics() {
        let code = r#"<?php
$total = 0;
for ($i = 0; $i < 5; $i++) {
    $total += $i * 2;
}
echo $total;
"#;
        let (r, out) = run_php(code).expect("run");
        assert_success(&r, &out);
        assert!(out.contains("20"), "expected 20, got: {out:?}");
    }

    #[test]
    fn test_optimization_preserves_try_catch() {
        let code = r#"<?php
function boom() {
    throw new Exception("x");
}
try {
    boom();
    echo "not reached";
} catch (Exception $e) {
    echo "caught:" . $e->getMessage();
} finally {
    echo "|finally";
}
"#;
        let (r, out) = run_php(code).expect("run");
        assert_success(&r, &out);
        assert!(
            out.contains("caught:x|finally"),
            "expected caught:x|finally, got: {out:?}"
        );
    }

    #[test]
    fn test_optimization_preserves_foreach_and_folds() {
        let code = r#"<?php
$sum = 0;
foreach (array(1, 2, 3) as $v) {
    $sum += $v;
}
echo $sum + 10 * 10;
"#;
        let (r, out) = run_php(code).expect("run");
        assert_success(&r, &out);
        assert!(out.contains("106"), "expected 106, got: {out:?}");
    }

    #[test]
    fn test_while_false_body_is_inert() {
        // The body stays in the op array (it is a jump target) but must not
        // execute.
        let code = r#"<?php
while (false) { echo "no"; }
echo "yes";
"#;
        let (r, out) = run_php(code).expect("run");
        assert_success(&r, &out);
        assert_eq!(out, "yes");
    }

    #[test]
    fn test_functions_and_methods_are_optimized() {
        let code = r#"<?php
function add($a, $b) { return 100 + 200 + $a; }
class Calc {
    public function ten() { return 5 + 5; }
}
$c = new Calc();
echo add(1, 2) . "," . $c->ten();
"#;
        let (r, out) = run_php(code).expect("run");
        assert_success(&r, &out);
        assert_eq!(out, "301,10");

        // White-box: the function body should contain the folded literal
        // (300) feeding the surviving Add($a); the literal-literal 100 + 200
        // must be gone.
        let (_, ft) =
            compile_string_with_functions(code, "opt_test.php").expect("compile");
        let add_fn = ft.lookup_function("add").expect("add fn");
        assert!(
            !add_fn.ops.iter().any(|op| op.opcode == Opcode::Add
                && matches!(op.op1.value, PhpValue::Long(100))),
            "literal-literal add inside function should fold"
        );
        assert!(add_fn
            .ops
            .iter()
            .any(|op| op.opcode == Opcode::Add
                && matches!(op.op1.value, PhpValue::Long(300))),
            "expected folded 300 propagated into the surviving Add");
    }

    #[test]
    fn test_optimization_is_idempotent() {
        let code = "<?php $x = 1 + 2 * 3; if ($x > 5) { echo $x; }";
        let (mut op_array, mut ft) =
            compile_string_with_functions(code, "idem.php").expect("compile");
        optimize_script(&mut op_array, &mut ft);
        let after_first = op_array.ops.len();
        optimize_script(&mut op_array, &mut ft);
        assert_eq!(op_array.ops.len(), after_first, "second pass must be a no-op");
    }
}

