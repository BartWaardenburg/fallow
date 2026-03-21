---
name: release
description: Bump version, tag, write changelog, and create a GitHub release for fallow
---

Create a new release for the fallow project. Handles version bumping, npm version syncing, changelog writing, tagging, and GitHub release creation.

## Usage

- `/release patch` — bump patch version (0.1.0 → 0.1.1)
- `/release minor` — bump minor version (0.1.0 → 0.2.0)
- `/release major` — bump major version (0.1.0 → 1.0.0)
- `/release 0.2.0` — set an explicit version
- `/release` — defaults to patch

## Steps

### 1. Determine the new version

Read the current version from the root `Cargo.toml` workspace config (`workspace.package.version`). Apply the requested bump (patch/minor/major). If the user passed a specific version like `1.0.0`, use that instead.

### 2. Pre-flight checks

- Ensure the working tree is clean (`git status` shows no uncommitted changes). If dirty, stop and ask the user to commit or stash first.
- Ensure you're on the `main` branch.
- Run `cargo test --workspace` to verify all tests pass. If tests fail, stop.
- Run `cargo clippy --workspace -- -D warnings` to check for lint issues. If clippy fails, stop.

### 3. Update CHANGELOG.md

Before bumping versions, update the `CHANGELOG.md` file:

1. Read `CHANGELOG.md` and the `[Unreleased]` section
2. Rename `[Unreleased]` to `[{version}] - {YYYY-MM-DD}` with today's date
3. Add a new empty `[Unreleased]` section above it
4. Update the comparison links at the bottom of the file:
   - Add: `[Unreleased]: https://github.com/fallow-rs/fallow/compare/v{version}...HEAD`
   - Add: `[{version}]: https://github.com/fallow-rs/fallow/compare/v{prev_version}...v{version}`
5. Review the changelog entries — ensure they cover all significant changes since the last release. Check `git log {prev_tag}..HEAD --oneline` for anything missing.

### 4. Bump versions

Use `cargo release version <bump> --execute --no-confirm` to bump all crate versions in Cargo.toml files.

Then sync npm package versions by running:

```bash
bash scripts/sync-npm-versions.sh "<old_version>" "<new_version>"
```

This updates:
- `npm/fallow/package.json` (version + optionalDependencies)
- All `npm/*/package.json` platform packages

### 5. Commit and tag

- Stage all changed files: `CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`, `npm/*/package.json`
- Commit with signed commit: `git commit -S -m "chore: release v{version}"`
- Create a signed annotated tag: `git tag -s v{version} -m "v{version}"`

### 6. Gather changelog inputs for GitHub release

Collect the raw material for the GitHub release notes:

```bash
# Get the previous tag (if any)
git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "(first release)"

# All commits since last tag (or all commits if first release)
git log {prev_tag}..HEAD --oneline  # or `git log --oneline` for first release

# Full diff stats
git diff {prev_tag}..HEAD --stat  # or `git diff --stat $(git rev-list --max-parents=0 HEAD)..HEAD` for first release
```

### 7. Write the GitHub release changelog

Write a high-quality changelog for the GitHub release. This is separate from `CHANGELOG.md` — the GitHub release notes should be more narrative and highlight what matters most.

**Structure by theme, not by commit.** Group related changes under descriptive headings like:
- Features
- Performance
- Bug fixes
- Breaking changes (if any)
- Infrastructure / CI

**Write for the audience** — developers evaluating fallow as a codebase analyzer for JS/TS projects. Explain what matters, not just what changed.

**Be honest about tradeoffs.** If something is experimental or has known limitations, say so.

**Keep it scannable.** Use bullet points, code blocks for CLI commands, and bold for emphasis. Don't pad with filler.

**Include at the bottom:**
```
**Full Changelog**: https://github.com/fallow-rs/fallow/compare/{prev_tag}...v{version}
```
For the first release, use:
```
**Full Changelog**: https://github.com/fallow-rs/fallow/commits/v{version}
```

### 8. Push

Ask the user for confirmation, then:

```bash
git push && git push --tags
```

Pushing the tag triggers the CI release workflow (`.github/workflows/release.yml`) which automatically:
- Publishes Rust crates to crates.io in dependency order (fallow-types → fallow-config → fallow-extract → fallow-graph → fallow-core → fallow-cli → fallow-mcp)
- Builds release binaries for 7 platform targets (macOS x64/ARM, Linux x64/ARM GNU/musl, Windows x64)
- Creates a GitHub Release with build artifacts
- Publishes all `@fallow-cli/*` npm platform packages with provenance
- Publishes the main `fallow` npm wrapper package
- Publishes the VS Code extension to the marketplace

There is no need to publish manually — CI handles everything.

### 9. Create the GitHub release

After pushing (so `--verify-tag` can find the tag on the remote):

```bash
gh release create v{version} --title "v{version} — {short_summary}" --notes "{changelog}" --verify-tag
```

Note: The CI workflow also creates a release with auto-generated notes. The `gh release create` command will fail if the release already exists. In that case, update the existing release:

```bash
gh release edit v{version} --title "v{version} — {short_summary}" --notes "{changelog}"
```

### 10. Monitor CI

After the release is created, check that the CI release workflow is running:

```bash
gh run list --workflow=release.yml --limit=1
```

Report the workflow run URL to the user so they can monitor publishing progress.

## Important notes

- Always use signed commits (`git commit -S`) and signed tags (`git tag -s`)
- Never add Co-Authored-By or AI attribution to commits
- Use conventional commit format: `chore: release v{version}`
- The changelog quality matters — this is what people see on GitHub. Take the time to write it well.
- CI publishes automatically on tag push — never publish manually unless CI fails and you need to retry
- The `release.toml` config has `push = true`, but we push manually to control timing (tag push triggers CI)
- For `cargo release`, we only use `cargo release version` (not the full `cargo release` which would also push)
- The CI also publishes Rust crates to crates.io — 7 crates in dependency order with 30s delays between each for index propagation
- The VS Code extension version is set from the git tag by CI — no need to update `editors/vscode/package.json` locally
