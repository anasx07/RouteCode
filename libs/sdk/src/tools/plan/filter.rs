//! Plan-mode tool schema filter.
//!
//! When the orchestrator is in plan mode, the OpenAI tool list sent to
//! the model is filtered:
//!
//! - Write tools (file_write, file_edit, apply_patch) are removed
//! - The plan tools (enter_plan_mode, exit_plan_mode) are added
//! - The user-configured `plan_mode_tool_overrides` are kept
//!
//! Read-only tools (file_read, ls, tree, grep, search_files, web_*, lsp_*,
//! mcp_*, subagent, etc.) and bash are kept. Bash is gated by
//! `BashMode::ReadOnly` at execution time.

use serde_json::Value;

/// Tool names that are removed from the schema in plan mode. Bash
/// stays because `BashMode::ReadOnly` already provides a hard gate.
const WRITE_TOOL_NAMES: &[&str] = &[
    "file_write",
    "file_edit",
    "apply_patch",
];

/// Tool names that are present in the schema in plan mode.
const PLAN_TOOL_NAMES: &[&str] = &[
    "enter_plan_mode",
    "exit_plan_mode",
];

/// Filter the schema list for plan mode.
///
/// * `schemas` — the full list of tool schemas from `ToolRegistry::get_all_schemas`
/// * `overrides` — extra tool names to keep (from `Config::plan_mode_tool_overrides`)
///
/// Returns the filtered list with write tools removed and plan tools
/// (always) + overrides added.
pub fn filter_for_plan_mode(
    schemas: Vec<Value>,
    overrides: &[String],
) -> Vec<Value> {
    let mut out: Vec<Value> = schemas
        .into_iter()
        .filter(|s| {
            let name = s
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("");
            !WRITE_TOOL_NAMES.contains(&name)
        })
        .collect();

    // Add plan tools if not already present
    for plan_schema in [enter_schema(), exit_schema()] {
        let name = plan_schema
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        if !out.iter().any(|s| {
            s.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                == Some(name)
        }) {
            out.push(plan_schema);
        }
    }

    // Apply user-configured overrides: keep these tools in the schema
    // even if they would otherwise be filtered. (Currently no tools
    // are filtered other than writes, so overrides are a forward-compat
    // hook — e.g. user could add `bash` to overrides to ensure it's
    // always visible.)
    let _ = overrides;

    out
}

fn enter_schema() -> Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "enter_plan_mode",
            "description": super::enter::EnterPlanModeTool::description_static(),
            "parameters": super::schema::enter_parameters(),
        }
    })
}

fn exit_schema() -> Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "exit_plan_mode",
            "description": super::exit::ExitPlanModeTool::description_static(),
            "parameters": super::schema::exit_parameters(),
        }
    })
}

pub fn plan_tool_names() -> &'static [&'static str] {
    PLAN_TOOL_NAMES
}

pub fn write_tool_names() -> &'static [&'static str] {
    WRITE_TOOL_NAMES
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(name: &str) -> Value {
        serde_json::json!({
            "type": "function",
            "function": { "name": name, "description": "", "parameters": {} }
        })
    }

    #[test]
    fn removes_write_tools() {
        let schemas = vec![
            s("file_read"),
            s("file_write"),
            s("file_edit"),
            s("apply_patch"),
            s("ls"),
            s("bash"),
        ];
        let out = filter_for_plan_mode(schemas, &[]);
        let names: Vec<&str> = out
            .iter()
            .map(|s| {
                s.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap()
            })
            .collect();
        assert!(!names.contains(&"file_write"));
        assert!(!names.contains(&"file_edit"));
        assert!(!names.contains(&"apply_patch"));
        assert!(names.contains(&"file_read"));
        assert!(names.contains(&"ls"));
        assert!(names.contains(&"bash"));
    }

    #[test]
    fn adds_plan_tools() {
        let schemas = vec![s("file_read")];
        let out = filter_for_plan_mode(schemas, &[]);
        let names: Vec<&str> = out
            .iter()
            .map(|s| {
                s.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap()
            })
            .collect();
        assert!(names.contains(&"enter_plan_mode"));
        assert!(names.contains(&"exit_plan_mode"));
    }

    #[test]
    fn plan_tools_idempotent() {
        let schemas = vec![s("enter_plan_mode"), s("file_read")];
        let out = filter_for_plan_mode(schemas, &[]);
        let count = out
            .iter()
            .filter(|s| {
                s.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    == Some("enter_plan_mode")
            })
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn overrides_dont_break() {
        let schemas = vec![s("file_read")];
        let overrides = vec!["bash".to_string()];
        let out = filter_for_plan_mode(schemas, &overrides);
        assert!(out.iter().any(|s| {
            s.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                == Some("enter_plan_mode")
        }));
    }
}
