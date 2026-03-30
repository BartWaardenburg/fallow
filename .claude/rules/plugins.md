---
paths:
  - "crates/core/src/plugins/**"
  - "crates/config/src/external_plugin.rs"
---

# Plugin system

84 built-in plugins implementing the `Plugin` trait with enablers (package.json detection), static patterns, and optional `resolve_config()` for AST-based config parsing.

## Rich config parsing (15 plugins)

- **ESLint**: Legacy plugin/extends/parser short-name resolution, flat config plugin keys, JSON config, shared config following (reads imported config packages' entry points one level deep to discover peer deps), settings["import/resolver"] (string/array/object formats)
- **Vite**: rollupOptions.input, lib.entry, optimizeDeps include/exclude, ssr.external/noExternal
- **Jest**: preset, setupFiles, globalSetup/Teardown, testMatch, transform, reporters, testEnvironment, watchPlugins, resolver, snapshotSerializers, testRunner, runner, JSON config
- **Storybook**: addons, framework (string/object), stories, core.builder, typescript.reactDocgen
- **Tailwind**: content globs, plugins (require/strings), presets
- **Webpack**: entry (string/array/object), plugins require(), externals, module.rules loader extraction
- **TypeScript**: extends (string/array TS 5.0+), compilerOptions.types → @types/*, jsxImportSource, plugins, references, JSONC
- **Babel**: presets/plugins with short-name resolution, extends, JSON/.babelrc
- **Rollup**: input entries, external deps
- **PostCSS**: plugins (object keys, require() calls, string arrays)
- **Prettier**: plugins array (JSON/.prettierrc and JS configs)
- **Nuxt**: modules, css, plugins, extends, postcss plugins; path aliases (`~`, `~~`, `#shared`)
- **Drizzle**: schema field (string/array/glob/directory → entry points), out directory
- **Angular**: angular.json projects.*.architect.build.options → entry points; peer dep awareness
- **Vitest**: test.include, setupFiles, globalSetup, environment, reporters, coverage.provider, typecheck.checker, browser.provider; projects[*] nested extraction
- **Nx**: project.json targets.*.executor → deps; targets.*.options.{main, tsConfig} → entry points

## Plugin trait extensions
- `path_aliases()` for framework-specific alias resolution (Nuxt `~/`, Next.js `@/`)
- `virtual_module_prefixes()` for framework virtual modules (Docusaurus `@theme/`, `@docusaurus/`)

## External plugins
Standalone definitions in JSONC/JSON/TOML or inline via `framework` config field. Discovered from `plugins` config, `.fallow/plugins/`, and `fallow-plugin-*` files. See `docs/plugin-authoring.md`.
