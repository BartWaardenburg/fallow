# Fallow for VS Code

Dead code and duplication analyzer for JavaScript/TypeScript projects. Powered by [fallow](https://docs.fallow.tools), a Rust-native alternative to knip that is 3-36x faster than knip v5 (2-14x faster than knip v6).

## Features

- **Real-time diagnostics** via the fallow LSP server: unused files, exports, types, dependencies, enum/class members, unresolved imports, unlisted deps, and duplicate exports
- **Quick-fix code actions**: remove unused exports, delete unused files
- **Tree views**: browse dead code by issue type and duplicates by clone family in the sidebar
- **Status bar**: see total issue count and duplication percentage at a glance
- **Auto-fix**: remove unused exports and dependencies with one command
- **Auto-download**: the extension downloads the `fallow-lsp` binary automatically

## Installation

### From the Marketplace

Search for "Fallow" in the VS Code extensions panel, or install from the command line:

```sh
code --install-extension fallow-rs.fallow-vscode
```

### Manual

1. Install the `fallow` and `fallow-lsp` binaries (see [fallow installation](https://docs.fallow.tools/installation))
2. Install the extension VSIX file: `code --install-extension fallow-vscode-*.vsix`

## Commands

| Command | Description |
|---------|-------------|
| `Fallow: Run Analysis` | Run full dead code + duplication analysis and update tree views |
| `Fallow: Auto-Fix Unused Exports & Dependencies` | Remove unused exports and dependencies |
| `Fallow: Preview Fixes (Dry Run)` | Show what fixes would be applied without changing files |
| `Fallow: Restart Language Server` | Restart the fallow-lsp process |
| `Fallow: Show Output Channel` | Open the Fallow output panel for debugging |

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `fallow.lspPath` | `""` | Path to the `fallow-lsp` binary. Leave empty for auto-detection. |
| `fallow.autoDownload` | `true` | Automatically download the binary if not found. |
| `fallow.issueTypes` | all enabled | Toggle individual issue types on/off. |
| `fallow.duplication.threshold` | `5` | Duplication threshold percentage. |
| `fallow.duplication.mode` | `"mild"` | Detection mode: `strict`, `mild`, `weak`, or `semantic`. |
| `fallow.production` | `false` | Production mode: exclude test/dev files, only production scripts. |
| `fallow.trace.server` | `"off"` | LSP trace level: `off`, `messages`, or `verbose`. |

## Binary resolution

The extension looks for the `fallow-lsp` binary in this order:

1. `fallow.lspPath` setting (if configured)
2. `fallow-lsp` in `PATH`
3. Previously downloaded binary in extension storage
4. Auto-download from GitHub releases (if `fallow.autoDownload` is enabled)

## Development

```sh
cd editors/vscode
npm install
npm run build        # Production build
npm run watch        # Watch mode for development
npm run lint         # Type check
npm run package      # Package as .vsix
```
