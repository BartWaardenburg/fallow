use rmcp::ServerHandler;

use super::super::FallowMcp;

#[test]
fn server_info_is_correct() {
    let server = FallowMcp::new();
    let info = ServerHandler::get_info(&server);
    assert_eq!(info.server_info.name, "fallow-mcp");
    assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
    assert!(info.capabilities.tools.is_some());
    assert!(info.instructions.is_some());
}

#[test]
fn all_tools_registered() {
    let server = FallowMcp::new();
    let tools = server.tool_router.list_all();
    let names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
    assert!(names.contains(&"analyze".to_string()));
    assert!(names.contains(&"check_changed".to_string()));
    assert!(names.contains(&"find_dupes".to_string()));
    assert!(names.contains(&"fix_preview".to_string()));
    assert!(names.contains(&"fix_apply".to_string()));
    assert!(names.contains(&"project_info".to_string()));
    assert!(names.contains(&"check_health".to_string()));
    assert_eq!(tools.len(), 7);
}

#[test]
fn read_only_tools_have_annotations() {
    let server = FallowMcp::new();
    let tools = server.tool_router.list_all();
    let read_only = [
        "analyze",
        "check_changed",
        "find_dupes",
        "fix_preview",
        "project_info",
        "check_health",
    ];
    for tool in &tools {
        let name = tool.name.to_string();
        if read_only.contains(&name.as_str()) {
            let ann = tool.annotations.as_ref().expect("annotations");
            assert_eq!(ann.read_only_hint, Some(true), "{name} should be read-only");
        }
    }
}

#[test]
fn fix_apply_is_destructive() {
    let server = FallowMcp::new();
    let tools = server.tool_router.list_all();
    let fix = tools.iter().find(|t| t.name == "fix_apply").unwrap();
    let ann = fix.annotations.as_ref().unwrap();
    assert_eq!(ann.destructive_hint, Some(true));
    assert_eq!(ann.read_only_hint, Some(false));
}
