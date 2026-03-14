use serde_json::Value;

#[path = "specs/background_shells.rs"]
mod background_shells;
#[path = "specs/orchestration.rs"]
mod orchestration;

pub(crate) fn dynamic_tool_specs() -> Value {
    let mut specs = Vec::new();
    specs.extend(orchestration::orchestration_tool_specs());
    specs.extend(background_shells::background_shell_tool_specs());
    Value::Array(specs)
}
