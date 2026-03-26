window.BENCHMARK_DATA = {
  "lastUpdate": 1774521435841,
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
      }
    ]
  }
}