//! PHP-RS Web Server
//!
//! Like PHP's built-in server (`php -S`), this serves both static pages
//! and executes PHP code — all from a single Rust binary.

use phprs::php::output::{php_output_end, php_output_start};
use phprs::engine::compile::compile_string;
use phprs::engine::vm::{execute_ex, ExecuteData};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;

const EXAMPLES_DIR: &str = "examples";

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Serialize)]
struct ExecResult {
    output: String,
    opcodes: Vec<OpcodeInfo>,
    filename: Option<String>,
    compile_time_ms: f64,
    exec_time_ms: f64,
}

#[derive(Serialize)]
struct OpcodeInfo {
    index: usize,
    opcode: String,
    extended_value: u32,
}

#[derive(Serialize)]
struct ExampleFile {
    name: String,
    path: String,
    content: String,
}

#[derive(Deserialize)]
struct ExecRequest {
    code: Option<String>,
    filename: Option<String>,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn start(port: u16) {
    let (listener, port) = bind_with_retry(port, 10);

    println!("PHP-RS Server running at http://localhost:{port}");
    println!("  GET  /              - Web UI");
    println!("  GET  /api/examples  - List PHP examples");
    println!("  POST /api/execute   - Execute PHP code");
    println!("  GET  /api/health    - Health check");

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                std::thread::spawn(move || {
                    if let Err(e) = handle_connection(s) {
                        eprintln!("Connection error: {e}");
                    }
                });
            }
            Err(e) => eprintln!("Accept error: {e}"),
        }
    }
}

/// Try to bind to `start_port`. If occupied, increment and retry up to `max_attempts` times.
fn bind_with_retry(start_port: u16, max_attempts: u16) -> (TcpListener, u16) {
    for offset in 0..max_attempts {
        let port = start_port + offset;
        let addr = format!("0.0.0.0:{port}");
        match TcpListener::bind(&addr) {
            Ok(listener) => return (listener, port),
            Err(e) if offset + 1 < max_attempts => {
                eprintln!("Port {port} in use ({e}), trying {}", port + 1);
            }
            Err(e) => {
                eprintln!("Failed to bind to ports {start_port}-{port}: {e}");
                std::process::exit(1);
            }
        }
    }
    unreachable!()
}

// ---------------------------------------------------------------------------
// HTTP handling
// ---------------------------------------------------------------------------

fn handle_connection(mut stream: std::net::TcpStream) -> Result<(), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);

    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|e| e.to_string())?;
    let request_line = request_line.trim().to_string();
    if request_line.is_empty() {
        return Ok(());
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }
    let method = parts[0];
    let path = parts[1];

    // Read headers
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            break;
        }
        if let Some(val) = line.to_lowercase().strip_prefix("content-length:") {
            content_length = val.trim().parse().unwrap_or(0);
        }
    }

    let body = if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
        String::from_utf8_lossy(&buf).to_string()
    } else {
        String::new()
    };

    println!("{method} {path}");

    let (status, content_type, response_body) = route_request(method, path, &body);

    let resp = format!(
        "HTTP/1.1 {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {len}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         Connection: close\r\n\
         \r\n",
        len = response_body.len()
    );

    stream
        .write_all(resp.as_bytes())
        .map_err(|e| e.to_string())?;
    stream
        .write_all(response_body.as_bytes())
        .map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

fn route_request(method: &str, path: &str, body: &str) -> (String, String, String) {
    if method == "OPTIONS" {
        return ("204 No Content".into(), "text/plain".into(), String::new());
    }

    let json = "application/json".to_string();

    match (method, path) {
        // ---- UI ----
        ("GET", "/" | "/index.html") => (
            "200 OK".into(),
            "text/html; charset=utf-8".into(),
            INDEX_HTML.to_string(),
        ),
        ("GET", "/favicon.ico") => (
            "204 No Content".into(),
            "image/x-icon".into(),
            String::new(),
        ),

        // ---- API ----
        ("GET", "/api/health") => {
            let r = serde_json::json!({"success":true,"data":{"status":"ok","version":"0.1.0"}});
            ("200 OK".into(), json, r.to_string())
        }
        ("GET", "/api/examples") => {
            let r = list_examples();
            (
                "200 OK".into(),
                json,
                serde_json::to_string(&r).unwrap_or_default(),
            )
        }
        ("GET", p) if p.starts_with("/api/examples/") => {
            let name = &p["/api/examples/".len()..];
            let r = get_example(name);
            (
                "200 OK".into(),
                json,
                serde_json::to_string(&r).unwrap_or_default(),
            )
        }
        ("POST", "/api/execute") => {
            let r = handle_execute(body);
            (
                "200 OK".into(),
                json,
                serde_json::to_string(&r).unwrap_or_default(),
            )
        }

        // ---- 404 ----
        _ => {
            let r = ApiResponse::<String> {
                success: false,
                data: None,
                error: Some(format!("Not found: {path}")),
            };
            (
                "404 Not Found".into(),
                json,
                serde_json::to_string(&r).unwrap_or_default(),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// API handlers
// ---------------------------------------------------------------------------

fn list_examples() -> ApiResponse<Vec<ExampleFile>> {
    let dir = Path::new(EXAMPLES_DIR);
    if !dir.is_dir() {
        return ApiResponse {
            success: false,
            data: None,
            error: Some("Examples directory not found".into()),
        };
    }
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "php").unwrap_or(false) {
                let name = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let content = std::fs::read_to_string(&p).unwrap_or_default();
                files.push(ExampleFile {
                    name: name.clone(),
                    path: p.to_string_lossy().to_string(),
                    content,
                });
            }
        }
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    ApiResponse {
        success: true,
        data: Some(files),
        error: None,
    }
}

fn get_example(name: &str) -> ApiResponse<ExampleFile> {
    let p = Path::new(EXAMPLES_DIR).join(name);
    if !p.exists() || !p.extension().map(|e| e == "php").unwrap_or(false) {
        return ApiResponse {
            success: false,
            data: None,
            error: Some(format!("Example not found: {name}")),
        };
    }
    let content = std::fs::read_to_string(&p).unwrap_or_default();
    ApiResponse {
        success: true,
        data: Some(ExampleFile {
            name: name.to_string(),
            path: p.to_string_lossy().to_string(),
            content,
        }),
        error: None,
    }
}

fn handle_execute(body: &str) -> ApiResponse<ExecResult> {
    let req: ExecRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Invalid JSON: {e}")),
            };
        }
    };

    let (code, filename) = if let Some(ref f) = req.filename {
        let fpath = Path::new(EXAMPLES_DIR).join(f);
        match std::fs::read_to_string(&fpath) {
            Ok(c) => (c, Some(f.clone())),
            Err(e) => {
                return ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to read file: {e}")),
                };
            }
        }
    } else if let Some(ref c) = req.code {
        (c.clone(), None)
    } else {
        return ApiResponse {
            success: false,
            data: None,
            error: Some("Provide 'code' or 'filename'".into()),
        };
    };

    // Compile
    let t0 = std::time::Instant::now();
    let fname = filename.as_deref().unwrap_or("inline.php");
    let op_array = match compile_string(&code, fname) {
        Ok(oa) => oa,
        Err(e) => {
            return ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Compile error: {e}")),
            };
        }
    };
    let compile_time_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let opcodes: Vec<OpcodeInfo> = op_array
        .ops
        .iter()
        .enumerate()
        .map(|(i, op)| OpcodeInfo {
            index: i,
            opcode: format!("{:?}", op.opcode),
            extended_value: op.extended_value,
        })
        .collect();

    // Execute
    let _ = php_output_start();
    let t1 = std::time::Instant::now();
    let mut exec_data = ExecuteData::new();
    let _result = execute_ex(&mut exec_data, &op_array);
    let exec_time_ms = t1.elapsed().as_secs_f64() * 1000.0;
    let output = php_output_end().unwrap_or_default();

    ApiResponse {
        success: true,
        data: Some(ExecResult {
            output,
            opcodes,
            filename,
            compile_time_ms,
            exec_time_ms,
        }),
        error: None,
    }
}

// ---------------------------------------------------------------------------
// Embedded HTML UI — Carbon-inspired dark/light theme, i18n, code editor
// ---------------------------------------------------------------------------

const INDEX_HTML: &str = r##"<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>PHP-RS Playground</title>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/codemirror.min.css">
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/theme/monokai.min.css">
<script src="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/codemirror.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/mode/xml/xml.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/mode/javascript/javascript.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/mode/css/css.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/mode/htmlmixed/htmlmixed.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/mode/clike/clike.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/mode/php/php.min.js"></script>
<style>
/* ---- CSS Reset & Carbon-inspired tokens ---- */
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
:root{
  --cds-background:#161616;--cds-layer:#262626;--cds-layer-hover:#353535;
  --cds-field:#393939;--cds-border-subtle:#525252;--cds-border-strong:#6f6f6f;
  --cds-text-primary:#f4f4f4;--cds-text-secondary:#c6c6c6;--cds-text-placeholder:#6f6f6f;
  --cds-link:#78a9ff;--cds-interactive:#4589ff;--cds-interactive-hover:#6ea6ff;
  --cds-support-success:#42be65;--cds-support-error:#fa4d56;--cds-support-warning:#f1c21b;
  --cds-focus:#ffffff;--cds-highlight:#002d9c;
  --cds-spacing-03:0.5rem;--cds-spacing-05:1rem;--cds-spacing-07:2rem;
  --font-mono:'IBM Plex Mono',ui-monospace,SFMono-Regular,Menlo,monospace;
  --font-sans:'IBM Plex Sans',-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;
  --radius:4px;
}
[data-theme="light"]{
  --cds-background:#ffffff;--cds-layer:#f4f4f4;--cds-layer-hover:#e8e8e8;
  --cds-field:#ffffff;--cds-border-subtle:#e0e0e0;--cds-border-strong:#8d8d8d;
  --cds-text-primary:#161616;--cds-text-secondary:#525252;--cds-text-placeholder:#a8a8a8;
  --cds-link:#0f62fe;--cds-interactive:#0f62fe;--cds-interactive-hover:#0353e9;
  --cds-support-success:#198038;--cds-support-error:#da1e28;--cds-support-warning:#f1c21b;
  --cds-focus:#0f62fe;--cds-highlight:#d0e2ff;
}
html{font-size:14px}
body{font-family:var(--font-sans);background:var(--cds-background);color:var(--cds-text-primary);min-height:100vh}

/* ---- Header ---- */
.header{display:flex;align-items:center;justify-content:space-between;padding:0 var(--cds-spacing-05);height:48px;background:var(--cds-layer);border-bottom:1px solid var(--cds-border-subtle)}
.header h1{font-size:1rem;font-weight:600;display:flex;align-items:center;gap:8px}
.header h1 .tag{font-size:0.7rem;background:var(--cds-interactive);color:#fff;padding:2px 8px;border-radius:10px;font-weight:400}
.header-actions{display:flex;align-items:center;gap:8px}

/* ---- Buttons ---- */
.btn{display:inline-flex;align-items:center;gap:6px;padding:8px 16px;border:none;border-radius:var(--radius);font-family:var(--font-sans);font-size:0.875rem;cursor:pointer;transition:background 0.15s}
.btn-primary{background:var(--cds-interactive);color:#fff}
.btn-primary:hover{background:var(--cds-interactive-hover)}
.btn-ghost{background:transparent;color:var(--cds-text-primary);border:1px solid var(--cds-border-subtle)}
.btn-ghost:hover{background:var(--cds-layer-hover)}
.btn-sm{padding:4px 12px;font-size:0.8rem}
.btn-icon{width:32px;height:32px;padding:0;display:inline-flex;align-items:center;justify-content:center;border-radius:var(--radius);background:transparent;border:none;color:var(--cds-text-primary);cursor:pointer}
.btn-icon:hover{background:var(--cds-layer-hover)}

/* ---- Layout ---- */
.container{display:flex;height:calc(100vh - 48px)}
.sidebar{width:240px;background:var(--cds-layer);border-right:1px solid var(--cds-border-subtle);overflow-y:auto;flex-shrink:0}
.main{flex:1;display:flex;flex-direction:column;overflow:hidden}

/* ---- Sidebar ---- */
.sidebar-header{padding:12px 16px;font-size:0.75rem;font-weight:600;text-transform:uppercase;letter-spacing:0.32px;color:var(--cds-text-secondary);border-bottom:1px solid var(--cds-border-subtle)}
.sidebar-item{display:block;width:100%;padding:10px 16px;background:none;border:none;text-align:left;color:var(--cds-text-primary);font-family:var(--font-sans);font-size:0.875rem;cursor:pointer;border-left:3px solid transparent;transition:all 0.15s}
.sidebar-item:hover{background:var(--cds-layer-hover)}
.sidebar-item.active{border-left-color:var(--cds-interactive);background:var(--cds-layer-hover);font-weight:600}

/* ---- Editor ---- */
.editor-area{flex:1;display:flex;flex-direction:column;min-height:0}
.editor-toolbar{display:flex;align-items:center;justify-content:space-between;padding:8px 16px;border-bottom:1px solid var(--cds-border-subtle);background:var(--cds-layer)}
.editor-toolbar .filename{font-family:var(--font-mono);font-size:0.8rem;color:var(--cds-text-secondary)}
.editor-wrapper{flex:1;display:flex;min-height:0}
.CodeMirror{flex:1;height:100%;font-family:var(--font-mono);font-size:0.875rem;line-height:1.6}
.CodeMirror-selected{background:var(--cds-highlight)}
.CodeMirror-cursor{border-left:2px solid var(--cds-interactive)}
.CodeMirror-gutters{background:var(--cds-layer);border-right:1px solid var(--cds-border-subtle)}

/* ---- Output panel ---- */
.output-panel{border-top:1px solid var(--cds-border-subtle);background:var(--cds-layer);display:flex;flex-direction:column;height:40%}
.output-tabs{display:flex;border-bottom:1px solid var(--cds-border-subtle)}
.output-tab{padding:8px 16px;background:none;border:none;border-bottom:2px solid transparent;color:var(--cds-text-secondary);font-family:var(--font-sans);font-size:0.8rem;cursor:pointer;transition:all 0.15s}
.output-tab:hover{color:var(--cds-text-primary)}
.output-tab.active{color:var(--cds-text-primary);border-bottom-color:var(--cds-interactive);font-weight:600}
.output-content{flex:1;overflow:auto;padding:16px;font-family:var(--font-mono);font-size:0.8rem;line-height:1.6;white-space:pre-wrap;word-break:break-all}
.output-content.error{color:var(--cds-support-error)}

/* ---- Opcode table ---- */
.opcode-table{width:100%;border-collapse:collapse;font-size:0.8rem}
.opcode-table th{text-align:left;padding:6px 12px;border-bottom:1px solid var(--cds-border-subtle);color:var(--cds-text-secondary);font-weight:600}
.opcode-table td{padding:6px 12px;border-bottom:1px solid var(--cds-border-subtle)}
.opcode-table tr:hover td{background:var(--cds-layer-hover)}

/* ---- Status bar ---- */
.status-bar{display:flex;align-items:center;justify-content:space-between;padding:4px 16px;font-size:0.75rem;color:var(--cds-text-secondary);background:var(--cds-layer);border-top:1px solid var(--cds-border-subtle)}
.status-dot{display:inline-block;width:8px;height:8px;border-radius:50%;margin-right:6px}
.status-dot.ok{background:var(--cds-support-success)}
.status-dot.err{background:var(--cds-support-error)}
.status-dot.loading{background:var(--cds-support-warning);animation:pulse 1s infinite}
@keyframes pulse{0%,100%{opacity:1}50%{opacity:0.4}}

/* ---- Responsive ---- */
@media(max-width:768px){.sidebar{display:none}.container{flex-direction:column}}

/* ---- Scrollbar ---- */
::-webkit-scrollbar{width:8px;height:8px}
::-webkit-scrollbar-track{background:var(--cds-background)}
::-webkit-scrollbar-thumb{background:var(--cds-border-strong);border-radius:4px}
</style>
</head>
<body>

<div class="header">
  <h1>
    <svg width="20" height="20" viewBox="0 0 32 32" fill="currentColor"><path d="M16 2a14 14 0 1 0 14 14A14 14 0 0 0 16 2zm0 26a12 12 0 1 1 12-12 12 12 0 0 1-12 12z"/><path d="M11.5 11h-2v10h2v-4h3l2 4h2.3l-2.2-4.4A3.5 3.5 0 0 0 11.5 11zm0 5v-3.5a1.5 1.5 0 0 1 3 0V16z"/></svg>
    <span data-i18n="title">PHP-RS Playground</span>
    <span class="tag">v0.1.0</span>
  </h1>
  <div class="header-actions">
    <select id="lang-select" class="btn btn-ghost btn-sm" title="Language">
      <option value="en">EN</option>
      <option value="zh">中文</option>
      <option value="ja">日本語</option>
    </select>
    <button class="btn-icon" id="theme-toggle" title="Toggle theme">
      <svg id="theme-icon" width="16" height="16" viewBox="0 0 32 32" fill="currentColor"><path d="M16 12a4 4 0 1 1-4 4 4 4 0 0 1 4-4m0-2a6 6 0 1 0 6 6 6 6 0 0 0-6-6zM5.4 6.8l1.4 1.4L5.4 9.6 4 8.2zM2 15h3v2H2zm3.4 10.8 1.4-1.4 1.4 1.4-1.4 1.4zM15 27h2v3h-2zm9.2-1.6 1.4-1.4 1.4 1.4-1.4 1.4zM27 15h3v2h-3zm-1.8-8.2 1.4-1.4 1.4 1.4-1.4 1.4zM15 2h2v3h-2z"/></svg>
    </button>
  </div>
</div>

<div class="container">
  <div class="sidebar" id="sidebar">
    <div class="sidebar-header" data-i18n="examples">Examples</div>
    <div id="example-list"></div>
  </div>

  <div class="main">
    <div class="editor-area">
      <div class="editor-toolbar">
        <span class="filename" id="current-file" data-i18n="untitled">untitled.php</span>
        <div style="display:flex;gap:8px">
          <button class="btn btn-ghost btn-sm" id="btn-clear" data-i18n="clear">Clear</button>
          <button class="btn btn-primary btn-sm" id="btn-run" data-i18n="run">&#9654; Run</button>
        </div>
      </div>
      <div class="editor-wrapper">
        <textarea id="editor" spellcheck="false"></textarea>
      </div>
    </div>

    <div class="output-panel">
      <div class="output-tabs">
        <button class="output-tab active" data-tab="output" data-i18n="output">Output</button>
        <button class="output-tab" data-tab="opcodes" data-i18n="opcodes">Opcodes</button>
      </div>
      <div class="output-content" id="output-pane"></div>
    </div>

    <div class="status-bar">
      <div><span class="status-dot ok" id="status-dot"></span><span id="status-text" data-i18n="ready">Ready</span></div>
      <div id="timing"></div>
    </div>
  </div>
</div>

<script>
// ---- i18n ----
const I18N = {
  en: { title:"PHP-RS Playground", examples:"Examples", untitled:"untitled.php", clear:"Clear", run:"\u25b6 Run", output:"Output", opcodes:"Opcodes", ready:"Ready", running:"Running...", compiled:"Compiled", error:"Error", compile_ms:"Compile: {0}ms", exec_ms:"Exec: {0}ms" },
  zh: { title:"PHP-RS \u6f14\u7ec3\u573a", examples:"\u793a\u4f8b", untitled:"\u672a\u547d\u540d.php", clear:"\u6e05\u9664", run:"\u25b6 \u8fd0\u884c", output:"\u8f93\u51fa", opcodes:"\u64cd\u4f5c\u7801", ready:"\u5c31\u7eea", running:"\u8fd0\u884c\u4e2d...", compiled:"\u5df2\u7f16\u8bd1", error:"\u9519\u8bef", compile_ms:"\u7f16\u8bd1: {0}ms", exec_ms:"\u6267\u884c: {0}ms" },
  ja: { title:"PHP-RS \u30d7\u30ec\u30a4\u30b0\u30e9\u30a6\u30f3\u30c9", examples:"\u4f8b", untitled:"\u7121\u984c.php", clear:"\u30af\u30ea\u30a2", run:"\u25b6 \u5b9f\u884c", output:"\u51fa\u529b", opcodes:"\u30aa\u30d7\u30b3\u30fc\u30c9", ready:"\u6e96\u5099\u5b8c\u4e86", running:"\u5b9f\u884c\u4e2d...", compiled:"\u30b3\u30f3\u30d1\u30a4\u30eb\u6e08\u307f", error:"\u30a8\u30e9\u30fc", compile_ms:"\u30b3\u30f3\u30d1\u30a4\u30eb: {0}ms", exec_ms:"\u5b9f\u884c: {0}ms" }
};
let lang = localStorage.getItem('phprs-lang') || 'en';
function t(key, ...args) {
  let s = (I18N[lang] || I18N.en)[key] || key;
  args.forEach((a,i) => s = s.replace('{'+i+'}', a));
  return s;
}
function applyI18n() {
  document.querySelectorAll('[data-i18n]').forEach(el => {
    const k = el.getAttribute('data-i18n');
    if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') el.placeholder = t(k);
    else el.textContent = t(k);
  });
}
document.getElementById('lang-select').value = lang;
document.getElementById('lang-select').addEventListener('change', e => {
  lang = e.target.value;
  localStorage.setItem('phprs-lang', lang);
  applyI18n();
});

// ---- Theme ----
let theme = localStorage.getItem('phprs-theme') || 'dark';
document.documentElement.setAttribute('data-theme', theme);
document.getElementById('theme-toggle').addEventListener('click', () => {
  theme = theme === 'dark' ? 'light' : 'dark';
  document.documentElement.setAttribute('data-theme', theme);
  localStorage.setItem('phprs-theme', theme);
  if (editor) editor.setOption('theme', theme === 'dark' ? 'monokai' : 'default');
});

// ---- State ----
const textarea = document.getElementById('editor');
let editor;
const outputPane = document.getElementById('output-pane');
const statusDot = document.getElementById('status-dot');
const statusText = document.getElementById('status-text');
const timing = document.getElementById('timing');
const currentFile = document.getElementById('current-file');
let activeTab = 'output';
let lastResult = null;

// ---- Tab switching ----
document.querySelectorAll('.output-tab').forEach(tab => {
  tab.addEventListener('click', () => {
    document.querySelectorAll('.output-tab').forEach(t => t.classList.remove('active'));
    tab.classList.add('active');
    activeTab = tab.getAttribute('data-tab');
    renderOutput();
  });
});

function renderOutput() {
  if (!lastResult) { outputPane.textContent = ''; return; }
  outputPane.classList.remove('error');
  if (!lastResult.success) {
    outputPane.classList.add('error');
    outputPane.textContent = lastResult.error || t('error');
    return;
  }
  const d = lastResult.data;
  if (activeTab === 'output') {
    outputPane.textContent = d.output || '(no output)';
  } else {
    if (!d.opcodes || d.opcodes.length === 0) {
      outputPane.textContent = '(no opcodes)';
      return;
    }
    let html = '<table class="opcode-table"><thead><tr><th>#</th><th>Opcode</th><th>Ext</th></tr></thead><tbody>';
    d.opcodes.forEach(op => {
      html += '<tr><td>'+op.index+'</td><td>'+op.opcode+'</td><td>'+op.extended_value+'</td></tr>';
    });
    html += '</tbody></table>';
    outputPane.innerHTML = html;
  }
}

// ---- Load examples ----
async function loadExamples() {
  try {
    const res = await fetch('/api/examples');
    const json = await res.json();
    if (!json.success) return;
    const list = document.getElementById('example-list');
    list.innerHTML = '';
    json.data.forEach(ex => {
      const btn = document.createElement('button');
      btn.className = 'sidebar-item';
      btn.textContent = ex.name;
      btn.addEventListener('click', () => {
        document.querySelectorAll('.sidebar-item').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        editor.setValue(ex.content);
        currentFile.textContent = ex.name;
      });
      list.appendChild(btn);
    });
  } catch(e) { console.error('Failed to load examples', e); }
}

// ---- Execute ----
async function executeCode() {
  const code = editor.getValue().trim();
  if (!code) return;
  statusDot.className = 'status-dot loading';
  statusText.textContent = t('running');
  timing.textContent = '';
  try {
    const res = await fetch('/api/execute', {
      method: 'POST',
      headers: {'Content-Type':'application/json'},
      body: JSON.stringify({code})
    });
    lastResult = await res.json();
    if (lastResult.success && lastResult.data) {
      statusDot.className = 'status-dot ok';
      statusText.textContent = t('compiled');
      const d = lastResult.data;
      timing.textContent = t('compile_ms', d.compile_time_ms.toFixed(2)) + '  |  ' + t('exec_ms', d.exec_time_ms.toFixed(2));
    } else {
      statusDot.className = 'status-dot err';
      statusText.textContent = t('error');
    }
  } catch(e) {
    lastResult = {success:false, error: e.message};
    statusDot.className = 'status-dot err';
    statusText.textContent = t('error');
  }
  renderOutput();
}

// ---- Initialize CodeMirror ----
editor = CodeMirror.fromTextArea(textarea, {
  mode: 'php',
  theme: theme === 'dark' ? 'monokai' : 'default',
  lineNumbers: true,
  autofocus: true,
  indentUnit: 4,
  tabSize: 4,
  lineWrapping: true,
  extraKeys: {
    'Ctrl-Enter': executeCode,
    'Cmd-Enter': executeCode
  }
});
editor.on('change', function() { editor.save(); });

// ---- Events ----
document.getElementById('btn-run').addEventListener('click', executeCode);
document.getElementById('btn-clear').addEventListener('click', () => {
  editor.setValue('');
  lastResult = null;
  outputPane.textContent = '';
  timing.textContent = '';
  statusDot.className = 'status-dot ok';
  statusText.textContent = t('ready');
  currentFile.textContent = t('untitled');
  document.querySelectorAll('.sidebar-item').forEach(b => b.classList.remove('active'));
});

// ---- Init ----
applyI18n();
loadExamples();
</script>
</body>
</html>
"##;
