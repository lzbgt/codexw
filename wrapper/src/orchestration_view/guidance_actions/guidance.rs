use super::*;

#[path = "guidance/focused.rs"]
mod focused;
#[path = "guidance/global.rs"]
mod global;

pub(super) use focused::guidance_lines_for_capability;
#[cfg(test)]
pub(super) use focused::guidance_lines_for_tool_capability;
pub(super) use global::guidance_lines;
#[cfg(test)]
pub(super) use global::guidance_lines_for_tool;
