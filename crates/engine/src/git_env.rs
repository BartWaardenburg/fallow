//! Git process environment helpers owned by the engine boundary.

use std::process::{Command, Stdio};

/// Environment variables that describe an enclosing git operation's repository
/// state and should not leak into fallow-owned git subprocesses.
pub const AMBIENT_GIT_ENV_VARS: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_COMMON_DIR",
    "GIT_PREFIX",
];

/// Strip ambient git repository-state environment variables from a `Command`.
///
/// Returns the `Command` for fluent chaining alongside `.args()` and
/// `.current_dir()`.
pub fn clear_ambient_git_env(cmd: &mut Command) -> &mut Command {
    for var in AMBIENT_GIT_ENV_VARS {
        cmd.env_remove(var);
    }
    cmd
}

/// Build a `git` command with the ambient repository-state environment cleared
/// and stdin closed. Long-lived embedders keep protocol stdin open, which Git
/// for Windows can inherit and hold.
#[expect(
    clippy::disallowed_methods,
    reason = "engine-owned git spawn wrapper clears ambient git env before every git subprocess"
)]
pub fn git_command() -> Command {
    let mut command = Command::new("git");
    clear_ambient_git_env(&mut command);
    command.stdin(Stdio::null());
    command
}
