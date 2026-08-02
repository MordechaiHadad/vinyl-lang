use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

struct LspProcess {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl LspProcess {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vinyl-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input,
            output,
        }
    }

    fn send(&mut self, message: Value) {
        let body = serde_json::to_vec(&message).unwrap();
        write!(self.input, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        self.input.write_all(&body).unwrap();
        self.input.flush().unwrap();
    }

    fn receive(&mut self) -> Value {
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            self.output.read_line(&mut line).unwrap();
            assert!(!line.is_empty(), "LSP exited before sending a response");
            if line == "\r\n" {
                break;
            }
            if let Some(length) = line.strip_prefix("Content-Length: ") {
                content_length = length.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; content_length];
        self.output.read_exact(&mut body).unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    fn response(&mut self, id: u64) -> Value {
        loop {
            let message = self.receive();
            if message.get("id") == Some(&json!(id)) {
                return message;
            }
        }
    }

    fn notification(&mut self, method: &str) -> Value {
        loop {
            let message = self.receive();
            if message.get("method") == Some(&json!(method)) {
                return message;
            }
        }
    }

    fn diagnostics_for(&mut self, uri: &str) -> Value {
        loop {
            let message = self.notification("textDocument/publishDiagnostics");
            if message["params"]["uri"] == uri {
                return message;
            }
        }
    }
}

impl Drop for LspProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

struct TestProject {
    root: PathBuf,
    main: PathBuf,
    math: PathBuf,
}

impl TestProject {
    fn new() -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vinyl_lsp_test_{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        let main = root.join("main.vn");
        let math = root.join("math.vn");
        std::fs::write(
            &main,
            "import math;\nfn main(): int {\n    math::answer()\n}\n",
        )
        .unwrap();
        std::fs::write(&math, "public fn answer(): int { 42 }\n").unwrap();
        Self { root, main, math }
    }

    fn uri(path: &PathBuf) -> String {
        tower_lsp::lsp_types::Url::from_file_path(path)
            .unwrap()
            .to_string()
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn serves_core_lsp_features_over_stdio() {
    let project = TestProject::new();
    let main_uri = TestProject::uri(&project.main);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "rootUri": root_uri,
            "capabilities": {}
        }
    }));
    let initialize = lsp.response(1);
    assert_eq!(initialize["result"]["capabilities"]["hoverProvider"], true);
    assert_eq!(
        initialize["result"]["capabilities"]["definitionProvider"],
        true
    );

    lsp.send(json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    }));
    lsp.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": math_uri,
                "languageId": "vinyl",
                "version": 1,
                "text": "public fn answer(): int { 42 }\n"
            }
        }
    }));
    assert!(
        lsp.notification("textDocument/publishDiagnostics")["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    lsp.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": main_uri,
                "languageId": "vinyl",
                "version": 1,
                "text": "import math;\nfn main(): int {\n    math::answer()\n}\n"
            }
        }
    }));
    assert!(
        lsp.notification("textDocument/publishDiagnostics")["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 10 }
        }
    }));
    let hover = lsp.response(2);
    let hover_text = hover["result"]["contents"].as_str().unwrap();
    assert!(
        hover_text.contains("fn answer(): int"),
        "hover should show the function signature, got: {hover_text}"
    );
    assert!(
        hover_text.contains("math"),
        "hover should mention the source module, got: {hover_text}"
    );

    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 10 }
        }
    }));
    let definition = lsp.response(3);
    assert_eq!(definition["result"]["uri"], math_uri);

    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "textDocument/formatting",
        "params": {
            "textDocument": { "uri": main_uri },
            "options": { "tabSize": 4, "insertSpaces": true }
        }
    }));
    let formatting = lsp.response(4);
    assert_eq!(
        formatting["result"][0]["range"]["start"],
        json!({"line": 0, "character": 0})
    );
    assert_eq!(
        formatting["result"][0]["newText"],
        "import math;\n\nfn main(): int {\n    math::answer()\n}\n"
    );

    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "textDocument/references",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 10 },
            "context": { "includeDeclaration": true }
        }
    }));
    assert!(!lsp.response(7)["result"].as_array().unwrap().is_empty());

    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "textDocument/rename",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 10 },
            "newName": "answer_new"
        }
    }));
    let rename = lsp.response(8);
    assert!(rename["result"]["changes"][&main_uri].is_array());
    assert!(!rename["result"]["changes"].as_object().unwrap().is_empty());

    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 9,
        "method": "textDocument/documentSymbol",
        "params": { "textDocument": { "uri": main_uri } }
    }));
    assert!(!lsp.response(9)["result"].as_array().unwrap().is_empty());

    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "textDocument/signatureHelp",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 10 }
        }
    }));
    assert!(
        !lsp.response(10)["result"]["signatures"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": { "uri": main_uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 3, "character": 1 }
            },
            "context": { "diagnostics": [] }
        }
    }));
    assert_eq!(lsp.response(11)["result"][0]["title"], "Format document");

    lsp.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": math_uri, "version": 2 },
            "contentChanges": [{ "text": "public fn answer(): int {\n" }]
        }
    }));
    // first notification: entry file (main_uri) — type errors from missing module exports
    let main_diag = lsp.notification("textDocument/publishDiagnostics");
    assert!(
        !main_diag["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    // second notification: math.vn — parse error
    let math_diag = lsp.notification("textDocument/publishDiagnostics");
    assert!(
        !math_diag["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    lsp.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": main_uri, "version": 2 },
            "contentChanges": [{ "text": "import math;\nfn main(): int {\n" }]
        }
    }));
    let diagnostics = lsp.notification("textDocument/publishDiagnostics");
    assert!(
        !diagnostics["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        diagnostics["params"]["diagnostics"][0]["range"]["end"]["line"]
            .as_u64()
            .is_some()
    );

    // completion returns type detail for local definitions in math.vn
    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": math_uri },
            "position": { "line": 0, "character": 11 }
        }
    }));
    let completion = lsp.response(12);
    let items = completion["result"].as_array().unwrap();
    let answer_item = items
        .iter()
        .find(|item| item["label"] == "answer")
        .expect("answer should be in completions");
    assert!(
        answer_item["detail"].as_str().is_some(),
        "completion item should have type detail"
    );
    assert!(
        answer_item["detail"].as_str().unwrap().contains("answer"),
        "detail should contain function signature"
    );

    // completion in main.vn also shows type detail for prefixed module functions
    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 13,
        "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 1, "character": 5 }
        }
    }));
    let completion_main = lsp.response(13);
    let main_items = completion_main["result"].as_array().unwrap();
    let math_answer = main_items
        .iter()
        .find(|item| item["label"] == "math::answer")
        .expect("math::answer should be in main completions");
    assert!(
        math_answer["detail"].as_str().is_some(),
        "math::answer should have type detail"
    );
    assert!(
        math_answer["detail"].as_str().unwrap().contains("answer"),
        "math::answer detail should contain signature"
    );

    // auto-import: create a self-contained module, open it, then request completion for its function
    // from a file that doesn't import it
    let utils_vn = project.root.join("utils.vn");
    std::fs::write(&utils_vn, "public fn helper(): int { 42 }\n").unwrap();
    let utils_uri = TestProject::uri(&utils_vn);
    lsp.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": utils_uri,
                "languageId": "vinyl",
                "version": 1,
                "text": "public fn helper(): int { 42 }\n"
            }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    // create a valid file that doesn't import utils and request completion at end
    let app_vn = project.root.join("app.vn");
    std::fs::write(&app_vn, "public fn run(): int {\n    0\n}\n").unwrap();
    let app_uri = TestProject::uri(&app_vn);
    lsp.send(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": app_uri,
                "languageId": "vinyl",
                "version": 1,
                "text": "public fn run(): int {\n    0\n}\n"
            }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    // completion at end of line 1 has prefix from line content
    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 14,
        "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": app_uri },
            "position": { "line": 0, "character": 0 }
        }
    }));
    let auto_import_completion = lsp.response(14);
    let auto_items = auto_import_completion["result"].as_array().unwrap();
    let auto_helper = auto_items
        .iter()
        .find(|item| item["label"].as_str().is_some_and(|l| l == "utils::helper"));
    assert!(
        auto_helper.is_some(),
        "utils::helper should appear as auto-import in app.vn"
    );
    if let Some(helper) = auto_helper {
        assert!(
            helper["additionalTextEdits"].as_array().is_some(),
            "auto-import completion should have additionalTextEdits"
        );
        assert!(
            helper["detail"].as_str().unwrap().contains("from parent::utils"),
            "auto-import detail should mention module: got {:?}",
            helper["detail"]
        );
        assert!(
            helper.get("textEdit").is_some(),
            "auto-import completion should have textEdit for qualified insertion"
        );
    }

    lsp.send(json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "shutdown",
        "params": null
    }));
    assert!(lsp.response(5)["result"].is_null());
}

#[test]
fn completion_module_ref() {
    let project = TestProject::new();
    let main_uri = TestProject::uri(&project.main);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "public fn answer(): int { 42 }\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "import math;\nfn main(): int {\n    math::\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    // update main.vn to have a partial after ::
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": main_uri, "version": 2 },
            "contentChanges": [{ "text": "import math;\nfn main(): int {\n    math::ans\n}\n" }]
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    // module-ref completion with partial "ans" after "math::"
    // should replace "ans" with "answer", not "math::answer"
    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 13 }
        }
    }));
    let resp = lsp.response(2);
    let items = resp["result"].as_array().unwrap();

    let answer = items.iter().find(|i| i["label"] == "answer")
        .expect("module-ref should offer answer");
    assert_eq!(answer["kind"], 3, "answer should be a function (kind=3)");
    assert!(answer["textEdit"].is_object(), "module-ref should have textEdit");

    assert!(answer["textEdit"].is_object(), "module-ref should have textEdit");
    let text_edit = &answer["textEdit"];
    assert_eq!(text_edit["newText"], "answer", "newText mismatch");
    assert_eq!(text_edit["range"]["start"], json!({"line": 2, "character": 10}));
    assert_eq!(text_edit["range"]["end"], json!({"line": 2, "character": 13}));

    // there should be NO auto-import "math::answer" since module is already imported
    let auto = items.iter().find(|i| i["label"] == "math::answer");
    assert!(auto.is_none(), "no auto-import when module is already imported");

    let _ = lsp;
}

#[test]
fn completion_colon_trigger_guard() {
    let project = TestProject::new();
    let main_uri = TestProject::uri(&project.main);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "import math;\nfn main(): int {\n    let x = \n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    // ":" pressed in a non-import, non-module context — should return empty
    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 10 },
            "context": { "triggerKind": 2, "triggerCharacter": ":" }
        }
    }));
    let resp = lsp.response(2);
    let result = &resp["result"];
    assert!(result.is_array(), "result should be an array");
    assert!(result.as_array().unwrap().is_empty(), ":: trigger in plain code should return empty");

    let _ = lsp;
}

#[test]
fn completion_colon_after_module() {
    let project = TestProject::new();
    let main_uri = TestProject::uri(&project.main);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "public fn answer(): int { 42 }\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    // file with "math:" (single colon) — simulating the first colon of "::"
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "import math;\nfn main(): int {\n    math:\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    // ":" triggered after a known module name — should NOT return empty
    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 9 },
            "context": { "triggerKind": 2, "triggerCharacter": ":" }
        }
    }));
    let resp = lsp.response(2);
    let items = resp["result"].as_array().unwrap();
    let answer = items.iter().find(|i| i["label"] == "math::answer");
    assert!(answer.is_some(), "\":\" after module name should show completions for answer");

    let _ = lsp;
}

#[test]
fn completion_no_duplicate_imported_colon() {
    let project = TestProject::new();
    let main_uri = TestProject::uri(&project.main);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "public fn answer(): int { 42 }\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "import math;\nfn main(): int {\n    math::\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": main_uri, "version": 2 },
            "contentChanges": [{ "text": "import math;\nfn main(): int {\n    math:\n}\n" }]
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 9 },
            "context": { "triggerKind": 2, "triggerCharacter": ":" }
        }
    }));
    let resp = lsp.response(2);
    let items = resp["result"].as_array().unwrap();
    let math_answer = items.iter().filter(|i| i["label"] == "math::answer").count();
    assert_eq!(math_answer, 1,
        "math: should offer math::answer exactly once, got: {:?}",
        items.iter().map(|i| i["label"].as_str().unwrap_or("")).collect::<Vec<_>>());

    let _ = lsp;
}

#[test]
fn completion_module_ref_reopen() {
    let project = TestProject::new();
    let main_uri = TestProject::uri(&project.main);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "public fn answer(): int { 42 }\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "import math;\nfn main(): int {\n    math::\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 10 }
        }
    }));
    let resp = lsp.response(2);
    let items = resp["result"].as_array().unwrap();
    let answer = items.iter().find(|i| i["label"] == "answer")
        .expect("keybind reopen at math:: should offer answer");
    assert_eq!(answer["kind"], 3, "answer should be a function (kind=3)");
    assert!(answer["textEdit"].is_object(), "module-ref should have textEdit");

    let _ = lsp;
}

#[test]
fn completion_colon_reopen() {
    let project = TestProject::new();
    let main_uri = TestProject::uri(&project.main);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "public fn answer(): int { 42 }\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "import math;\nfn main(): int {\n    math::\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    // simulate deleting last colon: "math:"
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": main_uri, "version": 2 },
            "contentChanges": [{ "text": "import math;\nfn main(): int {\n    math:\n}\n" }]
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    // colon trigger at "math:" — should show auto-import completions
    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 9 },
            "context": { "triggerKind": 2, "triggerCharacter": ":" }
        }
    }));
    let resp = lsp.response(2);
    let items = resp["result"].as_array().unwrap();
    let math_answer = items.iter().find(|i| i["label"] == "math::answer");
    assert!(math_answer.is_some(), "colon reopen should show math::answer, got: {:?}", items.iter().map(|i| i["label"].as_str()).collect::<Vec<_>>());

    // retype to "math::" and trigger ":" again
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": main_uri, "version": 3 },
            "contentChanges": [{ "text": "import math;\nfn main(): int {\n    math::\n}\n" }]
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 3, "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 10 },
            "context": { "triggerKind": 2, "triggerCharacter": ":" }
        }
    }));
    let resp = lsp.response(3);
    let items = resp["result"].as_array().unwrap();
    let answer = items.iter().find(|i| i["label"] == "answer");
    assert!(answer.is_some(), "second colon reopen should show answer, got: {:?}", items.iter().map(|i| i["label"].as_str()).collect::<Vec<_>>());

    let _ = lsp;
}

#[test]
fn completion_private_filtered() {
    let project = TestProject::new();
    let main_uri = TestProject::uri(&project.main);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "fn hidden(): int { 1 }\npublic fn visible(): int { 2 }\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "import math;\nfn main(): int {\n    math::\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 10 }
        }
    }));
    let resp = lsp.response(2);
    let items = resp["result"].as_array().unwrap();

    let visible = items.iter().find(|i| i["label"] == "visible");
    assert!(visible.is_some(), "public function 'visible' should appear in completions");

    let hidden = items.iter().find(|i| i["label"] == "hidden");
    assert!(hidden.is_none(), "private function 'hidden' should NOT appear in completions");

    let _ = lsp;
}

#[test]
fn import_prefix_completion_reopen() {
    let project = TestProject::new();
    let main_uri = TestProject::uri(&project.main);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "public fn answer(): int { 42 }\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "import parent::\nfn main(): int {\n    0\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 0, "character": 15 },
            "context": { "triggerKind": 2, "triggerCharacter": ":" }
        }
    }));
    let resp = lsp.response(2);
    let items = resp["result"].as_array().unwrap();
    let math = items.iter().find(|i| i["label"] == "math");
    assert!(math.is_some(), "import parent:: should show math module, got: {:?}", items.iter().map(|i| i["label"].as_str()).collect::<Vec<_>>());

    let _ = lsp;
}

#[test]
fn goto_definition_from_non_entry_file() {
    let project = TestProject::new();
    std::fs::write(&project.main, "fn main(): int {\n    0\n}\n").unwrap();
    let app = project.root.join("app.vn");
    std::fs::write(
        &app,
        "import math;\nfn run(): int {\n    math::answer()\n}\n",
    )
    .unwrap();
    let app_uri = TestProject::uri(&app);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "public fn answer(): int { 42 }\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": app_uri, "languageId": "vinyl", "version": 1,
                "text": "import math;\nfn run(): int {\n    math::answer()\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": TestProject::uri(&project.main), "languageId": "vinyl", "version": 1,
                "text": "fn main(): int {\n    0\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": app_uri },
            "position": { "line": 2, "character": 10 }
        }
    }));
    let definition = lsp.response(2);
    assert_eq!(
        definition["result"]["uri"],
        math_uri,
        "goto-definition from non-entry file should resolve to math.vn, got: {}",
        definition
    );

    let _ = lsp;
}

#[test]
fn private_access_diagnostic() {
    let project = TestProject::new();
    let main_uri = TestProject::uri(&project.main);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "fn secret(): int { 42 }\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "import math;\nfn main(): int {\n    math::secret()\n}\n" }
        }
    }));
    let diag = lsp.notification("textDocument/publishDiagnostics");
    let diagnostics = diag["params"]["diagnostics"].as_array().unwrap();
    assert!(!diagnostics.is_empty(), "calling private function should produce a diagnostic");
    let msg = diagnostics[0]["message"].as_str().unwrap();
    assert!(
        msg.contains("private") || msg.contains("not found"),
        "diagnostic should mention private or not found, got: {msg}"
    );

    let _ = lsp;
}

fn open_pair(lsp: &mut LspProcess, math_uri: &str, main_uri: &str) {
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "public fn answer(): int { 42 }\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "import math;\nfn main(): int {\n    math::answer()\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");
}

#[test]
fn rename_from_definition_updates_both_files() {
    let project = TestProject::new();
    let main_uri = TestProject::uri(&project.main);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));
    open_pair(&mut lsp, &math_uri, &main_uri);

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/rename",
        "params": {
            "textDocument": { "uri": math_uri },
            "position": { "line": 0, "character": 11 },
            "newName": "answer_new"
        }
    }));
    let rename = lsp.response(2);
    let changes = rename["result"]["changes"].as_object().unwrap();
    assert_eq!(
        changes.len(),
        2,
        "rename should touch declaration + call site, got: {rename}"
    );
    let math_edits = changes[&math_uri].as_array().unwrap();
    assert_eq!(math_edits.len(), 1);
    assert_eq!(
        math_edits[0]["range"],
        json!({"start": {"line": 0, "character": 10}, "end": {"line": 0, "character": 16}})
    );
    assert_eq!(math_edits[0]["newText"], "answer_new");
    let main_edits = changes[&main_uri].as_array().unwrap();
    assert_eq!(main_edits.len(), 1);
    assert_eq!(
        main_edits[0]["range"],
        json!({"start": {"line": 2, "character": 10}, "end": {"line": 2, "character": 16}})
    );
    assert_eq!(
        main_edits[0]["newText"], "answer_new",
        "call site should keep the math:: prefix, got: {rename}"
    );
}

#[test]
fn references_from_definition_include_declaration_and_site() {
    let project = TestProject::new();
    let main_uri = TestProject::uri(&project.main);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));
    open_pair(&mut lsp, &math_uri, &main_uri);

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/references",
        "params": {
            "textDocument": { "uri": math_uri },
            "position": { "line": 0, "character": 11 },
            "context": { "includeDeclaration": true }
        }
    }));
    let references = lsp.response(2);
    let locations = references["result"].as_array().unwrap();
    assert_eq!(
        locations.len(),
        2,
        "references should include declaration + call site, got: {references}"
    );
    let math_site = locations.iter().find(|l| l["uri"] == math_uri).unwrap();
    assert_eq!(
        math_site["range"],
        json!({"start": {"line": 0, "character": 10}, "end": {"line": 0, "character": 16}})
    );
    let main_site = locations.iter().find(|l| l["uri"] == main_uri).unwrap();
    assert_eq!(
        main_site["range"],
        json!({"start": {"line": 2, "character": 10}, "end": {"line": 2, "character": 16}})
    );
}

fn open_scope_file(lsp: &mut LspProcess, main_uri: &str) {
    let text = "fn double(n: int): int {\n    n * 2\n}\n\nfn square(n: int): int {\n    n * n\n}\n";
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": text }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");
}

#[test]
fn rename_local_symbol_is_scoped() {
    let project = TestProject::new();
    std::fs::write(
        &project.main,
        "fn double(n: int): int {\n    n * 2\n}\n\nfn square(n: int): int {\n    n * n\n}\n",
    )
    .unwrap();
    let main_uri = TestProject::uri(&project.main);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));
    open_scope_file(&mut lsp, &main_uri);

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/rename",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 0, "character": 10 },
            "newName": "num"
        }
    }));
    let rename = lsp.response(2);
    let changes = rename["result"]["changes"].as_object().unwrap();
    assert_eq!(changes.len(), 1, "rename should only touch main.vn, got: {rename}");
    let edits = changes[&main_uri].as_array().unwrap();
    let mut ranges: Vec<(u64, u64, u64)> = edits
        .iter()
        .map(|e| {
            (
                e["range"]["start"]["line"].as_u64().unwrap(),
                e["range"]["start"]["character"].as_u64().unwrap(),
                e["range"]["end"]["character"].as_u64().unwrap(),
            )
        })
        .collect();
    ranges.sort_unstable();
    assert_eq!(
        ranges,
        vec![(0, 10, 11), (1, 4, 5)],
        "only double's param + body reference should be renamed, got: {rename}"
    );
}

#[test]
fn goto_definition_across_module_with_type_error() {
    let project = TestProject::new();
    std::fs::write(
        &project.math,
        "public fn answer(): int { 42 }\nfn broken(): int { \"nope\" }\n",
    )
    .unwrap();
    let main_uri = TestProject::uri(&project.main);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "public fn answer(): int { 42 }\nfn broken(): int { \"nope\" }\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "import math;\nfn main(): int {\n    math::answer()\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 10 }
        }
    }));
    let definition = lsp.response(2);
    assert_eq!(
        definition["result"]["uri"],
        math_uri,
        "goto-definition should still resolve when the target module has a type error, got: {}",
        definition
    );
    assert_eq!(
        definition["result"]["range"]["start"],
        json!({"line": 0, "character": 10}),
        "definition range start mismatch, got: {}",
        definition
    );
    assert_eq!(
        definition["result"]["range"]["end"],
        json!({"line": 0, "character": 16}),
        "definition range end mismatch, got: {}",
        definition
    );
}

#[test]
fn goto_definition_parent_import_through_pipe() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("examples")
        .join("example_project");
    let main = root.join("main.vn");
    let math = root.join("math.vn");
    let main_uri = TestProject::uri(&main);
    let math_uri = TestProject::uri(&math.canonicalize().unwrap());
    let root_uri = TestProject::uri(&root);
    let mut lsp = LspProcess::start();
    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }));
    for (uri, text) in [
        (main_uri.clone(), "import parent::math;\n\nfn main() {\n    69 |> math::double()\n}\n"),
        (math_uri.clone(), "public fn double(n: int): int {\n    n * 2\n}\n"),
    ] {
        lsp.send(json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": { "textDocument": { "uri": uri, "languageId": "vinyl", "version": 1, "text": text } }
        }));
        lsp.notification("textDocument/publishDiagnostics");
    }
    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/definition",
        "params": { "textDocument": { "uri": main_uri }, "position": { "line": 3, "character": 16 } }
    }));
    let definition = lsp.response(2);
    assert_eq!(definition["result"]["uri"], math_uri);
    lsp.send(json!({
        "jsonrpc": "2.0", "id": 3, "method": "textDocument/hover",
        "params": { "textDocument": { "uri": main_uri }, "position": { "line": 3, "character": 16 } }
    }));
    let hover = lsp.response(3);
    assert!(hover["result"]["contents"].as_str().unwrap().contains("fn double"));
}

#[test]
fn imported_file_change_clears_parent_diagnostic() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("examples")
        .join("example_project");
    let main = root.join("main.vn");
    let math = root.join("math.vn");
    let main_uri = TestProject::uri(&main.canonicalize().unwrap());
    let math_uri = TestProject::uri(&math.canonicalize().unwrap());
    let root_uri = TestProject::uri(&root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "import parent::math;\n\nfn main() {\n    69 |> math::double()\n}\n" }
        }
    }));
    assert!(lsp.notification("textDocument/publishDiagnostics")["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .is_empty());

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "public fn double(n: int): int {\n    n * 2\n}\n" }
        }
    }));
    assert!(lsp.notification("textDocument/publishDiagnostics")["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .is_empty());

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": math_uri, "version": 2 },
            "contentChanges": [{ "text": "public fn double(n: int64): int {\n    n * 2\n}\n" }]
        }
    }));
    let diagnostics = lsp.notification("textDocument/publishDiagnostics");
    assert_eq!(diagnostics["params"]["uri"], main_uri);
    assert!(diagnostics["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .is_empty(), "main diagnostics should be cleared: {diagnostics}");
}

#[test]
fn imported_file_change_uses_nearest_project_entry() {
    let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..");
    let root = repository.join("examples").join("example_project");
    let main = root.join("main.vn");
    let math = root.join("math.vn");
    let main_uri = TestProject::uri(&main.canonicalize().unwrap());
    let math_uri = TestProject::uri(&math.canonicalize().unwrap());
    let root_uri = TestProject::uri(&repository);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }));

    for (uri, text) in [
        (
            main_uri.clone(),
            "import parent::math;\n\nfn main() {\n    69 |> math::double()\n}\n",
        ),
        (
            math_uri.clone(),
            "public fn double(n: int): int {\n    n * 2\n}\n",
        ),
    ] {
        lsp.send(json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": { "textDocument": { "uri": uri, "languageId": "vinyl", "version": 1, "text": text } }
        }));
        lsp.notification("textDocument/publishDiagnostics");
    }

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": math_uri, "version": 2 },
            "contentChanges": [{ "text": "public fn double(n: int64): int {\n    n * 2\n}\n" }]
        }
    }));
    let diagnostics = lsp.notification("textDocument/publishDiagnostics");
    assert_eq!(diagnostics["params"]["uri"], main_uri);
}

#[test]
fn incremental_changes_update_the_full_document() {
    let project = TestProject::new();
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }));
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "public fn double(n: int): int { 42 }\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": math_uri, "version": 2 },
            "contentChanges": [{
                "range": {
                    "start": { "line": 0, "character": 20 },
                    "end": { "line": 0, "character": 23 }
                },
                "rangeLength": 3,
                "text": "int6"
            }]
        }
    }));
    let diagnostics = lsp.diagnostics_for(&math_uri);
    assert!(diagnostics["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|diagnostic| diagnostic["message"] == "unexpected token `6`"),
        "intermediate edit should report its temporary parse error: {diagnostics}");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": math_uri, "version": 3 },
            "contentChanges": [{
                "range": {
                    "start": { "line": 0, "character": 24 },
                    "end": { "line": 0, "character": 24 }
                },
                "rangeLength": 0,
                "text": "4"
            }]
        }
    }));
    let recovered = lsp.diagnostics_for(&math_uri);
    assert!(recovered["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .is_empty(), "recovered document diagnostics should clear: {recovered}");
}

#[test]
fn goto_definition_on_type_annotation() {
    let project = TestProject::new();
    std::fs::write(
        &project.main,
        "struct Point { x: int, y: int }\n\nfn main(): int {\n    let origin: Point = Point { x: 0, y: 0 };\n    origin.x\n}\n",
    )
    .unwrap();
    let main_uri = TestProject::uri(&project.main);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "struct Point { x: int, y: int }\n\nfn main(): int {\n    let origin: Point = Point { x: 0, y: 0 };\n    origin.x\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 3, "character": 16 }
        }
    }));
    let definition = lsp.response(2);
    assert_eq!(
        definition["result"]["uri"],
        main_uri,
        "goto-definition on a type annotation should resolve to the struct, got: {}",
        definition
    );
    assert_eq!(
        definition["result"]["range"]["start"],
        json!({"line": 0, "character": 7}),
        "struct definition range start mismatch, got: {}",
        definition
    );
    assert_eq!(
        definition["result"]["range"]["end"],
        json!({"line": 0, "character": 12}),
        "struct definition range end mismatch, got: {}",
        definition
    );
}

#[test]
fn goto_definition_on_struct_field() {
    let project = TestProject::new();
    std::fs::write(
        &project.main,
        "struct Point { x: int, y: int }\n\nfn main(): int {\n    let p = Point { x: 0, y: 0 };\n    p.x\n}\n",
    )
    .unwrap();
    let main_uri = TestProject::uri(&project.main);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "struct Point { x: int, y: int }\n\nfn main(): int {\n    let p = Point { x: 0, y: 0 };\n    p.x\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 4, "character": 6 }
        }
    }));
    let definition = lsp.response(2);
    assert_eq!(
        definition["result"]["uri"],
        main_uri,
        "goto-definition on a struct field access should resolve to the field, got: {}",
        definition
    );
    assert_eq!(
        definition["result"]["range"]["start"],
        json!({"line": 0, "character": 15}),
        "field definition range start mismatch, got: {}",
        definition
    );
    assert_eq!(
        definition["result"]["range"]["end"],
        json!({"line": 0, "character": 16}),
        "field definition range end mismatch, got: {}",
        definition
    );
}

#[test]
fn goto_definition_on_enum_variant() {
    let project = TestProject::new();
    std::fs::write(
        &project.main,
        "enum Shape { Empty, Circle(int32), Square(int32) }\n\nfn main(): int {\n    let s = Shape::Empty();\n    if s == Shape::Empty() { 1 } else { 0 }\n}\n",
    )
    .unwrap();
    let main_uri = TestProject::uri(&project.main);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "enum Shape { Empty, Circle(int32), Square(int32) }\n\nfn main(): int {\n    let s = Shape::Empty();\n    if s == Shape::Empty() { 1 } else { 0 }\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 3, "character": 19 }
        }
    }));
    let definition = lsp.response(2);
    assert_eq!(
        definition["result"]["uri"],
        main_uri,
        "goto-definition on an enum variant should resolve to the variant, got: {}",
        definition
    );
    assert_eq!(
        definition["result"]["range"]["start"],
        json!({"line": 0, "character": 13}),
        "variant definition range start mismatch, got: {}",
        definition
    );
    assert_eq!(
        definition["result"]["range"]["end"],
        json!({"line": 0, "character": 18}),
        "variant definition range end mismatch, got: {}",
        definition
    );
}

#[test]
fn goto_definition_on_module_segment() {
    let project = TestProject::new();
    let main_uri = TestProject::uri(&project.main);
    let math_uri = TestProject::uri(&project.math);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": math_uri, "languageId": "vinyl", "version": 1,
                "text": "public fn answer(): int { 42 }\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "import math;\nfn main(): int {\n    math::answer()\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 4 }
        }
    }));
    let definition = lsp.response(2);
    assert_eq!(
        definition["result"]["uri"],
        math_uri,
        "goto-definition on a module segment should resolve to the module file, got: {}",
        definition
    );
    assert_eq!(
        definition["result"]["range"]["start"],
        json!({"line": 0, "character": 0}),
        "module location start mismatch, got: {}",
        definition
    );
}

#[test]
fn hover_on_let_binding_shows_type() {
    let project = TestProject::new();
    std::fs::write(
        &project.main,
        "fn main(): int {\n    let value = 42;\n    value\n}\n",
    )
    .unwrap();
    let main_uri = TestProject::uri(&project.main);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "fn main(): int {\n    let value = 42;\n    value\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 1, "character": 8 }
        }
    }));
    let hover = lsp.response(2);
    let hover_text = hover["result"]["contents"].as_str().unwrap();
    assert!(
        hover_text.contains("value: int"),
        "hover on a let binding should show its type, got: {hover_text}"
    );
}

#[test]
fn hover_on_local_function_shows_signature() {
    let project = TestProject::new();
    std::fs::write(
        &project.main,
        "fn helper(): int { 42 }\n\nfn main(): int {\n    helper()\n}\n",
    )
    .unwrap();
    let main_uri = TestProject::uri(&project.main);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": "fn helper(): int { 42 }\n\nfn main(): int {\n    helper()\n}\n" }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 3, "character": 4 }
        }
    }));
    let hover = lsp.response(2);
    let hover_text = hover["result"]["contents"].as_str().unwrap();
    assert!(
        hover_text.contains("fn helper(): int"),
        "hover on a local function call should show its signature, got: {hover_text}"
    );
}

#[test]
fn rename_updates_three_modules() {
    let project = TestProject::new();
    let b_vn = project.root.join("b.vn");
    let a_vn = project.root.join("a.vn");
    let c_vn = project.root.join("c.vn");
    std::fs::write(&b_vn, "public fn worker(): int { 42 }\n").unwrap();
    std::fs::write(
        &a_vn,
        "import parent::b;\nfn run(): int {\n    b::worker()\n}\n",
    )
    .unwrap();
    std::fs::write(
        &c_vn,
        "import parent::b;\nfn run(): int {\n    b::worker()\n}\n",
    )
    .unwrap();
    let b_uri = TestProject::uri(&b_vn);
    let a_uri = TestProject::uri(&a_vn);
    let c_uri = TestProject::uri(&c_vn);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    for (uri, text) in [
        (b_uri.clone(), "public fn worker(): int { 42 }\n"),
        (
            a_uri.clone(),
            "import parent::b;\nfn run(): int {\n    b::worker()\n}\n",
        ),
        (
            c_uri.clone(),
            "import parent::b;\nfn run(): int {\n    b::worker()\n}\n",
        ),
    ] {
        lsp.send(json!({
            "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {
                "textDocument": { "uri": uri, "languageId": "vinyl", "version": 1, "text": text }
            }
        }));
        lsp.notification("textDocument/publishDiagnostics");
    }

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/rename",
        "params": {
            "textDocument": { "uri": b_uri },
            "position": { "line": 0, "character": 10 },
            "newName": "worker2"
        }
    }));
    let rename = lsp.response(2);
    let changes = rename["result"]["changes"].as_object().unwrap();
    let touched: Vec<String> = changes.keys().cloned().collect();
    assert!(
        touched.contains(&b_uri) && touched.contains(&a_uri) && touched.contains(&c_uri),
        "rename should touch the definition plus both call sites, got: {touched:?}"
    );
    for uri in [&b_uri, &a_uri, &c_uri] {
        let edits = changes.get(uri).unwrap().as_array().unwrap();
        assert_eq!(edits.len(), 1, "{uri} should get exactly one edit, got: {rename}");
        assert_eq!(edits[0]["newText"], "worker2");
    }
    let b_edits = changes[&b_uri].as_array().unwrap();
    assert_eq!(
        b_edits[0]["range"],
        json!({"start": {"line": 0, "character": 10}, "end": {"line": 0, "character": 16}})
    );
    let a_edits = changes[&a_uri].as_array().unwrap();
    assert_eq!(
        a_edits[0]["range"],
        json!({"start": {"line": 2, "character": 7}, "end": {"line": 2, "character": 13}})
    );
}

#[test]
fn cursor_on_whitespace_returns_nothing() {
    let project = TestProject::new();
    std::fs::write(
        &project.main,
        "fn double(n: int): int {\n    n * 2\n}\n\nfn square(n: int): int {\n    n * n\n}\n",
    )
    .unwrap();
    let main_uri = TestProject::uri(&project.main);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));
    open_scope_file(&mut lsp, &main_uri);

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/references",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 0 },
            "context": { "includeDeclaration": true }
        }
    }));
    let references = lsp.response(2);
    assert!(
        references["result"].as_array().unwrap().is_empty(),
        "cursor on a closing brace should not match a symbol, got: {references}"
    );

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 3, "method": "textDocument/rename",
        "params": {
            "textDocument": { "uri": main_uri },
            "position": { "line": 2, "character": 0 },
            "newName": "num"
        }
    }));
    let rename = lsp.response(3);
    assert!(
        rename["result"].is_null(),
        "cursor on a closing brace should not rename anything, got: {rename}"
    );
}

#[test]
fn format_noop_and_code_action_hidden_when_already_formatted() {
    let project = TestProject::new();
    let already_formatted = "fn main(): int {\n    answer()\n}\n";
    std::fs::write(&project.main, already_formatted).unwrap();
    let main_uri = TestProject::uri(&project.main);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": already_formatted }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/formatting",
        "params": {
            "textDocument": { "uri": main_uri },
            "options": { "tabSize": 4, "insertSpaces": true }
        }
    }));
    let formatting = lsp.response(2);
    assert!(
        formatting["result"].is_null(),
        "formatting an already-formatted file should return null, got: {}",
        formatting
    );

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 3, "method": "textDocument/codeAction",
        "params": {
            "textDocument": { "uri": main_uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 2, "character": 0 }
            },
            "context": { "diagnostics": [] }
        }
    }));
    let actions = lsp.response(3);
    assert!(
        !actions["result"].as_array().unwrap().iter().any(|action| {
            action["title"].as_str() == Some("Format document")
        }),
        "an already-formatted file should not offer a Format document action, got: {}",
        actions
    );
}

#[test]
fn formatting_is_idempotent_after_reopen() {
    let project = TestProject::new();
    let raw = "import   math ;\nfn main() {\n69 |> math::answer()\n}\n";
    std::fs::write(&project.main, raw).unwrap();
    let main_uri = TestProject::uri(&project.main);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": raw }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/formatting",
        "params": {
            "textDocument": { "uri": main_uri },
            "options": { "tabSize": 4, "insertSpaces": true }
        }
    }));
    let formatted = lsp.response(2)["result"][0]["newText"].as_str().unwrap().to_string();

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didClose",
        "params": { "textDocument": { "uri": main_uri } }
    }));
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 2,
                "text": &formatted }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 3, "method": "textDocument/formatting",
        "params": {
            "textDocument": { "uri": main_uri },
            "options": { "tabSize": 4, "insertSpaces": true }
        }
    }));
    let again = lsp.response(3);
    assert!(
        again["result"].is_null(),
        "formatting the already-formatted file after reopen should return null, got: {}",
        again
    );

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didClose",
        "params": { "textDocument": { "uri": main_uri } }
    }));
    let editor_saved = if formatted.ends_with('\n') {
        formatted
    } else {
        format!("{formatted}\n")
    };
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 3,
                "text": &editor_saved }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 4, "method": "textDocument/formatting",
        "params": {
            "textDocument": { "uri": main_uri },
            "options": { "tabSize": 4, "insertSpaces": true }
        }
    }));
    let with_newline = lsp.response(4);
    assert!(
        with_newline["result"].is_null(),
        "formatting should not churn on an editor-added final newline, got: {}",
        with_newline
    );
}

#[test]
fn formatting_is_idempotent_with_crlf() {
    let project = TestProject::new();
    let raw = "import   math;\r\nfn main() {\r\n69 |> math::answer()\r\n}\r\n";
    std::fs::write(&project.main, raw).unwrap();
    let main_uri = TestProject::uri(&project.main);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": root_uri, "capabilities": {} }
    }));
    lsp.response(1);
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "initialized", "params": {}
    }));

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
                "text": raw }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 2, "method": "textDocument/formatting",
        "params": {
            "textDocument": { "uri": main_uri },
            "options": { "tabSize": 4, "insertSpaces": true }
        }
    }));
    let formatted = lsp.response(2)["result"][0]["newText"].as_str().unwrap().to_string();
    assert!(
        formatted.contains("\r\n"),
        "format of a CRLF file should keep CRLF line endings, got: {formatted:?}"
    );

    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didClose",
        "params": { "textDocument": { "uri": main_uri } }
    }));
    lsp.send(json!({
        "jsonrpc": "2.0", "method": "textDocument/didOpen",
        "params": {
            "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 2,
                "text": &formatted }
        }
    }));
    lsp.notification("textDocument/publishDiagnostics");

    lsp.send(json!({
        "jsonrpc": "2.0", "id": 3, "method": "textDocument/formatting",
        "params": {
            "textDocument": { "uri": main_uri },
            "options": { "tabSize": 4, "insertSpaces": true }
        }
    }));
    let again = lsp.response(3);
    assert!(
        again["result"].is_null(),
        "formatting an already-formatted CRLF file after reopen should return null, got: {}",
        again
    );
}

#[test]
fn hover_and_signature_help_preserve_written_primitive_names() {
    let project = TestProject::new();
    std::fs::write(
        &project.main,
        "fn alias(n: int): int { n }\nfn explicit(n: int64): int64 { n }\nstruct Point { x: int, y: int64 }\nfn main() {\n    alias(1)\n}\n",
    )
    .unwrap();
    let main_uri = TestProject::uri(&project.main);
    let root_uri = TestProject::uri(&project.root);
    let mut lsp = LspProcess::start();
    lsp.send(json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "rootUri": root_uri, "capabilities": {} }}));
    lsp.response(1);
    lsp.send(json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));
    lsp.send(json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
        "textDocument": { "uri": main_uri, "languageId": "vinyl", "version": 1,
            "text": "fn alias(n: int): int { n }\nfn explicit(n: int64): int64 { n }\nstruct Point { x: int, y: int64 }\nfn main() {\n    alias(1)\n}\n" } }}));
    lsp.notification("textDocument/publishDiagnostics");

    for (id, line, character, expected) in [
        (2, 0, 4, "fn alias(n: int): int"),
        (3, 1, 4, "fn explicit(n: int64): int64"),
        (4, 2, 7, "struct Point { x: int, y: int64 }"),
        (5, 4, 5, "fn alias(n: int): int"),
    ] {
        lsp.send(json!({"jsonrpc": "2.0", "id": id, "method": "textDocument/hover", "params": {
            "textDocument": { "uri": main_uri }, "position": { "line": line, "character": character } }}));
        let hover = lsp.response(id);
        let hover_text = hover["result"]["contents"].as_str().unwrap();
        assert!(
            hover_text.contains(expected),
            "hover should preserve the written primitive name, got: {hover_text}"
        );
    }

    lsp.send(json!({"jsonrpc": "2.0", "id": 6, "method": "textDocument/signatureHelp", "params": {
        "textDocument": { "uri": main_uri }, "position": { "line": 4, "character": 9 } }}));
    let sig = lsp.response(6);
    let label = sig["result"]["signatures"][0]["label"].as_str().unwrap();
    assert_eq!(
        label, "fn alias(n: int): int",
        "signature help should preserve the written primitive name, got: {label}"
    );
}


