//! Class hierarchy resolution: `extends` (parent class) and `implements`
//! (interfaces).
//!
//! PHP resolves members along two axes:
//!
//! - **The parent chain** — `class B extends A` inherits A's methods,
//!   properties, constants and static properties. The chain can be several
//!   levels deep (`Leaf extends Mid extends Base`).
//! - **The interface list** — `class C implements I` inherits I's method
//!   *contract* (which methods must exist), and the interface's own default
//!   implementations if it declares any.
//!
//! Both axes can be combined: a class may extend a parent *and* implement
//! interfaces, and the parent may itself implement interfaces.
//!
//! ## Why a separate module
//!
//! Method/property resolution is needed in several places (instance calls,
//! static calls, property reads, `new`, reflection). Before this module each
//! of those sites had its own `class_table.get(name)` lookup that only saw the
//! class's *own* members, so inherited members were invisible. Centralising
//! the walk keeps the rules identical everywhere.
//!
//! ## Ordering
//!
//! PHP resolves a member on the class itself first; only if absent does it
//! look up the parent chain, then the interfaces. A child may therefore
//! override an inherited method (the usual case) and, in practice, override
//! an interface's default implementation.

use std::collections::HashMap;

use crate::engine::types::{ClassEntry, Val};
use crate::engine::vm::Op;

/// The parts of a method that callers need in order to execute it.
///
/// `ClassMethod` is not `Clone` (its op array owns compiled ops), so the
/// hierarchy walk copies the pieces it needs — the same approach the trait
/// copier uses.
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub params: Vec<String>,
    pub ops: Vec<Op>,
    pub file_label: String,
    pub variadic_param: Option<String>,
    pub ref_params: Vec<bool>,
}

/// Copy a method's executable parts.
pub fn method_info(method: &crate::engine::types::ClassMethod, class_name: &str) -> MethodInfo {
    let ops = method
        .op_array
        .ops
        .iter()
        .map(|op| {
            Op::new(
                op.opcode,
                crate::engine::vm::execute_data::clone_val(&op.op1),
                crate::engine::vm::execute_data::clone_val(&op.op2),
                crate::engine::vm::execute_data::clone_val(&op.result),
                op.extended_value,
            )
        })
        .collect();
    let file_label = method
        .op_array
        .filename
        .clone()
        .filter(|f| !f.is_empty())
        .unwrap_or_else(|| format!("{}::{}", class_name, method.name));
    MethodInfo {
        params: method.params.clone(),
        ops,
        file_label,
        variadic_param: method.op_array.variadic_param.clone(),
        ref_params: method.op_array.ref_params.clone(),
    }
}

/// Prefix used to key interfaces in the class table, so an interface never
/// collides with a class of the same name (mirrors the `__trait_` prefix).
pub const INTERFACE_PREFIX: &str = "__interface_";

/// Build the class-table key for an interface name.
pub fn interface_key(resolved_name: &str) -> String {
    format!("{}{}", INTERFACE_PREFIX, resolved_name)
}

/// The name of an interface without the internal prefix.
pub fn interface_display_name(key: &str) -> &str {
    key.strip_prefix(INTERFACE_PREFIX).unwrap_or(key)
}

/// The number of interfaces an interface may extend directly. PHP allows an
/// interface to list several parents; the cap keeps a malformed script from
/// building an unbounded chain.
pub const MAX_EXTENDED_INTERFACES: usize = 32;

/// Resolve a method by walking the class, then its parent chain, then its
/// interfaces (and their parents/interfaces).
///
/// Returns an owned copy so callers can mutate the class table while holding
/// the result.
pub fn resolve_method(
    class_table: &HashMap<String, ClassEntry>,
    class_name: &str,
    method_name: &str,
) -> Option<MethodInfo> {
    let mut visited: Vec<String> = Vec::new();
    resolve_method_inner(class_table, class_name, method_name, &mut visited)
}

fn resolve_method_inner(
    class_table: &HashMap<String, ClassEntry>,
    class_name: &str,
    method_name: &str,
    visited: &mut Vec<String>,
) -> Option<MethodInfo> {
    let ce = class_table.get(class_name)?;
    if visited.iter().any(|v| v == class_name) {
        return None;
    }
    visited.push(class_name.to_string());

    // The class's own method wins.
    if let Some(m) = ce.methods.get(method_name) {
        return Some(method_info(m, class_name));
    }

    // Then the parent chain.
    if let Some(parent) = &ce.parent_name
        && let Some(m) = resolve_method_inner(class_table, parent, method_name, visited)
    {
        return Some(m);
    }

    // Then the interfaces (which may themselves extend other interfaces).
    for iface in &ce.interfaces {
        if let Some(m) = resolve_method_inner(class_table, iface, method_name, visited) {
            return Some(m);
        }
    }

    None
}

/// Resolve an instance property (default value) along the hierarchy.
///
/// PHP does not inherit the *value* of a redeclared property, but a property
/// declared only on a parent is available on the child, so the parent's
/// default is used when the child does not declare the property itself.
pub fn resolve_default_property(
    class_table: &HashMap<String, ClassEntry>,
    class_name: &str,
    prop_name: &str,
) -> Option<Val> {
    let mut visited: Vec<String> = Vec::new();
    resolve_default_property_inner(class_table, class_name, prop_name, &mut visited)
}

fn resolve_default_property_inner(
    class_table: &HashMap<String, ClassEntry>,
    class_name: &str,
    prop_name: &str,
    visited: &mut Vec<String>,
) -> Option<Val> {
    let ce = class_table.get(class_name)?;
    if visited.iter().any(|v| v == class_name) {
        return None;
    }
    visited.push(class_name.to_string());

    if let Some(v) = ce.default_properties.get(prop_name) {
        return Some(v.clone());
    }
    if let Some(parent) = &ce.parent_name
        && let Some(v) = resolve_default_property_inner(class_table, parent, prop_name, visited)
    {
        return Some(v);
    }
    for iface in &ce.interfaces {
        if let Some(v) = resolve_default_property_inner(class_table, iface, prop_name, visited) {
            return Some(v);
        }
    }
    None
}

/// Resolve a class constant along the hierarchy.
pub fn resolve_constant(
    class_table: &HashMap<String, ClassEntry>,
    class_name: &str,
    const_name: &str,
) -> Option<Val> {
    let mut visited: Vec<String> = Vec::new();
    resolve_constant_inner(class_table, class_name, const_name, &mut visited)
}

fn resolve_constant_inner(
    class_table: &HashMap<String, ClassEntry>,
    class_name: &str,
    const_name: &str,
    visited: &mut Vec<String>,
) -> Option<Val> {
    let ce = class_table.get(class_name)?;
    if visited.iter().any(|v| v == class_name) {
        return None;
    }
    visited.push(class_name.to_string());

    if let Some(v) = ce.constants.get(const_name) {
        return Some(v.clone());
    }
    if let Some(parent) = &ce.parent_name
        && let Some(v) = resolve_constant_inner(class_table, parent, const_name, visited)
    {
        return Some(v);
    }
    for iface in &ce.interfaces {
        if let Some(v) = resolve_constant_inner(class_table, iface, const_name, visited) {
            return Some(v);
        }
    }
    None
}

/// Resolve a static property along the hierarchy.
pub fn resolve_static_property(
    class_table: &HashMap<String, ClassEntry>,
    class_name: &str,
    prop_name: &str,
) -> Option<Val> {
    let mut visited: Vec<String> = Vec::new();
    resolve_static_property_inner(class_table, class_name, prop_name, &mut visited)
}

fn resolve_static_property_inner(
    class_table: &HashMap<String, ClassEntry>,
    class_name: &str,
    prop_name: &str,
    visited: &mut Vec<String>,
) -> Option<Val> {
    let ce = class_table.get(class_name)?;
    if visited.iter().any(|v| v == class_name) {
        return None;
    }
    visited.push(class_name.to_string());

    if let Some(v) = ce.static_properties.get(prop_name) {
        return Some(v.clone());
    }
    if let Some(parent) = &ce.parent_name
        && let Some(v) = resolve_static_property_inner(class_table, parent, prop_name, visited)
    {
        return Some(v);
    }
    for iface in &ce.interfaces {
        if let Some(v) = resolve_static_property_inner(class_table, iface, prop_name, visited) {
            return Some(v);
        }
    }
    None
}

/// Whether `class_name` is an instance of `target` — either because it is
/// that class, extends it, or implements it (transitively).
///
/// This is the `instanceof` relation for user-defined classes.
pub fn is_a(class_table: &HashMap<String, ClassEntry>, class_name: &str, target: &str) -> bool {
    let mut visited: Vec<String> = Vec::new();
    is_a_inner(class_table, class_name, target, &mut visited)
}

fn is_a_inner(
    class_table: &HashMap<String, ClassEntry>,
    class_name: &str,
    target: &str,
    visited: &mut Vec<String>,
) -> bool {
    if class_name == target {
        return true;
    }
    if visited.iter().any(|v| v == class_name) {
        return false;
    }
    visited.push(class_name.to_string());
    let Some(ce) = class_table.get(class_name) else {
        return false;
    };
    if let Some(parent) = &ce.parent_name
        && is_a_inner(class_table, parent, target, visited)
    {
        return true;
    }
    for iface in &ce.interfaces {
        if iface == target {
            return true;
        }
        if is_a_inner(class_table, iface, target, visited) {
            return true;
        }
    }
    false
}

/// The method names an interface *requires* an implementing class to provide.
///
/// An interface's contract is the set of **bodyless** method declarations —
/// `public function bar();`. A method that *has* a body supplies a default
/// implementation, which the implementing class inherits and need not repeat.
pub fn interface_required_methods(
    class_table: &HashMap<String, ClassEntry>,
    interface_key: &str,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut visited: Vec<String> = Vec::new();
    collect_required_methods(class_table, interface_key, &mut out, &mut visited);
    out
}

fn collect_required_methods(
    class_table: &HashMap<String, ClassEntry>,
    interface_key: &str,
    out: &mut Vec<String>,
    visited: &mut Vec<String>,
) {
    if visited.iter().any(|v| v == interface_key) {
        return;
    }
    visited.push(interface_key.to_string());
    let Some(ce) = class_table.get(interface_key) else {
        return;
    };
    for (name, method) in &ce.methods {
        // A method with a body has a default implementation; only bodyless
        // declarations form the contract.
        if !method.op_array.ops.is_empty() {
            continue;
        }
        if !out.iter().any(|v| v == name) {
            out.push(name.clone());
        }
    }
    // An interface extending another inherits its contract.
    for parent in &ce.interfaces {
        collect_required_methods(class_table, parent, out, visited);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::ClassMethod;
    use crate::engine::vm::OpArray;

    /// A method with a non-empty op array (a default implementation).
    fn method_with_body(name: &str) -> ClassMethod {
        let mut op_array = OpArray::new(String::new());
        op_array.ops.push(Op::new(
            crate::engine::vm::Opcode::Return,
            crate::engine::facade::null_val(),
            crate::engine::facade::null_val(),
            crate::engine::facade::null_val(),
            0,
        ));
        ClassMethod {
            name: name.to_string(),
            visibility: crate::engine::types::Visibility::Public,
            is_static: false,
            params: Vec::new(),
            op_array,
        }
    }

    /// A bodyless method declaration (contract only).
    fn method_contract_only(name: &str) -> ClassMethod {
        ClassMethod {
            name: name.to_string(),
            visibility: crate::engine::types::Visibility::Public,
            is_static: false,
            params: Vec::new(),
            op_array: OpArray::new(String::new()),
        }
    }

    fn iface_key(name: &str) -> String {
        interface_key(name)
    }

    #[test]
    fn test_is_a_across_extends_chain() {
        let mut ct = HashMap::new();
        let mut base = ClassEntry::new("Base");
        base.methods
            .insert("greet".into(), method_with_body("greet"));
        ct.insert("Base".into(), base);
        let mut mid = ClassEntry::new("Mid");
        mid.parent_name = Some("Base".into());
        ct.insert("Mid".into(), mid);
        let mut leaf = ClassEntry::new("Leaf");
        leaf.parent_name = Some("Mid".into());
        ct.insert("Leaf".into(), leaf);

        assert!(is_a(&ct, "Leaf", "Leaf")); // identity
        assert!(is_a(&ct, "Leaf", "Mid")); // one level
        assert!(is_a(&ct, "Leaf", "Base")); // two levels
        assert!(!is_a(&ct, "Base", "Leaf")); // not upwards
        assert!(!is_a(&ct, "Leaf", "Unrelated")); // unknown target
    }

    #[test]
    fn test_is_a_through_implements_and_interface_extends() {
        let mut ct = HashMap::new();
        let mut p = ClassEntry::new(&iface_key("P"));
        p.methods.insert("p_m".into(), method_contract_only("p_m"));
        ct.insert(iface_key("P"), p);
        let mut q = ClassEntry::new(&iface_key("Q"));
        q.methods.insert("q_m".into(), method_contract_only("q_m"));
        q.interfaces = vec![iface_key("P")]; // interface Q extends P
        ct.insert(iface_key("Q"), q);
        let mut c = ClassEntry::new("C");
        c.interfaces = vec![iface_key("Q")];
        ct.insert("C".into(), c);

        assert!(is_a(&ct, "C", &iface_key("Q"))); // direct implements
        assert!(is_a(&ct, "C", &iface_key("P"))); // via interface extends
        assert!(!is_a(&ct, "C", &iface_key("R")));
    }

    #[test]
    fn test_hierarchy_walks_terminate_on_cycles() {
        // A cyclic hierarchy must not hang the resolver.
        let mut ct = HashMap::new();
        let mut a = ClassEntry::new("A");
        a.parent_name = Some("B".into());
        ct.insert("A".into(), a);
        let mut b = ClassEntry::new("B");
        b.parent_name = Some("A".into());
        ct.insert("B".into(), b);

        // Both directions return (no infinite recursion); B has no methods.
        assert!(resolve_method(&ct, "A", "greet").is_none());
        assert!(resolve_default_property(&ct, "A", "x").is_none());
        assert!(resolve_constant(&ct, "A", "K").is_none());
        assert!(resolve_static_property(&ct, "A", "s").is_none());
    }

    #[test]
    fn test_required_methods_exclude_default_bodies() {
        let mut ct = HashMap::new();
        let mut i = ClassEntry::new(&iface_key("I"));
        i.methods
            .insert("must".into(), method_contract_only("must"));
        i.methods
            .insert("optional".into(), method_with_body("optional"));
        ct.insert(iface_key("I"), i);

        let required = interface_required_methods(&ct, &iface_key("I"));
        assert_eq!(required, vec!["must".to_string()]);
    }

    #[test]
    fn test_required_methods_inherit_from_extended_interface() {
        let mut ct = HashMap::new();
        let mut p = ClassEntry::new(&iface_key("P"));
        p.methods.insert("p_m".into(), method_contract_only("p_m"));
        ct.insert(iface_key("P"), p);
        let mut q = ClassEntry::new(&iface_key("Q"));
        q.methods.insert("q_m".into(), method_contract_only("q_m"));
        q.interfaces = vec![iface_key("P")];
        ct.insert(iface_key("Q"), q);

        let mut required = interface_required_methods(&ct, &iface_key("Q"));
        required.sort();
        assert_eq!(required, vec!["p_m".to_string(), "q_m".to_string()]);
    }

    #[test]
    fn test_child_declaration_shadows_inherited_member() {
        let mut ct = HashMap::new();
        let mut base = ClassEntry::new("Base");
        base.methods
            .insert("greet".into(), method_with_body("greet"));
        base.default_properties
            .insert("x".into(), crate::engine::facade::long_val(1));
        ct.insert("Base".into(), base);
        let mut child = ClassEntry::new("Child");
        child.parent_name = Some("Base".into());
        child
            .methods
            .insert("greet".into(), method_with_body("greet"));
        child
            .default_properties
            .insert("x".into(), crate::engine::facade::long_val(2));
        ct.insert("Child".into(), child);

        let m = resolve_method(&ct, "Child", "greet").expect("method");
        assert_eq!(m.file_label, "Child::greet");
        let v = resolve_default_property(&ct, "Child", "x").expect("prop");
        assert_eq!(
            crate::engine::operators::zval_get_long(&v),
            2,
            "child declaration must win"
        );
    }
}
