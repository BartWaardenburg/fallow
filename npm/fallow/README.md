# fallow

The codebase analyzer for JavaScript and TypeScript, built in Rust.

[![CI](https://github.com/fallow-rs/fallow/actions/workflows/ci.yml/badge.svg)](https://github.com/fallow-rs/fallow/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/fallow.svg)](https://www.npmjs.com/package/fallow)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/fallow-rs/fallow/blob/main/LICENSE)

Unused code, circular dependencies, and code duplication. Found in seconds, not minutes. fallow analyzes your codebase for unused files, exports, dependencies, and types, detects circular dependencies, and finds duplicated code blocks. **3-36x faster** than [knip](https://knip.dev) v5 (**2-14x faster** than knip v6), **20-33x faster** than [jscpd](https://github.com/kucherenko/jscpd) for duplication detection, with no Node.js runtime dependency.

## Installation

```bash
npm install -g fallow
```

## Usage

```bash
fallow check                     # Unused code analysis -- zero config, sub-second
fallow dupes                     # Duplication detection -- find copy-paste clones
fallow dupes --mode semantic     # Catch clones with renamed variables
fallow fix --dry-run             # Preview auto-removal of unused exports and deps
```

## What it finds

- **Unused files** -- not reachable from any entry point
- **Unused exports** -- exported symbols never imported elsewhere
- **Unused types** -- type aliases and interfaces never referenced
- **Unused dependencies** -- packages in `dependencies` never imported
- **Unused devDependencies** -- dev packages not referenced
- **Unused enum members** -- enum values never referenced
- **Unused class members** -- class methods and properties never referenced
- **Unresolved imports** -- import specifiers that cannot be resolved
- **Unlisted dependencies** -- imported packages missing from `package.json`
- **Duplicate exports** -- same symbol exported from multiple modules
- **Circular dependencies** -- import cycles in the module graph
- **Type-only dependencies** -- production deps only used via `import type`

## Code duplication

```bash
fallow dupes                       # Default: mild mode
fallow dupes --mode semantic       # Catch clones with renamed variables
fallow dupes --threshold 5         # Fail CI if duplication exceeds 5%
fallow dupes --save-baseline       # Save current duplication as baseline
```

4 detection modes (strict, mild, weak, semantic), clone family grouping with refactoring suggestions, baseline tracking, and cross-language TS/JS matching.

## Framework support

84 built-in plugins covering Next.js, Nuxt, Remix, SvelteKit, Gatsby, Astro, Angular, NestJS, Vite, Webpack, Vitest, Jest, Playwright, Cypress, Storybook, ESLint, TypeScript, Tailwind, Prisma, Drizzle, Turborepo, and many more. Auto-detected from your `package.json`.

## Configuration

Create a config file in your project root, or run `fallow init`:

```jsonc
// .fallowrc.json
{
  "$schema": "https://raw.githubusercontent.com/fallow-rs/fallow/main/schema.json",
  "entry": ["src/workers/*.ts", "scripts/*.ts"],
  "ignorePatterns": ["**/*.generated.ts"],
  "rules": {
    "unused-files": "error",
    "unused-exports": "warn",
    "unused-types": "off"
  }
}
```

Also supports TOML (`fallow init --toml` creates `fallow.toml`).

## Documentation

- [Full documentation](https://docs.fallow.tools)
- [GitHub repository](https://github.com/fallow-rs/fallow)
- [Plugin Authoring Guide](https://github.com/fallow-rs/fallow/blob/main/docs/plugin-authoring.md)

## License

MIT
