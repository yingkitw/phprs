//! Interface (`implements` / `interface`) and class-hierarchy test suite
//!
//! `class C implements I {}` compiles the interface list onto the class entry
//! and verifies the contract at compile time: every **bodyless** interface
//! method must be provided by the class or one of its ancestors. An interface
//! method that *has* a body supplies a default implementation the class may
//! inherit instead. Method, property, constant and static-property resolution
//! walk the parent chain and the interface list (see
//! `src/engine/vm/class_hierarchy.rs`), so `extends` inheritance works too.

use phprs::engine::compile::compile_string_with_functions;
use phprs::engine::types::PhpResult;
use phprs::engine::vm::{ExecuteData, execute_ex};
use std::sync::Arc;

/// Compile and run a PHP code string, returning (result, output)
fn run_php(code: &str) -> Result<(PhpResult, String), String> {
    let (op_array, ft) = compile_string_with_functions(code, "interface_test.php")?;
    phprs::php::output::php_output_start().map_err(|e| e.to_string())?;
    let mut ed = ExecuteData::new();
    ed.function_table = Some(Arc::new(ft));
    let result = execute_ex(&mut ed, &op_array);
    let output = phprs::php::output::php_output_end().map_err(|e| e.to_string())?;
    Ok((result, output))
}

fn assert_success(r: &PhpResult, out: &str) {
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
}

// ---------------------------------------------------------------------------
// interface declaration + implements
// ---------------------------------------------------------------------------

#[test]
fn test_class_implements_single_interface() {
    let (r, out) = run_php(
        r#"<?php
interface Greeter {
    public function greet($name);
}
class Hello implements Greeter {
    public function greet($name) { return "Hello, $name!"; }
}
$h = new Hello();
echo $h->greet("World");
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "Hello, World!");
}

#[test]
fn test_class_implements_multiple_interfaces() {
    let (r, out) = run_php(
        r#"<?php
interface A { public function a(); }
interface B { public function b(); }
class C implements A, B {
    public function a() { return "a"; }
    public function b() { return "b"; }
}
$c = new C();
echo $c->a(), $c->b();
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "ab");
}

#[test]
fn test_interface_method_with_default_body_is_inherited() {
    // An interface method that HAS a body provides a default implementation.
    // The implementing class need not (and here does not) redeclare it.
    let (r, out) = run_php(
        r#"<?php
interface D { public function d() { return "default-d"; } }
class E implements D { }
$e = new E();
echo $e->d();
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "default-d");
}

#[test]
fn test_interface_default_body_can_be_overridden() {
    let (r, out) = run_php(
        r#"<?php
interface D { public function d() { return "default-d"; } }
class E implements D { public function d() { return "override-d"; } }
$e = new E();
echo $e->d();
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "override-d");
}

#[test]
fn test_interface_constant_accessible_via_implementing_class() {
    let (r, out) = run_php(
        r#"<?php
interface F { const VERSION = "1.0"; }
class G implements F { }
var_dump(G::VERSION);
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert!(out.contains("string(3) \"1.0\""), "output: {out:?}");
}

#[test]
fn test_interface_extends_interface_contract_inherited() {
    // A class implementing Q must satisfy P's contract too, because
    // `interface Q extends P`.
    let (r, out) = run_php(
        r#"<?php
interface P { public function p(); }
interface Q extends P { public function q(); }
class R implements Q {
    public function p() { return "p"; }
    public function q() { return "q"; }
}
$r = new R();
echo $r->p(), $r->q();
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "pq");
}

// ---------------------------------------------------------------------------
// contract enforcement (compile-time errors)
// ---------------------------------------------------------------------------

#[test]
fn test_missing_interface_method_is_compile_error() {
    let err = run_php(
        r#"<?php
interface Greeter { public function greet($name); }
class Hello implements Greeter { }
"#,
    )
    .expect_err("missing greet() must fail to compile");
    assert!(
        err.contains("Greeter::greet"),
        "error should name the missing method: {err:?}"
    );
}

#[test]
fn test_satisfying_contract_via_parent_class_is_accepted() {
    // The parent provides greet(); the child completes the contract.
    let (r, out) = run_php(
        r#"<?php
interface Greeter { public function greet(); }
class Base { public function greet() { return "base"; } }
class Child extends Base implements Greeter { }
$c = new Child();
echo $c->greet();
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "base");
}

// ---------------------------------------------------------------------------
// extends + implements combined
// ---------------------------------------------------------------------------

#[test]
fn test_extends_and_implements_together() {
    let (r, out) = run_php(
        r#"<?php
class Base { public function hello() { return "base-hello"; } }
interface I2 { public function i2(); }
class H extends Base implements I2 {
    public function i2() { return "i2"; }
}
$h = new H();
echo $h->hello(), $h->i2();
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "base-helloi2");
}

#[test]
fn test_interface_with_static_method() {
    let (r, out) = run_php(
        r#"<?php
interface Factory { public static function create(); }
class WidgetFactory implements Factory {
    public static function create() { return "widget"; }
}
echo WidgetFactory::create();
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "widget");
}

// ---------------------------------------------------------------------------
// class hierarchy resolution (extends)
// ---------------------------------------------------------------------------

#[test]
fn test_inherited_instance_method() {
    let (r, out) = run_php(
        r#"<?php
class A { public function greet() { return "A::greet"; } }
class C extends A { }
$c = new C();
echo $c->greet();
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "A::greet");
}

#[test]
fn test_method_override_child_wins() {
    let (r, out) = run_php(
        r#"<?php
class A { public function greet() { return "A"; } }
class B extends A { public function greet() { return "B"; } }
$b = new B();
echo $b->greet();
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "B");
}

#[test]
fn test_multi_level_inheritance() {
    // C extends B extends A: C sees B's override, not A's original.
    let (r, out) = run_php(
        r#"<?php
class A { public function greet() { return "A"; } }
class B extends A { public function greet() { return "B"; } }
class C extends B { }
$d = new C();
echo $d->greet();
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "B");
}

#[test]
fn test_inherited_property_default() {
    let (r, out) = run_php(
        r#"<?php
class A { public $x = 5; }
class C extends A { public $y = 7; }
$c = new C();
echo $c->x, ",", $c->y;
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "5,7");
}

#[test]
fn test_property_override_child_value_wins() {
    // A redeclared property keeps the child's default, per PHP.
    let (r, out) = run_php(
        r#"<?php
class A { public $x = 5; }
class B extends A { public $x = 42; }
$b = new B();
echo $b->x;
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "42");
}

#[test]
fn test_inherited_constant() {
    let (r, out) = run_php(
        r#"<?php
class P { const MSG = "hello-const"; }
class Q extends P { }
echo Q::MSG;
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "hello-const");
}

#[test]
fn test_inherited_static_method() {
    let (r, out) = run_php(
        r#"<?php
class A { public static function who() { return "A::who"; } }
class B extends A { }
echo B::who();
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "A::who");
}

#[test]
fn test_late_static_binding_through_hierarchy() {
    // static::who() inside A must dispatch to B's override when called on B.
    let (r, out) = run_php(
        r#"<?php
class A {
    public static function who() { return "A::who"; }
    public static function test() { return static::who(); }
}
class B extends A {
    public static function who() { return "B::who"; }
}
echo B::test();
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "B::who");
}

// ---------------------------------------------------------------------------
// interfaces used with traits/enums still compile
// ---------------------------------------------------------------------------

#[test]
fn test_interface_alongside_trait() {
    let (r, out) = run_php(
        r#"<?php
interface Loggable { public function log($msg); }
trait Logger { public function log($msg) { return "[log] $msg"; } }
class App implements Loggable {
    use Logger;
}
$app = new App();
echo $app->log("started");
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "[log] started");
}

#[test]
fn test_namespaced_interface() {
    let (r, out) = run_php(
        r#"<?php
namespace App\Contracts;
interface Repository { public function find($id); }
namespace App;
class DbRepo implements \App\Contracts\Repository {
    public function find($id) { return "row-$id"; }
}
$r = new DbRepo();
echo $r->find(7);
"#,
    )
    .expect("compile+run");
    assert_success(&r, &out);
    assert_eq!(out, "row-7");
}
