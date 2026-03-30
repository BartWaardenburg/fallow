---
paths:
  - "**/*.rs"
  - "Cargo.toml"
  - ".clippy.toml"
---

# Rust code quality

## Clippy configuration
- Lint groups: `all`, `pedantic`, `nursery`, `cargo` (priority -1) with strategic allow-list
- Restriction lints including `excessive_nesting` (threshold 7 in `.clippy.toml`); see `[workspace.lints.clippy]` in `Cargo.toml` for the full list
- Compiler lints: `unsafe_op_in_unsafe_fn`, `unused_unsafe`, `non_ascii_idents`
- All suppressions use `#[expect(clippy::...)]` — warns when unnecessary, preventing dead annotations

## Size assertions
`ModuleNode` (96 bytes), `ModuleInfo` (256 bytes), `ExportInfo`/`ImportInfo` (88 bytes), `Edge` (32 bytes) — prevents accidental struct bloat.

## Dev profile
`debug = false` for faster builds. Selective `opt-level` for proc-macro crates (`serde_derive`, `clap_derive`) and snapshot test deps (`insta`, `similar`).

## Cross-platform test paths
CI runs on Linux, macOS, and Windows. Tests that assert on file paths MUST normalize separators:
- Use `.replace('\\', "/")` on any `to_string_lossy()` path before comparing with string literals containing `/`
- Or use `Path::ends_with()` / `Path::components()` which are separator-agnostic
- Never hardcode `"src\\foo.ts"` or `"src/foo.ts"` without normalizing first

## Disallowed types
`HashMap` and `HashSet` from `std::collections` are forbidden (configured in `.clippy.toml`). Use `FxHashMap` / `FxHashSet` from `rustc_hash` — faster hashing, deterministic iteration order for test stability. New proc-macro dependencies must be added to `[profile.dev.package.*]` with `opt-level = 1` (see existing entries for `serde_derive`, `clap_derive`).

## CI hardening
- `permissions: {}` deny-all baseline on all workflows
- `git diff --exit-code` to catch uncommitted generated code
- `--document-private-items` rustdoc check
- `cargo-shear` for unused dependency detection
- `zizmor` for GitHub Actions security scanning
- `cargo-bloat` for binary size tracking
