use std::io::{self, BufRead, Read, Write};
use serde_json::{json, Value};

fn main() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());
    let mut stdout = io::stdout();

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break;
        }

        let line = line.trim();
        if !line.starts_with("Content-Length:") {
            continue;
        }

        let len: usize = line["Content-Length:".len()..].trim().parse()?;

        // Read the empty line separator \r\n
        let mut blank = String::new();
        reader.read_line(&mut blank)?;

        // Read the JSON payload
        let mut body = vec![0u8; len];
        reader.read_exact(&mut body)?;

        let msg: Value = serde_json::from_slice(&body)?;
        handle_message(&msg, &mut stdout)?;
    }

    Ok(())
}

fn handle_message(msg: &Value, out: &mut io::Stdout) -> anyhow::Result<()> {
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = msg.get("id");

    match method {
        "initialize" => {
            let res = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "capabilities": {
                        "textDocumentSync": 1, // Full sync
                        "completionProvider": {
                            "resolveProvider": false,
                            "triggerCharacters": [".", ":"]
                        },
                        "hoverProvider": true
                    },
                    "serverInfo": {
                        "name": "york-lsp",
                        "version": "1.1.1"
                    }
                }
            });
            send_response(out, &res)?;
        }
        "initialized" => {
            // Client confirmed initialization
        }
        "textDocument/didOpen" | "textDocument/didChange" => {
            let params = msg.get("params");
            let uri = params.and_then(|p| p.get("textDocument")).and_then(|t| t.get("uri")).and_then(|u| u.as_str()).unwrap_or("");
            
            let text = if method == "textDocument/didOpen" {
                params.and_then(|p| p.get("textDocument")).and_then(|t| t.get("text")).and_then(|s| s.as_str()).unwrap_or("")
            } else {
                params.and_then(|p| p.get("contentChanges"))
                      .and_then(|c| c.as_array())
                      .and_then(|a| a.first())
                      .and_then(|ch| ch.get("text"))
                      .and_then(|s| s.as_str()).unwrap_or("")
            };

            if !uri.is_empty() && !text.is_empty() {
                publish_diagnostics(out, uri, text)?;
            }
        }
        "textDocument/completion" => {
            let completions = get_completions();
            let res = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": completions
            });
            send_response(out, &res)?;
        }
        "textDocument/hover" => {
            let res = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "contents": {
                        "kind": "markdown",
                        "value": "**York Systems Language (v1.1.1)**\n\nFast, zero-overhead C11 compiled systems language."
                    }
                }
            });
            send_response(out, &res)?;
        }
        "shutdown" => {
            let res = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": null
            });
            send_response(out, &res)?;
        }
        "exit" => {
            std::process::exit(0);
        }
        _ => {
            // For unhandled requests with an ID, return method not found
            if let Some(id) = id {
                let res = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": "Method not found"
                    }
                });
                send_response(out, &res)?;
            }
        }
    }

    Ok(())
}

/// Convert a byte offset (u32) into LSP line/character using the source text.
fn position_at(source: &str, byte: u32) -> (u64, u64) {
    let mut line = 0u64;
    let mut char_at_line = 0u64;
    let mut seen = 0usize;
    for (i, c) in source.char_indices() {
        if (i as u32) >= byte {
            break;
        }
        if c == '\n' {
            line += 1;
            char_at_line = 0;
            seen = i + c.len_utf8();
        }
    }
    // Count characters on the final line up to the byte offset.
    if (seen as u32) < byte {
        let tail = &source[seen..(byte as usize).min(source.len())];
        char_at_line = tail.chars().count() as u64;
    }
    (line, char_at_line)
}

fn range_from_span(source: &str, lo: u32, hi: u32) -> Value {
    let (sl, sc) = position_at(source, lo);
    let (el, ec) = position_at(source, hi);
    json!({
        "start": { "line": sl, "character": sc },
        "end": { "line": el, "character": ec }
    })
}

fn publish_diagnostics(out: &mut io::Stdout, uri: &str, text: &str) -> anyhow::Result<()> {
    let mut diagnostics = Vec::new();

    // 1. Run Lexer
    let lexed = york_lexer::lex(text);
    for e in &lexed.errors {
        diagnostics.push(json!({
            "range": range_from_span(text, e.span.lo.0, e.span.hi.0),
            "severity": 1, // Error
            "source": "york-lexer",
            "message": format!("{}: {}", e.error, e.span)
        }));
    }

    // 2. Run Parser if no lexer errors
    if lexed.errors.is_empty() {
        let parsed = york_parser::parse(&lexed.tokens);
        for e in &parsed.errors {
            diagnostics.push(json!({
                "range": range_from_span(text, e.span.lo.0, e.span.hi.0),
                "severity": 1,
                "source": "york-parser",
                "message": format!("{}: {}", e.error, e.span)
            }));
        }

        // 3. Run Semantic Analyzer if parser succeeded
        if parsed.errors.is_empty() {
            let sema = york_sema::analyze(&parsed.program);
            for e in &sema.errors {
                diagnostics.push(json!({
                    "range": range_from_span(text, e.span.lo.0, e.span.hi.0),
                    "severity": 1,
                    "source": "york-sema",
                    "message": format!("{}: {}", e.error, e.span)
                }));
            }
        }
    }

    let notification = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "diagnostics": diagnostics
        }
    });

    send_response(out, &notification)
}

fn get_completions() -> Vec<Value> {
    let keywords = [
        ("fn", "Function declaration", 14),
        ("struct", "Struct declaration", 7),
        ("impl", "Method implementation block", 7),
        ("enum", "Enumeration definition", 13),
        ("switch", "Switch statement (no fallthrough)", 15),
        ("case", "Switch case arm", 15),
        ("default", "Switch default arm", 15),
        ("pub", "Public visibility modifier", 14),
        ("Arena", "Generic contiguous memory arena", 7),
        ("HashMap", "Built-in generic key-value dictionary", 7),
        ("Result", "Error handling sum type (Ok | Err)", 13),
        ("Option", "Optional value sum type (Some | None)", 13),
        ("String", "Heap-allocated UTF-8 string", 7),
        ("println", "Print to standard output with newline", 3),
        ("print", "Print to standard output", 3),
        ("read_line", "Read line from standard input", 3),
        ("math_abs", "Compute absolute value", 3),
        ("math_sqrt", "Compute square root", 3),
        ("math_pow", "Compute power (base^exp)", 3),
        ("net_listen", "Open listening TCP socket", 3),
        ("net_connect", "Connect to remote TCP host:port", 3),
        ("thread_spawn", "Spawn native OS thread", 3),
        ("window_create", "Create native Win32 window", 3),
    ];

    keywords.iter().map(|(label, detail, kind)| {
        json!({
            "label": label,
            "kind": kind,
            "detail": detail,
            "insertText": label
        })
    }).collect()
}

fn send_response(out: &mut io::Stdout, res: &Value) -> anyhow::Result<()> {
    let body = serde_json::to_string(res)?;
    write!(out, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    out.flush()?;
    Ok(())
}
