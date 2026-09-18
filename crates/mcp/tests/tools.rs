use recallgate_mcp::tools;

#[test]
fn tools_list_names_four_gate_methods() {
    let names = tools::tool_names();
    assert_eq!(names.len(), 4);
    assert!(names.contains(&"gate_lock"));
    assert!(names.contains(&"gate_push_prompt"));
    assert!(names.contains(&"gate_status"));
    assert!(names.contains(&"gate_due"));
}

#[test]
fn tools_omit_unlock() {
    let names = tools::tool_names();
    assert!(!names.iter().any(|name| *name == "gate_unlock"));
    let listed = tools::tools_list();
    let tools = listed["tools"].as_array().expect("tools array");
    for tool in tools {
        let name = tool["name"].as_str().expect("tool name");
        assert_ne!(name, "gate_unlock");
    }
}

#[test]
fn initialize_then_tools_list() {
    let mut server = recallgate_mcp::server::McpServer::default();
    let init = server
        .handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#)
        .expect("init response");
    let body: serde_json::Value = serde_json::from_str(&init).expect("json");
    assert_eq!(body["result"]["protocolVersion"], "2024-11-05");

    let list = server
        .handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#)
        .expect("tools/list response");
    let body: serde_json::Value = serde_json::from_str(&list).expect("json");
    assert_eq!(body["result"]["tools"].as_array().unwrap().len(), 4);
}

#[test]
fn gate_unlock_tool_call_is_error() {
    let mut server = recallgate_mcp::server::McpServer::default();
    server
        .handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#)
        .expect("init");
    let raw = server
        .handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"gate_unlock","arguments":{}}}"#)
        .expect("call");
    let body: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(body["result"]["isError"], true);
}

#[test]
fn gate_abort_is_unknown_tool() {
    let mut server = recallgate_mcp::server::McpServer::default();
    server
        .handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#)
        .expect("init");
    let names = tools::tool_names();
    assert!(!names.iter().any(|name| *name == "gate_abort"));
    let raw = server
        .handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"gate_abort","arguments":{}}}"#)
        .expect("call");
    let body: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(body["result"]["isError"], true);
}
