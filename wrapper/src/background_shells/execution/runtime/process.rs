#[path = "process/io.rs"]
mod io;
#[path = "process/spawn.rs"]
mod spawn;

pub(crate) use self::io::spawn_output_reader;
pub(crate) use self::io::terminate_jobs;
pub(crate) use self::spawn::parse_background_shell_capabilities;
pub(crate) use self::spawn::parse_background_shell_intent;
pub(crate) use self::spawn::parse_background_shell_label;
pub(crate) use self::spawn::parse_background_shell_optional_string;
pub(crate) use self::spawn::parse_background_shell_ready_pattern;
pub(crate) use self::spawn::parse_background_shell_timeout_ms;
pub(crate) use self::spawn::resolve_background_cwd;
pub(crate) use self::spawn::spawn_shell_process;
pub(crate) use self::spawn::validate_alias;
pub(crate) use self::spawn::validate_service_capability;
