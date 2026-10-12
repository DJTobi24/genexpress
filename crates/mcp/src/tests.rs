use serde_json::{Value, json};

use crate::{Headless, Server};

fn call(s: &mut Server<Headless>, msg: Value) -> Value {
    serde_json::from_str(&s.handle_line(&msg.to_string()).unwrap()).unwrap()
}

#[test]
fn protocol_round_trip() {
    let mut s = Server::new(Headless::default());
    let r = call(&mut s, json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}));
    assert_eq!(r["result"]["serverInfo"]["name"], "genha");
    assert!(s.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
    let r = call(&mut s, json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    assert!(r["result"]["tools"].as_array().unwrap().len() >= 5);
    let r = call(
        &mut s,
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "run_command", "arguments": {"command": "file.new", "params": {"preset": "instagram-story"}}}}),
    );
    assert_eq!(r["result"]["isError"], false);
    let r = call(
        &mut s,
        json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "run_command", "arguments": {"command": "text.add", "params": {"text": "Hi"}}}}),
    );
    assert_eq!(r["result"]["isError"], false);
    let r = call(&mut s, json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "inspect_design", "arguments": {}}}));
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("\"Hi\""));
    let r =
        call(&mut s, json!({"jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": {"name": "run_command", "arguments": {"command": "nope"}}}));
    assert_eq!(r["result"]["isError"], true);
    let r = call(&mut s, json!({"jsonrpc": "2.0", "id": 7, "method": "bogus"}));
    assert_eq!(r["error"]["code"], -32601);
}

#[test]
fn garbage_never_panics() {
    let mut s = Server::new(Headless::default());
    for line in [
        "",
        "{",
        "[]",
        "null",
        r#"{"id":1}"#,
        r#"{"id":1,"method":"tools/call","params":5}"#,
        r#"{"id":{},"method":"tools/call","params":{"name":5,"arguments":[]}}"#,
    ] {
        let _ = s.handle_line(line);
    }
}
