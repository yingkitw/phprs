//! Unit tests for PHP built-in functions dispatched through [`super::builtins::execute_builtin_function`].
//! Covers the surface area implemented in `builtins.rs` (not the full PHP language).

use super::builtins::execute_builtin_function;
use super::execute_data::ExecuteData;
use crate::engine::hash::{hash_add_or_update, hash_init};
use crate::engine::operators::{zval_get_bool, zval_get_double, zval_get_long, zval_get_string};
use crate::engine::string::string_init;
use crate::engine::types::{PhpArray, PhpType, PhpValue, Val};

fn str_val(s: &str) -> Val {
    Val::new(
        PhpValue::String(Box::new(string_init(s, false))),
        PhpType::String,
    )
}

fn long_val(n: i64) -> Val {
    Val::new(PhpValue::Long(n), PhpType::Long)
}

fn double_val(n: f64) -> Val {
    Val::new(PhpValue::Double(n), PhpType::Double)
}

fn true_val() -> Val {
    Val::new(PhpValue::Long(1), PhpType::True)
}

fn false_val() -> Val {
    Val::new(PhpValue::Long(0), PhpType::False)
}

fn null_val() -> Val {
    Val::new(PhpValue::Long(0), PhpType::Null)
}

fn array_from_str_keys(pairs: &[(&str, &str)]) -> Val {
    let mut arr = PhpArray::new();
    hash_init(&mut arr, 8);
    for (k, v) in pairs {
        let key = string_init(k, false);
        let val = str_val(v);
        hash_add_or_update(&mut arr, Some(&key), 0, val, 0);
    }
    Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array)
}

fn run(
    name: &'static str,
    args: &[Val],
    ed: &mut ExecuteData,
) -> Result<Option<Val>, String> {
    execute_builtin_function(name, args, ed)
}

fn drain_output_buffers() {
    while crate::php::output::php_output_get_level() > 0 {
        let _ = crate::php::output::php_output_end_clean();
    }
}

// --- Core / unknown ---

#[test]
fn unknown_builtin_returns_none() {
    drain_output_buffers();
    let mut ed = ExecuteData::new();
    assert!(matches!(
        run("not_a_phprs_builtin_xyz", &[], &mut ed),
        Ok(None)
    ));
}

#[test]
fn unset_builtin_returns_none() {
    let mut ed = ExecuteData::new();
    assert!(matches!(run("unset", &[str_val("x")], &mut ed), Ok(None)));
}

// --- Strings ---

#[test]
fn string_builtins_strlen_substr_strpos_replace_case_trim() {
    let mut ed = ExecuteData::new();
    assert_eq!(
        zval_get_long(&run("strlen", &[str_val("hello")], &mut ed).unwrap().unwrap()),
        5
    );
    assert_eq!(
        zval_get_string(&run("substr", &[str_val("abcdef"), long_val(2), long_val(3)], &mut ed).unwrap().unwrap())
            .as_str(),
        "cde"
    );
    assert_eq!(
        zval_get_long(&run("strpos", &[str_val("haystack"), str_val("stack")], &mut ed).unwrap().unwrap()),
        3
    );
    let nope = run("strpos", &[str_val("abc"), str_val("z")], &mut ed).unwrap().unwrap();
    assert_eq!(nope.get_type(), PhpType::False);
    assert_eq!(
        zval_get_string(
            &run(
                "str_replace",
                &[str_val("a"), str_val("o"), str_val("banana")],
                &mut ed
            )
            .unwrap()
            .unwrap()
        )
        .as_str(),
        "bonono"
    );
    assert_eq!(
        zval_get_string(&run("strtolower", &[str_val("AbC")], &mut ed).unwrap().unwrap()).as_str(),
        "abc"
    );
    assert_eq!(
        zval_get_string(&run("strtoupper", &[str_val("AbC")], &mut ed).unwrap().unwrap()).as_str(),
        "ABC"
    );
    assert_eq!(
        zval_get_string(&run("trim", &[str_val("  x  ")], &mut ed).unwrap().unwrap()).as_str(),
        "x"
    );
}

#[test]
fn explode_implode_sprintf_ucfirst() {
    let mut ed = ExecuteData::new();
    let ex = run("explode", &[str_val(","), str_val("a,b,c")], &mut ed)
        .unwrap()
        .unwrap();
    if let PhpValue::Array(ref a) = ex.value {
        assert_eq!(a.ar_data.len(), 3);
    } else {
        panic!("explode should return array");
    }
    let im = run("implode", &[str_val("-"), ex], &mut ed)
        .unwrap()
        .unwrap();
    assert_eq!(zval_get_string(&im).as_str(), "a-b-c");

    let sp = run(
        "sprintf",
        &[str_val("%s:%d"), str_val("n"), long_val(7)],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    assert_eq!(zval_get_string(&sp).as_str(), "n:7");

    assert_eq!(
        zval_get_string(&run("ucfirst", &[str_val("hello")], &mut ed).unwrap().unwrap()).as_str(),
        "Hello"
    );
}

// --- Type conversion / checks ---

#[test]
fn intval_floatval_strval() {
    let mut ed = ExecuteData::new();
    assert_eq!(
        zval_get_long(&run("intval", &[str_val("42")], &mut ed).unwrap().unwrap()),
        42
    );
    assert!(
        (zval_get_double(&run("floatval", &[str_val("3.5")], &mut ed).unwrap().unwrap()) - 3.5).abs()
            < 0.001
    );
    assert_eq!(
        zval_get_string(&run("strval", &[str_val("x")], &mut ed).unwrap().unwrap()).as_str(),
        "x"
    );
}

#[test]
fn isset_empty() {
    let mut ed = ExecuteData::new();
    assert!(zval_get_bool(
        &run("isset", &[str_val("a")], &mut ed).unwrap().unwrap()
    ));
    assert!(!zval_get_bool(
        &run("isset", &[null_val()], &mut ed).unwrap().unwrap()
    ));
    assert!(zval_get_bool(&run("empty", &[long_val(0)], &mut ed).unwrap().unwrap()));
    assert!(!zval_get_bool(
        &run("empty", &[long_val(1)], &mut ed).unwrap().unwrap()
    ));
}

#[test]
fn is_array_string_object() {
    let mut ed = ExecuteData::new();
    let arr = array_from_str_keys(&[("k", "v")]);
    assert!(zval_get_bool(&run("is_array", &[arr], &mut ed).unwrap().unwrap()));
    assert!(zval_get_bool(
        &run("is_string", &[str_val("x")], &mut ed).unwrap().unwrap()
    ));
    assert!(!zval_get_bool(
        &run("is_object", &[str_val("x")], &mut ed).unwrap().unwrap()
    ));
}

// --- Array helpers ---

#[test]
fn array_key_exists_in_array_count_merge() {
    let mut ed = ExecuteData::new();
    let probe = || array_from_str_keys(&[("name", "PHP-RS"), ("v", "1")]);
    assert!(zval_get_bool(
        &run("array_key_exists", &[str_val("name"), probe()], &mut ed).unwrap().unwrap()
    ));
    assert!(!zval_get_bool(
        &run("array_key_exists", &[str_val("missing"), probe()], &mut ed).unwrap().unwrap()
    ));
    assert!(zval_get_bool(
        &run("in_array", &[str_val("PHP-RS"), probe()], &mut ed).unwrap().unwrap()
    ));

    let c = run("count", &[probe()], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_long(&c), 2);
    assert_eq!(zval_get_long(&run("count", &[str_val("scalar")], &mut ed).unwrap().unwrap()), 1);

    let a2 = array_from_str_keys(&[("b", "2")]);
    let merged = run("array_merge", &[probe(), a2], &mut ed).unwrap().unwrap();
    if let PhpValue::Array(ref m) = merged.value {
        assert!(m.ar_data.len() >= 2);
    } else {
        panic!("array_merge");
    }

    let _push = run(
        "array_push",
        &[merged.clone(), str_val("tail")],
        &mut ed,
    )
    .unwrap()
    .unwrap();
}

// --- JSON ---

#[test]
fn json_encode_decode() {
    let mut ed = ExecuteData::new();
    let enc = run("json_encode", &[long_val(99)], &mut ed).unwrap().unwrap();
    assert!(zval_get_string(&enc).as_str().contains('9'));
    let dec = run("json_decode", &[str_val(r#"{"a":1}"#)], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&dec).as_str(), r#"{"a":1}"#);
}

// --- Constants / path / exit ---

#[test]
fn define_defined_constant_dirname() {
    let mut ed = ExecuteData::new();
    assert!(zval_get_bool(
        &run("define", &[str_val("MY_CONST"), long_val(99)], &mut ed).unwrap().unwrap()
    ));
    assert!(zval_get_bool(
        &run("defined", &[str_val("MY_CONST")], &mut ed).unwrap().unwrap()
    ));
    let v = run("constant", &[str_val("MY_CONST")], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_long(&v), 99);
    assert!(run("constant", &[str_val("NOPE")], &mut ed).is_err());

    let d = run("dirname", &[str_val("/tmp/foo/bar.txt")], &mut ed)
        .unwrap()
        .unwrap();
    assert!(zval_get_string(&d).as_str().contains("foo"));
}

#[test]
fn die_sets_exit_and_echo_writes_output() {
    drain_output_buffers();
    crate::php::output::php_output_start().unwrap();
    let mut ed = ExecuteData::new();
    let r = run("die", &[str_val("bye")], &mut ed).unwrap();
    assert!(r.is_none());
    assert_eq!(ed.exit_requested, Some(0));
    let out = crate::php::output::php_output_end().unwrap();
    assert!(out.contains("bye"));
    drain_output_buffers();
}

#[test]
fn exit_with_code() {
    drain_output_buffers();
    let mut ed = ExecuteData::new();
    let _ = run("exit", &[long_val(7)], &mut ed).unwrap();
    assert_eq!(ed.exit_requested, Some(7));
}

// --- WordPress hook shims / escaping ---

#[test]
fn do_action_apply_filters_shortcode_atts() {
    let mut ed = ExecuteData::new();
    assert!(run("do_action", &[str_val("init")], &mut ed).unwrap().is_none());
    let v = run(
        "apply_filters",
        &[str_val("the_title"), str_val("Hello")],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    assert_eq!(zval_get_string(&v).as_str(), "Hello");

    let defs = array_from_str_keys(&[("a", "1")]);
    let atts = array_from_str_keys(&[("b", "2")]);
    let sc = run("shortcode_atts", &[defs, atts], &mut ed)
        .unwrap()
        .unwrap();
    if let PhpValue::Array(ref a) = sc.value {
        assert!(!a.ar_data.is_empty());
    } else {
        panic!("shortcode_atts");
    }
}

#[test]
fn htmlspecialchars_htmlentities_esc_attr_esc_url() {
    let mut ed = ExecuteData::new();
    let raw = "<div class=\"x\">'a' & b</div>";
    let h = run("htmlspecialchars", &[str_val(raw)], &mut ed)
        .unwrap()
        .unwrap();
    let hs = zval_get_string(&h).as_str().to_string();
    assert!(hs.contains("&lt;") && hs.contains("&amp;"));

    let e = run("htmlentities", &[str_val(raw)], &mut ed)
        .unwrap()
        .unwrap();
    assert!(!zval_get_string(&e).as_str().is_empty());

    let ea = run("esc_attr", &[str_val("\"a\"")], &mut ed).unwrap().unwrap();
    assert!(zval_get_string(&ea).as_str().contains("&quot;"));

    let u = run("esc_url", &[str_val("https://ex.test/x")], &mut ed)
        .unwrap()
        .unwrap();
    assert_eq!(zval_get_string(&u).as_str(), "https://ex.test/x");
}

// --- PCRE via builtins ---

#[test]
fn preg_match_replace_split_match_all() {
    let mut ed = ExecuteData::new();
    let m = run(
        "preg_match",
        &[str_val("/[0-9]+/"), str_val("a99b")],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    assert_eq!(zval_get_long(&m), 1);

    let rep = run(
        "preg_replace",
        &[str_val("/a/"), str_val("b"), str_val("aaa")],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    assert_eq!(zval_get_string(&rep).as_str(), "bbb");

    let spl = run(
        "preg_split",
        &[str_val("/\\s+/"), str_val("a  b\tc")],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    if let PhpValue::Array(ref a) = spl.value {
        assert_eq!(a.ar_data.len(), 3);
    } else {
        panic!("preg_split");
    }

    let all = run(
        "preg_match_all",
        &[str_val("/o/"), str_val("foofoo")],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    assert_eq!(zval_get_long(&all), 4);
}

// --- Output / debug ---

#[test]
fn echo_print_var_dump_print_r_phpinfo_phpversion() {
    drain_output_buffers();
    crate::php::output::php_output_start().unwrap();
    let mut ed = ExecuteData::new();
    let _ = run("echo", &[str_val("hi")], &mut ed).unwrap();
    let _ = run("print", &[long_val(2)], &mut ed).unwrap();
    let _ = run("var_dump", &[long_val(1)], &mut ed).unwrap();
    let pr = run(
        "print_r",
        &[array_from_str_keys(&[("k", "v")]), true_val()],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    assert!(zval_get_string(&pr).as_str().contains('k'));
    let _ = run("phpinfo", &[], &mut ed).unwrap();
    let ver = run("phpversion", &[], &mut ed).unwrap().unwrap();
    assert!(zval_get_string(&ver).as_str().contains("phprs"));
    let out = crate::php::output::php_output_end().unwrap();
    assert!(out.contains("hi"));
    drain_output_buffers();
}

// --- Output buffering stack ---

#[test]
fn ob_stack_functions_smoke() {
    drain_output_buffers();
    let mut ed = ExecuteData::new();
    assert!(zval_get_bool(
        &run("ob_start", &[], &mut ed).unwrap().unwrap()
    ));
    let _ = run("echo", &[str_val("layer1")], &mut ed).unwrap();
    assert_eq!(
        zval_get_long(&run("ob_get_level", &[], &mut ed).unwrap().unwrap()),
        1
    );
    let contents = run("ob_get_contents", &[], &mut ed).unwrap().unwrap();
    assert!(zval_get_string(&contents).as_str().contains("layer1"));
    let _ = run("ob_clean", &[], &mut ed).unwrap();
    let clean = run("ob_get_contents", &[], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&clean).as_str(), "");
    let _ = run("echo", &[str_val("x")], &mut ed).unwrap();
    let gc = run("ob_get_clean", &[], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&gc).as_str(), "x");
    assert!(zval_get_bool(
        &run("ob_end_clean", &[], &mut ed).unwrap().unwrap()
    ));
    drain_output_buffers();
}

// --- Error / ini / include path (minimal implementations) ---

#[test]
fn error_handlers_shutdown_ini_include_path() {
    let mut ed = ExecuteData::new();
    let _ = run(
        "set_error_handler",
        &[str_val("my_err_handler")],
        &mut ed,
    )
    .unwrap();
    assert!(zval_get_bool(
        &run("restore_error_handler", &[], &mut ed).unwrap().unwrap()
    ));

    let _ = run(
        "set_exception_handler",
        &[str_val("my_ex_handler")],
        &mut ed,
    )
    .unwrap();
    assert!(zval_get_bool(
        &run("restore_exception_handler", &[], &mut ed).unwrap().unwrap()
    ));

    assert!(run("register_shutdown_function", &[str_val("fini")], &mut ed)
        .unwrap()
        .is_none());
    assert_eq!(ed.shutdown_functions, vec!["fini".to_string()]);

    assert_eq!(zval_get_long(&run("error_reporting", &[], &mut ed).unwrap().unwrap()), 0);
    assert!(zval_get_bool(
        &run("trigger_error", &[str_val("msg")], &mut ed).unwrap().unwrap()
    ));

    let _ = run("set_include_path", &[str_val("/tmp")], &mut ed).unwrap();
    let p = run("get_include_path", &[], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&p).as_str(), ".");

    let _ = run("ini_set", &[str_val("x"), str_val("y")], &mut ed).unwrap();
    let ig = run("ini_get", &[str_val("x")], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&ig).as_str(), "");
}

// --- Math (delegates to php::math) ---

#[test]
fn math_builtins_delegate() {
    let mut ed = ExecuteData::new();
    assert_eq!(zval_get_long(&run("abs", &[long_val(-3)], &mut ed).unwrap().unwrap()), 3);
    assert_eq!(
        zval_get_double(&run("ceil", &[double_val(2.1)], &mut ed).unwrap().unwrap()),
        3.0
    );
    assert_eq!(
        zval_get_double(&run("floor", &[double_val(2.9)], &mut ed).unwrap().unwrap()),
        2.0
    );
    assert!(
        (zval_get_double(&run("sqrt", &[long_val(25)], &mut ed).unwrap().unwrap()) - 5.0).abs()
            < 0.001
    );
    assert!(
        (zval_get_double(
            &run("pow", &[long_val(2), long_val(4)], &mut ed).unwrap().unwrap()
        ) - 16.0)
            .abs()
            < 0.001
    );
    let pi = run("pi", &[], &mut ed).unwrap().unwrap();
    assert!(zval_get_double(&pi) > 3.0);
    assert_eq!(
        zval_get_long(&run("max", &[long_val(1), long_val(9), long_val(3)], &mut ed).unwrap().unwrap()),
        9
    );
    assert_eq!(
        zval_get_long(&run("min", &[long_val(1), long_val(9), long_val(3)], &mut ed).unwrap().unwrap()),
        1
    );
    let r = run("rand", &[long_val(1), long_val(2)], &mut ed).unwrap().unwrap();
    let rv = zval_get_long(&r);
    assert!((1..=2).contains(&rv));
}

// --- Hash / crypto ---

#[test]
fn hash_builtins_md5_sha1_hash_hmac_crc32_base64_bin_hex() {
    let mut ed = ExecuteData::new();
    let md = run("md5", &[str_val("hello")], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&md).as_str().len(), 32);
    let sh = run("sha1", &[str_val("hello")], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&sh).as_str().len(), 40);

    let h = run("hash", &[str_val("md5"), str_val("x")], &mut ed)
        .unwrap()
        .unwrap();
    assert_eq!(zval_get_string(&h).as_str().len(), 32);

    let hm = run(
        "hash_hmac",
        &[str_val("md5"), str_val("data"), str_val("key")],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    assert!(!zval_get_string(&hm).as_str().is_empty());

    let crc = run("crc32", &[str_val("abc")], &mut ed).unwrap().unwrap();
    assert_ne!(zval_get_long(&crc), 0);

    let b2h = run("bin2hex", &[str_val("ab")], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&b2h).as_str(), "6162");
    let h2b = run("hex2bin", &[str_val("6162")], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&h2b).as_str(), "ab");

    let b64 = run("base64_encode", &[str_val("hi")], &mut ed).unwrap().unwrap();
    let dec = run("base64_decode", &[b64], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&dec).as_str(), "hi");
}

#[test]
fn random_and_password_builtins() {
    let mut ed = ExecuteData::new();
    let rb = run("random_bytes", &[long_val(8)], &mut ed).unwrap().unwrap();
    // Bytes are UTF-8 lossy–encoded; strlen in Rust may not equal 8.
    assert!(!zval_get_string(&rb).as_str().is_empty());
    let ri = run("random_int", &[long_val(10), long_val(10)], &mut ed)
        .unwrap()
        .unwrap();
    assert_eq!(zval_get_long(&ri), 10);

    let ph = run(
        "password_hash",
        &[str_val("secret"), str_val("PASSWORD_DEFAULT")],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    let hash_s = zval_get_string(&ph).as_str().to_string();
    assert!(!hash_s.is_empty());
    let ok = run(
        "password_verify",
        &[str_val("secret"), str_val(&hash_s)],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    assert!(zval_get_bool(&ok));
}

// --- Date / time ---

#[test]
fn datetime_builtins_smoke() {
    let mut ed = ExecuteData::new();
    let t = run("time", &[], &mut ed).unwrap().unwrap();
    assert!(zval_get_long(&t) > 1_000_000_000);

    let mt = run("microtime", &[false_val()], &mut ed).unwrap().unwrap();
    assert!(!zval_get_string(&mt).as_str().is_empty());
    let mtf = run("microtime", &[true_val()], &mut ed).unwrap().unwrap();
    assert!(zval_get_double(&mtf) > 1.0);

    let d = run("date", &[str_val("Y-m-d"), long_val(0)], &mut ed)
        .unwrap()
        .unwrap();
    assert!(!zval_get_string(&d).as_str().is_empty());

    let mk = run(
        "mktime",
        &[long_val(0), long_val(0), long_val(0), long_val(1), long_val(1), long_val(2020)],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    assert_ne!(zval_get_long(&mk), 0);

    let st = run(
        "strtotime",
        &[str_val("+1 day"), long_val(0)],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    assert_eq!(zval_get_long(&st), 86400);
}

// --- URL ---

#[test]
fn url_builtins_parse_build_encode_decode_parse_str() {
    let mut ed = ExecuteData::new();
    let pu = run("parse_url", &[str_val("https://ex.org:8080/p?q=1#h")], &mut ed)
        .unwrap()
        .unwrap();
    if let PhpValue::Array(ref a) = pu.value {
        assert!(!a.ar_data.is_empty());
    } else {
        panic!("parse_url");
    }

    let q = run(
        "http_build_query",
        &[array_from_str_keys(&[("a", "1"), ("b", "2")])],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    let qs = zval_get_string(&q).as_str().to_string();
    assert!(qs.contains('a') && qs.contains('b'));

    let enc = run("urlencode", &[str_val("a b")], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&enc).as_str(), "a+b");
    let dec = run("urldecode", &[enc], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&dec).as_str(), "a b");

    let re = run("rawurlencode", &[str_val("a/b")], &mut ed).unwrap().unwrap();
    let rd = run("rawurldecode", &[re], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&rd).as_str(), "a/b");

    let ps = run("parse_str", &[str_val("a=1&b=two")], &mut ed)
        .unwrap()
        .unwrap();
    if let PhpValue::Array(ref a) = ps.value {
        assert!(a.ar_data.len() >= 2);
    } else {
        panic!("parse_str");
    }

    let gh = run("get_headers", &[str_val("/not-http")], &mut ed)
        .unwrap()
        .unwrap();
    assert_eq!(gh.get_type(), PhpType::False);
}

// --- Multibyte ---

#[test]
fn mbstring_builtins_smoke() {
    let mut ed = ExecuteData::new();
    assert_eq!(
        zval_get_long(&run("mb_strlen", &[str_val("hi")], &mut ed).unwrap().unwrap()),
        2
    );
    let sub = run("mb_substr", &[str_val("abcde"), long_val(1), long_val(3)], &mut ed)
        .unwrap()
        .unwrap();
    assert_eq!(zval_get_string(&sub).as_str(), "bcd");
    let lo = run("mb_strtolower", &[str_val("AB")], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&lo).as_str(), "ab");
    let up = run("mb_strtoupper", &[str_val("ab")], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_string(&up).as_str(), "AB");
    let pos = run("mb_strpos", &[str_val("abc"), str_val("b")], &mut ed)
        .unwrap()
        .unwrap();
    assert_eq!(zval_get_long(&pos), 1);
    let rpos = run("mb_strrpos", &[str_val("abcb"), str_val("b")], &mut ed)
        .unwrap()
        .unwrap();
    assert_eq!(zval_get_long(&rpos), 3);

    let conv = run(
        "mb_convert_encoding",
        &[str_val("x"), str_val("UTF-8"), str_val("UTF-8")],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    assert_eq!(zval_get_string(&conv).as_str(), "x");

    let cnt = run(
        "mb_substr_count",
        &[str_val("abab"), str_val("ab")],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    assert_eq!(zval_get_long(&cnt), 2);

    let w = run("mb_strwidth", &[str_val("abc")], &mut ed).unwrap().unwrap();
    assert_eq!(zval_get_long(&w), 3);

    let trimw = run(
        "mb_strimwidth",
        &[str_val("abcdef"), long_val(0), long_val(4), str_val("…")],
        &mut ed,
    )
    .unwrap()
    .unwrap();
    assert!(!zval_get_string(&trimw).as_str().is_empty());
}

// --- Filesystem (local) ---

#[test]
fn file_exists_and_file_get_contents_resolved() {
    drain_output_buffers();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::fs::write(&path, "payload").unwrap();
    let dir = path.parent().unwrap().to_string_lossy().into_owned();

    let mut ed = ExecuteData::new();
    ed.current_script_dir = Some(dir);

    let base = path.file_name().unwrap().to_string_lossy().into_owned();
    let ex = run("file_exists", &[str_val(&base)], &mut ed).unwrap().unwrap();
    assert!(zval_get_bool(&ex));

    let contents = run("file_get_contents", &[str_val(&base)], &mut ed)
        .unwrap()
        .unwrap();
    assert_eq!(zval_get_string(&contents).as_str(), "payload");
    drain_output_buffers();
}
