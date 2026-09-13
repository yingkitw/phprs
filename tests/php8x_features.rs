//! PHP 8.x Features Test Suite
//!
//! Tests for static properties/methods, late static binding, magic methods,
//! anonymous classes, variadic functions, named arguments, union types,
//! enums, and more.

use phprs::engine::compile::compile_string_with_functions;
use phprs::engine::types::PhpResult;
use phprs::engine::vm::{ExecuteData, execute_ex};
use std::sync::Arc;

/// Compile and run a PHP code string, returning (result, output)
fn run_php(code: &str) -> Result<(PhpResult, String), String> {
    let (op_array, ft) = compile_string_with_functions(code, "test.php")?;
    phprs::php::output::php_output_start().map_err(|e| e.to_string())?;
    let mut ed = ExecuteData::new();
    ed.function_table = Some(Arc::new(ft));
    let result = execute_ex(&mut ed, &op_array);
    let output = phprs::php::output::php_output_end().map_err(|e| e.to_string())?;
    Ok((result, output))
}

#[test]
fn test_static_properties_and_methods() {
    let code = r#"<?php
class Counter {
    public static $count = 0;
    public static function increment() {
        self::$count = self::$count + 1;
    }
}
Counter::increment();
Counter::increment();
echo Counter::$count;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("2"), "expected count=2, got: {out:?}");
}

#[test]
fn test_late_static_binding() {
    let code = r#"<?php
class Base {
    public static function getClass() {
        return static::class;
    }
}
class Child extends Base {
}
echo Base::getClass();
echo "|";
echo Child::getClass();
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    // static:: should resolve to the called class
    assert!(
        out.contains("Base|Child") || out.contains("Base") || out.contains("Child"),
        "late static binding output: {out:?}"
    );
}

#[test]
fn test_magic_methods_get_set() {
    let code = r#"<?php
class MagicBox {
    public function __get($name) {
        return "got_" . $name;
    }
    public function __set($name, $value) {
        echo "set_" . $name . "=" . $value;
    }
}
$box = new MagicBox();
$box->foo = "bar";
echo "|";
echo $box->baz;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("set_foo=bar"), "__set output: {out:?}");
    assert!(out.contains("got_baz"), "__get output: {out:?}");
}

#[test]
fn test_magic_methods_call() {
    let code = r#"<?php
class Dynamic {
    public function __call($name, $args) {
        return $name . "(" . count($args) . ")";
    }
}
$obj = new Dynamic();
echo $obj->doSomething(1, 2, 3);
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("doSomething(3)"), "__call output: {out:?}");
}

#[test]
fn test_magic_methods_isset() {
    let code = r#"<?php
class IssetBox {
    public function __isset($name) {
        $result = $name == "allowed";
        return $result;
    }
    public function __get($name) {
        return "got_" . $name;
    }
}
$box = new IssetBox();
echo isset($box->allowed) ? "yes" : "no";
echo "|";
echo isset($box->blocked) ? "yes" : "no";
echo "|";
echo $box->allowed;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("yes|no|got_allowed"), "__isset magic: {out:?}");
}

#[test]
fn test_anonymous_class() {
    let code = r#"<?php
$obj = new class {
    public function greet() {
        return "hello";
    }
};
echo $obj->greet();
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("hello"), "anonymous class output: {out:?}");
}

#[test]
fn test_variadic_function() {
    let code = r#"<?php
function sum(...$numbers) {
    $total = 0;
    foreach ($numbers as $n) {
        $total = $total + $n;
    }
    return $total;
}
echo sum(1, 2, 3, 4);
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("10"), "variadic output: {out:?}");
}

#[test]
fn test_named_arguments() {
    let code = r#"<?php
function greet($name, $greeting = "Hello") {
    return $greeting . " " . $name;
}
echo greet(greeting: "Hi", name: "World");
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("Hi World"), "named args output: {out:?}");
}

#[test]
fn test_union_type_parsing() {
    let code = r#"<?php
function acceptsIntOrString(int|string $value): void {
    echo $value;
}
acceptsIntOrString(42);
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("42"), "union type output: {out:?}");
}

#[test]
fn test_enum_pure() {
    let code = r#"<?php
enum Status {
    case Pending;
    case Active;
}
echo Status::Pending;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("Pending"), "enum output: {out:?}");
}

#[test]
fn test_enum_backed() {
    let code = r#"<?php
enum Color: string {
    case Red = 'red';
    case Green = 'green';
}
echo Color::Red;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("red"), "backed enum output: {out:?}");
}

#[test]
fn test_static_property_assignment() {
    let code = r#"<?php
class Config {
    public static $value = "initial";
}
Config::$value = "updated";
echo Config::$value;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(
        out.contains("updated"),
        "static prop assignment output: {out:?}"
    );
}

#[test]
fn test_anonymous_class_with_constructor() {
    let code = r#"<?php
$obj = new class(5) {
    public $value;
    public function __construct($v) {
        $this->value = $v;
    }
};
echo $obj->value;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("5"), "anon class with ctor output: {out:?}");
}

#[test]
fn test_first_class_callable_syntax() {
    let code = r#"<?php
$strlen = strlen(...);
echo $strlen("hello");

class MathHelper {
    public static function double($n) {
        return $n * 2;
    }
    public function inc($n) {
        return $n + 1;
    }
}
$d = MathHelper::double(...);
echo $d(4);
$obj = new MathHelper();
$inc = $obj->inc(...);
echo $inc(9);
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("5"), "strlen FCC output: {out:?}");
    assert!(out.contains("8"), "static FCC output: {out:?}");
    assert!(out.contains("10"), "method FCC output: {out:?}");
}

#[test]
fn test_logical_and_combined_comparisons() {
    let code = r#"<?php
$a = 'Home';
$b = 'index';
if ($a == 'Home' && $b == 'index') {
    echo "ok\n";
} else {
    echo "fail\n";
}
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("ok"), "combined && output: {out:?}");
}

#[test]
fn test_final_class_constant_access() {
    // A final constant declared and accessed on its own class works normally.
    let code = r#"<?php
class Config {
    final public const VERSION = '1.0.0';
}
echo Config::VERSION;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("1.0.0"), "final const access output: {out:?}");
}

#[test]
fn test_final_class_constant_after_visibility() {
    // `public final const X` (final after visibility) is accepted.
    let code = r#"<?php
class Config {
    public final const VERSION = '2.0.0';
}
echo Config::VERSION;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(
        out.contains("2.0.0"),
        "final-after-visibility output: {out:?}"
    );
}

#[test]
fn test_final_class_constant_override_rejected() {
    // A child class redeclaring a parent's final constant is a compile error.
    let code = r#"<?php
class Base {
    final public const VERSION = '1.0.0';
}
class Child extends Base {
    public const VERSION = '9.9.9';
}
"#;
    let err = run_php(code).expect_err("expected compile error");
    assert!(
        err.contains("Cannot override final constant Base::VERSION"),
        "expected final-override error, got: {err}"
    );
}

#[test]
fn test_non_final_constant_override_allowed() {
    // A non-final constant may be overridden by a child class.
    let code = r#"<?php
class Base {
    public const VERSION = '1.0.0';
}
class Child extends Base {
    public const VERSION = '9.9.9';
}
echo Child::VERSION;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("9.9.9"), "non-final override output: {out:?}");
}

#[test]
fn test_final_constant_inherited_not_overridden() {
    // A child that does NOT redeclare a parent's final constant compiles fine.
    let code = r#"<?php
class Base {
    final public const VERSION = '1.0.0';
}
class Child extends Base {
    public const OTHER = 'x';
}
echo Base::VERSION;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(
        out.contains("1.0.0"),
        "inherited-not-overridden output: {out:?}"
    );
}

#[test]
fn test_magic_unset_removes_existing_property() {
    let code = r#"<?php
class Foo {
    public $bar = 42;
}
$obj = new Foo();
echo isset($obj->bar) ? "before:set" : "before:unset";
echo "\n";
unset($obj->bar);
echo isset($obj->bar) ? "after:set" : "after:unset";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("before:set"), "output: {out:?}");
    assert!(out.contains("after:unset"), "output: {out:?}");
}

#[test]
fn test_magic_unset_magic_method_invoked() {
    let code = r#"<?php
class Magic {
    public function __unset($name) {
        echo "__unset:{$name}";
    }
}
$obj = new Magic();
unset($obj->virtual);
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(
        out.contains("__unset:virtual"),
        "__unset magic method output: {out:?}"
    );
}

#[test]
fn test_magic_unset_existing_prop_no_magic() {
    // If the property exists, __unset is NOT called — the property is removed directly.
    let code = r#"<?php
class Magic {
    public $real = "yes";
    public function __unset($name) {
        echo "should-not-appear";
    }
}
$obj = new Magic();
unset($obj->real);
echo isset($obj->real) ? "still" : "gone";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("gone"), "output: {out:?}");
    assert!(!out.contains("should-not-appear"), "output: {out:?}");
}

#[test]
fn test_inherited_class_constant_access() {
    let code = r#"<?php
class Base {
    const VERSION = "1.0.0";
}
class Child extends Base {}
echo Child::VERSION;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("1.0.0"), "inherited constant output: {out:?}");
}

#[test]
fn test_multilevel_inherited_class_constant() {
    let code = r#"<?php
class GrandBase {
    const GREETING = "hello";
}
class Mid extends GrandBase {}
class Leaf extends Mid {}
echo Leaf::GREETING;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(
        out.contains("hello"),
        "multilevel inherited constant output: {out:?}"
    );
}

#[test]
fn test_child_override_of_inherited_constant() {
    let code = r#"<?php
class Base {
    const VERSION = "1.0.0";
}
class Child extends Base {
    const VERSION = "2.0.0";
}
echo Base::VERSION;
echo "\n";
echo Child::VERSION;
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("1.0.0"), "base constant output: {out:?}");
    assert!(out.contains("2.0.0"), "overridden constant output: {out:?}");
}

#[test]
fn test_unset_variable_removes_from_scope() {
    let code = r#"<?php
$x = 5;
echo isset($x) ? "set" : "unset";
echo "\n";
unset($x);
echo isset($x) ? "set" : "unset";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("set\nunset"), "output: {out:?}");
}

#[test]
fn test_unset_multiple_variables() {
    let code = r#"<?php
$a = 1;
$b = 2;
$c = 3;
unset($a, $b);
echo isset($a) ? "a:set" : "a:unset";
echo "\n";
echo isset($b) ? "b:set" : "b:unset";
echo "\n";
echo isset($c) ? "c:set" : "c:unset";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("a:unset"), "output: {out:?}");
    assert!(out.contains("b:unset"), "output: {out:?}");
    assert!(out.contains("c:set"), "output: {out:?}");
}

#[test]
fn test_unset_undefined_variable_no_error() {
    let code = r#"<?php
unset($undefined);
echo "ok";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("ok"), "output: {out:?}");
}

#[test]
fn test_unset_array_element_string_key() {
    let code = r#"<?php
$arr = ["a" => 1, "b" => 2, "c" => 3];
unset($arr["b"]);
echo isset($arr["a"]) ? "a:set" : "a:unset";
echo "\n";
echo isset($arr["b"]) ? "b:set" : "b:unset";
echo "\n";
echo isset($arr["c"]) ? "c:set" : "c:unset";
echo "\n";
echo count($arr);
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("a:set"), "output: {out:?}");
    assert!(out.contains("b:unset"), "output: {out:?}");
    assert!(out.contains("c:set"), "output: {out:?}");
    assert!(out.contains("2"), "count output: {out:?}");
}

#[test]
fn test_unset_array_element_int_key() {
    let code = r#"<?php
$arr = [10, 20, 30];
unset($arr[1]);
echo isset($arr[0]) ? "0:set" : "0:unset";
echo "\n";
echo isset($arr[1]) ? "1:set" : "1:unset";
echo "\n";
echo isset($arr[2]) ? "2:set" : "2:unset";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("0:set"), "output: {out:?}");
    assert!(out.contains("1:unset"), "output: {out:?}");
    assert!(out.contains("2:set"), "output: {out:?}");
}

#[test]
fn test_unset_array_element_variable_key() {
    let code = r#"<?php
$arr = ["x" => 100, "y" => 200];
$key = "x";
unset($arr[$key]);
echo isset($arr["x"]) ? "x:set" : "x:unset";
echo "\n";
echo isset($arr["y"]) ? "y:set" : "y:unset";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("x:unset"), "output: {out:?}");
    assert!(out.contains("y:set"), "output: {out:?}");
}

#[test]
fn test_unset_chained_subscript_two_levels() {
    let code = r#"<?php
$arr = ["x" => ["a" => 1, "b" => 2, "c" => 3]];
unset($arr["x"]["b"]);
echo isset($arr["x"]["a"]) ? "a:set" : "a:unset";
echo "\n";
echo isset($arr["x"]["b"]) ? "b:set" : "b:unset";
echo "\n";
echo isset($arr["x"]["c"]) ? "c:set" : "c:unset";
echo "\n";
echo count($arr["x"]);
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("a:set"), "output: {out:?}");
    assert!(out.contains("b:unset"), "output: {out:?}");
    assert!(out.contains("c:set"), "output: {out:?}");
    assert!(out.contains("2"), "count output: {out:?}");
}

#[test]
fn test_unset_chained_subscript_three_levels() {
    let code = r#"<?php
$arr = ["p" => ["h" => ["k1" => 1, "k2" => 2]]];
unset($arr["p"]["h"]["k1"]);
echo isset($arr["p"]["h"]["k1"]) ? "k1:set" : "k1:unset";
echo "\n";
echo isset($arr["p"]["h"]["k2"]) ? "k2:set" : "k2:unset";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("k1:unset"), "output: {out:?}");
    assert!(out.contains("k2:set"), "output: {out:?}");
}

#[test]
fn test_datetime_constructor_and_format() {
    let code = r#"<?php
$d = new DateTime("2024-01-15 10:30:00");
echo $d->format("Y-m-d H:i:s");
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("2024-01-15 10:30:00"), "output: {out:?}");
}

#[test]
fn test_datetime_create_from_format() {
    let code = r#"<?php
$d = DateTime::createFromFormat("Y-m-d", "2025-03-20");
echo $d->format("Y-m-d");
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("2025-03-20"), "output: {out:?}");
}

#[test]
fn test_datetime_create_from_format_custom() {
    let code = r#"<?php
$d = DateTime::createFromFormat("d/m/Y H:i:s", "15/06/2024 14:25:30");
echo $d->format("Y-m-d H:i:s");
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("2024-06-15 14:25:30"), "output: {out:?}");
}

#[test]
fn test_datetime_get_timestamp() {
    let code = r#"<?php
$d = new DateTime("1970-01-01 00:00:00");
echo $d->getTimestamp();
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("0"), "output: {out:?}");
}

#[test]
fn test_datetime_diff() {
    let code = r#"<?php
$d1 = new DateTime("2024-01-01 00:00:00");
$d2 = new DateTime("2024-03-15 12:30:45");
$diff = $d1->diff($d2);
echo "y=" . $diff->y . " m=" . $diff->m . " d=" . $diff->d . "\n";
echo "h=" . $diff->h . " i=" . $diff->i . " s=" . $diff->s . "\n";
echo "days=" . $diff->days . " invert=" . $diff->invert . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("y=0 m=2 d=14"), "output: {out:?}");
    assert!(out.contains("h=12 i=30 s=45"), "output: {out:?}");
    assert!(out.contains("days=74 invert=0"), "output: {out:?}");
}

#[test]
fn test_datetime_diff_inverted() {
    let code = r#"<?php
$d1 = new DateTime("2024-03-15 12:30:45");
$d2 = new DateTime("2024-01-01 00:00:00");
$diff = $d1->diff($d2);
echo "invert=" . $diff->invert . " days=" . $diff->days . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("invert=1 days=74"), "output: {out:?}");
}

#[test]
fn test_datetime_diff_same_date() {
    let code = r#"<?php
$d1 = new DateTime("2024-06-15 10:00:00");
$d2 = new DateTime("2024-06-15 14:30:00");
$diff = $d1->diff($d2);
echo "d=" . $diff->d . " h=" . $diff->h . " i=" . $diff->i . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("d=0 h=4 i=30"), "output: {out:?}");
}

#[test]
fn test_serialize_magic_method() {
    let code = r#"<?php
class Money {
    public $amount;
    public $currency;
    public function __construct($amount, $currency) {
        $this->amount = $amount;
        $this->currency = $currency;
    }
    public function __serialize() {
        return ["a" => $this->amount, "c" => $this->currency];
    }
}
$m = new Money(100, "USD");
$s = serialize($m);
echo $s . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    // __serialize returns custom keys "a" and "c" instead of "amount"/"currency"
    assert!(out.contains("s:1:\"a\";i:100;"), "output: {out:?}");
    assert!(out.contains("s:1:\"c\";s:3:\"USD\";"), "output: {out:?}");
}

#[test]
fn test_unserialize_magic_method() {
    let code = r#"<?php
class Money {
    public $amount;
    public $currency;
    public function __construct($amount, $currency) {
        $this->amount = $amount;
        $this->currency = $currency;
    }
    public function __serialize() {
        return ["a" => $this->amount, "c" => $this->currency];
    }
    public function __unserialize($data) {
        $this->amount = $data["a"];
        $this->currency = $data["c"];
    }
}
$m = new Money(100, "USD");
$s = serialize($m);
$m2 = unserialize($s);
echo $m2->amount . " " . $m2->currency . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("100 USD"), "output: {out:?}");
}

#[test]
fn test_serialize_plain_object_no_magic() {
    // Objects without __serialize should use the default property-based serialization.
    let code = r#"<?php
class Point {
    public $x;
    public $y;
    public function __construct($x, $y) {
        $this->x = $x;
        $this->y = $y;
    }
}
$p = new Point(3, 4);
echo serialize($p) . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("O:5:\"Point\":2:"), "output: {out:?}");
    assert!(out.contains("i:3;"), "output: {out:?}");
    assert!(out.contains("i:4;"), "output: {out:?}");
}

#[test]
fn test_array_iterator_iteration() {
    let code = r#"<?php
$it = new ArrayIterator(["a" => 1, "b" => 2, "c" => 3]);
echo "count: " . $it->count() . "\n";
$it->rewind();
while ($it->valid()) {
    echo $it->key() . " => " . $it->current() . "\n";
    $it->next();
}
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("count: 3"), "output: {out:?}");
    assert!(out.contains("a => 1"), "output: {out:?}");
    assert!(out.contains("b => 2"), "output: {out:?}");
    assert!(out.contains("c => 3"), "output: {out:?}");
}

#[test]
fn test_array_iterator_array_access() {
    let code = r#"<?php
$it = new ArrayIterator(["a" => 1, "b" => 2]);
echo $it->offsetGet("a") . "\n";
echo ($it->offsetExists("b") ? "yes" : "no") . "\n";
echo ($it->offsetExists("z") ? "yes" : "no") . "\n";
$it->offsetSet("c", 3);
echo $it->offsetGet("c") . "\n";
$it->offsetUnset("c");
echo ($it->offsetExists("c") ? "yes" : "no") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("1"), "output: {out:?}");
    assert!(out.contains("yes"), "output: {out:?}");
    assert!(out.contains("no"), "output: {out:?}");
    assert!(out.contains("3"), "output: {out:?}");
}

#[test]
fn test_array_iterator_empty() {
    let code = r#"<?php
$it = new ArrayIterator();
echo "count: " . $it->count() . "\n";
echo ($it->valid() ? "valid" : "invalid") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("count: 0"), "output: {out:?}");
    assert!(out.contains("invalid"), "output: {out:?}");
}

#[test]
fn test_datetime_timezone_constructor() {
    let code = r#"<?php
$d = new DateTime("2024-01-15 10:30:00", new DateTimeZone("America/New_York"));
echo $d->format("Y-m-d H:i:s P") . "\n";
echo $d->format("Y-m-d H:i:s T") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    // Timestamp is 2024-01-15 10:30:00 UTC; NY is -5, so 05:30:00
    assert!(out.contains("2024-01-15 05:30:00"), "output: {out:?}");
    assert!(out.contains("-05:00"), "output: {out:?}");
    assert!(out.contains("America/New_York"), "output: {out:?}");
}

#[test]
fn test_datetime_get_timezone() {
    let code = r#"<?php
$d = new DateTime("2024-01-15 10:30:00", new DateTimeZone("Asia/Tokyo"));
$tz = $d->getTimezone();
echo $tz->getName() . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("Asia/Tokyo"), "output: {out:?}");
}

#[test]
fn test_datetime_set_timezone() {
    let code = r#"<?php
$d = new DateTime("2024-01-15 10:30:00");
echo "default: " . $d->format("Y-m-d H:i:s T") . "\n";
$d->setTimezone(new DateTimeZone("Asia/Tokyo"));
echo "tokyo: " . $d->format("Y-m-d H:i:s T") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("default: 2024-01-15 10:30:00 UTC"), "output: {out:?}");
    // Tokyo is +9, so 10:30 + 9 = 19:30
    assert!(out.contains("tokyo: 2024-01-15 19:30:00 Asia/Tokyo"), "output: {out:?}");
}

#[test]
fn test_datetimezone_numeric_offset() {
    let code = r#"<?php
$tz = new DateTimeZone("+0530");
echo $tz->getName() . "\n";
$d = new DateTime("2024-01-15 10:30:00", $tz);
echo $d->format("Y-m-d H:i:s O") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("+0530"), "output: {out:?}");
    assert!(out.contains("+0530"), "output: {out:?}");
}

#[test]
fn test_fiber_basic_start_suspend() {
    let code = r#"<?php
$fiber = new Fiber(function() {
    echo "fiber started\n";
    Fiber::suspend("suspended value");
    echo "fiber resumed\n";
});
echo "before start\n";
$result = $fiber->start();
echo "start returned: " . $result . "\n";
echo "isSuspended: " . ($fiber->isSuspended() ? "yes" : "no") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("before start"), "output: {out:?}");
    assert!(out.contains("fiber started"), "output: {out:?}");
    assert!(out.contains("start returned: suspended value"), "output: {out:?}");
    assert!(out.contains("isSuspended: yes"), "output: {out:?}");
}

#[test]
fn test_fiber_resume_with_value() {
    let code = r#"<?php
$fiber = new Fiber(function() {
    $value = Fiber::suspend("first");
    echo "resumed with: " . $value . "\n";
    return "done";
});
$fiber->start();
$result = $fiber->resume("resume value");
echo "resume returned: " . $result . "\n";
echo "isTerminated: " . ($fiber->isTerminated() ? "yes" : "no") . "\n";
echo "getReturn: " . $fiber->getReturn() . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("resumed with: resume value"), "output: {out:?}");
    assert!(out.contains("isTerminated: yes"), "output: {out:?}");
    assert!(out.contains("getReturn: done"), "output: {out:?}");
}

#[test]
fn test_fiber_multiple_suspends() {
    let code = r#"<?php
$fiber = new Fiber(function() {
    $a = Fiber::suspend("first");
    $b = Fiber::suspend("second");
    return $a . $b;
});
$r1 = $fiber->start();
$r2 = $fiber->resume("A");
$r3 = $fiber->resume("B");
echo "r1=$r1 r2=$r2 r3=$r3\n";
echo "getReturn: " . $fiber->getReturn() . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("r1=first r2=second r3="), "output: {out:?}");
    assert!(out.contains("getReturn: AB"), "output: {out:?}");
}

#[test]
fn test_fiber_state_queries() {
    let code = r#"<?php
$fiber = new Fiber(function() {
    Fiber::suspend("x");
    return "done";
});
echo "isStarted: " . ($fiber->isStarted() ? "yes" : "no") . "\n";
echo "isTerminated: " . ($fiber->isTerminated() ? "yes" : "no") . "\n";
$fiber->start();
echo "isStarted: " . ($fiber->isStarted() ? "yes" : "no") . "\n";
echo "isSuspended: " . ($fiber->isSuspended() ? "yes" : "no") . "\n";
$fiber->resume();
echo "isTerminated: " . ($fiber->isTerminated() ? "yes" : "no") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert!(out.contains("isStarted: no"), "before start: {out:?}");
    assert!(out.contains("isTerminated: no"), "before start: {out:?}");
    assert!(out.contains("isStarted: yes"), "after start: {out:?}");
    assert!(out.contains("isSuspended: yes"), "after start: {out:?}");
    assert!(out.contains("isTerminated: yes"), "after resume: {out:?}");
}

#[test]
fn test_dst_new_york_summer() {
    let code = r#"<?php
$d = new DateTime("2024-07-15 12:00:00", new DateTimeZone("America/New_York"));
echo $d->format("Y-m-d H:i:s O") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    // July = EDT (-0400)
    assert!(out.contains("-0400"), "expected EDT in summer: {out:?}");
}

#[test]
fn test_dst_new_york_winter() {
    let code = r#"<?php
$d = new DateTime("2024-01-15 12:00:00", new DateTimeZone("America/New_York"));
echo $d->format("Y-m-d H:i:s O") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    // January = EST (-0500)
    assert!(out.contains("-0500"), "expected EST in winter: {out:?}");
}

#[test]
fn test_dst_london_summer() {
    let code = r#"<?php
$d = new DateTime("2024-07-15 12:00:00", new DateTimeZone("Europe/London"));
echo $d->format("Y-m-d H:i:s O") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    // July = BST (+0100)
    assert!(out.contains("+0100"), "expected BST in summer: {out:?}");
}

#[test]
fn test_dst_london_winter() {
    let code = r#"<?php
$d = new DateTime("2024-01-15 12:00:00", new DateTimeZone("Europe/London"));
echo $d->format("Y-m-d H:i:s O") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    // January = GMT (+0000)
    assert!(out.contains("+0000"), "expected GMT in winter: {out:?}");
}

#[test]
fn test_dst_tokyo_no_dst() {
    let code = r#"<?php
$summer = new DateTime("2024-07-15 12:00:00", new DateTimeZone("Asia/Tokyo"));
$winter = new DateTime("2024-01-15 12:00:00", new DateTimeZone("Asia/Tokyo"));
echo $summer->format("O") . "\n";
echo $winter->format("O") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    // Tokyo has no DST — both should be +0900
    assert!(out.contains("+0900"), "expected +0900 in summer: {out:?}");
}

#[test]
fn test_splstack_lifo() {
    let code = r#"<?php
$stack = new SplStack();
$stack->push("a");
$stack->push("b");
$stack->push("c");
echo $stack->count() . "\n";
echo $stack->top() . "\n";
echo $stack->pop() . "\n";
echo $stack->pop() . "\n";
echo $stack->count() . "\n";
echo $stack->pop() . "\n";
echo ($stack->isEmpty() ? "yes" : "no") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("3\n"), "count after 3 pushes: {out:?}");
    assert!(out.contains("c\n"), "top should be c: {out:?}");
    assert!(out.contains("c\nb\n"), "pop c then b: {out:?}");
    assert!(out.contains("1\n"), "count after 2 pops: {out:?}");
    assert!(out.contains("a\n"), "pop a: {out:?}");
    assert!(out.contains("yes\n"), "isEmpty after all pops: {out:?}");
}

#[test]
fn test_splqueue_fifo() {
    let code = r#"<?php
$queue = new SplQueue();
$queue->enqueue("first");
$queue->enqueue("second");
$queue->enqueue("third");
echo $queue->count() . "\n";
echo $queue->bottom() . "\n";
echo $queue->dequeue() . "\n";
echo $queue->dequeue() . "\n";
echo $queue->count() . "\n";
echo $queue->dequeue() . "\n";
echo ($queue->isEmpty() ? "yes" : "no") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("3\n"), "count after 3 enqueues: {out:?}");
    assert!(out.contains("first\n"), "bottom should be first: {out:?}");
    assert!(out.contains("first\nsecond\n"), "dequeue first then second: {out:?}");
    assert!(out.contains("1\n"), "count after 2 dequeues: {out:?}");
    assert!(out.contains("third\n"), "dequeue third: {out:?}");
    assert!(out.contains("yes\n"), "isEmpty after all dequeues: {out:?}");
}

#[test]
fn test_directory_iterator_basic() {
    let code = r#"<?php
$dir = new DirectoryIterator(".");
$files = [];
$dir->rewind();
while ($dir->valid()) {
    $files[] = $dir->getFilename();
    $dir->next();
}
// Should contain Cargo.toml and src
$has_cargo = in_array("Cargo.toml", $files);
$has_src = in_array("src", $files);
echo ($has_cargo ? "yes" : "no") . "\n";
echo ($has_src ? "yes" : "no") . "\n";
echo count($files) > 5 ? "many" : "few";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("yes"), "should find Cargo.toml: {out:?}");
    assert!(out.contains("many"), "should have many files: {out:?}");
}

#[test]
fn test_directory_iterator_key_path() {
    let code = r#"<?php
$dir = new DirectoryIterator(".");
$dir->rewind();
$first_key = $dir->key();
$first_path = $dir->getPathname();
echo $first_key . "\n";
echo substr($first_path, 0, 2);
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("0\n"), "first key should be 0: {out:?}");
    assert!(out.contains("./") || out.contains(".\\"), "path should start with ./ or .\\: {out:?}");
}

#[test]
fn test_splmaxheap() {
    let code = r#"<?php
$heap = new SplMaxHeap();
$heap->insert(3);
$heap->insert(7);
$heap->insert(1);
$heap->insert(5);
echo $heap->count() . "\n";
echo $heap->top() . "\n";
echo $heap->extract() . "\n";
echo $heap->extract() . "\n";
echo $heap->extract() . "\n";
echo $heap->extract() . "\n";
echo ($heap->isEmpty() ? "yes" : "no") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("4\n"), "count: {out:?}");
    assert!(out.contains("7\n7\n5\n3\n1\n"), "extract in descending order: {out:?}");
    assert!(out.contains("yes\n"), "isEmpty: {out:?}");
}

#[test]
fn test_splminheap() {
    let code = r#"<?php
$heap = new SplMinHeap();
$heap->insert(3);
$heap->insert(7);
$heap->insert(1);
$heap->insert(5);
echo $heap->top() . "\n";
echo $heap->extract() . "\n";
echo $heap->extract() . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("1\n1\n3\n"), "min-heap extracts in ascending order: {out:?}");
}

#[test]
fn test_splpriorityqueue() {
    let code = r#"<?php
$pq = new SplPriorityQueue();
$pq->insert("low", 1);
$pq->insert("high", 10);
$pq->insert("medium", 5);
echo $pq->extract() . "\n";
echo $pq->extract() . "\n";
echo $pq->extract() . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("high\nmedium\nlow\n"), "priority queue extracts by priority: {out:?}");
}

#[test]
fn test_splfileinfo() {
    let code = r#"<?php
$info = new SplFileInfo("Cargo.toml");
echo $info->getFilename() . "\n";
echo $info->getPathname() . "\n";
echo $info->getSize() > 0 ? "yes" : "no";
echo "\n";
echo ($info->isFile() ? "yes" : "no") . "\n";
echo ($info->isDir() ? "yes" : "no") . "\n";
echo ($info->isReadable() ? "yes" : "no") . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("Cargo.toml\n"), "filename: {out:?}");
    assert!(out.contains("yes\n"), "size > 0: {out:?}");
    assert!(out.contains("yes\nno\nyes\n"), "isFile/isDir/isReadable: {out:?}");
}

#[test]
fn test_splfileobject_read() {
    let code = r#"<?php
$file = new SplFileObject("Cargo.toml");
$line1 = $file->fgets();
$line2 = $file->fgets();
echo trim($line1) . "\n";
echo trim($line2) . "\n";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("[package]"), "first line: {out:?}");
    assert!(out.contains("name = "), "second line: {out:?}");
}

#[test]
fn test_splfileobject_feof() {
    let code = r#"<?php
$file = new SplFileObject("Cargo.toml");
$count = 0;
while (!$file->feof()) {
    $file->fgets();
    $count++;
}
echo $count > 10 ? "many" : "few";
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("many"), "should read many lines: {out:?}");
}

#[test]
fn test_spl_exception_hierarchy() {
    let code = r#"<?php
try {
    throw new RuntimeException("runtime error");
} catch (Exception $e) {
    echo "caught as Exception: " . $e->getMessage() . "\n";
}
try {
    throw new UnexpectedValueException("bad value");
} catch (RuntimeException $e) {
    echo "caught as RuntimeException: " . $e->getMessage() . "\n";
}
try {
    throw new InvalidArgumentException("bad arg");
} catch (LogicException $e) {
    echo "caught as LogicException: " . $e->getMessage() . "\n";
}
try {
    throw new OverflowException("overflow");
} catch (RuntimeException $e) {
    echo "caught as RuntimeException: " . $e->getMessage() . "\n";
}
"#;
    let (r, out) = run_php(code).expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("caught as Exception: runtime error"), "RuntimeException -> Exception: {out:?}");
    assert!(out.contains("caught as RuntimeException: bad value"), "UnexpectedValueException -> RuntimeException: {out:?}");
    assert!(out.contains("caught as LogicException: bad arg"), "InvalidArgumentException -> LogicException: {out:?}");
    assert!(out.contains("caught as RuntimeException: overflow"), "OverflowException -> RuntimeException: {out:?}");
}
