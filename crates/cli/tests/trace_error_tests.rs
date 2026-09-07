//! `fallow trace-error [FILE|-]` behavior.
//!
//! The verb invites overclaiming, so the pinned properties are the refusals: a
//! frame with several matches stays ambiguous, a frame with none stays visible
//! as not-found, a frame outside project source is neither, and the counts
//! always close over the reported frames.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration tests keep fixture setup concise"
)]

#[path = "common/mod.rs"]
mod common;

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use common::{CommandOutput, fallow_bin, parse_json, run_fallow_in_root};
use tempfile::tempdir;

/// Run `fallow trace-error` with the trace piped in on stdin.
fn run_trace_error_stdin(root: &Path, trace: &str, args: &[&str]) -> CommandOutput {
    let mut cmd = Command::new(fallow_bin());
    cmd.arg("trace-error")
        .arg("--root")
        .arg(root)
        .env("RUST_LOG", "")
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for arg in args {
        cmd.arg(arg);
    }
    let mut child = cmd.spawn().expect("failed to spawn fallow binary");
    child
        .stdin
        .as_mut()
        .expect("stdin is piped")
        .write_all(trace.as_bytes())
        .expect("failed to write the trace to stdin");
    let output = child.wait_with_output().expect("failed to run fallow");
    CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        code: output.status.code().unwrap_or(-1),
    }
}

/// A project with one unambiguous export, one name that two definitions in the
/// same module answer to, and one module-local function that is not a
/// definition the graph knows.
fn write_project(root: &Path) {
    std::fs::create_dir_all(root.join("src/services")).unwrap();
    std::fs::create_dir_all(root.join("node_modules/vendor")).unwrap();
    std::fs::create_dir_all(root.join("dist")).unwrap();
    std::fs::write(
        root.join("package.json"),
        r#"{"name":"trace-error-fixture","type":"module"}"#,
    )
    .unwrap();
    std::fs::write(root.join("tsconfig.json"), r#"{"include":["src"]}"#).unwrap();
    std::fs::write(root.join(".fallowrc.json"), r#"{"entry":["src/index.ts"]}"#).unwrap();
    std::fs::write(
        root.join("src/index.ts"),
        "import { loadUser } from './services/user';\nimport { Task, run } from './task';\nloadUser();\nrun();\nnew Task().run();\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/services/user.ts"),
        "const helper = () => 1;\nexport const loadUser = () => helper();\n",
    )
    .unwrap();
    // `run` is both a standalone export and a method of the exported class, so
    // a `Task.run` frame legitimately names two definitions.
    std::fs::write(
        root.join("src/task.ts"),
        "export const run = (): number => 0;\nexport class Task {\n  run(): number {\n    return run();\n  }\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("node_modules/vendor/index.js"),
        "export const vendored = () => 0;\n",
    )
    .unwrap();
    std::fs::write(root.join("dist/bundle.js"), "console.log(1);\n").unwrap();
}

#[test]
fn a_frame_naming_one_definition_resolves_to_it() {
    let dir = tempdir().unwrap();
    write_project(dir.path());

    let output = run_trace_error_stdin(
        dir.path(),
        "TypeError: helper is not a function\n    at loadUser (src/services/user.ts:2:32)\n",
        &["--format", "json"],
    );

    assert_eq!(output.code, 0, "stderr:\n{}", output.stderr);
    let value = parse_json(&output);
    assert_eq!(value["kind"], "trace-error");
    assert_eq!(value["schema_version"], "1");
    assert_eq!(value["source"], "stdin");
    assert_eq!(value["header"], "TypeError: helper is not a function");
    assert_eq!(value["frames"][0]["origin"], "in_project");
    assert_eq!(value["frames"][0]["resolution"], "resolved");
    assert_eq!(value["frames"][0]["function"], "loadUser");
    assert_eq!(value["frames"][0]["line"], 2);
    assert_eq!(
        value["frames"][0]["candidates"][0]["file"],
        "src/services/user.ts"
    );
    assert_eq!(value["frames"][0]["candidates"][0]["symbol"], "loadUser");
    assert_eq!(value["frames"][0]["candidates"][0]["kind"], "export");
    assert_eq!(value["frames"][0]["candidates"][0]["line"], 2);
    assert_eq!(value["counts"]["resolved"], 1);
    assert_eq!(value["counts"]["frames"], 1);
}

#[test]
fn a_frame_naming_several_definitions_says_so_instead_of_picking_one() {
    let dir = tempdir().unwrap();
    write_project(dir.path());

    let output = run_trace_error_stdin(
        dir.path(),
        "Error: boom\n    at Task.run (src/task.ts:4:12)\n",
        &["--format", "json"],
    );

    assert_eq!(output.code, 0, "stderr:\n{}", output.stderr);
    let value = parse_json(&output);
    let frame = &value["frames"][0];
    assert_eq!(frame["resolution"], "ambiguous");
    assert_eq!(
        frame["candidates"].as_array().unwrap().len(),
        2,
        "both the standalone export and the class method are listed: {frame}"
    );
    assert_eq!(frame["candidates"][0]["symbol"], "Task");
    assert_eq!(frame["candidates"][0]["member"], "run");
    assert_eq!(frame["candidates"][0]["kind"], "class-method");
    assert_eq!(frame["candidates"][1]["symbol"], "run");
    assert!(frame["candidates"][1].get("member").is_none());
    assert_eq!(frame["candidates_omitted"], 0);
    assert_eq!(value["counts"]["ambiguous"], 1);
    assert_eq!(value["counts"]["resolved"], 0);
}

#[test]
fn a_frame_naming_a_module_local_function_is_not_found_not_dropped() {
    let dir = tempdir().unwrap();
    write_project(dir.path());

    let output = run_trace_error_stdin(
        dir.path(),
        "Error: boom\n    at helper (src/services/user.ts:1:16)\n",
        &["--format", "json"],
    );

    assert_eq!(output.code, 0, "stderr:\n{}", output.stderr);
    let value = parse_json(&output);
    let frame = &value["frames"][0];
    assert_eq!(frame["origin"], "in_project");
    assert_eq!(frame["resolution"], "not_found");
    assert_eq!(frame["function"], "helper");
    assert_eq!(frame["candidates"].as_array().unwrap().len(), 0);
    assert_eq!(value["counts"]["not_found"], 1);
    assert_eq!(
        value["counts"]["frames"], 1,
        "a not-found frame still occupies a row"
    );
}

#[test]
fn frames_outside_project_source_are_kept_classified_and_never_asked() {
    let dir = tempdir().unwrap();
    write_project(dir.path());

    let output = run_trace_error_stdin(
        dir.path(),
        "Error: boom\n\
         \x20   at loadUser (src/services/user.ts:2:32)\n\
         \x20   at vendored (node_modules/vendor/index.js:1:22)\n\
         \x20   at n (dist/bundle.js:1:200)\n\
         \x20   at process.processTicksAndRejections (node:internal/process/task_queues:95:5)\n\
         \x20   at Array.forEach (<anonymous>)\n",
        &["--format", "json"],
    );

    assert_eq!(output.code, 0, "stderr:\n{}", output.stderr);
    let value = parse_json(&output);
    let frames = value["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 5, "no frame is filtered out of the array");
    assert_eq!(frames[0]["origin"], "in_project");
    assert_eq!(frames[1]["origin"], "node_modules");
    assert_eq!(frames[1]["resolution"], "not_attempted");
    assert_eq!(frames[2]["origin"], "out_of_corpus");
    assert_eq!(frames[2]["resolution"], "not_attempted");
    assert!(
        frames[2]["reason"].as_str().unwrap().contains("source map"),
        "a generated-bundle frame names why it was not resolved: {}",
        frames[2]["reason"]
    );
    assert_eq!(frames[3]["origin"], "out_of_corpus");
    assert_eq!(frames[4]["origin"], "out_of_corpus");
    assert!(frames[4].get("file").is_none());

    let counts = &value["counts"];
    assert_eq!(counts["frames"], 5);
    assert_eq!(counts["in_project"], 1);
    assert_eq!(counts["node_modules"], 1);
    assert_eq!(counts["out_of_corpus"], 3);
    assert_eq!(counts["resolved"], 1);
    assert_eq!(counts["not_attempted"], 4);
    let sum = counts["resolved"].as_u64().unwrap()
        + counts["ambiguous"].as_u64().unwrap()
        + counts["not_found"].as_u64().unwrap()
        + counts["not_attempted"].as_u64().unwrap();
    assert_eq!(sum, counts["frames"].as_u64().unwrap());
}

#[test]
fn an_empty_trace_is_an_answer_not_an_error() {
    let dir = tempdir().unwrap();
    write_project(dir.path());

    let output = run_trace_error_stdin(dir.path(), "", &["--format", "json"]);

    assert_eq!(output.code, 0, "stderr:\n{}", output.stderr);
    let value = parse_json(&output);
    assert_eq!(value["kind"], "trace-error");
    assert_eq!(value["frames"].as_array().unwrap().len(), 0);
    assert_eq!(value["counts"]["frames"], 0);
    assert_eq!(value["counts"]["unparsed_lines"], 0);
    assert!(value.get("header").is_none());
    assert_eq!(value["reason"], "no stack frames in the input");
}

#[test]
fn a_trace_with_no_recognisable_frames_reports_the_lines_it_could_not_read() {
    let dir = tempdir().unwrap();
    write_project(dir.path());

    let output = run_trace_error_stdin(
        dir.path(),
        "something went wrong\nsee the logs\nand the dashboard\n",
        &["--format", "json"],
    );

    assert_eq!(output.code, 0, "stderr:\n{}", output.stderr);
    let value = parse_json(&output);
    assert_eq!(value["counts"]["frames"], 0);
    assert_eq!(
        value["counts"]["unparsed_lines"], 2,
        "the first line is reported as the header, the rest are counted"
    );
    assert_eq!(value["header"], "something went wrong");
    assert!(
        value["reason"]
            .as_str()
            .unwrap()
            .contains("no stack frames recognised"),
        "reason was {}",
        value["reason"]
    );
}

#[test]
fn a_trace_read_from_a_file_reports_the_path_as_its_source() {
    let dir = tempdir().unwrap();
    write_project(dir.path());
    let trace_path = dir.path().join("crash.txt");
    std::fs::write(
        &trace_path,
        "Error: boom\n    at loadUser (src/services/user.ts:2:32)\n",
    )
    .unwrap();

    let output = run_fallow_in_root(
        "trace-error",
        dir.path(),
        &[trace_path.to_str().unwrap(), "--format", "json"],
    );

    assert_eq!(output.code, 0, "stderr:\n{}", output.stderr);
    let value = parse_json(&output);
    assert_eq!(value["source"], trace_path.to_str().unwrap());
    assert_eq!(value["frames"][0]["resolution"], "resolved");
}

#[test]
fn an_unreadable_trace_file_exits_two() {
    let dir = tempdir().unwrap();
    write_project(dir.path());

    let output = run_fallow_in_root(
        "trace-error",
        dir.path(),
        &["does-not-exist.txt", "--format", "json"],
    );

    assert_eq!(output.code, 2, "stdout:\n{}", output.stdout);
}

#[test]
fn the_firefox_frame_form_resolves_the_same_way() {
    let dir = tempdir().unwrap();
    write_project(dir.path());

    let output = run_trace_error_stdin(
        dir.path(),
        "loadUser@src/services/user.ts:2:32\n",
        &["--format", "json"],
    );

    assert_eq!(output.code, 0, "stderr:\n{}", output.stderr);
    let value = parse_json(&output);
    assert_eq!(value["frames"][0]["resolution"], "resolved");
    assert_eq!(value["frames"][0]["function"], "loadUser");
}

#[test]
fn repeated_runs_return_a_byte_identical_payload() {
    let dir = tempdir().unwrap();
    write_project(dir.path());
    let trace = "Error: boom\n    at Task.run (src/task.ts:4:12)\n    at loadUser (src/services/user.ts:2:32)\n";

    let first = run_trace_error_stdin(dir.path(), trace, &["--format", "json"]);
    let second = run_trace_error_stdin(dir.path(), trace, &["--format", "json"]);

    assert_eq!(first.code, 0, "stderr:\n{}", first.stderr);
    let strip = |value: serde_json::Value| {
        let mut value = value;
        value.as_object_mut().map(|object| object.remove("_meta"));
        serde_json::to_string(&value).unwrap()
    };
    assert_eq!(strip(parse_json(&first)), strip(parse_json(&second)));
}

#[test]
fn human_output_prints_every_candidate_of_an_ambiguous_frame() {
    let dir = tempdir().unwrap();
    write_project(dir.path());

    let output = run_trace_error_stdin(
        dir.path(),
        "Error: boom\n    at Task.run (src/task.ts:4:12)\n",
        &[],
    );

    assert_eq!(output.code, 0, "stderr:\n{}", output.stderr);
    assert!(
        output.stdout.contains("in-project/ambiguous"),
        "stdout:\n{}",
        output.stdout
    );
    assert!(
        output.stdout.contains("Task.run (class-method)"),
        "stdout:\n{}",
        output.stdout
    );
    assert!(
        output.stdout.contains("run (export)"),
        "stdout:\n{}",
        output.stdout
    );
}

#[test]
fn a_relative_trace_path_resolves_against_the_project_root() {
    let dir = tempdir().unwrap();
    write_project(dir.path());
    std::fs::write(
        dir.path().join("crash.txt"),
        "Error: boom\n    at loadUser (src/services/user.ts:2:32)\n",
    )
    .unwrap();

    let output = run_fallow_in_root(
        "trace-error",
        dir.path(),
        &["crash.txt", "--format", "json"],
    );

    assert_eq!(output.code, 0, "stderr:\n{}", output.stderr);
    let value = parse_json(&output);
    assert_eq!(
        value["source"], "crash.txt",
        "the reported source keeps the caller's spelling, not the resolved path"
    );
    assert_eq!(value["frames"][0]["resolution"], "resolved");
}
