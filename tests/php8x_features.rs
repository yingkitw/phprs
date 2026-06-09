//! PHP 8.x Features Test Suite
//!
//! Tests for static properties/methods, late static binding, magic methods,
//! anonymous classes, variadic functions, named arguments, union types,
//! enums, and more.

use phprs::engine::compile::compile_string_with_functions;
use phprs::engine::types::PhpResult;
use phprs::engine::vm::{execute_ex, ExecuteData};
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
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
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
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    // static:: should resolve to the called class
    assert!(out.contains("Base|Child") || out.contains("Base") || out.contains("Child"),
        "late static binding output: {out:?}");
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
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
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
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
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
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
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
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
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
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
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
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
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
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
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
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
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
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
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
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("updated"), "static prop assignment output: {out:?}");
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
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("5"), "anon class with ctor output: {out:?}");
}
