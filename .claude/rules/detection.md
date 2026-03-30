---
paths:
  - "crates/core/src/analyze/**"
  - "crates/extract/src/visitor/**"
  - "crates/graph/src/graph/**"
  - "crates/graph/src/resolve/**"
---

# Detection capabilities

Non-obvious implementation details for each detection feature. These are NOT discoverable from reading the code alone.

## Parser-level
- **Vue/Svelte SFC**: handles `>` in quoted attributes like `generic="T extends Foo<Bar>"`, `<script src="...">` external script support, HTML comment filtering
- **Namespace destructuring**: `const { a, b } = ns` → member accesses. Rest patterns (`const { foo, ...rest } = ns`) → conservative whole-object use. Works with static/dynamic imports and require.
- **Unused import bindings**: via `oxc_semantic` scope-aware symbol analysis. Dead imports don't count as references, improving unused-export precision.
- **TypeScript overload dedup**: `export function foo(): void; export function foo(x: string): string;` treated as single export
- **Class instance members**: `const svc = new MyService(); svc.greet()` tracks `greet` as used. Scope-unaware — false matches produce false negatives, not false positives.
- **Type-level member access**: `TSQualifiedName` (e.g., `type X = Status.Active`) tracked as member access. Mapped type constraints (`{ [K in Enum]: ... }`, `{ [K in keyof typeof Enum]: ... }`) and `Record<Enum, T>` mark all enum members as used via whole-object use.

## Resolution-level
- **Package.json `exports` subpath**: output dirs (dist/build/out/esm/cjs) mapped back to src/ with source extension fallback, including nested subdirs
- **Pnpm virtual store**: `.pnpm` paths mapped back to workspace source files. Handles injected deps, scoped/unscoped packages, peer dependency suffixes.
- **Package.json `imports` (`#subpath`)**: simple mappings, wildcard patterns, conditional exports. Per-package scoping.
- **React Native platform extensions**: `.web.ts`, `.ios.ts`, `.android.ts`, `.native.ts` resolved alongside standard extensions
- **Tsconfig path aliases**: per-file discovery resolves `@/utils` by finding nearest tsconfig.json per file

## Graph-level
- **`export *` chain propagation**: multi-level barrel file chains fully resolved for transitive usage tracking
- **CSS Modules**: default imports (`import styles from '...'`) resolve `styles.className` to named exports via graph-level narrowing. Spread/`Object.values` handled conservatively.

## Analysis-level
- **Decorated class members**: members with decorators (NestJS `@Get()`, Angular `@Input()`, TypeORM `@Column()`) not reported unused
- **JSDoc `@public` tag**: exports annotated with `/** @public */` never reported unused. Only `/** */` block comments recognized.
- **Infrastructure entry points**: Dockerfiles, Procfiles, fly.toml scanned for source file refs. Searches root and config/docker/deploy subdirs.
- **TypeScript project references**: tsconfig.json `references` field discovered as workspaces (additive with npm/pnpm workspaces)
- **CI file scanning**: `.gitlab-ci.yml` and `.github/workflows/*.yml` scanned for binary invocations (npx, direct binaries). Maps binaries to package names to prevent false "unused dependency" reports for CI-only packages.
- **Production mode**: excludes test/dev files, only start/build scripts, skips unused devDeps, detects type-only deps
- **@types/ unlisted dep suppression**: imports from `pkg` not flagged as unlisted when `@types/pkg` is in deps (TypeScript resolves types from `@types/` and erases the import regardless of `import type` syntax). Scoped packages use DefinitelyTyped convention (`@scope/pkg` → `@types/scope__pkg`).
