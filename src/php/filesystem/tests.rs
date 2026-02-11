//! Unit tests for PHP Filesystem Functions

use crate::php::filesystem::*;
use std::fs;

#[test]
fn test_php_file_exists() {
    assert!(php_file_exists("."));
    assert!(!php_file_exists("/nonexistent/path/that/does/not/exist"));
}

#[test]
fn test_php_is_dir() {
    assert!(php_is_dir("."));
    assert!(php_is_dir("src"));
    assert!(!php_is_dir("Cargo.toml"));
}

#[test]
fn test_php_is_file() {
    assert!(php_is_file("Cargo.toml"));
    assert!(!php_is_file("."));
    assert!(!php_is_file("nonexistent.txt"));
}

#[test]
fn test_php_filesize() {
    if php_file_exists("Cargo.toml") {
        let size = php_filesize("Cargo.toml").unwrap();
        assert!(size > 0);
    }
    assert!(php_filesize("nonexistent.txt").is_err());
}

#[test]
fn test_php_file_get_contents() {
    if php_file_exists("Cargo.toml") {
        let content = php_file_get_contents("Cargo.toml").unwrap();
        assert!(content.contains("phprs"));
    }
    assert!(php_file_get_contents("nonexistent.txt").is_err());
}

#[test]
fn test_php_file_put_contents() {
    let test_file = "test_put_contents.txt";
    let test_content = "Hello, PHP-RS!";

    let result = php_file_put_contents(test_file, test_content);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), test_content.len());

    let read_content = php_file_get_contents(test_file).unwrap();
    assert_eq!(read_content, test_content);
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_php_file_append_contents() {
    let test_file = "test_append_contents.txt";
    let _ = fs::remove_file(test_file);

    php_file_put_contents(test_file, "Hello").unwrap();
    php_file_append_contents(test_file, ", World!").unwrap();

    let content = php_file_get_contents(test_file).unwrap();
    assert_eq!(content, "Hello, World!");
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_php_scandir() {
    let result = php_scandir(".");
    assert!(result.is_ok());
    let entries = result.unwrap();
    assert!(!entries.is_empty());
    assert!(entries.contains(&"Cargo.toml".to_string()) || entries.contains(&"src".to_string()));
    assert!(php_scandir("/nonexistent/directory").is_err());
}

#[test]
fn test_php_mkdir_rmdir() {
    let test_dir = "test_mkdir_dir";
    let _ = fs::remove_dir_all(test_dir);

    assert!(php_mkdir(test_dir, false).is_ok());
    assert!(php_is_dir(test_dir));
    assert!(php_rmdir(test_dir).is_ok());
    assert!(!php_is_dir(test_dir));
}

#[test]
fn test_php_mkdir_recursive() {
    let test_dir = "test_mkdir_recursive/a/b/c";
    let _ = fs::remove_dir_all("test_mkdir_recursive");

    assert!(php_mkdir(test_dir, true).is_ok());
    assert!(php_is_dir(test_dir));
    let _ = fs::remove_dir_all("test_mkdir_recursive");
}

#[test]
fn test_php_copy() {
    let src = "test_copy_src.txt";
    let dst = "test_copy_dst.txt";
    let _ = fs::remove_file(src);
    let _ = fs::remove_file(dst);

    php_file_put_contents(src, "copy me").unwrap();
    assert!(php_copy(src, dst).is_ok());
    assert_eq!(php_file_get_contents(dst).unwrap(), "copy me");

    let _ = fs::remove_file(src);
    let _ = fs::remove_file(dst);
}

#[test]
fn test_php_rename() {
    let old = "test_rename_old.txt";
    let new = "test_rename_new.txt";
    let _ = fs::remove_file(old);
    let _ = fs::remove_file(new);

    php_file_put_contents(old, "rename me").unwrap();
    assert!(php_rename(old, new).is_ok());
    assert!(!php_file_exists(old));
    assert_eq!(php_file_get_contents(new).unwrap(), "rename me");

    let _ = fs::remove_file(new);
}

#[test]
fn test_php_unlink() {
    let f = "test_unlink.txt";
    php_file_put_contents(f, "delete me").unwrap();
    assert!(php_file_exists(f));
    assert!(php_unlink(f).is_ok());
    assert!(!php_file_exists(f));
}

#[test]
fn test_php_basename() {
    assert_eq!(php_basename("/home/user/file.txt"), "file.txt");
    assert_eq!(php_basename("file.txt"), "file.txt");
    assert_eq!(php_basename("/home/user/"), "user");
}

#[test]
fn test_php_dirname() {
    assert_eq!(php_dirname("/home/user/file.txt"), "/home/user");
    assert_eq!(php_dirname("file.txt"), "");
}

#[test]
fn test_php_pathinfo_extension() {
    assert_eq!(php_pathinfo_extension("file.txt"), Some("txt".to_string()));
    assert_eq!(php_pathinfo_extension("file.tar.gz"), Some("gz".to_string()));
    assert_eq!(php_pathinfo_extension("noext"), None);
}

#[test]
fn test_php_realpath() {
    // Cargo.toml should resolve to an absolute path
    let result = php_realpath("Cargo.toml");
    assert!(result.is_ok());
    let abs = result.unwrap();
    assert!(abs.contains("Cargo.toml"));
    assert!(abs.starts_with('/'));

    assert!(php_realpath("nonexistent_file_xyz.txt").is_err());
}

#[test]
fn test_php_is_readable() {
    assert!(php_is_readable("Cargo.toml"));
    assert!(!php_is_readable("nonexistent_xyz.txt"));
}

#[test]
fn test_php_glob() {
    // Match all .toml files in current dir
    let result = php_glob("*.toml");
    assert!(result.is_ok());
    let matches = result.unwrap();
    assert!(matches.iter().any(|m| m.contains("Cargo.toml")));
}

#[test]
fn test_php_tempnam() {
    let result = php_tempnam(".", "phprs_test_");
    assert!(result.is_ok());
    let path = result.unwrap();
    assert!(php_file_exists(&path));
    let _ = fs::remove_file(&path);
}
