# Plugin Authoring Guide

Fallow supports external plugin definitions that let you add framework and tool support without writing Rust code. External plugins use a simple TOML format and provide the same declarative capabilities as built-in plugins.

## Quick Start

Create a file named `fallow-plugin-<name>.toml` in your project root:

```toml
name = "my-framework"
enablers = ["my-framework"]
entry_points = ["src/routes/**/*.{ts,tsx}"]
always_used = ["src/setup.ts"]
tooling_dependencies = ["my-framework-cli"]

[[used_exports]]
pattern = "src/routes/**/*.{ts,tsx}"
exports = ["default", "loader", "action"]
```

That's it. Fallow automatically discovers `fallow-plugin-*.toml` files in your project root.

## Plugin File Format

External plugins are TOML files with the following fields:

### Required

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Unique plugin name (shown in `fallow list --plugins`) |

### Optional

| Field | Type | Description |
|-------|------|-------------|
| `enablers` | string[] | Package names that activate this plugin |
| `entry_points` | string[] | Glob patterns for framework entry point files |
| `config_patterns` | string[] | Glob patterns for config files (marked always-used) |
| `always_used` | string[] | Glob patterns for files always considered used |
| `tooling_dependencies` | string[] | Packages used via CLI, not source imports |
| `used_exports` | table[] | Exports always considered used in matching files |

### `enablers`

Package names checked against `package.json` dependencies. The plugin activates if **any** enabler matches.

Supports prefix matching with a trailing `/`:

```toml
enablers = ["@myorg/"]  # matches @myorg/core, @myorg/cli, etc.
```

### `entry_points`

Glob patterns for files that serve as entry points to your application. These files are never flagged as unused, and their imports are traced through the module graph.

```toml
entry_points = [
  "src/routes/**/*.{ts,tsx}",
  "src/middleware.{ts,js}",
  "src/plugins/**/*.ts",
]
```

### `config_patterns`

Glob patterns for framework config files. When the plugin is active, these files are marked as always-used (they won't be flagged as unused files).

```toml
config_patterns = [
  "my-framework.config.{ts,js,mjs}",
  ".my-frameworkrc.{json,yaml}",
]
```

### `always_used`

Files that should always be considered used when this plugin is active, even if nothing imports them.

```toml
always_used = [
  "src/setup.ts",
  "public/**/*",
  "src/global.d.ts",
]
```

### `tooling_dependencies`

Packages that are tooling dependencies — used via CLI commands or config files, not imported in source code. These won't be flagged as unused dev dependencies.

```toml
tooling_dependencies = [
  "my-framework-cli",
  "@my-framework/dev-tools",
]
```

### `used_exports`

Exports that are always considered used for files matching a glob pattern. Use this for convention-based frameworks where specific export names have special meaning.

```toml
[[used_exports]]
pattern = "src/routes/**/*.{ts,tsx}"
exports = ["default", "loader", "action", "meta"]

[[used_exports]]
pattern = "src/middleware.ts"
exports = ["default"]
```

## Discovery

Fallow discovers external plugins in this order (first occurrence of a plugin name wins):

1. **Explicit paths** from the `plugins` config field
2. **`.fallow/plugins/`** directory — all `*.toml` files
3. **Project root** — `fallow-plugin-*.toml` files

### Using the `plugins` config field

Point to specific plugin files or directories:

```jsonc
// fallow.jsonc
{
  "plugins": [
    "tools/fallow-plugins/",
    "vendor/my-plugin.toml"
  ]
}
```

```toml
# fallow.toml
plugins = [
  "tools/fallow-plugins/",
  "vendor/my-plugin.toml",
]
```

### Using `.fallow/plugins/`

Place plugin TOML files in `.fallow/plugins/` for automatic discovery:

```
my-project/
  .fallow/
    plugins/
      my-framework.toml
      custom-tool.toml
  src/
  package.json
```

### Using project root

Name plugin files with the `fallow-plugin-` prefix:

```
my-project/
  fallow-plugin-my-framework.toml
  src/
  package.json
```

## Examples

### React Router / TanStack Router

```toml
name = "react-router"
enablers = ["react-router", "@tanstack/react-router"]

entry_points = [
  "src/routes/**/*.{ts,tsx}",
  "app/routes/**/*.{ts,tsx}",
]

config_patterns = [
  "react-router.config.{ts,js}",
]

tooling_dependencies = ["@react-router/dev"]

[[used_exports]]
pattern = "src/routes/**/*.{ts,tsx}"
exports = ["default", "loader", "action", "meta", "handle", "shouldRevalidate"]

[[used_exports]]
pattern = "app/routes/**/*.{ts,tsx}"
exports = ["default", "loader", "action", "meta", "handle", "shouldRevalidate"]
```

### Custom CMS

```toml
name = "my-cms"
enablers = ["@my-cms/core"]

entry_points = [
  "content/**/*.{ts,tsx}",
  "schemas/**/*.ts",
]

always_used = [
  "cms.config.ts",
  "content/**/*.mdx",
]

config_patterns = ["cms.config.{ts,js}"]
tooling_dependencies = ["@my-cms/cli"]

[[used_exports]]
pattern = "content/**/*.{ts,tsx}"
exports = ["default", "metadata", "getStaticProps"]
```

### Internal Tooling

```toml
name = "our-build-system"
enablers = ["@internal/build"]

config_patterns = [
  "build.config.{ts,js}",
  ".buildrc",
]

always_used = [
  "scripts/build/**/*.ts",
  "config/**/*.ts",
]

tooling_dependencies = [
  "@internal/build",
  "@internal/lint-rules",
  "@internal/test-utils",
]
```

## Sharing Plugins

External plugins are plain TOML files — share them however you share config:

- **Git**: check `fallow-plugin-*.toml` files into your repo
- **Monorepo**: put shared plugins in a central `tools/` directory and reference via `plugins` config
- **npm package**: publish a package containing plugin TOML files, then reference them: `plugins = ["node_modules/@my-org/fallow-plugins/"]`

## Built-in vs External Plugins

| Capability | Built-in | External |
|-----------|---------|---------|
| Entry points | Yes | Yes |
| Always-used files | Yes | Yes |
| Used exports | Yes | Yes |
| Tooling dependencies | Yes | Yes |
| Config file patterns | Yes | Yes |
| AST-based config parsing | Yes | No |
| Custom detection logic | Yes | No (enablers only) |

External plugins cover the vast majority of use cases. AST-based config parsing (extracting entry points from `vite.config.ts`, resolving ESLint plugin short names, etc.) requires a built-in Rust plugin.

## Verifying

Check that your plugin is detected:

```bash
fallow list --plugins
```

This shows all active plugins, including external ones.
