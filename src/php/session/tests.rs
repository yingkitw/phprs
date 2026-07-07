use super::*;
use crate::engine::hash::{hash_add_or_update, hash_init};
use crate::engine::string::string_init;
use crate::engine::types::{PhpType, PhpValue, Val};
use crate::engine::vm::ExecuteData;

#[test]
fn session_start_binds_session_superglobal() {
    let dir = tempfile::tempdir().unwrap();
    configure_save_path(dir.path().to_path_buf());

    let mut ed = ExecuteData::new();
    assert!(session_start(&mut ed).unwrap());
    ed.set_var(
        "_SESSION",
        {
            let mut arr = crate::engine::types::PhpArray::new();
            hash_init(&mut arr, 8);
            let key = string_init("user", false);
            let _ = hash_add_or_update(
                &mut arr,
                Some(&key),
                0,
                Val::new(
                    PhpValue::String(Box::new(string_init("alice", false))),
                    PhpType::String,
                ),
                0,
            );
            Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array)
        },
    );

    session_write_close(&ed).unwrap();
    assert!(!ed.session_id.is_empty());

    let mut ed2 = ExecuteData::new();
    apply_incoming_session_id(&mut ed2, &ed.session_id);
    session_start(&mut ed2).unwrap();
    let loaded = ed2.get_var("_SESSION");
    if let PhpValue::Array(ref arr) = loaded.value {
        let key = string_init("user", false);
        let found = crate::engine::hash::hash_find(arr, &key);
        assert!(found.is_some());
    } else {
        panic!("expected array _SESSION");
    }
}

#[test]
fn session_destroy_clears_state() {
    let mut ed = ExecuteData::new();
    session_start(&mut ed).unwrap();
    session_destroy(&mut ed).unwrap();
    assert!(!ed.session_active);
    assert!(ed.session_id.is_empty());
}

#[test]
fn session_id_get_and_set() {
    let mut ed = ExecuteData::new();
    session_start(&mut ed).unwrap();
    let id = session_id(&[], &mut ed).unwrap();
    assert_eq!(id, ed.session_id);

    let new_id = session_id(
        &[Val::new(
            PhpValue::String(Box::new(string_init("customid", false))),
            PhpType::String,
        )],
        &mut ed,
    )
    .unwrap();
    assert_eq!(new_id, "customid");
}
