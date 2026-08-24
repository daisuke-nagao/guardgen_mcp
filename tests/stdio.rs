// SPDX-FileCopyrightText: 2026 Daisuke Nagao
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

struct McpProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: Receiver<std::io::Result<String>>,
}

impl McpProcess {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_guardgen_mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn guardgen_mcp");
        let stdin = child.stdin.take().expect("child stdin");
        let child_stdout = child.stdout.take().expect("child stdout");
        let (stdout_sender, stdout) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(child_stdout).lines() {
                if stdout_sender.send(line).is_err() {
                    break;
                }
            }
        });
        let mut process = Self {
            child,
            stdin,
            stdout,
        };
        let initialize = process.request(
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "guardgen-mcp-tests", "version": "0.1.0"}
            }),
        );
        assert!(
            initialize.get("result").is_some(),
            "initialize: {initialize}"
        );
        assert_eq!(initialize["result"]["serverInfo"]["name"], "guardgen_mcp");
        assert_eq!(initialize["result"]["serverInfo"]["version"], "0.1.0");
        let instructions = initialize["result"]["instructions"]
            .as_str()
            .expect("server instructions");
        assert!(instructions.contains("scaffold"));
        assert!(instructions.contains("between"));
        assert!(instructions.contains("opening"));
        assert!(instructions.contains("closing"));
        assert!(!instructions.contains("both lines"));
        assert!(!instructions.contains("collision-proof"));
        process.notify("notifications/initialized", json!({}));
        process
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.write_message(json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    fn request(&mut self, id: i64, method: &str, params: Value) -> Value {
        self.write_message(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }));
        loop {
            let response = self.read_message();
            if response.get("id") == Some(&json!(id)) {
                return response;
            }
        }
    }

    fn write_message(&mut self, message: Value) {
        serde_json::to_writer(&mut self.stdin, &message).expect("encode JSON-RPC request");
        self.stdin.write_all(b"\n").expect("write JSON-RPC request");
        self.stdin.flush().expect("flush JSON-RPC request");
    }

    fn read_message(&mut self) -> Value {
        let line = self
            .stdout
            .recv_timeout(Duration::from_secs(10))
            .expect("receive JSON-RPC response within 10 seconds")
            .expect("read JSON-RPC response");
        assert!(
            !line.is_empty(),
            "server exited before returning JSON: {line:?}"
        );
        serde_json::from_str(&line)
            .unwrap_or_else(|error| panic!("STDOUT line is not JSON ({error}): {line:?}"))
    }

    fn call(&mut self, id: i64, arguments: Value) -> Value {
        self.request(
            id,
            "tools/call",
            json!({"name": "generate_include_guard", "arguments": arguments}),
        )
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn text_result(response: &Value) -> &str {
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("missing text content: {response}"))
}

fn guard_name(text: &str) -> &str {
    text.lines()
        .next()
        .and_then(|line| line.strip_prefix("#ifndef "))
        .expect("#ifndef guard line")
}

#[test]
fn stdio_initialize_lists_one_tool_and_defaults_generate_v7() {
    let mut process = McpProcess::start();
    let listed = process.request(2, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "generate_include_guard");
    let tool_description = tools[0]["description"].as_str().expect("tool description");
    assert!(!tool_description.contains("invariant when the header is renamed or moved"));
    let schema = &tools[0]["inputSchema"];
    let properties = schema["properties"].as_object().expect("schema properties");
    for property in [
        "prefix",
        "suffix",
        "language",
        "line_ending",
        "uuid_version",
    ] {
        assert!(
            properties.contains_key(property),
            "missing schema property: {property}"
        );
    }
    assert!(!properties.contains_key("output"));
    assert!(!properties.contains_key("overwrite"));
    assert!(schema.get("required").is_none());
    assert_eq!(properties["prefix"]["default"], "UUID");
    assert_eq!(properties["suffix"]["default"], Value::Null);
    assert_eq!(properties["language"]["default"], "none");
    assert_eq!(properties["line_ending"]["default"], "none");
    assert_eq!(properties["uuid_version"]["default"], "v7");
    let schema_text = schema.to_string();
    for value in ["none", "c", "cxx", "lf", "crlf", "v7", "v4"] {
        assert!(
            schema_text.contains(&format!("\"{value}\"")),
            "missing enum value {value}"
        );
    }
    let language_description = schema["$defs"]["Language"]["description"]
        .as_str()
        .expect("language description");
    assert!(language_description.contains("extern"));
    assert!(language_description.contains("linkage"));
    assert!(language_description.contains("none"));
    assert!(language_description.contains("cxx"));
    assert!(!language_description.contains("no trailing comment"));

    let prefix_description = properties["prefix"]["description"]
        .as_str()
        .expect("prefix description");
    assert!(!prefix_description.contains("rename churn"));
    assert!(!prefix_description.contains("cross-file collisions"));

    let first = process.call(3, json!({}));
    let first_text = text_result(&first);
    let first_name = guard_name(first_text);
    assert!(first_name.starts_with("UUID_"));
    assert_eq!(first_name.split('_').count(), 6);
    assert_eq!(
        first_name.split('_').nth(3).unwrap().chars().next(),
        Some('7')
    );

    let second = process.call(4, json!({}));
    assert_ne!(guard_name(text_result(&second)), first_name);
}

#[test]
fn uuid_version_and_suffix_are_forwarded_to_guardgen() {
    let mut process = McpProcess::start();
    let v4 = process.call(
        2,
        json!({"prefix": "MY_GUARD", "suffix": "9_SUFFIX", "uuid_version": "v4"}),
    );
    let v4_name = guard_name(text_result(&v4));
    assert!(v4_name.starts_with("MY_GUARD_"));
    assert!(v4_name.ends_with("_9_SUFFIX"));
    assert_eq!(v4_name.split('_').nth(4).unwrap().chars().next(), Some('4'));
}

#[test]
fn every_language_and_line_ending_mapping_is_exposed() {
    let mut process = McpProcess::start();
    for (id, language, has_extern_c) in [(2, "none", false), (3, "c", true), (4, "cxx", false)] {
        for line_ending in ["lf", "crlf", "none"] {
            let response = process.call(
                id,
                json!({
                    "prefix": "MAP",
                    "language": language,
                    "line_ending": line_ending,
                }),
            );
            let text = text_result(&response);
            assert_eq!(text.contains("extern \"C\""), has_extern_c);
            let expect_crlf = line_ending == "crlf" || (line_ending == "none" && cfg!(windows));
            if expect_crlf {
                assert!(text.contains("\r\n"));
            } else {
                assert!(text.contains('\n'));
                assert!(!text.contains("\r\n"));
            }
        }
    }
}

#[test]
fn invalid_semantic_arguments_and_unknown_fields_are_invalid_params() {
    let mut process = McpProcess::start();
    for arguments in [
        json!({"prefix": ""}),
        json!({"prefix": "9PREFIX"}),
        json!({"prefix": "pr\u{e9}fix"}),
        json!({"prefix": "OK", "suffix": ""}),
        json!({"prefix": "OK", "suffix": "bad-suffix"}),
        json!({"prefix": "OK", "suffix": "\u{e9}"}),
        json!({"prefix": "OK", "language": "rust"}),
        json!({"prefix": "OK", "unknown": true}),
    ] {
        let response = process.call(2, arguments);
        assert_eq!(response["error"]["code"], -32602, "response: {response}");
    }
}
