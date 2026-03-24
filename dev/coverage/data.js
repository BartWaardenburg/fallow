window.BENCHMARK_DATA = {
  "lastUpdate": 1774364279290,
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
      }
    ]
  }
}