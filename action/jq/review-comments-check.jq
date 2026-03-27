def prefix: $ENV.PREFIX // "";
def footer(rule): "\n\n---\n<sub><a href=\"https://docs.fallow.tools/explanations/dead-code#" + rule + "\">Docs</a> \u00b7 Disagree? <a href=\"https://docs.fallow.tools/configuration/suppression\">Configure or suppress</a></sub>";
[
  (.unused_files[]? | {
    path: (prefix + .path),
    line: 1,
    body: ":warning: **Unused file**\n\nThis file is not imported by any module and is unreachable from all entry points.\n\n<details>\n<summary>Why this matters</summary>\n\nUnused files mislead developers into thinking this code is still active \u2014 leading to wasted time reading, maintaining, and reviewing dead code. They also slow down IDE indexing and search results.\n</details>\n\n**Action:** Delete this file, or import it where needed.\(footer("unused-files"))"
  }),
  (.unused_exports[]? | {
    path: (prefix + .path),
    line: .line,
    body: ":warning: **Unused \(if .is_type_only then "type " else "" end)export**\n\n\(if .is_re_export then "Re-exported" else "Exported" end) \(if .is_type_only then "type" else "value" end) `\(.export_name)` is never imported by other modules.\n\n<details>\n<summary>Why this matters</summary>\n\nUnused exports signal to other developers that this code is used elsewhere \u2014 so nobody touches it, even when it should change. They also prevent bundlers from tree-shaking this code out of production.\n</details>\n\n**Action:** Remove the `export` keyword, or delete the declaration entirely.\n\n> Intentionally public? Add this file to [`entry`](https://docs.fallow.tools/configuration/overview) in your `.fallowrc.json` so fallow knows it\u2019s an API boundary.\(footer("unused-exports"))"
  }),
  (.unused_types[]? | {
    path: (prefix + .path),
    line: .line,
    body: ":warning: **Unused type export**\n\n\(if .is_re_export then "Re-exported" else "Exported" end) type `\(.export_name)` is never imported by other modules.\n\n**Action:** Remove the `export` keyword if only used internally.\(footer("unused-types"))"
  }),
  (.unused_dependencies[]? | {
    path: (prefix + .path),
    line: (if .line > 0 then .line else 1 end),
    body: ":warning: **Unused dependency**\n\n`\(.package_name)` is listed in `\(.location)` but no file imports it directly.\n\n**Action:** If nothing in your code imports this package, remove it:\n\n```sh\nnpm uninstall \(.package_name)\n```\n\n<details>\n<summary>Why this matters</summary>\n\nUnused dependencies slow down installs, inflate `node_modules`, and add noise to security audits.\n</details>\n\n> Some packages are used indirectly (peer dependencies, framework internals, or plugin systems). If that\u2019s the case, add it to [`ignoreDependencies`](https://docs.fallow.tools/configuration/overview) in `.fallowrc.json`.\(footer("unused-dependencies"))"
  }),
  (.unused_dev_dependencies[]? | {
    path: (prefix + .path),
    line: (if .line > 0 then .line else 1 end),
    body: ":warning: **Unused devDependency**\n\n`\(.package_name)` is listed in `devDependencies` but no file imports it.\n\n```sh\nnpm uninstall \(.package_name)\n```\n\n> Used by a tool that doesn\u2019t import directly (e.g., a CLI, plugin, or preset)? Add it to [`ignoreDependencies`](https://docs.fallow.tools/configuration/overview) in `.fallowrc.json`.\(footer("unused-dependencies"))"
  }),
  (.unused_optional_dependencies[]? | {
    path: (prefix + .path),
    line: (if .line > 0 then .line else 1 end),
    body: ":warning: **Unused optionalDependency**\n\n`\(.package_name)` is listed in `optionalDependencies` but no file imports it.\n\n```sh\nnpm uninstall \(.package_name)\n```\(footer("unused-dependencies"))"
  }),
  (.unused_enum_members[]? | {
    path: (prefix + .path),
    line: .line,
    body: ":warning: **Unused enum member**\n\n`\(.parent_name).\(.member_name)` is never referenced anywhere in the codebase.\n\n**Action:** Remove this member to keep the enum minimal.\n\n> Run `fallow fix` to auto-remove unused enum members.\(footer("unused-enum-members"))"
  }),
  (.unused_class_members[]? | {
    path: (prefix + .path),
    line: .line,
    body: ":warning: **Unused class member**\n\n`\(.parent_name).\(.member_name)` is never referenced.\n\n**Action:** Remove it or restrict its visibility.\(footer("unused-class-members"))"
  }),
  (.unresolved_imports[]? | {
    path: (prefix + .path),
    line: .line,
    body: ":x: **Unresolved import**\n\nImport `\(.specifier)` could not be resolved to a file or package.\n\n**Check for:**\n- Typo in the import path\n- File exists but isn\u2019t included in `tsconfig.json` (`include`/`exclude`)\n- Missing dependency \u2014 run `npm install <package>`\n- Path alias mismatch in `tsconfig.json` `paths`\(footer("unresolved-imports"))"
  }),
  (.unlisted_dependencies[]? | (.package_name) as $pkg | .imported_from[]? | {
    path: (prefix + .path),
    line: .line,
    body: ":x: **Unlisted dependency**\n\n`\($pkg)` is imported here but not declared in `package.json`. This will fail on a clean install.\n\n```sh\nnpm install \($pkg)\n```\(footer("unlisted-dependencies"))"
  }),
  (.duplicate_exports[]? | .locations as $locs | .locations[0] as $loc | {
    path: (prefix + $loc.path),
    line: $loc.line,
    body: ":warning: **Duplicate export**\n\nExport `\(.export_name)` is defined in \($locs | length) modules:\n\n\($locs | map("- `\(.path):\(.line)`") | join("\n"))\n\nThis causes ambiguity \u2014 barrel files may re-export the wrong one.\n\n**Action:** Keep one canonical location and remove the others.\(footer("duplicate-exports"))"
  }),
  (.circular_dependencies[]? | {
    path: (prefix + .files[0]),
    line: (if .line > 0 then .line else 1 end),
    body: ":warning: **Circular dependency**\n\nCircular import chain detected:\n\n```\n\(.files | join(" \u2192 ")) \u2192 \(.files[0])\n```\n\n<details>\n<summary>Why this matters</summary>\n\nCircular dependencies can cause:\n- **Runtime crashes** \u2014 modules may be `undefined` because they haven\u2019t finished initializing when first imported\n- **Import-order bugs** \u2014 behavior changes depending on which file is loaded first\n- **Broken code splitting** \u2014 bundlers may merge circular modules into a single chunk, defeating lazy loading\n</details>\n\n**Action:** Extract shared logic into a separate module that both files can import.\(footer("circular-dependencies"))"
  }),
  (.type_only_dependencies[]? | {
    path: (prefix + .path),
    line: (if .line > 0 then .line else 1 end),
    body: ":blue_book: **Type-only dependency**\n\n`\(.package_name)` is only used in `import type` statements \u2014 it\u2019s not needed at runtime.\n\n**Action:** Move it to `devDependencies`:\n\n```sh\nnpm install --save-dev \(.package_name) && npm uninstall \(.package_name)\n```\n\n> Publishing a library? If consumers need these types, keep it in `dependencies`.\(footer("type-only-dependencies"))"
  })
] | .[:($ENV.MAX | tonumber)]
