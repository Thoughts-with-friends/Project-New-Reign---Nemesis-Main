//! Development task runner for the workspace.
//!
//! Invoked through the `cargo xtask` alias defined in `.cargo/config.toml`:
//!
//! | Task                   | Runs                                                           |
//! | ---------------------- | -------------------------------------------------------------- |
//! | `cargo xtask fmt`      | `cargo +nightly fmt --all`                                     |
//! | `cargo xtask lint:fix` | `cargo clippy --workspace --fix --allow-staged --allow-dirty` |
//! | `cargo xtask build`    | `cargo build --release`                                        |
//!
//! Any extra arguments after the task name are appended to the cargo command,
//! e.g. `cargo xtask build --locked`.

use std::process::{Command, ExitCode};

/// A named task and the cargo command line it runs.
struct Task {
    /// Name typed after `cargo xtask`.
    name: &'static str,
    /// Alternative names accepted for the task.
    aliases: &'static [&'static str],
    /// One-line description for the usage text.
    description: &'static str,
    /// Arguments passed to `cargo`.
    args: &'static [&'static str],
}

/// Every available task, in the order shown by `cargo xtask help`.
const TASKS: &[Task] = &[
    Task {
        name: "fmt",
        aliases: &[],
        description: "Format every crate with nightly rustfmt",
        args: &["+nightly", "fmt", "--all"],
    },
    Task {
        name: "lint:fix",
        aliases: &["lint-fix"],
        description: "Apply clippy's automatic fixes to the whole workspace",
        args: &[
            "clippy",
            "--workspace",
            "--fix",
            "--allow-staged",
            "--allow-dirty",
        ],
    },
    Task {
        name: "build",
        aliases: &[],
        description: "Build the GUI in release mode",
        args: &["build", "--release"],
    },
];

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);

    let Some(name) = args.next() else {
        print_usage();
        return ExitCode::FAILURE;
    };

    if matches!(name.as_str(), "help" | "-h" | "--help") {
        print_usage();
        return ExitCode::SUCCESS;
    }

    let Some(task) = find_task(&name) else {
        eprintln!("xtask: unknown task `{name}`\n");
        print_usage();
        return ExitCode::FAILURE;
    };

    run(task, &args.collect::<Vec<_>>())
}

/// Looks up a task by name or alias.
fn find_task(name: &str) -> Option<&'static Task> {
    TASKS
        .iter()
        .find(|task| task.name == name || task.aliases.contains(&name))
}

/// Runs `cargo <task args> <extra>` and forwards its exit status.
///
/// The `cargo` found on `PATH` (the rustup proxy) is used rather than the
/// `CARGO` variable, because only the proxy understands toolchain overrides
/// such as `+nightly`.
fn run(task: &Task, extra: &[String]) -> ExitCode {
    let mut command = Command::new("cargo");
    command.args(task.args).args(extra);

    let shown: Vec<&str> = task
        .args
        .iter()
        .copied()
        .chain(extra.iter().map(String::as_str))
        .collect();
    eprintln!("xtask: > cargo {}", shown.join(" "));

    match command.status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => {
            eprintln!("xtask: `{}` failed ({status})", task.name);
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("xtask: failed to invoke cargo: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Prints the available tasks and the command each one runs.
fn print_usage() {
    eprintln!("usage: cargo xtask <task> [extra cargo args...]\n\ntasks:");

    for task in TASKS {
        eprintln!("  {:<10} {}", task.name, task.description);
        eprintln!("  {:<10} -> cargo {}", "", task.args.join(" "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_tasks_by_name_and_alias() {
        assert!(find_task("fmt").is_some_and(|task| task.args[0] == "+nightly"));
        assert!(find_task("lint:fix").is_some_and(|task| task.args.contains(&"--fix")));
        assert!(find_task("lint-fix").is_some_and(|task| task.name == "lint:fix"));
        assert!(find_task("build").is_some_and(|task| task.args == ["build", "--release"]));
        assert!(find_task("deploy").is_none());
    }
}
