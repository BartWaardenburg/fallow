# fallow

Find dead code in JavaScript and TypeScript projects. Written in Rust.

[![CI](https://github.com/BartWaardenburg/fallow/actions/workflows/ci.yml/badge.svg)](https://github.com/BartWaardenburg/fallow/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/fallow.svg)](https://www.npmjs.com/package/fallow)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bartwaardenburg/fallow/blob/main/LICENSE)

Fallow detects unused files, exports, dependencies, types, enum members, and class members across your codebase. It is a drop-in alternative to [knip](https://knip.dev) that runs **25-50x faster** on real-world projects by using the [Oxc](https://oxc.rs) parser instead of the TypeScript compiler.

## Installation

```bash
npm install -g fallow
```

## Usage

```bash
# Analyze your project
fallow check

# Watch mode
fallow watch

# Auto-fix unused exports and dependencies
fallow fix --dry-run
fallow fix

# JSON output for CI
fallow check --format json
```

## What it finds

1. **Unused files** - files not imported anywhere
2. **Unused exports** - exported symbols nobody imports
3. **Unused types** - exported type aliases and interfaces
4. **Unused dependencies** - packages in package.json not imported
5. **Unused devDependencies** - dev packages not referenced
6. **Unused enum members** - enum variants never accessed
7. **Unused class members** - methods/properties never used
8. **Unresolved imports** - imports that don't resolve to a file
9. **Unlisted dependencies** - imported packages not in package.json
10. **Duplicate exports** - same symbol exported from multiple files

## Framework support

Auto-detects and configures entry points for: Next.js, Vite, Vitest, Jest, Storybook, Remix, Astro, Nuxt, Angular, Playwright, Prisma, ESLint, TypeScript, Webpack, Tailwind, GraphQL Codegen, React Router.

## Configuration

Create a `fallow.toml` in your project root:

```toml
[entry]
patterns = ["src/index.ts", "src/main.ts"]

[ignore]
files = ["**/*.test.ts", "**/*.spec.ts"]
exports = ["src/public-api.ts"]
dependencies = ["@types/*"]
```

Or generate one: `fallow init`

## Documentation

Full documentation at [github.com/bartwaardenburg/fallow](https://github.com/bartwaardenburg/fallow).

## License

MIT
