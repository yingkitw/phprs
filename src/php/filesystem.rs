//! PHP Filesystem Functions
//!
//! Migrated from main/php_scandir.h and main/php_scandir.c

#[cfg(test)]
mod tests;

use std::fs;
use std::path::Path;

/// Scan directory and return entries
pub fn php_scandir(dirname: &str) -> Result<Vec<String>, String> {
    let path = Path::new(dirname);

    if !path.exists() {
        return Err(format!("Directory does not exist: {dirname}"));
    }

    if !path.is_dir() {
        return Err(format!("Not a directory: {dirname}"));
    }

    let entries = fs::read_dir(path).map_err(|e| format!("Failed to read directory: {e}"))?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
        if let Some(name) = entry.file_name().to_str() {
            files.push(name.to_string());
        }
    }

    files.sort();
    Ok(files)
}

/// Check if file exists
pub fn php_file_exists(filename: &str) -> bool {
    Path::new(filename).exists()
}

/// Check if path is a directory
pub fn php_is_dir(path: &str) -> bool {
    Path::new(path).is_dir()
}

/// Check if path is a file
pub fn php_is_file(path: &str) -> bool {
    Path::new(path).is_file()
}

/// Get file size
pub fn php_filesize(filename: &str) -> Result<u64, String> {
    let metadata =
        fs::metadata(filename).map_err(|e| format!("Failed to get file metadata: {e}"))?;
    Ok(metadata.len())
}

/// Read file contents
pub fn php_file_get_contents(filename: &str) -> Result<String, String> {
    fs::read_to_string(filename).map_err(|e| format!("Failed to read file: {e}"))
}

/// Write file contents
pub fn php_file_put_contents(filename: &str, data: &str) -> Result<usize, String> {
    fs::write(filename, data).map_err(|e| format!("Failed to write file: {e}"))?;
    Ok(data.len())
}

/// Append data to a file (FILE_APPEND flag equivalent)
pub fn php_file_append_contents(filename: &str, data: &str) -> Result<usize, String> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(filename)
        .map_err(|e| format!("Failed to open file for append: {e}"))?;
    file.write_all(data.as_bytes())
        .map_err(|e| format!("Failed to append to file: {e}"))?;
    Ok(data.len())
}

/// Create a directory
pub fn php_mkdir(pathname: &str, recursive: bool) -> Result<(), String> {
    let path = Path::new(pathname);
    if recursive {
        fs::create_dir_all(path).map_err(|e| format!("Failed to create directory: {e}"))
    } else {
        fs::create_dir(path).map_err(|e| format!("Failed to create directory: {e}"))
    }
}

/// Remove a directory
pub fn php_rmdir(dirname: &str) -> Result<(), String> {
    let path = Path::new(dirname);
    if !path.is_dir() {
        return Err(format!("Not a directory: {dirname}"));
    }
    fs::remove_dir(path).map_err(|e| format!("Failed to remove directory: {e}"))
}

/// Copy a file
pub fn php_copy(source: &str, dest: &str) -> Result<u64, String> {
    fs::copy(source, dest).map_err(|e| format!("Failed to copy file: {e}"))
}

/// Rename/move a file or directory
pub fn php_rename(oldname: &str, newname: &str) -> Result<(), String> {
    fs::rename(oldname, newname).map_err(|e| format!("Failed to rename: {e}"))
}

/// Delete a file
pub fn php_unlink(filename: &str) -> Result<(), String> {
    fs::remove_file(filename).map_err(|e| format!("Failed to delete file: {e}"))
}

/// Get the real (absolute) path
pub fn php_realpath(path: &str) -> Result<String, String> {
    fs::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to resolve path: {e}"))
}

/// Get the trailing name component of a path
pub fn php_basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Get the directory name component of a path
pub fn php_dirname(path: &str) -> String {
    Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string())
}

/// Get file extension
pub fn php_pathinfo_extension(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
}

/// Check if path is readable
pub fn php_is_readable(path: &str) -> bool {
    fs::metadata(path).is_ok()
}

/// Check if path is writable
pub fn php_is_writable(path: &str) -> bool {
    // On Unix, check write permission; on other platforms, try opening for write
    let p = Path::new(path);
    if p.exists() {
        fs::OpenOptions::new().write(true).open(path).is_ok()
    } else {
        // Check if parent directory is writable
        p.parent()
            .map(|_parent| {
                fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(path)
                    .map(|_| {
                        let _ = fs::remove_file(path);
                        true
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }
}

/// Create a temporary file name
pub fn php_tempnam(dir: &str, prefix: &str) -> Result<String, String> {
    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        return Err(format!("Not a directory: {dir}"));
    }
    let filename = format!(
        "{prefix}{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let full_path = dir_path.join(filename);
    // Create the file
    fs::File::create(&full_path).map_err(|e| format!("Failed to create temp file: {e}"))?;
    Ok(full_path.to_string_lossy().to_string())
}

/// Simple glob pattern matching for files
pub fn php_glob(pattern: &str) -> Result<Vec<String>, String> {
    let path = Path::new(pattern);
    let dir = match path.parent() {
        Some(p) if p.as_os_str().is_empty() => Path::new("."),
        Some(p) => p,
        None => Path::new("."),
    };
    let file_pattern = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "*".to_string());

    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {e}"))?;
    let mut matches = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if glob_match(&file_pattern, &name) {
            matches.push(
                dir.join(&name).to_string_lossy().to_string(),
            );
        }
    }

    matches.sort();
    Ok(matches)
}

/// Simple glob pattern matching (supports * and ?)
fn glob_match(pattern: &str, text: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let txt: Vec<char> = text.chars().collect();
    glob_match_impl(&pat, &txt, 0, 0)
}

fn glob_match_impl(pat: &[char], txt: &[char], pi: usize, ti: usize) -> bool {
    if pi == pat.len() && ti == txt.len() {
        return true;
    }
    if pi == pat.len() {
        return false;
    }
    if pat[pi] == '*' {
        // Match zero or more characters
        for i in ti..=txt.len() {
            if glob_match_impl(pat, txt, pi + 1, i) {
                return true;
            }
        }
        return false;
    }
    if ti == txt.len() {
        return false;
    }
    if pat[pi] == '?' || pat[pi] == txt[ti] {
        return glob_match_impl(pat, txt, pi + 1, ti + 1);
    }
    false
}
