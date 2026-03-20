# Backwards Compatibility Policy

Starting with v1.0, fallow follows [semantic versioning](https://semver.org/).

## What is stable

These interfaces are covered by semver — breaking changes only happen in major version bumps:

### Configuration format

- **Config file names**: `.fallowrc.json`, `fallow.toml`, `.fallow.toml`
- **All documented config fields**: `extends`, `ignorePatterns`, `rules`, `overrides`, `entry`, `ignoreDependencies`, `ignoreExports`, `duplicates`, `production`, `framework`, `workspaces`, `plugins`
- **Rule names and severity values**: `unused-files`, `unused-exports`, etc. with `error`/`warn`/`off`
- **Extends and overrides semantics**: merge behavior, glob matching, override precedence
- **Inline suppression comment syntax**: `fallow-ignore-next-line`, `fallow-ignore-file`

### JSON output schema

- **Top-level structure**: `schema_version`, `version`, `elapsed_ms`, `total_issues`, and all issue arrays
- **Issue type arrays**: `unused_files`, `unused_exports`, `unused_types`, `unused_dependencies`, `unused_dev_dependencies`, `unused_enum_members`, `unused_class_members`, `unresolved_imports`, `unlisted_dependencies`, `duplicate_exports`, `type_only_dependencies`, `circular_dependencies`
- **Issue object fields**: all fields documented in `docs/output-schema.json`
- **Schema version**: the `schema_version` field follows its own versioning (independent of the tool version). The schema version is bumped when the JSON output structure changes.

### CLI interface

- **Subcommands**: `check`, `dupes`, `fix`, `watch`, `init`, `migrate`, `list`, `schema`, `config-schema`, `plugin-schema`
- **Exit codes**: 0 (success/no errors), 1 (issues with error severity found), 2 (runtime error)
- **Global flags**: `--format`, `--config`, `--workspace`, `--performance`
- **Environment variables**: `FALLOW_FORMAT`, `FALLOW_QUIET`, `FALLOW_BIN`

### External plugin format

- **Plugin file structure**: as documented in `docs/plugin-authoring.md`
- **Detection types**: `dependency`, `fileExists`, `all`, `any`

## What may change in minor/patch versions

These are explicitly **not** covered by the stability guarantee:

- **New fields** may be added to config, JSON output, or plugin format (additive changes)
- **New issue types** may be added
- **New plugins** may be added to the built-in set
- **Detection accuracy**: false positive/negative rates may improve
- **Human-readable output**: terminal formatting, colors, wording
- **Performance characteristics**: timing, memory usage, parallelism
- **SARIF output details**: beyond what the SARIF spec requires
- **LSP protocol details**: diagnostics, code actions, Code Lens behavior
- **Internal crate APIs**: `fallow-core`, `fallow-config`, etc. are not public API

## Deprecation process

When a stable interface needs to change:

1. The old behavior is deprecated with a warning in the current major version
2. The new behavior is available alongside the old one
3. The old behavior is removed in the next major version

## Config format migration

The `fallow migrate` command helps migrate between config formats. When breaking config changes happen in a major version, `migrate` will be updated to handle the transition.
