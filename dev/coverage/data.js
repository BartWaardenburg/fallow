window.BENCHMARK_DATA = {
  "lastUpdate": 1774611247908,
  "repoUrl": "https://github.com/fallow-rs/fallow",
  "entries": {
    "Fallow Coverage": [
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "bd1e9597c24d5083496cde7e016efa4246bcf52a",
          "message": "feat: add historical metric tracking and GitHub Pages dashboard\n\nAdd benchmark-action/github-action-benchmark tracking to allocs,\ncoverage, and conformance workflows alongside the existing bench\nworkflow. Each pushes to its own gh-pages directory (dev/allocs/,\ndev/coverage/, dev/conformance/) with auto-generated trend charts.\n\n- allocs.yml: track allocations over time (customSmallerIsBetter),\n  PR comments on allocation changes\n- coverage.yml: track coverage % over time (customBiggerIsBetter),\n  migrate badge push from fragile git stash to git worktree\n- conformance.yml: track agreement rate over time\n  (customBiggerIsBetter), upgrade permissions for gh-pages push\n- bench.yml: deploy dashboard index.html to gh-pages root\n- Add .github/pages/index.html dashboard linking to all 4 charts",
          "timestamp": "2026-03-23T10:55:20+01:00",
          "tree_id": "d760a210ccec7fe6e80dedc6efb46d2559f7201a",
          "url": "https://github.com/fallow-rs/fallow/commit/bd1e9597c24d5083496cde7e016efa4246bcf52a"
        },
        "date": 1774259818379,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.4,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "9a3969a2a764bb454c484e8a0a9d93490c6d69b3",
          "message": "refactor: migrate vscode extension bundler from esbuild to rolldown\n\nRolldown is built on Oxc, aligning the entire toolchain (parser + bundler)\non the same ecosystem. Output is comparable: 360KB minified, 57ms build.",
          "timestamp": "2026-03-23T11:11:20+01:00",
          "tree_id": "3f0a1c7f45b05bf2f940187b04e209e5633e8e10",
          "url": "https://github.com/fallow-rs/fallow/commit/9a3969a2a764bb454c484e8a0a9d93490c6d69b3"
        },
        "date": 1774260777325,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.4,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "eb707bfebfe7aba7168b56eca51af00e4ee64ecb",
          "message": "fix: add missing type_only_dependencies field in snapshot test",
          "timestamp": "2026-03-23T11:15:50+01:00",
          "tree_id": "3a196e9777ef417ef043e1da34a35fde455dd63b",
          "url": "https://github.com/fallow-rs/fallow/commit/eb707bfebfe7aba7168b56eca51af00e4ee64ecb"
        },
        "date": 1774261048282,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "9ee22e0ac9ee52892c55a4e6267c91a351533322",
          "message": "feat: add JSDoc @public tag support for library exports\n\nExports annotated with `/** @public */` or `/** @api public */` are\nnever reported as unused. Designed for library authors whose exports\nare consumed by external projects outside the analyzed repository.\n\nPipeline: ExportInfo.is_public extracted during parsing via\nComment.attached_to matching, cached through bincode (CACHE_VERSION\n12→13), propagated to ExportSymbol in the graph, and checked before\nreporting in find_unused_exports.\n\nWorks with all export types: named, default, class, interface, enum,\ntype alias, and multi-specifier exports. Only /** */ JSDoc block\ncomments are recognized.",
          "timestamp": "2026-03-23T13:24:21+01:00",
          "tree_id": "cbbdcd445be02a1f71452bc5c36acf170735a416",
          "url": "https://github.com/fallow-rs/fallow/commit/9ee22e0ac9ee52892c55a4e6267c91a351533322"
        },
        "date": 1774268758286,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "f51532c34fee568e60cd01e014c829366596e283",
          "message": "test: add integration test for package.json #subpath imports\n\nVerify that imports using # prefixes (e.g., `import { foo } from '#utils'`)\nresolve correctly via the package.json `imports` field. oxc_resolver already\nhandles this natively via its default `imports_fields` configuration.\n\nAdds test fixture with simple and wildcard patterns, and integration test\nasserting zero unresolved imports and correct unused-export detection.\n\nAlso verified against real-world Vite (74 # imports) and Svelte fixtures\nwith zero false positives.",
          "timestamp": "2026-03-23T14:41:15+01:00",
          "tree_id": "ac55c34a44be8860482520045cf6f8a08116dbeb",
          "url": "https://github.com/fallow-rs/fallow/commit/f51532c34fee568e60cd01e014c829366596e283"
        },
        "date": 1774273376840,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "ef25d6dfb0a344ee4565ea6a5aedbaffbd82cadd",
          "message": "feat: track class member usage through instance variables\n\nDetect `const x = new ClassName()` patterns and map subsequent\n`x.method()` / `x.property` accesses to `ClassName.method` /\n`ClassName.property`, so the unused class member detector correctly\nrecognizes instance-level usage.\n\nOn TanStack Query this eliminates 89 false positives (134 → 45 unused\nclass members). Built-in constructors (URL, Map, Set, etc.) are\nfiltered out to avoid spurious mappings.",
          "timestamp": "2026-03-23T15:26:22+01:00",
          "tree_id": "d46975e75975b260e1118bec84460380b601a405",
          "url": "https://github.com/fallow-rs/fallow/commit/ef25d6dfb0a344ee4565ea6a5aedbaffbd82cadd"
        },
        "date": 1774276087224,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.6,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "1f2adb8064b1b657ffd231563492e89533f86291",
          "message": "docs: add demo screencast and GIF",
          "timestamp": "2026-03-23T16:21:13+01:00",
          "tree_id": "76f77b82099a4d203a3211bf07cd049099c68d27",
          "url": "https://github.com/fallow-rs/fallow/commit/1f2adb8064b1b657ffd231563492e89533f86291"
        },
        "date": 1774279870232,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.6,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "c323a2e6d01c2aef0fb7aa9a69e27ceb1eae896b",
          "message": "chore: release v1.4.0",
          "timestamp": "2026-03-23T16:32:34+01:00",
          "tree_id": "12c0e584694d2450a2fc49a47ccf1987efbf4288",
          "url": "https://github.com/fallow-rs/fallow/commit/c323a2e6d01c2aef0fb7aa9a69e27ceb1eae896b"
        },
        "date": 1774280535021,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.6,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "c54237920bcc158308fd04a4f2e6b4144b22892f",
          "message": "docs: replace static screenshot with animated GIF in README",
          "timestamp": "2026-03-23T19:54:57+01:00",
          "tree_id": "a428621e41104b436106221d2beecb772488978a",
          "url": "https://github.com/fallow-rs/fallow/commit/c54237920bcc158308fd04a4f2e6b4144b22892f"
        },
        "date": 1774292285103,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.6,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "d61097c7c1194ca2ef9a35f2480481a8da2bfd2a",
          "message": "docs: update demo GIF with Victor Mono rendering",
          "timestamp": "2026-03-23T20:00:19+01:00",
          "tree_id": "ed81d0796cde7e0bea5cbcd989239120452b39c4",
          "url": "https://github.com/fallow-rs/fallow/commit/d61097c7c1194ca2ef9a35f2480481a8da2bfd2a"
        },
        "date": 1774292526261,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.6,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "79829d0c4b031a76c1efa31b40a23e8647f26270",
          "message": "chore: remove VHS tape, add .vscode to gitignore",
          "timestamp": "2026-03-23T20:07:32+01:00",
          "tree_id": "33536f69840db39da0217a238899b9327dfa9509",
          "url": "https://github.com/fallow-rs/fallow/commit/79829d0c4b031a76c1efa31b40a23e8647f26270"
        },
        "date": 1774292952289,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.6,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "cfc40e10e5c7565c9f289b246a6b1ad35915f9dd",
          "message": "fix: use relative paths in JSON output for readable CI summaries",
          "timestamp": "2026-03-23T20:16:02+01:00",
          "tree_id": "7d790ee98f30b035b64c44c56b4d87a15ce279f1",
          "url": "https://github.com/fallow-rs/fallow/commit/cfc40e10e5c7565c9f289b246a6b1ad35915f9dd"
        },
        "date": 1774293461897,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.6,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "16d1132e33db35f5a6d4a61a3632bbb0a27afa30",
          "message": "feat: redesign GitHub Action job summary with tables, icons, and collapsible sections",
          "timestamp": "2026-03-23T20:23:49+01:00",
          "tree_id": "133075b0bbea092a5c2bd3d693912f7bc8a64e6f",
          "url": "https://github.com/fallow-rs/fallow/commit/16d1132e33db35f5a6d4a61a3632bbb0a27afa30"
        },
        "date": 1774293936711,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.6,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "84926039eb3df83afefa1dbc9b66b447e0375445",
          "message": "refactor: remove emojis from GitHub Action job summary",
          "timestamp": "2026-03-23T20:49:34+01:00",
          "tree_id": "ed5dd341e7bdffbd34265b643be75fb7cfaa7105",
          "url": "https://github.com/fallow-rs/fallow/commit/84926039eb3df83afefa1dbc9b66b447e0375445"
        },
        "date": 1774295478261,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.6,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "459128955d419e835fa2ebba606c758f3335b4bd",
          "message": "chore: release v1.5.0",
          "timestamp": "2026-03-24T00:42:37+01:00",
          "tree_id": "9ebc48e41e3ff5edbea0edb6f48ff4a40233c805",
          "url": "https://github.com/fallow-rs/fallow/commit/459128955d419e835fa2ebba606c758f3335b4bd"
        },
        "date": 1774309467247,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.1,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "c15ef2db6b7f25836bc1804b8f0943291d2f801c",
          "message": "feat: add precise line/column locations to all SARIF results\n\nSARIF results for unlisted deps, unused deps, type-only deps, and\ncircular deps previously had no region, causing GitHub Code Scanning\nto default to line 1. Now all issue types emit precise locations:\n\n- Unlisted dependencies: point to the actual import statement in the\n  source file (with line/col from resolved module spans)\n- Unused dependencies: point to the dependency entry line in package.json\n  (via text scan of raw JSON, with JSONC comment skipping)\n- Type-only dependencies: same package.json line scan approach\n- Circular dependencies: point to the import statement that starts the\n  cycle (via graph edge span lookup)\n\nAlso fixes pre-existing output-schema.json bugs (missing\nunused_optional_dependencies, wrong DuplicateExport.locations type)\nand bumps JSON schema_version to 3.",
          "timestamp": "2026-03-24T09:11:17+01:00",
          "tree_id": "7bec724bf30cda7622f46b1179f4d719e5835115",
          "url": "https://github.com/fallow-rs/fallow/commit/c15ef2db6b7f25836bc1804b8f0943291d2f801c"
        },
        "date": 1774339922140,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.1,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "d64a62b31c6d71e32f97372beee47d5cd259bf43",
          "message": "chore: release v1.6.0",
          "timestamp": "2026-03-24T09:33:24+01:00",
          "tree_id": "863dd4d48f4a05aea1dcd165780b1955cbb8d296",
          "url": "https://github.com/fallow-rs/fallow/commit/d64a62b31c6d71e32f97372beee47d5cd259bf43"
        },
        "date": 1774341272689,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.1,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "922cc4a192f6a876a408af70dfadb6afb5b18c4a",
          "message": "fix: make npm publish idempotent with provenance fallback\n\nSkip already-published packages on re-run via `npm view` check.\nFall back to publishing without `--provenance` when Sigstore tlog\nreturns a 409 conflict, which blocks retries after partial failures.",
          "timestamp": "2026-03-24T09:56:59+01:00",
          "tree_id": "5f06e0bbeaed1c297aae7dfab34c99ca7074fbf5",
          "url": "https://github.com/fallow-rs/fallow/commit/922cc4a192f6a876a408af70dfadb6afb5b18c4a"
        },
        "date": 1774342664200,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.1,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "30c09c79bb4ab0a643344f586b72809211df5107",
          "message": "feat: add markdown and SARIF output formats for health command\n\n- Add --format markdown support for fallow health: renders a table\n  with file, function, cyclomatic, cognitive, and line count columns.\n  Functions exceeding thresholds get a **!** marker. Pipes cleanly\n  to gh pr comment.\n\n- Add --format sarif support for fallow health: emits SARIF 2.1.0\n  with three rule IDs (fallow/high-cyclomatic-complexity,\n  fallow/high-cognitive-complexity, fallow/high-complexity) so\n  GitHub Code Scanning shows inline annotations on PR diffs.\n\n- Update action.yml to support command: health with inputs for\n  max-cyclomatic, max-cognitive, top, sort. Includes job summary,\n  PR comment, SARIF upload, and input validation. The --top flag\n  is intentionally omitted from the SARIF fallback run to ensure\n  Code Scanning sees all findings.\n\n- Refresh public ROADMAP.md with user-facing narrative.",
          "timestamp": "2026-03-24T10:00:36+01:00",
          "tree_id": "a17bcec9ad105d466f4a9a512be7f0296469cf51",
          "url": "https://github.com/fallow-rs/fallow/commit/30c09c79bb4ab0a643344f586b72809211df5107"
        },
        "date": 1774342888141,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "8a7e806633aa3f00fcb6b8398d92cb3b13a846cd",
          "message": "feat: wire --workspace and --baseline/--save-baseline for health command\n\nThese flags were accepted by the CLI parser but silently ignored.\nNow they work:\n\n- --workspace scopes findings to a single workspace package\n  (path-based filtering, same as check command)\n- --save-baseline saves findings as path:name:line keys to JSON\n- --baseline filters out known findings from a previous baseline\n  (save happens before --top truncation, filter before --top)\n\nAlso re-exports resolve_workspace_filter from check module for\nreuse by health.",
          "timestamp": "2026-03-24T10:15:16+01:00",
          "tree_id": "164c27ce47d4f9ddf0c9ee6ef973df54bc09ceb7",
          "url": "https://github.com/fallow-rs/fallow/commit/8a7e806633aa3f00fcb6b8398d92cb3b13a846cd"
        },
        "date": 1774343765199,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "f2bc1c02ea1a9dc0468addd7df06da9ab4faa30f",
          "message": "fix: hard error on health baseline I/O failures, capture threshold count before baseline filter\n\n- Baseline read/write/parse failures now return exit code 2 (matching\n  dupes behavior) instead of printing a warning and continuing silently.\n- functions_above_threshold is now captured before baseline filtering,\n  so the summary always reflects the true count of threshold violations\n  regardless of whether --baseline is active.",
          "timestamp": "2026-03-24T10:18:09+01:00",
          "tree_id": "000eb1d146e844b5c6a6fd1764d1f49f683b9277",
          "url": "https://github.com/fallow-rs/fallow/commit/f2bc1c02ea1a9dc0468addd7df06da9ab4faa30f"
        },
        "date": 1774343932494,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "50f3861f70ac5cc170362ea263c6611720c2815a",
          "message": "chore: bump platform package versions to 1.6.0",
          "timestamp": "2026-03-24T10:21:50+01:00",
          "tree_id": "4dfdffdb98980f5696b933ca9c7fd62e0c96ddec",
          "url": "https://github.com/fallow-rs/fallow/commit/50f3861f70ac5cc170362ea263c6611720c2815a"
        },
        "date": 1774344156907,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 83.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "5b1a6b4a03ab7b65097c6769a34b53595cd6ee5c",
          "message": "fix: deterministic re-export chain propagation and bare specifier resolution\n\nTwo correctness fixes in the module graph:\n\n1. Entry point barrels (e.g., src/index.js re-exporting from ./render)\n   now propagate \"used\" status to source exports even with zero in-graph\n   consumers. Named and star re-exports are both handled. Star re-exports\n   correctly exclude default exports per ES spec.\n\n2. Remove the DashMap-backed BareSpecifierCache that caused non-deterministic\n   results in multi-threaded mode. With TsconfigDiscovery::Auto, the same\n   bare specifier can resolve to InternalModule (via tsconfig paths) or\n   NpmPackage (via node_modules) depending on the importing file's context.\n   The cache poisoned cross-context results. Replaced with a deterministic\n   post-resolution pass that upgrades NpmPackage to InternalModule when any\n   file resolved the specifier to a source file. A fast path skips\n   canonicalize() for non-workspace non-pnpm bare specifiers to recover\n   most of the lost cache performance.",
          "timestamp": "2026-03-24T13:17:40+01:00",
          "tree_id": "4c37559e9f28e5bf3709ee4a1c7d191686652682",
          "url": "https://github.com/fallow-rs/fallow/commit/5b1a6b4a03ab7b65097c6769a34b53595cd6ee5c"
        },
        "date": 1774354713456,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 89.8,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "4e0b6b886431611ee78dc0dd8ac1042d8946bab6",
          "message": "chore: release v1.6.1",
          "timestamp": "2026-03-24T13:19:35+01:00",
          "tree_id": "9c3adafd5dab74d3e2262cd25c07a9f45c40d21f",
          "url": "https://github.com/fallow-rs/fallow/commit/4e0b6b886431611ee78dc0dd8ac1042d8946bab6"
        },
        "date": 1774354847562,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 89.8,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "fa929d6334e8c2286869ac75785133bcefc5f500",
          "message": "fix: typo checker false positive and Windows path separator in list tests\n\n- Rename `fo` variable to `fan_out` in markdown health report to avoid\n  typos CI flagging it as a misspelling\n- Use platform-aware path assertions in list_tests.rs: accept both\n  forward and backslash separators, and Windows UNC/drive-letter paths",
          "timestamp": "2026-03-24T13:39:11+01:00",
          "tree_id": "55b16625b9a7a591d2fccabd4a89d5fcdf3f3649",
          "url": "https://github.com/fallow-rs/fallow/commit/fa929d6334e8c2286869ac75785133bcefc5f500"
        },
        "date": 1774356018250,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 89.8,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "eea2ee65096c378e3ce1cede9dbc260c8a9b5f1b",
          "message": "fix: skip /bin/sh-dependent MCP tests on Windows\n\nGate MCP server tests that shell out to `/bin/sh` with `#[cfg(unix)]`\nsince `/bin/sh` does not exist on Windows.",
          "timestamp": "2026-03-24T13:53:58+01:00",
          "tree_id": "1afd5f598e5e0f57af12661989b944ee89f64c42",
          "url": "https://github.com/fallow-rs/fallow/commit/eea2ee65096c378e3ce1cede9dbc260c8a9b5f1b"
        },
        "date": 1774356882414,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 89.8,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "60af661085bbc0e3207180679b882773fa00d197",
          "message": "chore: release v1.7.0",
          "timestamp": "2026-03-24T15:23:48+01:00",
          "tree_id": "086b0c428fa093968631ea159046eae448eb662e",
          "url": "https://github.com/fallow-rs/fallow/commit/60af661085bbc0e3207180679b882773fa00d197"
        },
        "date": 1774362456747,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 89.3,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "df293aa9727a78d323217d03b320932f950828e5",
          "message": "refactor: replace eprintln! with tracing::warn! in config crate\n\nLibrary crates should not print directly to stderr. Replace all\neprintln! calls in fallow-config with tracing::warn! so warnings\nflow through the structured logging pipeline. Removes all\n#[expect(clippy::print_stderr)] annotations from the config crate.",
          "timestamp": "2026-03-24T15:55:09+01:00",
          "tree_id": "f7c77d0e446e76fbdd0faae764b01648a6a3aeb8",
          "url": "https://github.com/fallow-rs/fallow/commit/df293aa9727a78d323217d03b320932f950828e5"
        },
        "date": 1774364278920,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 89.3,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "4b2f9a09e497a7aac6222cd0a25b3c327738ef5f",
          "message": "docs: update descriptions to include complexity hotspots, normalize TypeScript/JavaScript ordering\n\nFallow is no longer just an unused code and duplication tool. Updates\nall public-facing descriptions (README, npm, MCP, AGENTS, positioning)\nto reflect health/complexity/hotspot capabilities. Standardizes on\n\"TypeScript/JavaScript\" ordering everywhere.",
          "timestamp": "2026-03-24T16:28:58+01:00",
          "tree_id": "dc74524d53d8bf8434b7148988c3c9a58a344e19",
          "url": "https://github.com/fallow-rs/fallow/commit/4b2f9a09e497a7aac6222cd0a25b3c327738ef5f"
        },
        "date": 1774366211884,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 89.1,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "0f69c6c15c5de568fccf928d7bfa4b7482b66d0c",
          "message": "feat: add --explain flag for metric explainability in all output formats\n\nAdd centralized metric/rule definitions (explain.rs) and wire them through\nall output formats so both humans and AI agents can understand what CLI\noutput means:\n\n- JSON: --explain adds _meta object with metric definitions, ranges,\n  interpretations, and docs URLs (check/health/dupes)\n- SARIF: fullDescription and helpUri on all 17 rules (always included)\n- Markdown: collapsible metric legend after health tables\n- Human: docs footer link after health findings\n- MCP: always passes --explain so agents always get _meta\n\nAlso completes the DupesOutput stub in docs/output-schema.json and adds\nthe Meta type definition.",
          "timestamp": "2026-03-24T20:02:19+01:00",
          "tree_id": "2e0c6e9b9bbcfebae78f7b82825c60450b6fedd1",
          "url": "https://github.com/fallow-rs/fallow/commit/0f69c6c15c5de568fccf928d7bfa4b7482b66d0c"
        },
        "date": 1774382068944,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "347f885469a2dd45befff4f0dd9ccc2bb0ef52ba",
          "message": "feat: per-section explain footers, mirrored dir detection, consistency polish\n\n- Add per-section footer with description + docs link to all human output\n  sections (check, health, dupes) — always shown, no --explain needed\n- Add mirrored directory detection in dupes output (e.g., src/ ↔ deno/lib/)\n- Group circular deps by hub file with path elision\n- Add elide_common_prefix utility for shorter paths in chains\n- Truncate health file scores (max 10), dupes mirrored/families sections\n- Compact health footer to 1 line, add ✗ prefix to dupes footer\n- Shorten summary footer labels (files, exports, types vs unused files, etc.)\n- Update all docs URLs to /explanations/{dead-code,health,duplication}\n- Add 21 new tests for utilities, truncation, mirroring, footers\n- Remove unused _explain params from dupes/health human functions",
          "timestamp": "2026-03-24T23:21:26+01:00",
          "tree_id": "5716f1bf5ca92a849467b703097cdf82e20ffc15",
          "url": "https://github.com/fallow-rs/fallow/commit/347f885469a2dd45befff4f0dd9ccc2bb0ef52ba"
        },
        "date": 1774390952879,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.9,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "da022b097ad3208e0364e1f3e76a80ad15adb2a0",
          "message": "chore: release v1.8.0",
          "timestamp": "2026-03-24T23:24:41+01:00",
          "tree_id": "f5de5912764469863e2e543043d1d1def12feff6",
          "url": "https://github.com/fallow-rs/fallow/commit/da022b097ad3208e0364e1f3e76a80ad15adb2a0"
        },
        "date": 1774391143129,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.9,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "49cad71ec5ee2978e6a70789649d73098032668e",
          "message": "Merge branch 'worktree-agent-a8f8b47c'",
          "timestamp": "2026-03-24T23:32:43+01:00",
          "tree_id": "21c55b2f8b467219a5008141256af3017db92fb3",
          "url": "https://github.com/fallow-rs/fallow/commit/49cad71ec5ee2978e6a70789649d73098032668e"
        },
        "date": 1774391734860,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 90.3,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "9cfedd96d8195004b83382bd3cf7f33d664c7c8b",
          "message": "refactor: split human.rs into health, dupes, traces, and perf modules",
          "timestamp": "2026-03-24T23:51:15+01:00",
          "tree_id": "dfc69fd5587361bd7d508b7760ea55b26cb2f17c",
          "url": "https://github.com/fallow-rs/fallow/commit/9cfedd96d8195004b83382bd3cf7f33d664c7c8b"
        },
        "date": 1774392843975,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 90.4,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "5d4e06f89e7bb8da14741c36b1f5490ceb5b30ba",
          "message": "chore: apply cargo fmt to test code from refactoring agents",
          "timestamp": "2026-03-24T23:58:24+01:00",
          "tree_id": "22eb7bda7c26dbb7a4490be0e478e6454a3e7ff9",
          "url": "https://github.com/fallow-rs/fallow/commit/5d4e06f89e7bb8da14741c36b1f5490ceb5b30ba"
        },
        "date": 1774393157064,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 90.4,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "851e9a2ec31f34155064750d2be1fd1c94a21fd3",
          "message": "chore: release v1.8.1",
          "timestamp": "2026-03-25T09:48:26+01:00",
          "tree_id": "59f42d726540a477846f896d0b20ea5b29b618b2",
          "url": "https://github.com/fallow-rs/fallow/commit/851e9a2ec31f34155064750d2be1fd1c94a21fd3"
        },
        "date": 1774428591910,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 90.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "c479b3f59c31c6ac203f19e8ac04c956fb27188c",
          "message": "fix: qualify Span in benchmark to resolve ambiguous import",
          "timestamp": "2026-03-25T09:51:51+01:00",
          "tree_id": "2bf39f35c8b234f291084a777104795c11dd36df",
          "url": "https://github.com/fallow-rs/fallow/commit/c479b3f59c31c6ac203f19e8ac04c956fb27188c"
        },
        "date": 1774428793491,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 90.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "5867ace807e61bfa7458df8c94c972a4e9e3225d",
          "message": "chore: suppress dependabot-cooldown zizmor finding",
          "timestamp": "2026-03-25T09:57:38+01:00",
          "tree_id": "fd9a522e6f50cb84953c79b6bee0fe3d027438de",
          "url": "https://github.com/fallow-rs/fallow/commit/5867ace807e61bfa7458df8c94c972a4e9e3225d"
        },
        "date": 1774429098616,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 90.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "ecd5de1c2245b2d5bb187159a8ae81ae940bfc8b",
          "message": "chore: add dependabot cooldown to fix zizmor security audit",
          "timestamp": "2026-03-25T09:59:16+01:00",
          "tree_id": "1c09236c1f4735c7aae68aba811d84f1be61daec",
          "url": "https://github.com/fallow-rs/fallow/commit/ecd5de1c2245b2d5bb187159a8ae81ae940bfc8b"
        },
        "date": 1774429198437,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 90.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "e7b1a130d03aa4624ef82fd16de8387171d542f6",
          "message": "chore: suppress dependabot-cooldown and use-trusted-publishing in zizmor",
          "timestamp": "2026-03-25T10:04:39+01:00",
          "tree_id": "47374bcb9622901f3862209ab590aa08227d278b",
          "url": "https://github.com/fallow-rs/fallow/commit/e7b1a130d03aa4624ef82fd16de8387171d542f6"
        },
        "date": 1774429531277,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 90.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "b37b3bf35624f5ca2a13d62eafdff381752a858f",
          "message": "chore: fix zizmor dependabot-cooldown with recommended 7-day cooldown\n\nUse default-days: 7 (zizmor recommended value) instead of 3.\nRemove broken glob ignore pattern — zizmor expects filenames not globs.",
          "timestamp": "2026-03-25T10:07:07+01:00",
          "tree_id": "38d203c8482e1af63485a5f961156380ec34666c",
          "url": "https://github.com/fallow-rs/fallow/commit/b37b3bf35624f5ca2a13d62eafdff381752a858f"
        },
        "date": 1774429672735,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 90.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "65f8edaf07e434ac76a23ad85515e2f2db901dea",
          "message": "test: add ~100 unit tests across 6 crates to close coverage gaps\n\nAdd test coverage for previously untested functions and edge cases:\n\n- types/extract: compute_line_offsets, byte_offset_to_line_col, ExportName\n- types/serde_path: backslash→forward slash normalization\n- types/suppress: code-duplication parse token, near-misses, discriminant edges\n- types/results: fix total_issues test to cover all 13 issue categories\n- config/duplicates_config: ResolvedNormalization 4 modes + overrides, DetectionMode\n- config/health: HealthConfig defaults and JSON deserialization\n- config/rules: Severity Display, apply_partial completeness\n- graph/resolve/fallbacks: make_glob_from_pattern\n- extract/complexity: do-while, for-of, for-in, optional chaining cyclomatic\n- extract/tests/js_ts: JSX retry fallback for .js files with JSX\n- core/churn: parse_git_log, compute_trend boundary cases\n- cli/watch: is_relevant_source, is_relevant_config, collect_changed_paths\n- cli/explain: rule_by_id, rule_docs_url, meta builders, rule completeness\n- cli/check/rules: override-aware apply_rules and has_error_severity_issues\n- cli/baseline: circular_dep_key sort-stability",
          "timestamp": "2026-03-25T11:04:08+01:00",
          "tree_id": "6557d7ca20a396ea9ee3315cdd4602c9cf925b2a",
          "url": "https://github.com/fallow-rs/fallow/commit/65f8edaf07e434ac76a23ad85515e2f2db901dea"
        },
        "date": 1774433166625,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 91,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "46742ccb2fcbbf94530539da650b33b871a43acb",
          "message": "chore: release v1.9.0",
          "timestamp": "2026-03-25T16:07:04+01:00",
          "tree_id": "717075cb447508d1ebda424b1968a2f935e97cbb",
          "url": "https://github.com/fallow-rs/fallow/commit/46742ccb2fcbbf94530539da650b33b871a43acb"
        },
        "date": 1774451288096,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 90,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "e2532d0dcdf3aee2e9a0db9763978cc4a3fd4e3a",
          "message": "chore: run conformance suite daily instead of weekly",
          "timestamp": "2026-03-25T17:17:11+01:00",
          "tree_id": "b1e0d46c40d9e2e8d46de4c9ac8d4d87187be688",
          "url": "https://github.com/fallow-rs/fallow/commit/e2532d0dcdf3aee2e9a0db9763978cc4a3fd4e3a"
        },
        "date": 1774455609130,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 90,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "2436f291ef4d8bf690d0aec9c4d4d517533888a8",
          "message": "fix: update action.yml name and description for Marketplace compliance\n\nName must match existing Marketplace listing and description must be\nunder 125 characters.",
          "timestamp": "2026-03-25T17:35:31+01:00",
          "tree_id": "7e01123220b5a6875599af866abfdaf18154b297",
          "url": "https://github.com/fallow-rs/fallow/commit/2436f291ef4d8bf690d0aec9c4d4d517533888a8"
        },
        "date": 1774456603488,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 90,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "f14114192cd0bdc7872792759291bd2138d70339",
          "message": "fix: rename GitHub Action to reflect broader codebase health positioning\n\n\"Dead Code Analysis\" cements commodity positioning that the roadmap\nexplicitly aims to move beyond. \"Codebase Health\" aligns with the\nhealth intelligence thesis (hotspots, trends, dependency risk).",
          "timestamp": "2026-03-25T17:44:35+01:00",
          "tree_id": "de96067531530c62524bc687788289a8123c962d",
          "url": "https://github.com/fallow-rs/fallow/commit/f14114192cd0bdc7872792759291bd2138d70339"
        },
        "date": 1774457123774,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 90,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "36b6d5197bc085ff4c055b75bd15a21f5e4e7620",
          "message": "refactor: rename check to dead-code across MCP, benchmarks, and docs\n\nUpdate all remaining references to `fallow check` to use `fallow dead-code`\n(or bare `fallow`) after the CLI restructure. Covers MCP server tools and\ntests, benchmark scripts, screenshot generator, CLAUDE.md build command,\nand explain.rs docs URL.",
          "timestamp": "2026-03-25T18:50:14+01:00",
          "tree_id": "718e12e0ad76bd980055c744dda6cf6d74b1d859",
          "url": "https://github.com/fallow-rs/fallow/commit/36b6d5197bc085ff4c055b75bd15a21f5e4e7620"
        },
        "date": 1774461100869,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "9c2259e68bc01cf07b9aed940e42fd601afebc3d",
          "message": "docs: add dead-code section to README with flag examples",
          "timestamp": "2026-03-25T18:59:58+01:00",
          "tree_id": "65ad7fcfea5e5cba24ae099e4bfb972d27f07f46",
          "url": "https://github.com/fallow-rs/fallow/commit/9c2259e68bc01cf07b9aed940e42fd601afebc3d"
        },
        "date": 1774461650465,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "3e8568d2d21782f0a8f25619495d5dc34f669d6d",
          "message": "docs: add clear benchmark section headers for dead code vs duplication",
          "timestamp": "2026-03-25T19:05:22+01:00",
          "tree_id": "cabab3ea0edfe7c268467fcb49535205517bcaad",
          "url": "https://github.com/fallow-rs/fallow/commit/3e8568d2d21782f0a8f25619495d5dc34f669d6d"
        },
        "date": 1774461973546,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "81d804e3bd71041e0036a3eb5d982238ccd0ca44",
          "message": "docs: polish README — remove implementation details, add incremental adoption example",
          "timestamp": "2026-03-25T19:25:34+01:00",
          "tree_id": "cb7c168c7beada26842100ca0848c53712c57d71",
          "url": "https://github.com/fallow-rs/fallow/commit/81d804e3bd71041e0036a3eb5d982238ccd0ca44"
        },
        "date": 1774463177073,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "b5355ff0299f405dfc0b12da19fc641528562b95",
          "message": "docs: escape angle brackets in doc comments for rustdoc compatibility",
          "timestamp": "2026-03-25T19:50:41+01:00",
          "tree_id": "8c4b2d0f8360f5d56f7a7408058337c3ac75b855",
          "url": "https://github.com/fallow-rs/fallow/commit/b5355ff0299f405dfc0b12da19fc641528562b95"
        },
        "date": 1774464689877,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "48650d8d08b8185490c17e10497098a52cc3e971",
          "message": "chore: release v2.0.1",
          "timestamp": "2026-03-25T22:23:18+01:00",
          "tree_id": "c12c630748e302aa7e030961d4b7643729bb538e",
          "url": "https://github.com/fallow-rs/fallow/commit/48650d8d08b8185490c17e10497098a52cc3e971"
        },
        "date": 1774473886516,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "06ae1122bd093070ac5b2a4264b7d96a3fa99937",
          "message": "fix: SARIF version redaction, action bare invocation, Miri resolve tests\n\n- SARIF snapshot tests: redact_sarif_version now targets the tool driver\n  version contextually (matching \"name\": \"fallow\" prefix) instead of\n  blanket-replacing \"2.1.0\" which collided with the SARIF spec version.\n- GitHub Action: bare invocation (no command) no longer passes empty\n  string as first arg, which clap rejected as unrecognized subcommand.\n- Miri: gate with_empty_ctx with #[cfg(not(miri))] since oxc_resolver\n  uses statx syscall unsupported by Miri. Pure logic resolve tests in\n  fallbacks, react_native, path_info still run under Miri.",
          "timestamp": "2026-03-26T09:53:33+01:00",
          "tree_id": "35f0262fa8f192c171a31358be896668a38efd52",
          "url": "https://github.com/fallow-rs/fallow/commit/06ae1122bd093070ac5b2a4264b7d96a3fa99937"
        },
        "date": 1774515272808,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "45aeb4f8b473f4343e6d875587c4ae7fce760832",
          "message": "fix: correct alert-threshold for customBiggerIsBetter benchmarks\n\nFor customBiggerIsBetter, benchmark-action calculates ratio as\nprevious/current and alerts when ratio >= threshold. With 95%, identical\nvalues (ratio 1.0 = 100%) always triggered false alerts. Changed to 105%\nso it only alerts on actual regressions (>5% drop).",
          "timestamp": "2026-03-26T09:58:01+01:00",
          "tree_id": "08ef6ce718b82f1dbf561c9a5405d41d7a29457d",
          "url": "https://github.com/fallow-rs/fallow/commit/45aeb4f8b473f4343e6d875587c4ae7fce760832"
        },
        "date": 1774515543169,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "62ce7db6b7e9444e772342cae52c3a31aab56a64",
          "message": "fix: correct dead-code command description and MCP analyze tool description\n\ndead-code subcommand help text incorrectly claimed it finds \"code\nduplication\" — it only finds unused code and circular dependencies.\nMCP analyze tool description similarly claimed it finds \"code duplication,\ncomplexity hotspots\" — those are find_dupes and check_health respectively.",
          "timestamp": "2026-03-26T11:33:36+01:00",
          "tree_id": "74076a47437f367d1b4cb19201550285befb9e2b",
          "url": "https://github.com/fallow-rs/fallow/commit/62ce7db6b7e9444e772342cae52c3a31aab56a64"
        },
        "date": 1774521324113,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "49699333+dependabot[bot]@users.noreply.github.com",
            "name": "dependabot[bot]",
            "username": "dependabot[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b04395c60fd4c8b2ae37a2b8ab492b4d0170ea35",
          "message": "chore(deps): bump actions/upload-artifact from 4.6.2 to 7.0.0 (#29)\n\nBumps [actions/upload-artifact](https://github.com/actions/upload-artifact) from 4.6.2 to 7.0.0.\n- [Release notes](https://github.com/actions/upload-artifact/releases)\n- [Commits](https://github.com/actions/upload-artifact/compare/v4.6.2...bbbca2ddaa5d8feaa63e36b76fdaad77386f024f)\n\n---\nupdated-dependencies:\n- dependency-name: actions/upload-artifact\n  dependency-version: 7.0.0\n  dependency-type: direct:production\n  update-type: version-update:semver-major\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-26T12:01:09+01:00",
          "tree_id": "d83dec68b29fdc2b89f16895f5b36ffe43a360f0",
          "url": "https://github.com/fallow-rs/fallow/commit/b04395c60fd4c8b2ae37a2b8ab492b4d0170ea35"
        },
        "date": 1774522926199,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "649bbd4c342f634da0996af76c6a7ddbc3cec864",
          "message": "fix: test-json job uses dead-code command to match verify assertions\n\nThe test-json job ran a bare invocation (no command) which produces\ncombined JSON output, but the verify step expected dead-code output\nstructure (.total_issues at root level). A separate test-bare-combined\njob already covers the bare invocation case.",
          "timestamp": "2026-03-26T12:57:29+01:00",
          "tree_id": "af175352950d824e897a72c12c00cfd1bd0f5e64",
          "url": "https://github.com/fallow-rs/fallow/commit/649bbd4c342f634da0996af76c6a7ddbc3cec864"
        },
        "date": 1774526373436,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "084f6c5a0a00a639a35c16c1960543d365ab9046",
          "message": "docs: restore coverage badge to README",
          "timestamp": "2026-03-26T13:01:12+01:00",
          "tree_id": "3a21e4b1e1365287eed9e9bdb661fa50e9a5de55",
          "url": "https://github.com/fallow-rs/fallow/commit/084f6c5a0a00a639a35c16c1960543d365ab9046"
        },
        "date": 1774526526300,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "900a7d0f3b1f0280ff04a05ce4efe8d69e595566",
          "message": "fix: use Alpine image and detect package manager in GitLab CI template\n\n- Change default image from node:20-slim to node:22-alpine for smaller\n  image size and to avoid glibc version mismatch (binary requires 2.39,\n  Bookworm has 2.36). Alpine uses the musl binary instead.\n- Add package manager detection so the template works with both Alpine\n  (apk) and Debian/Ubuntu (apt-get) images when users override the image.",
          "timestamp": "2026-03-26T13:31:52+01:00",
          "tree_id": "35145be12bf968703dd7e24d778fea21c974af19",
          "url": "https://github.com/fallow-rs/fallow/commit/900a7d0f3b1f0280ff04a05ce4efe8d69e595566"
        },
        "date": 1774528371721,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.5,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "00e1a5d688340bf3f5af293cc3af354fab03482e",
          "message": "feat: add efficiency score, confidence levels, and adaptive thresholds to refactoring targets\n\nTargets are now sorted by efficiency (priority/effort) to surface quick wins\nfirst. Each target includes a confidence level (high/medium/low) based on data\nsource reliability, and fan-in/fan-out thresholds adapt to the project's\ndistribution using percentiles instead of hardcoded constants.\n\nOutput improvements from expert panel review:\n- Human: efficiency as hero number, labeled effort:/confidence:, effort summary\n- Compact: underscore category labels (circular_dep, dead_code) for parsing\n- Markdown: reduced from 7 to 5 columns\n- SARIF: priority, efficiency, confidence in message\n- JSON: target_thresholds object with adaptive percentile values\n- Cycle path evidence deduplicated",
          "timestamp": "2026-03-26T16:07:01+01:00",
          "tree_id": "bff7f2618da14866803f188a8941bbc937ab9a9b",
          "url": "https://github.com/fallow-rs/fallow/commit/00e1a5d688340bf3f5af293cc3af354fab03482e"
        },
        "date": 1774537742911,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "8428221faca4cbd5f6447b6c5deb296f1b96a47d",
          "message": "feat: add efficiency score, confidence levels, and adaptive thresholds to refactoring targets\n\nTargets are now sorted by efficiency (priority/effort) to surface quick wins\nfirst. Each target includes a confidence level (high/medium/low) based on data\nsource reliability, and fan-in/fan-out thresholds adapt to the project's\ndistribution using percentiles instead of hardcoded constants.\n\nOutput improvements from expert panel review:\n- Human: efficiency as hero number, labeled effort:/confidence:, effort summary\n- Compact: underscore category labels (circular_dep, dead_code) for parsing\n- Markdown: reduced from 7 to 5 columns\n- SARIF: priority, efficiency, confidence in message\n- JSON: target_thresholds object with adaptive percentile values\n- Cycle path evidence deduplicated",
          "timestamp": "2026-03-26T16:07:01+01:00",
          "tree_id": "01dbc5c80d8c4b577077b25a579f0a3ed3446f96",
          "url": "https://github.com/fallow-rs/fallow/commit/8428221faca4cbd5f6447b6c5deb296f1b96a47d"
        },
        "date": 1774538546320,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "65a26631b2fa3979f1f05857fa3044d361b1afa2",
          "message": "chore: release v2.2.0",
          "timestamp": "2026-03-26T16:26:30+01:00",
          "tree_id": "4075bac2045b7933724cb44671815061d88947ee",
          "url": "https://github.com/fallow-rs/fallow/commit/65a26631b2fa3979f1f05857fa3044d361b1afa2"
        },
        "date": 1774538857968,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "6bd74234fd32f865b4b772c584c3699590ceac21",
          "message": "perf: lazy canonicalize with OnceLock fallback + code quality fixes\n\nSkip bulk canonicalize() of all source files when project root is\nalready canonical (common case). A lazy OnceLock-based fallback\nhandles the rare intra-project symlink edge case on first miss.\n\nAlso:\n- Eliminate raw_file_paths Vec<PathBuf> allocation (21k heap allocs)\n- O(n²) active_plugins dedup → FxHashSet O(n)\n- Upgrade CanonicalFallback log to warn for observability\n- Guard bench.mjs against null status (timeout/signal kills)\n- Add CanonicalFallback unit tests (miri-gated)\n- Fix stale comment referencing old field name\n- Fix missing canonical_fallback in test ctx constructor\n\nBenchmarks (cold, no-cache, median of 3):\n- next.js: 1532ms → 1207ms (-21%)\n- vite:    507ms → 408ms  (-20%)\n- query:   191ms → 160ms  (-16%)\n\nAll 8 benchmark projects produce identical issue counts.",
          "timestamp": "2026-03-26T20:50:22+01:00",
          "tree_id": "1ac04c5104d269d27fa1928eb4b05c6232079b41",
          "url": "https://github.com/fallow-rs/fallow/commit/6bd74234fd32f865b4b772c584c3699590ceac21"
        },
        "date": 1774554680043,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "200961e2a6a661502bcf00a52c7e508051ed707c",
          "message": "docs: update benchmark numbers to v2.2.0 measurements\n\nRebenchmarked on Apple M5 with 5 runs. Key changes:\n- Dead code vs knip v5: 5-41x (was 3-36x)\n- Dead code vs knip v6: 2-18x (was 2-14x)\n- Duplication vs jscpd: 8-26x (was 20-33x)\n- Added preact to dead code table\n- knip now correctly shown as erroring on next.js (was '--')",
          "timestamp": "2026-03-26T21:02:53+01:00",
          "tree_id": "e0263b5b95f1d1d423f308a606a52c2eb70796c9",
          "url": "https://github.com/fallow-rs/fallow/commit/200961e2a6a661502bcf00a52c7e508051ed707c"
        },
        "date": 1774555432940,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "246502e30e80a5606cfbd6341bf925ff78136a29",
          "message": "chore: release v2.2.1",
          "timestamp": "2026-03-26T21:04:40+01:00",
          "tree_id": "51da0e7c03aa11dc67c945ddf21f625867943025",
          "url": "https://github.com/fallow-rs/fallow/commit/246502e30e80a5606cfbd6341bf925ff78136a29"
        },
        "date": 1774555537596,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "ae5fa1ea8e0970762d49a18b669e8dea80a4fd92",
          "message": "feat: run conformance and benchmarks against 8 real-world projects\n\nConformance and performance benchmarks previously ran against tiny\nsynthetic fixtures, making them ineffective at catching real issues.\nBoth now clone and test 8 open-source projects (preact, fastify, zod,\nvue-core, svelte, query, vite, next.js) — the same set used by the\nmanual benchmark suite.\n\nChanges:\n- Add run-all.sh + aggregate.py for multi-project conformance\n- Add bench-ci.sh + bench-real-world.yml for daily perf tracking\n- Update conformance.yml to use run-all.sh with per-project reporting\n- Add real-world benchmarks card to metrics dashboard\n- Add favicon from fallow-docs to GitHub Pages\n- Update conformance README for multi-project setup",
          "timestamp": "2026-03-26T22:12:55+01:00",
          "tree_id": "7e8cd477c5bc5506d967dbe7acc4a8369b52afbb",
          "url": "https://github.com/fallow-rs/fallow/commit/ae5fa1ea8e0970762d49a18b669e8dea80a4fd92"
        },
        "date": 1774559636247,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "fb850bf9d3c0eae4a46e40fc0e8fad9f1b05d40c",
          "message": "fix: use correct pnpm/action-setup commit hash\n\nThe previous hash fe02b34 does not exist in the pnpm/action-setup\nrepo. Use fc06bc1 which matches the ecosystem-full workflow.",
          "timestamp": "2026-03-26T22:24:29+01:00",
          "tree_id": "426e268379444f7a6d82f330474aae37899f97d8",
          "url": "https://github.com/fallow-rs/fallow/commit/fb850bf9d3c0eae4a46e40fc0e8fad9f1b05d40c"
        },
        "date": 1774560325974,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "389cdb5c09408b30b937cf31eebb2a20fde79922",
          "message": "fix: handle dict locations in duplicate_exports comparison\n\nThe duplicate_exports JSON format uses {path, line, col} objects in\nthe locations array, not plain strings. compare.py was passing the\ndict directly to normalize_path, causing a TypeError on real projects.",
          "timestamp": "2026-03-26T22:32:25+01:00",
          "tree_id": "153f7f7db2a486d8816085cf9075145f5b65dfba",
          "url": "https://github.com/fallow-rs/fallow/commit/389cdb5c09408b30b937cf31eebb2a20fde79922"
        },
        "date": 1774560793822,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "6466c29f10f928356aaadbd9c519cdf9565e5716",
          "message": "fix: suppress npm/pnpm install stdout leaking into JSON reports\n\nnpm install output was written to stdout, polluting the JSON report\ncaptured by the workflow. Redirect both stdout and stderr to /dev/null.",
          "timestamp": "2026-03-26T22:38:39+01:00",
          "tree_id": "e361253c67449f5ee448b2b7d9374b0d107c1cf4",
          "url": "https://github.com/fallow-rs/fallow/commit/6466c29f10f928356aaadbd9c519cdf9565e5716"
        },
        "date": 1774561176159,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "e7d831cfb1c958dde87f70d656058b50257431d7",
          "message": "refactor: remove RSS tracking from real-world benchmarks\n\nRSS values are machine-specific and can't be backfilled, causing\ninconsistent data point counts between cold/warm and RSS metrics.\nTrack only cold and warm analysis times for consistent charts.",
          "timestamp": "2026-03-27T00:10:27+01:00",
          "tree_id": "17d04c42c1c0c4e37fcebce9cd610201d1a82837",
          "url": "https://github.com/fallow-rs/fallow/commit/e7d831cfb1c958dde87f70d656058b50257431d7"
        },
        "date": 1774566678598,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "82208086b343566f4c91241229be5030b0757f31",
          "message": "test: add ~283 tests across extract, graph, core, and config crates\n\nCover complexity metrics, JSDoc @public tags, config extends/merge,\nre-export chain propagation, dynamic import patterns, declaration\nextraction, visitor helpers, analysis predicates, cycle detection,\nand file discovery with infrastructure entry points.\n\nAlso fix test_dir to use tempfile::tempdir() for panic-safe cleanup\nand replace tautological production pattern checks with GlobSet\nmatching assertions.",
          "timestamp": "2026-03-27T08:08:06+01:00",
          "tree_id": "d1ea455c08a614f5cf1ade6cc5632fde3346b832",
          "url": "https://github.com/fallow-rs/fallow/commit/82208086b343566f4c91241229be5030b0757f31"
        },
        "date": 1774595426261,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.1,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "280fa8f27836a15549c515ddfb44285475067879",
          "message": "chore: release v2.2.2",
          "timestamp": "2026-03-27T08:14:09+01:00",
          "tree_id": "561149cf0c664947ec592587ac7b5fbcb35c34a4",
          "url": "https://github.com/fallow-rs/fallow/commit/280fa8f27836a15549c515ddfb44285475067879"
        },
        "date": 1774595737001,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.1,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "87a04f23f8d13d1209efe56f45af20c96296e1c2",
          "message": "fix: normalize path separators in discovery tests for Windows",
          "timestamp": "2026-03-27T08:22:42+01:00",
          "tree_id": "e8bc879c46922544d685e5103f01832cdd30f8e2",
          "url": "https://github.com/fallow-rs/fallow/commit/87a04f23f8d13d1209efe56f45af20c96296e1c2"
        },
        "date": 1774596229725,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.1,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "46f631633f91e246dabdb1956c009221c39bae94",
          "message": "fix: run cargo fmt on entry_points test path normalization",
          "timestamp": "2026-03-27T08:41:36+01:00",
          "tree_id": "5a0ba06b24e125b28524417591bc2b6648f9e93c",
          "url": "https://github.com/fallow-rs/fallow/commit/46f631633f91e246dabdb1956c009221c39bae94"
        },
        "date": 1774597367376,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.1,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "56b4a06bd9fc6f2bed96134bdea34f968795a24c",
          "message": "fix: add bench-real-world.yml to zizmor superfluous-actions ignore list",
          "timestamp": "2026-03-27T08:46:07+01:00",
          "tree_id": "5ae10669e4ecc4f57c0039106c86b6146b7479b5",
          "url": "https://github.com/fallow-rs/fallow/commit/56b4a06bd9fc6f2bed96134bdea34f968795a24c"
        },
        "date": 1774597620237,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.1,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "22ee80595999ee8784e93520d18f1c761accf564",
          "message": "Improve code quality and harden watch reloads",
          "timestamp": "2026-03-27T09:33:16+01:00",
          "tree_id": "488046ee4e9dfd8066d522d088c12d74f7ceb002",
          "url": "https://github.com/fallow-rs/fallow/commit/22ee80595999ee8784e93520d18f1c761accf564"
        },
        "date": 1774600464209,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "cd53d47846621068903e58382b74cbdd06fdc34c",
          "message": "Improve VS Code extension quality",
          "timestamp": "2026-03-27T10:21:41+01:00",
          "tree_id": "0f985a9a613a4aad3e00bcfb4a4c93fe7cbc9d41",
          "url": "https://github.com/fallow-rs/fallow/commit/cd53d47846621068903e58382b74cbdd06fdc34c"
        },
        "date": 1774603373563,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "3cd88222fa193fd203a41196a527734f0ef5773e",
          "message": "feat: expand VS Code extension UX and pnpm tooling",
          "timestamp": "2026-03-27T10:29:12+01:00",
          "tree_id": "55d28599bacf2e47b511563a66d807564e0e9802",
          "url": "https://github.com/fallow-rs/fallow/commit/3cd88222fa193fd203a41196a527734f0ef5773e"
        },
        "date": 1774603862626,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "439a334f81e342617841d912bb15ffb58e7846cf",
          "message": "feat: add auto-changed-since for automatic PR scoping\n\nAutomatically detects pull_request context and scopes analysis to files\nchanged since the PR base SHA. Enabled by default (auto-changed-since: true).\nIgnored when changed-since is explicitly set.\n\nThis eliminates the need for separate PR-scoped jobs in workflows.",
          "timestamp": "2026-03-27T12:03:36+01:00",
          "tree_id": "7a487f18cf3d6aed21406a1002a283433aa205b7",
          "url": "https://github.com/fallow-rs/fallow/commit/439a334f81e342617841d912bb15ffb58e7846cf"
        },
        "date": 1774609472686,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "a76fa95e1a62d26b2e6ad18089fcefdde83c8a07",
          "message": "fix: handle pull_request_target event in auto-changed-since\n\nAlso supports pull_request_target for workflows that need default branch\npermissions (common with external fork PRs).",
          "timestamp": "2026-03-27T12:09:12+01:00",
          "tree_id": "f4ae1627955a9d1cbb0400440ee658452da8496c",
          "url": "https://github.com/fallow-rs/fallow/commit/a76fa95e1a62d26b2e6ad18089fcefdde83c8a07"
        },
        "date": 1774609811060,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "fcaf88a17200e3e90706983f6301cf1bc8e3f2da",
          "message": "feat: enrich inline PR annotations with actionable context\n\n- Unused exports: distinguish type vs value, suggest fix action\n- Unused deps: include uninstall command\n- Duplication: multi-line ranges (endLine), show other locations, token count, code fragment\n- Complexity: show all three metrics (cyclomatic, cognitive, lines), explain what each means\n- Circular deps: show full chain, explain why it matters\n- Refactoring targets: surface top 5 as ::notice annotations with priority/effort/confidence\n- All annotations: add newlines (%0A) for structured multi-line messages",
          "timestamp": "2026-03-27T12:22:59+01:00",
          "tree_id": "3d337cd458db1b39932c7465d84e364c4ca0fb84",
          "url": "https://github.com/fallow-rs/fallow/commit/fcaf88a17200e3e90706983f6301cf1bc8e3f2da"
        },
        "date": 1774610643064,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 88.2,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "d99ca6bef225c0aebdfbbba24fffc4c92476b0fd",
          "message": "refactor: comprehensive code quality improvements across all crates\n\nPerformance:\n- Eliminate health double pipeline by reusing parsed modules via\n  new analyze_with_parse_result() API in fallow-core\n- Convert unused_import_bindings from Vec to FxHashSet for O(1) lookups\n- Optimize mark_member_exports_referenced to avoid per-export\n  to_string() allocation and linear scan\n- Convert GENERAL_TOOLING_EXACT to OnceLock<FxHashSet> for O(1) lookups\n- Cache line offsets in collect_export_usages (was recomputed per ref)\n\nCode deduplication:\n- Add emit_json(), plural(), build_json_envelope() report helpers\n- Consolidate 6 copies of sample_results into shared test helper\n- Extract read_source/write_fixed_content in fix module\n- Deduplicate TSTypeAlias/TSInterface export push via push_type_export()\n- Consolidate parse test helper into shared parse_ts()\n- Deduplicate config parser traversal with shared closure helper\n- Extend define_plugin! macro with imports_only resolve_config variant,\n  migrate Cypress/Commitlint/Remark plugins\n\nRefactoring:\n- Add ReportContext struct for unified report dispatcher signatures\n- Replace score 6-tuple with Option<FileScoreOutput> + TargetAuxData::From\n- Extract build_ignore_set, collect_findings, save_health_baseline\n  from execute_health god-function\n- Add Default derive for FallowConfig and ModuleInfoExtractor\n- Remove dead _config parameters from public analysis functions\n- Remove orphaned graph/build/ directory (1,986 lines never compiled)\n\nCleanup:\n- Fix redundant closures with method references (String::as_str, etc.)\n- Add const fn to match-based lookup functions\n- Fix single_match_else patterns in LSP and MCP\n- Update CLAUDE.md build commands to use --all-targets",
          "timestamp": "2026-03-27T12:27:00+01:00",
          "tree_id": "a9a74c5879883d62974546a9f082101459a095e8",
          "url": "https://github.com/fallow-rs/fallow/commit/d99ca6bef225c0aebdfbbba24fffc4c92476b0fd"
        },
        "date": 1774610872944,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "de836c95c858d3877c254d261307f948c515a53e",
          "message": "feat: add review-comments for rich inline PR feedback with markdown\n\nNew input `review-comments: true` posts findings as GitHub PR review\ncomments with full markdown support: code blocks, tables, links,\nand actionable suggestions.\n\n- Creates a single review per run (one notification)\n- Dismisses previous fallow review on re-runs (no stacking)\n- Uses COMMENT event (doesn't block merging)\n- Prepends root path for monorepo support\n- Falls back to individual comments if batch fails (diff line mismatch)\n- Automatically disables plain-text annotations when enabled\n- Covers all issue types: dead code, duplication, complexity, refactoring targets",
          "timestamp": "2026-03-27T12:29:45+01:00",
          "tree_id": "1e06398da4386d6face56e03059051568bc17b1d",
          "url": "https://github.com/fallow-rs/fallow/commit/de836c95c858d3877c254d261307f948c515a53e"
        },
        "date": 1774611048607,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "91e8882dcb496cd04a2e21fe48e4aba59161be5c",
          "message": "fix: add missing review-comments input definition",
          "timestamp": "2026-03-27T12:31:28+01:00",
          "tree_id": "3f938607817918b7029115be72a27c53a593c5a9",
          "url": "https://github.com/fallow-rs/fallow/commit/91e8882dcb496cd04a2e21fe48e4aba59161be5c"
        },
        "date": 1774611142540,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "committer": {
            "email": "bart@waardenburg.dev",
            "name": "Bart Waardenburg",
            "username": "BartWaardenburg"
          },
          "distinct": true,
          "id": "d3bab5f365cd95979ca14894dd88958929dec064",
          "message": "fix: export PREFIX/MAX env vars and use temp files for combined jq pipeline",
          "timestamp": "2026-03-27T12:33:15+01:00",
          "tree_id": "b19f600d5d57e0ef1dcf40f22227afd62893a8ec",
          "url": "https://github.com/fallow-rs/fallow/commit/d3bab5f365cd95979ca14894dd88958929dec064"
        },
        "date": 1774611247316,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Code Coverage",
            "value": 87.7,
            "unit": "%"
          }
        ]
      }
    ]
  }
}