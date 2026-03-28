window.BENCHMARK_DATA = {
  "lastUpdate": 1774735076563,
  "repoUrl": "https://github.com/fallow-rs/fallow",
  "entries": {
    "Fallow Binary Size": [
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
        "date": 1774362594426,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 114448816,
            "unit": "bytes"
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
        "date": 1774364284284,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 114486888,
            "unit": "bytes"
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
        "date": 1774366302641,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 115474896,
            "unit": "bytes"
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
        "date": 1774382183315,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 116270472,
            "unit": "bytes"
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
        "date": 1774391067151,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 116325312,
            "unit": "bytes"
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
        "date": 1774391245728,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 116350648,
            "unit": "bytes"
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
        "date": 1774391850278,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 116308536,
            "unit": "bytes"
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
        "date": 1774392934538,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 116272400,
            "unit": "bytes"
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
        "date": 1774393265131,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 116272400,
            "unit": "bytes"
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
        "date": 1774428707177,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 116273800,
            "unit": "bytes"
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
        "date": 1774428943563,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 116273800,
            "unit": "bytes"
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
        "date": 1774433275713,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 116273856,
            "unit": "bytes"
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
        "date": 1774451404615,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 118105296,
            "unit": "bytes"
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
        "date": 1774461207548,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 118931680,
            "unit": "bytes"
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
          "id": "1dc7818b00ce8eae81af8649f11f1e48628dc079",
          "message": "chore: release v2.0.0",
          "timestamp": "2026-03-25T18:54:56+01:00",
          "tree_id": "0a211b776e610b05f9cd42299921440fc685bfe2",
          "url": "https://github.com/fallow-rs/fallow/commit/1dc7818b00ce8eae81af8649f11f1e48628dc079"
        },
        "date": 1774461470449,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 118931600,
            "unit": "bytes"
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
        "date": 1774464805609,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 118931600,
            "unit": "bytes"
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
        "date": 1774473984844,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 118955304,
            "unit": "bytes"
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
          "id": "09d744cc09e7a81f292deba39d0cecbc67f5d2d2",
          "message": "chore: release v2.1.0",
          "timestamp": "2026-03-26T09:28:15+01:00",
          "tree_id": "74a9125eaf2ff8ce8354fe35b60fd4a4b6540c4d",
          "url": "https://github.com/fallow-rs/fallow/commit/09d744cc09e7a81f292deba39d0cecbc67f5d2d2"
        },
        "date": 1774514024435,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 118998816,
            "unit": "bytes"
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
        "date": 1774515376435,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 118998816,
            "unit": "bytes"
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
        "date": 1774521434871,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 118998800,
            "unit": "bytes"
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
          "id": "fe288bfa67f5cbb67871928f3b21610aa7fc5462",
          "message": "chore(deps): bump proptest from 1.10.0 to 1.11.0 (#26)\n\nBumps [proptest](https://github.com/proptest-rs/proptest) from 1.10.0 to 1.11.0.\n- [Release notes](https://github.com/proptest-rs/proptest/releases)\n- [Changelog](https://github.com/proptest-rs/proptest/blob/main/CHANGELOG.md)\n- [Commits](https://github.com/proptest-rs/proptest/compare/v1.10.0...v1.11.0)\n\n---\nupdated-dependencies:\n- dependency-name: proptest\n  dependency-version: 1.11.0\n  dependency-type: direct:production\n  update-type: version-update:semver-minor\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-26T12:00:23+01:00",
          "tree_id": "85b110308c2d91e803f2eb1e1a242fec3a4755d2",
          "url": "https://github.com/fallow-rs/fallow/commit/fe288bfa67f5cbb67871928f3b21610aa7fc5462"
        },
        "date": 1774522981154,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 118998800,
            "unit": "bytes"
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
          "id": "f9669178a3b48fad5d8829f063139c67194137ef",
          "message": "chore(deps): bump actions/checkout from 4.3.1 to 6.0.2 (#31)\n\nBumps [actions/checkout](https://github.com/actions/checkout) from 4.3.1 to 6.0.2.\n- [Release notes](https://github.com/actions/checkout/releases)\n- [Changelog](https://github.com/actions/checkout/blob/main/CHANGELOG.md)\n- [Commits](https://github.com/actions/checkout/compare/v4.3.1...de0fac2e4500dabe0009e67214ff5f5447ce83dd)\n\n---\nupdated-dependencies:\n- dependency-name: actions/checkout\n  dependency-version: 6.0.2\n  dependency-type: direct:production\n  update-type: version-update:semver-major\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-26T12:55:13+01:00",
          "tree_id": "dadd32cc5f457ed74353d945333b8cfa3f97fcdd",
          "url": "https://github.com/fallow-rs/fallow/commit/f9669178a3b48fad5d8829f063139c67194137ef"
        },
        "date": 1774526283732,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 119007392,
            "unit": "bytes"
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
        "date": 1774537867025,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 119269024,
            "unit": "bytes"
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
        "date": 1774538659898,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 119269024,
            "unit": "bytes"
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
        "date": 1774538986410,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 119290192,
            "unit": "bytes"
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
        "date": 1774554797529,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120381800,
            "unit": "bytes"
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
        "date": 1774555653004,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120361448,
            "unit": "bytes"
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
        "date": 1774595537920,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120887320,
            "unit": "bytes"
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
        "date": 1774595845855,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120888272,
            "unit": "bytes"
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
        "date": 1774596359861,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120888272,
            "unit": "bytes"
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
        "date": 1774597470228,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120888272,
            "unit": "bytes"
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
        "date": 1774600574185,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120887776,
            "unit": "bytes"
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
        "date": 1774603976449,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120887776,
            "unit": "bytes"
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
        "date": 1774610988227,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120926328,
            "unit": "bytes"
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
        "date": 1774611151853,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120895944,
            "unit": "bytes"
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
          "id": "a2f72ea6a3b4476f50e8120339d38c7d2c817759",
          "message": "refactor: migrate 5 more plugins to define_plugin! imports_only macro",
          "timestamp": "2026-03-27T12:40:16+01:00",
          "tree_id": "1406214f0a632d83d59aa87ef497b9fd7ed8553e",
          "url": "https://github.com/fallow-rs/fallow/commit/a2f72ea6a3b4476f50e8120339d38c7d2c817759"
        },
        "date": 1774611811973,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120895224,
            "unit": "bytes"
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
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "93867ff0f4e0aceb5df35285f14a9bfb2dce38cc",
          "message": "refactor: extract CI setup into composite actions\n\n- Create .github/actions/setup-rust and .github/actions/store-benchmark composite actions\n- Migrate 11 workflows to use composite actions (-162 lines)\n- Add --all-targets to cargo test/clippy in ci.yml\n- Fix MSRV job: add explicit toolchain: '1.92.0'\n- Simplify release.yml: 7 artifact downloads → 1 pattern download\n- Fix pre-existing issues exposed by --all-targets: Windows clippy warnings, env var test race, needless_collect",
          "timestamp": "2026-03-27T14:30:50+01:00",
          "tree_id": "1881e34156acd840a46b32565e3c07bfa3f8089e",
          "url": "https://github.com/fallow-rs/fallow/commit/93867ff0f4e0aceb5df35285f14a9bfb2dce38cc"
        },
        "date": 1774618513268,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120895224,
            "unit": "bytes"
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
          "id": "664b4fa8418057e9870c9080a2877287016ce211",
          "message": "chore: release v2.3.0",
          "timestamp": "2026-03-27T17:44:30+01:00",
          "tree_id": "9a236179ddd12c9258009a737eec5f806097f307",
          "url": "https://github.com/fallow-rs/fallow/commit/664b4fa8418057e9870c9080a2877287016ce211"
        },
        "date": 1774630038154,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120920600,
            "unit": "bytes"
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
          "id": "71607c663bf77959a6df31a588ca3340f24bb29c",
          "message": "fix: remove unnecessary raw string hashes in plugin test",
          "timestamp": "2026-03-27T19:39:40+01:00",
          "tree_id": "7e1b4ee0a1b35446c0d8872e798cb9ac8297b792",
          "url": "https://github.com/fallow-rs/fallow/commit/71607c663bf77959a6df31a588ca3340f24bb29c"
        },
        "date": 1774637183442,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120921152,
            "unit": "bytes"
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
          "id": "db9bff2d56b45a518445f0103d88fc930832ec2b",
          "message": "test: add integration tests and additional unit test coverage\n\nIntegration test for unreachable mixed exports, additional visitor tests,\nplugin config resolution tests, and detect algorithm edge cases.",
          "timestamp": "2026-03-27T21:08:55+01:00",
          "tree_id": "cc72c280ae6ce0d84ef5a2353a18d0c73e9e1c87",
          "url": "https://github.com/fallow-rs/fallow/commit/db9bff2d56b45a518445f0103d88fc930832ec2b"
        },
        "date": 1774642320540,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120969016,
            "unit": "bytes"
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
          "id": "6ca1111298dee70f8c2ac40085800dc4efc39f18",
          "message": "Merge branch 'worktree-agent-aee04371'",
          "timestamp": "2026-03-27T21:20:23+01:00",
          "tree_id": "0441f5a5c28ab8036efebdfe8f8666161febbd0d",
          "url": "https://github.com/fallow-rs/fallow/commit/6ca1111298dee70f8c2ac40085800dc4efc39f18"
        },
        "date": 1774643114439,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120969552,
            "unit": "bytes"
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
          "id": "3d6ad8957f919db4e8db202668dde479ad29a698",
          "message": "chore: release v2.3.1",
          "timestamp": "2026-03-27T21:48:28+01:00",
          "tree_id": "ddade8129abe358f6baafdf9b360423259969b52",
          "url": "https://github.com/fallow-rs/fallow/commit/3d6ad8957f919db4e8db202668dde479ad29a698"
        },
        "date": 1774644770237,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120995032,
            "unit": "bytes"
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
          "id": "483bb84e822fb970851c062bfdcd68964209a906",
          "message": "fix: Windows CI test failure and typos check\n\n- Use platform-appropriate absolute path in extends_absolute_path_rejected\n  test (Unix: /path, Windows: C:\\path)\n- Add German unicode test words (Konfiguration, Paket) to typos allowlist",
          "timestamp": "2026-03-27T22:03:32+01:00",
          "tree_id": "095f6e0a0ae1b9adc2855b655a4f79d49bd9326c",
          "url": "https://github.com/fallow-rs/fallow/commit/483bb84e822fb970851c062bfdcd68964209a906"
        },
        "date": 1774645569388,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 120995032,
            "unit": "bytes"
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
          "id": "1df6ff665a644d3ada4d68ea44ec4bdfdf27ba21",
          "message": "fix: remove tooling prefix over-exemptions and add config-based detection\n\nRemove 8 over-broad prefixes from GENERAL_TOOLING_PREFIXES (eslint,\n@typescript-eslint, postcss, autoprefixer, tailwindcss, @tailwindcss,\n@storybook/, storybook) that blanket-exempted devDeps without checking\nconfig files. Plugins with config parsing now handle these accurately.\n\nTrim tooling_dependencies() in 6 plugins (ESLint, Storybook, Tailwind,\nJest, Vitest, Prettier) to only packages genuinely not discoverable by\nconfig parsing.\n\nAdd ESLint shared config following: reads imported config packages'\nentry points one level deep to discover peer deps (e.g., @sveltejs/\neslint-config → typescript-eslint, eslint-plugin-svelte, @eslint/js).\n\nAdd PrettierPlugin config parsing: extracts plugins array from\n.prettierrc and prettier.config.* files, with package_json_config_key\nfor inline config.\n\nAdd EslintPlugin package_json_config_key(\"eslintConfig\") for inline\nconfig in package.json.\n\nAlso includes test_only_dependencies issue type (parallel agent work)\nwith full pipeline wiring and snapshot updates.",
          "timestamp": "2026-03-27T22:29:11+01:00",
          "tree_id": "8550629d55f2da8b971f1c8c0c2eaac3c30e06a1",
          "url": "https://github.com/fallow-rs/fallow/commit/1df6ff665a644d3ada4d68ea44ec4bdfdf27ba21"
        },
        "date": 1774647128644,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 121341456,
            "unit": "bytes"
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
          "id": "6c07b7a474dbc42decd683345774b85dcb05aff5",
          "message": "docs: add test-only-dependencies to README and output schema\n\nUpdate \"What it finds\" list to include the new test-only production\ndependency diagnostic (14 issue types total). Add TestOnlyDependency\ndefinition and field to docs/output-schema.json. Regenerate schema.json.\nIncludes residual formatting from rustfmt.",
          "timestamp": "2026-03-27T22:36:33+01:00",
          "tree_id": "7b92b205753a426cc46ee4a133cf128c10fda990",
          "url": "https://github.com/fallow-rs/fallow/commit/6c07b7a474dbc42decd683345774b85dcb05aff5"
        },
        "date": 1774647562397,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 121341456,
            "unit": "bytes"
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
          "id": "4eee0a6dceff37b708cc2109b2c29e5c301c5bd3",
          "message": "fix: handle exports field in shared config resolution, storybook addon object form\n\n- read_package_entry now resolves \"exports\".\".\" (string, import, default)\n  before falling back to \"module\" → \"main\" → \"index.js\"\n- Storybook addon parsing now handles object form { name: \"...\", options: {} }\n  via extract_config_property_strings supplementary pass\n- Add unit tests: shared config following with tempdir fixtures, exports\n  field resolution, storybook addon string/object forms, graceful fallback\n  when node_modules is missing",
          "timestamp": "2026-03-27T22:39:30+01:00",
          "tree_id": "7e7c0f1c550a0144fe800c7747f17e6bcf5a9915",
          "url": "https://github.com/fallow-rs/fallow/commit/4eee0a6dceff37b708cc2109b2c29e5c301c5bd3"
        },
        "date": 1774647735529,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 121390752,
            "unit": "bytes"
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
          "id": "7dff1ea823869e9c9400fb8bb9c07d1865721ab8",
          "message": "fix: add missing type-only-dependency suppression and update issue counts to 14\n\nAdd TypeOnlyDependency variant to IssueKind (discriminant 13, bumps\ntest-only to 14). This was a pre-existing gap where type-only deps\ncould not be suppressed with fallow-ignore-next-line comments.\n\nUpdate stale \"13 issue types\" references to 14 in ROADMAP.md and\ntest comments in compact.rs and sarif.rs.",
          "timestamp": "2026-03-27T22:45:17+01:00",
          "tree_id": "5f8f60bf18d09a4fecfbcb5dee6461b193736a50",
          "url": "https://github.com/fallow-rs/fallow/commit/7dff1ea823869e9c9400fb8bb9c07d1865721ab8"
        },
        "date": 1774648081880,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 121390880,
            "unit": "bytes"
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
          "id": "3776223d02022ab8988308dbb4ce7222b2f210d1",
          "message": "chore: release v2.4.0",
          "timestamp": "2026-03-27T23:01:46+01:00",
          "tree_id": "338d929dcec5d0055534bfc06bbb6d41fd501e81",
          "url": "https://github.com/fallow-rs/fallow/commit/3776223d02022ab8988308dbb4ce7222b2f210d1"
        },
        "date": 1774649131970,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 121397352,
            "unit": "bytes"
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
          "id": "9e6d4afaf61e53928ed2c5db945f849730b5304b",
          "message": "feat: add regression detection with config-embedded baselines\n\nAdd `--fail-on-regression` and `--tolerance` flags for CI regression gates.\nBaselines are stored in the config file by default (no separate file to manage),\nwith `--regression-baseline <PATH>` as an explicit override.\n\nNew global CLI flags:\n- `--fail-on-regression` — exit 1 if issue count increased beyond tolerance\n- `--tolerance <N>` — \"2%\" (percentage) or \"5\" (absolute), default \"0\"\n- `--regression-baseline <PATH>` — explicit baseline file (overrides config)\n- `--save-regression-baseline [PATH]` — no path writes to config, with path writes file\n\nConfig embedding:\n- `regression.baseline` section in .fallowrc.json / fallow.toml\n- Text-level patching preserves JSONC comments\n- Creates config file if none exists\n\nIntegration:\n- JSON output includes optional `regression` object when flag is active\n- MCP server: 4 new params on analyze + check_changed tools\n- GitHub Action: 4 new inputs with env var wiring\n- GitLab CI: 4 new variables with arg building\n- Skips with warning when --changed-since or --workspace active\n- Warns when saving baseline from scoped results\n- Warns when saved path is gitignored\n\nAlso fixes pre-existing test gap: adds unused_optional_dependencies to shared\nsample_results (fixes compact + SARIF issue-count assertions).",
          "timestamp": "2026-03-28T00:39:04+01:00",
          "tree_id": "eea40667279c1665795b6b32627148ea2f27ee9f",
          "url": "https://github.com/fallow-rs/fallow/commit/9e6d4afaf61e53928ed2c5db945f849730b5304b"
        },
        "date": 1774654927260,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 122643720,
            "unit": "bytes"
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
          "id": "e894387049ccf059c0dab63718a19b2a132a3ff3",
          "message": "feat: add project health score with letter grade (`--score`, `--min-score`)\n\nAggregates vital signs into a single 0-100 score with A/B/C/D/F grade.\nPenalties for dead code, complexity, maintainability, hotspots, unused\nand circular dependencies. --min-score enables CI quality gates.\n\nIncludes documentation updates across all companion repos, snapshot\ntests, and backwards-compatibility policy additions.\n\n# Conflicts:\n#\tAGENTS.md\n#\tcrates/cli/src/report/sarif.rs\n#\tdocs/output-schema.json",
          "timestamp": "2026-03-28T00:44:26+01:00",
          "tree_id": "fea7e88659795b9f937be13e403443755df26787",
          "url": "https://github.com/fallow-rs/fallow/commit/e894387049ccf059c0dab63718a19b2a132a3ff3"
        },
        "date": 1774655610367,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 122862264,
            "unit": "bytes"
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
          "id": "944bae67330ef74a86de8b28f9316cf758d2608a",
          "message": "fix: show health score by default when no section flags are set\n\nThe score is a project-level summary — it should appear alongside\ncomplexity, file-scores, hotspots, and targets when running bare\n`fallow health`. Using --score as a section flag still works to\nshow only the score.",
          "timestamp": "2026-03-28T00:55:02+01:00",
          "tree_id": "c25472cb40ba3931c9caf5606d509a8018a8afa7",
          "url": "https://github.com/fallow-rs/fallow/commit/944bae67330ef74a86de8b28f9316cf758d2608a"
        },
        "date": 1774655939850,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 122860544,
            "unit": "bytes"
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
          "id": "14945524c744dd0cc4defe847daef5213b7a5437",
          "message": "docs: update --score docs to reflect default-on behavior\n\nThe score is shown by default when no section flags are set.\n--save-snapshot now automatically includes the score.\nUpdate CLI help, AGENTS.md, and companion repos to match.",
          "timestamp": "2026-03-28T01:06:27+01:00",
          "tree_id": "97cd8d662e4fb9e9d09d539bf0d96acbabec0f94",
          "url": "https://github.com/fallow-rs/fallow/commit/14945524c744dd0cc4defe847daef5213b7a5437"
        },
        "date": 1774656562427,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 122860752,
            "unit": "bytes"
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
          "id": "9e65ad1327a908010b3815c3411e8e7e2348d680",
          "message": "chore: release v2.5.0",
          "timestamp": "2026-03-28T01:11:56+01:00",
          "tree_id": "a5a2c9a1d34ec52fb3a2eae028837e5ab807c06f",
          "url": "https://github.com/fallow-rs/fallow/commit/9e65ad1327a908010b3815c3411e8e7e2348d680"
        },
        "date": 1774656891692,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 122830728,
            "unit": "bytes"
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
          "id": "004661077c460895cb9502437c6f172668bb8f14",
          "message": "fix: update discriminant_roundtrip boundary check for new IssueKind variants\n\nThe out-of-range assertion used discriminant 14, which is now occupied by\nTestOnlyDependency. Bump to 15 to match the current variant count.",
          "timestamp": "2026-03-28T13:25:24+01:00",
          "tree_id": "46bda71149042d8e493d49508581a16c3f158773",
          "url": "https://github.com/fallow-rs/fallow/commit/004661077c460895cb9502437c6f172668bb8f14"
        },
        "date": 1774700906431,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 122830728,
            "unit": "bytes"
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
          "id": "dac5541e0c759703fa8ffd858cc957e1ebfd870f",
          "message": "fix: use relative paths in duplication refactoring suggestions\n\nThe location hint in clone family suggestions (e.g. \"Extract ... into\nsrc/hooks\") was printing absolute paths from the filesystem. Thread\nthe project root through group_into_families and strip_prefix it from\ndirectory paths so suggestions always show project-relative paths.",
          "timestamp": "2026-03-28T19:33:17+01:00",
          "tree_id": "31e81a09eca4ac9c6cec8023403aafd1433205c6",
          "url": "https://github.com/fallow-rs/fallow/commit/dac5541e0c759703fa8ffd858cc957e1ebfd870f"
        },
        "date": 1774722970757,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 122852496,
            "unit": "bytes"
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
          "id": "14bdc956c891961215879504eca8a29cb727ab45",
          "message": "fix: use relative paths in duplication refactoring suggestions\n\nThe location hint in clone family suggestions (e.g. \"Extract ... into\nsrc/hooks\") was printing absolute paths from the filesystem. Thread\nthe project root through group_into_families and strip_prefix it from\ndirectory paths so suggestions always show project-relative paths.",
          "timestamp": "2026-03-28T19:36:46+01:00",
          "tree_id": "2c1c21ae1393cd380ec889c230494c41cd47b186",
          "url": "https://github.com/fallow-rs/fallow/commit/14bdc956c891961215879504eca8a29cb727ab45"
        },
        "date": 1774723191698,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 122855952,
            "unit": "bytes"
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
          "id": "8e04e73aa6ff54683886ba6af4a3f39e018ff6e6",
          "message": "chore: release v2.5.1",
          "timestamp": "2026-03-28T19:39:48+01:00",
          "tree_id": "3d34306926606d8655034e6331c93397f478e04b",
          "url": "https://github.com/fallow-rs/fallow/commit/8e04e73aa6ff54683886ba6af4a3f39e018ff6e6"
        },
        "date": 1774723365907,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 122834968,
            "unit": "bytes"
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
          "id": "5b17d92e1b8378597aba50c279dca682a12d170e",
          "message": "docs: document safety invariants for u32 bounds and interval merging",
          "timestamp": "2026-03-28T19:43:20+01:00",
          "tree_id": "dfebe92b9327f4e4a19dfaaa67ed89fb6b2f7272",
          "url": "https://github.com/fallow-rs/fallow/commit/5b17d92e1b8378597aba50c279dca682a12d170e"
        },
        "date": 1774723586341,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 122834976,
            "unit": "bytes"
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
          "id": "32080e97352e96a61a13e5c85c0e1dc364021237",
          "message": "chore: release v2.5.2",
          "timestamp": "2026-03-28T21:15:18+01:00",
          "tree_id": "65d6f23bcf57f754cf1a7f8892bec2f955276c24",
          "url": "https://github.com/fallow-rs/fallow/commit/32080e97352e96a61a13e5c85c0e1dc364021237"
        },
        "date": 1774729154799,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 123025264,
            "unit": "bytes"
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
          "id": "d24146869615e35f37e05b669afae94ba60b483c",
          "message": "fix: reduce false positives for unused enum members in type-level usage\n\nTrack enum member access in TypeScript type positions:\n- TSQualifiedName (e.g., `type X = Status.Active`) as member access\n- TSMappedType constraint (e.g., `{ [K in Enum]: ... }`) as whole-object use",
          "timestamp": "2026-03-28T22:02:33+01:00",
          "tree_id": "7316b9824b9473338f4fb369923874af38bc55e9",
          "url": "https://github.com/fallow-rs/fallow/commit/d24146869615e35f37e05b669afae94ba60b483c"
        },
        "date": 1774731932235,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 123060352,
            "unit": "bytes"
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
          "id": "22fa64be94ed9ab619c325e79737a4b11b97396a",
          "message": "chore: release v2.5.3",
          "timestamp": "2026-03-28T22:06:05+01:00",
          "tree_id": "f9c9ae2311ce3dc1b23ef372ea0fa9aa5aef0b0a",
          "url": "https://github.com/fallow-rs/fallow/commit/22fa64be94ed9ab619c325e79737a4b11b97396a"
        },
        "date": 1774732215305,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 123077168,
            "unit": "bytes"
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
          "id": "d2f217aa5752319c356b964a1bb56acaebb6fe05",
          "message": "chore: release v2.5.4",
          "timestamp": "2026-03-28T22:41:27+01:00",
          "tree_id": "46ae10555f88fa4a595b285fd9ec273727bcc985",
          "url": "https://github.com/fallow-rs/fallow/commit/d2f217aa5752319c356b964a1bb56acaebb6fe05"
        },
        "date": 1774734290067,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 123041320,
            "unit": "bytes"
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
          "id": "e566c19e440413d2eedd19e6fb254c9294996b33",
          "message": "chore: release v2.5.5",
          "timestamp": "2026-03-28T22:54:59+01:00",
          "tree_id": "fb09e9b3d825dfd48705d90c478f22cf06f97a1a",
          "url": "https://github.com/fallow-rs/fallow/commit/e566c19e440413d2eedd19e6fb254c9294996b33"
        },
        "date": 1774735075343,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Binary Size (fallow)",
            "value": 123084664,
            "unit": "bytes"
          }
        ]
      }
    ]
  }
}