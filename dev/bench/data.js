window.BENCHMARK_DATA = {
  "lastUpdate": 1774735140333,
  "repoUrl": "https://github.com/fallow-rs/fallow",
  "entries": {
    "Fallow Benchmarks": [
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
          "id": "d004ea84285796d55c417b4de1c6c8accf38b283",
          "message": "fix: add Windows symlink support for workspace integration test\n\nThe workspace_project_discovers_workspace_packages test only created\nsymlinks on Unix, causing the cross-workspace resolution test to fail\non Windows CI. Add #[cfg(windows)] block using symlink_dir.",
          "timestamp": "2026-03-21T07:51:35+01:00",
          "tree_id": "ba8312d7a89e20c8fd3e62678774b8037c121544",
          "url": "https://github.com/fallow-rs/fallow/commit/d004ea84285796d55c417b4de1c6c8accf38b283"
        },
        "date": 1774076170050,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28905,
            "range": "± 296",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1094729,
            "range": "± 65986",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2631742,
            "range": "± 28315",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16732955,
            "range": "± 286442",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 283071,
            "range": "± 1756",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1980,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 215433,
            "range": "± 12290",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 919636,
            "range": "± 7154",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1724717,
            "range": "± 16493",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 515696,
            "range": "± 27479",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4237058,
            "range": "± 34977",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2529488,
            "range": "± 12907",
            "unit": "ns/iter"
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
          "id": "1eca935f0c37bd0f61588d8b81994f7200ff1f92",
          "message": "fix: resolve CI failures across typos, cargo-deny, docs, MSRV, and clippy\n\n- Add _typos.toml to exclude vscode dist, fixtures, and allow legitimate\n  identifiers (ue, ba, caf, fpr) used in variable names and test data\n- Update deny.toml for cargo-deny v2: remove deprecated unmaintained/yanked\n  fields, add MPL-2.0 and CC0-1.0 licenses, ignore bincode advisory\n- Fix rustdoc errors: backtick-wrap Vec<String> in doc comment, remove\n  intra-doc link to private FallowErrorKind\n- Bump MSRV from 1.85 to 1.92 (required by oxc 0.120.0 dependencies)\n- Fix clippy collapsible_if in mcp server binary resolution",
          "timestamp": "2026-03-21T08:06:01+01:00",
          "tree_id": "c3b261347252a51baa81fd47314eb891fa0b905e",
          "url": "https://github.com/fallow-rs/fallow/commit/1eca935f0c37bd0f61588d8b81994f7200ff1f92"
        },
        "date": 1774076995745,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28224,
            "range": "± 179",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1093349,
            "range": "± 24320",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2631597,
            "range": "± 30143",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16925857,
            "range": "± 116467",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 281440,
            "range": "± 1740",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2022,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 206180,
            "range": "± 28084",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 918450,
            "range": "± 15578",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1738861,
            "range": "± 21186",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 515251,
            "range": "± 9792",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4264046,
            "range": "± 43812",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2531112,
            "range": "± 117221",
            "unit": "ns/iter"
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
          "id": "ab1cb8f4b357569ef53b221b8277eb3ab92a4b3f",
          "message": "chore: release v1.0.4",
          "timestamp": "2026-03-21T08:12:55+01:00",
          "tree_id": "480c76589a8dfcdba3b7ba43894b590b9b203f26",
          "url": "https://github.com/fallow-rs/fallow/commit/ab1cb8f4b357569ef53b221b8277eb3ab92a4b3f"
        },
        "date": 1774077424039,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 22953,
            "range": "± 139",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 887820,
            "range": "± 16555",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 1960347,
            "range": "± 27516",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 12501452,
            "range": "± 461281",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 249964,
            "range": "± 2408",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2756,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 161948,
            "range": "± 11680",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 745188,
            "range": "± 13662",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1528366,
            "range": "± 29594",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 471679,
            "range": "± 27134",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3547683,
            "range": "± 26830",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2039585,
            "range": "± 8631",
            "unit": "ns/iter"
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
          "id": "a0ceeec375bb55b10fa3d7acbda1ab21b1b4c76b",
          "message": "chore(deps): bump toml from 1.0.6+spec-1.1.0 to 1.0.7+spec-1.1.0 (#13)\n\nBumps [toml](https://github.com/toml-rs/toml) from 1.0.6+spec-1.1.0 to 1.0.7+spec-1.1.0.\n- [Commits](https://github.com/toml-rs/toml/compare/toml-v1.0.6...toml-v1.0.7)\n\n---\nupdated-dependencies:\n- dependency-name: toml\n  dependency-version: 1.0.7+spec-1.1.0\n  dependency-type: direct:production\n  update-type: version-update:semver-patch\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-21T08:42:30+01:00",
          "tree_id": "5995b5a095906106874f938357a552f7241a2aa8",
          "url": "https://github.com/fallow-rs/fallow/commit/a0ceeec375bb55b10fa3d7acbda1ab21b1b4c76b"
        },
        "date": 1774079167477,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28013,
            "range": "± 1542",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1109778,
            "range": "± 28362",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2618050,
            "range": "± 24478",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16834878,
            "range": "± 137456",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 281766,
            "range": "± 1241",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1990,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 203374,
            "range": "± 3745",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 903442,
            "range": "± 12344",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1694903,
            "range": "± 33220",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 511894,
            "range": "± 25202",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4199247,
            "range": "± 46821",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2530292,
            "range": "± 9772",
            "unit": "ns/iter"
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
          "id": "f23f31f0657152fdcc2adf227d4e664512f7cd66",
          "message": "chore: release v1.1.0",
          "timestamp": "2026-03-21T10:16:50+01:00",
          "tree_id": "607474b552ef8442f312393c8f431e191ef771f7",
          "url": "https://github.com/fallow-rs/fallow/commit/f23f31f0657152fdcc2adf227d4e664512f7cd66"
        },
        "date": 1774084837859,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 29018,
            "range": "± 2073",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1082908,
            "range": "± 142744",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2596059,
            "range": "± 37435",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16760486,
            "range": "± 168180",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 279909,
            "range": "± 1926",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1961,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 201052,
            "range": "± 723",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 897912,
            "range": "± 10468",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1694621,
            "range": "± 20397",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 514790,
            "range": "± 9439",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4179917,
            "range": "± 109824",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2544909,
            "range": "± 23585",
            "unit": "ns/iter"
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
          "id": "2a4d74cfd00d5ec79733ff08e31dc311961a9ffc",
          "message": "fix: restrict ModuleNode size assertion to Unix (PathBuf differs on Windows)",
          "timestamp": "2026-03-21T10:26:39+01:00",
          "tree_id": "f83a7d02feecd2549717e3d56cfcae87576739a0",
          "url": "https://github.com/fallow-rs/fallow/commit/2a4d74cfd00d5ec79733ff08e31dc311961a9ffc"
        },
        "date": 1774085422873,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28324,
            "range": "± 1425",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1093204,
            "range": "± 62863",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2611032,
            "range": "± 32646",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16721651,
            "range": "± 269414",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 276516,
            "range": "± 1569",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1966,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 201196,
            "range": "± 3506",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 897980,
            "range": "± 9814",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1690988,
            "range": "± 16746",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 515760,
            "range": "± 28358",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4363817,
            "range": "± 94257",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2497857,
            "range": "± 60394",
            "unit": "ns/iter"
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
          "id": "ad3c6ec83c75aaa3303cb67814d3e569518a66dc",
          "message": "fix: changelog, homepage URL, npm binary fallback, docs accuracy\n\n- Add missing CHANGELOG.md entry for v1.1.0\n- Change homepage from fallow.tools to docs.fallow.tools\n- Add try/catch around detect-libc in npm bin wrapper (fallback to musl)\n- Add workflow_dispatch trigger to Miri CI\n- Fix schema.rs: mark unused-enum-member as fixable, add markdown format\n- Fix AGENTS.md: accurate compact format description, schema_version field\n- Fix BENCHMARKS.md: correct bench target names\n- Fix docs/positioning.md: remove complexity hotspot claims\n- Fix docs/plugin-authoring.md: external plugins support detection logic",
          "timestamp": "2026-03-21T18:45:13+01:00",
          "tree_id": "a9912c446b8b8d77e52c032969f80141e0308f5e",
          "url": "https://github.com/fallow-rs/fallow/commit/ad3c6ec83c75aaa3303cb67814d3e569518a66dc"
        },
        "date": 1774115339798,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28809,
            "range": "± 542",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1086420,
            "range": "± 9061",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2621487,
            "range": "± 30688",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16671957,
            "range": "± 219688",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 277337,
            "range": "± 815",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1996,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 200943,
            "range": "± 1073",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 895631,
            "range": "± 9819",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1684273,
            "range": "± 17178",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 510033,
            "range": "± 27569",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4165592,
            "range": "± 46352",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2464431,
            "range": "± 6629",
            "unit": "ns/iter"
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
          "id": "51db3d56712e7df6443a60ed680fc8a112b6137f",
          "message": "fix: escape [default] in rustdoc comment to fix doc build\n\nRustdoc interprets `[default]` as an intra-doc link. Escape the\nbrackets to prevent the broken-intra-doc-links warning.",
          "timestamp": "2026-03-21T18:49:15+01:00",
          "tree_id": "0f223d64ca2f5644db05b4232d775f4f336e6a35",
          "url": "https://github.com/fallow-rs/fallow/commit/51db3d56712e7df6443a60ed680fc8a112b6137f"
        },
        "date": 1774115579195,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28527,
            "range": "± 1066",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1088595,
            "range": "± 12908",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2622348,
            "range": "± 43970",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16740702,
            "range": "± 286464",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 280592,
            "range": "± 2695",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1968,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 201917,
            "range": "± 19095",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 895986,
            "range": "± 12195",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1689452,
            "range": "± 31547",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 515649,
            "range": "± 10111",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4165745,
            "range": "± 35280",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2473687,
            "range": "± 3215",
            "unit": "ns/iter"
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
          "id": "ed499c258e70639bd923c65028dbc18026af6aad",
          "message": "refactor: adopt Oxc-style Miri CI pattern\n\n- Pin nightly toolchain (nightly-2026-03-20) to avoid Miri regressions\n- Use #[cfg(not(miri))] source guards instead of --skip CLI flags\n  for parser-heavy tests (visitor.rs, tests/ integration module)\n- Add --lib --tests to skip doctests (unsupported under Miri)\n- Add permissions: {} deny-all baseline\n- Keep suppress, sfc, css unit tests running (no Oxc parser calls)",
          "timestamp": "2026-03-21T21:02:46+01:00",
          "tree_id": "31dc901904139bccec698ff5be745118fc199e6c",
          "url": "https://github.com/fallow-rs/fallow/commit/ed499c258e70639bd923c65028dbc18026af6aad"
        },
        "date": 1774123591530,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 28634,
            "range": "± 346",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1113739,
            "range": "± 30215",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2646637,
            "range": "± 28864",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17006620,
            "range": "± 631111",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 279535,
            "range": "± 3409",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1957,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 201454,
            "range": "± 5988",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 898000,
            "range": "± 6391",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1685676,
            "range": "± 25387",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 519964,
            "range": "± 30096",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4200984,
            "range": "± 77015",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2496070,
            "range": "± 17527",
            "unit": "ns/iter"
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
          "id": "b475cfc20fe03c88353288ae3ba1d22152de4836",
          "message": "chore: release v1.2.0",
          "timestamp": "2026-03-22T12:57:27+01:00",
          "tree_id": "089417efd2637f2506a27a3c55aba1b955015d20",
          "url": "https://github.com/fallow-rs/fallow/commit/b475cfc20fe03c88353288ae3ba1d22152de4836"
        },
        "date": 1774181577468,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41033,
            "range": "± 358",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1105231,
            "range": "± 17303",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2668846,
            "range": "± 34236",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17859815,
            "range": "± 206342",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 296438,
            "range": "± 2060",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1987,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 200811,
            "range": "± 3676",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 902521,
            "range": "± 66830",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1705331,
            "range": "± 85196",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 512037,
            "range": "± 35313",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4308773,
            "range": "± 54348",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2517511,
            "range": "± 11238",
            "unit": "ns/iter"
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
          "id": "fb09d1a8465e2cc8791203b9b5bcb9a6e687a8ab",
          "message": "feat: workspace perf fix, optionalDependencies, overload dedup, re-export optimization\n\nPerformance:\n- Workspace discovery: check package.json before canonicalize(), reducing\n  syscalls from 759 to ~20 on vite (245ms → ~30ms for that stage)\n- Re-export chain: replace ExportName::to_string() allocations with\n  zero-alloc matches_str() comparisons in hot inner loops\n\nAccuracy:\n- Parse optionalDependencies from package.json with full pipeline support:\n  detection, rules (unused-optional-dependencies), all report formats\n  (human, json, sarif, compact, markdown), auto-fix, LSP diagnostics\n- Deduplicate TypeScript function overload signatures during extraction:\n  `export function foo(): void; export function foo(x: string): string;\n  export function foo(x?: string) {}` produces 1 export, not 3",
          "timestamp": "2026-03-22T16:22:45+01:00",
          "tree_id": "1432c9685bf8d4d8be2ecc6e249ebbae2e5523e7",
          "url": "https://github.com/fallow-rs/fallow/commit/fb09d1a8465e2cc8791203b9b5bcb9a6e687a8ab"
        },
        "date": 1774193202166,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 39714,
            "range": "± 1368",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1105867,
            "range": "± 27145",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2650603,
            "range": "± 40578",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16958678,
            "range": "± 257266",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 106036,
            "range": "± 1287",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1968,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 201353,
            "range": "± 873",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 898148,
            "range": "± 10129",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1687186,
            "range": "± 19317",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 543181,
            "range": "± 28003",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4173677,
            "range": "± 40550",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2480920,
            "range": "± 5001",
            "unit": "ns/iter"
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
          "id": "523e701ba71f8eaf95a961258aaa0a30de2c4a6a",
          "message": "chore: release v1.3.0",
          "timestamp": "2026-03-22T17:52:34+01:00",
          "tree_id": "5401d0e941ba62b3a5353dc552c34d21b67c9b3a",
          "url": "https://github.com/fallow-rs/fallow/commit/523e701ba71f8eaf95a961258aaa0a30de2c4a6a"
        },
        "date": 1774198593588,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 40870,
            "range": "± 486",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1110631,
            "range": "± 55920",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2669147,
            "range": "± 27573",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17348720,
            "range": "± 177463",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 106177,
            "range": "± 368",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2006,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 206701,
            "range": "± 6931",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 929181,
            "range": "± 19641",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1732569,
            "range": "± 23589",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 521770,
            "range": "± 13248",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4352772,
            "range": "± 64480",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2583007,
            "range": "± 31021",
            "unit": "ns/iter"
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
          "id": "e77bf1449e6dd9d4b1ae81f1f1da8d461f506c77",
          "message": "test: add optionalDependencies integration test and snapshot coverage",
          "timestamp": "2026-03-22T19:33:53+01:00",
          "tree_id": "988af83b41a1b6ff0e13d65d67f1a9210910cbba",
          "url": "https://github.com/fallow-rs/fallow/commit/e77bf1449e6dd9d4b1ae81f1f1da8d461f506c77"
        },
        "date": 1774204664382,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 33636,
            "range": "± 842",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 895947,
            "range": "± 17580",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 1997106,
            "range": "± 27833",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 12346281,
            "range": "± 340146",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 96610,
            "range": "± 1133",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2853,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 162481,
            "range": "± 829",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 733975,
            "range": "± 3330",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1526969,
            "range": "± 24412",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 468350,
            "range": "± 15047",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3485392,
            "range": "± 31405",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2034593,
            "range": "± 12756",
            "unit": "ns/iter"
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
          "id": "5e83aee8c5a7bf37afb505e7ccd535a144b97211",
          "message": "chore: release v1.3.1",
          "timestamp": "2026-03-23T00:13:45+01:00",
          "tree_id": "1a86251710998d54f51f3434b4850782b304a780",
          "url": "https://github.com/fallow-rs/fallow/commit/5e83aee8c5a7bf37afb505e7ccd535a144b97211"
        },
        "date": 1774221550569,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41363,
            "range": "± 1594",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1185336,
            "range": "± 18709",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2803866,
            "range": "± 25173",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17781842,
            "range": "± 217479",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 103301,
            "range": "± 1123",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1974,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 222659,
            "range": "± 28984",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1001027,
            "range": "± 6219",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1783626,
            "range": "± 18731",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 548513,
            "range": "± 20921",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4375573,
            "range": "± 46769",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2875973,
            "range": "± 7276",
            "unit": "ns/iter"
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
          "id": "09a5da5de787845af5e061fb7a8117c2bbddd6fe",
          "message": "fix: fix conformance workflow JSON corruption and update CI action versions\n\n- Remove incorrect `2>&1 >&2` redirect in conformance run.sh that leaked\n  Python's stderr (human summary) into stdout, corrupting the JSON report\n- Redirect npm install stdout to /dev/null to prevent it contaminating output\n- Add JSON validation in conformance workflow before parsing report\n- Improve error handling: show stderr + raw report content on failure\n- Update action versions to fix Node.js 20 deprecation warnings:\n  checkout v4->v6, setup-node v4->v6, upload-artifact v4->v7, cache v4->v5",
          "timestamp": "2026-03-23T10:18:43+01:00",
          "tree_id": "a8c96a56a67f41c97956a9444e7d4b08e605e7a2",
          "url": "https://github.com/fallow-rs/fallow/commit/09a5da5de787845af5e061fb7a8117c2bbddd6fe"
        },
        "date": 1774257757869,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 40298,
            "range": "± 401",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1188528,
            "range": "± 8829",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2815230,
            "range": "± 34487",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17909293,
            "range": "± 121232",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 101937,
            "range": "± 3982",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1980,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226884,
            "range": "± 4515",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1015316,
            "range": "± 9109",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1787795,
            "range": "± 26285",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 548575,
            "range": "± 11394",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4441250,
            "range": "± 30945",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2819656,
            "range": "± 7635",
            "unit": "ns/iter"
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
          "id": "2a8ab7e14e1bf69949bfc2d2ffe6c7f0243a349e",
          "message": "feat: add progress spinners to analysis pipeline and improve watch mode UX\n\nWire AnalysisProgress spinners into the analysis pipeline stages (discover,\nplugins, parse, resolve, graph, analyze). Progress is shown when stderr is a\nTTY and output format is human-readable — suppressed for --quiet, --format\njson/sarif, and non-TTY environments.\n\nAdd quiet field to ResolvedConfig to control progress bar visibility from\nthe CLI layer. Tracing level set to WARN when spinners are active to prevent\nstderr corruption between indicatif and tracing-subscriber output.",
          "timestamp": "2026-03-23T10:23:06+01:00",
          "tree_id": "86178de0d493c8c33df5a0597670b50f0b1f180a",
          "url": "https://github.com/fallow-rs/fallow/commit/2a8ab7e14e1bf69949bfc2d2ffe6c7f0243a349e"
        },
        "date": 1774258026351,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41436,
            "range": "± 1146",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1242692,
            "range": "± 16947",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2857671,
            "range": "± 49539",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18317242,
            "range": "± 341391",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 103779,
            "range": "± 2334",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1998,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226963,
            "range": "± 4129",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1025613,
            "range": "± 10090",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1791847,
            "range": "± 20386",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 544173,
            "range": "± 34155",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4505971,
            "range": "± 51940",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2850329,
            "range": "± 10663",
            "unit": "ns/iter"
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
          "id": "bca6412b007042e650249de0915e4396a10b50a3",
          "message": "refactor: split 5 largest functions into focused helpers\n\nExtract helpers from main() (258→50 lines), discover_workspaces() (216→12),\nrun_check() (215→130), build_markdown() (195→150), and\nvisit_variable_declaration() (155→40) to improve readability and\nmaintainability.",
          "timestamp": "2026-03-23T10:45:52+01:00",
          "tree_id": "355fd6771f769c139bfa535f81c21f0ed1f2ba4b",
          "url": "https://github.com/fallow-rs/fallow/commit/bca6412b007042e650249de0915e4396a10b50a3"
        },
        "date": 1774259415058,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 40795,
            "range": "± 336",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1217288,
            "range": "± 11736",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2829390,
            "range": "± 115474",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17983930,
            "range": "± 120978",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 110482,
            "range": "± 1687",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1967,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 225685,
            "range": "± 14712",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1013221,
            "range": "± 11195",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1790488,
            "range": "± 19914",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 552393,
            "range": "± 9521",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4455116,
            "range": "± 52943",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2830567,
            "range": "± 18186",
            "unit": "ns/iter"
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
          "id": "bd1e9597c24d5083496cde7e016efa4246bcf52a",
          "message": "feat: add historical metric tracking and GitHub Pages dashboard\n\nAdd benchmark-action/github-action-benchmark tracking to allocs,\ncoverage, and conformance workflows alongside the existing bench\nworkflow. Each pushes to its own gh-pages directory (dev/allocs/,\ndev/coverage/, dev/conformance/) with auto-generated trend charts.\n\n- allocs.yml: track allocations over time (customSmallerIsBetter),\n  PR comments on allocation changes\n- coverage.yml: track coverage % over time (customBiggerIsBetter),\n  migrate badge push from fragile git stash to git worktree\n- conformance.yml: track agreement rate over time\n  (customBiggerIsBetter), upgrade permissions for gh-pages push\n- bench.yml: deploy dashboard index.html to gh-pages root\n- Add .github/pages/index.html dashboard linking to all 4 charts",
          "timestamp": "2026-03-23T10:55:20+01:00",
          "tree_id": "d760a210ccec7fe6e80dedc6efb46d2559f7201a",
          "url": "https://github.com/fallow-rs/fallow/commit/bd1e9597c24d5083496cde7e016efa4246bcf52a"
        },
        "date": 1774259960790,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41686,
            "range": "± 736",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1216368,
            "range": "± 23794",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2834890,
            "range": "± 39965",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 19263584,
            "range": "± 212691",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 111643,
            "range": "± 559",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1990,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226595,
            "range": "± 5647",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1014782,
            "range": "± 12373",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1804960,
            "range": "± 23651",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 548446,
            "range": "± 18595",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4478689,
            "range": "± 55656",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2843045,
            "range": "± 16298",
            "unit": "ns/iter"
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
          "id": "dc388543a716872d2e4999785ecff85eea6e903e",
          "message": "feat: add type-only-dependencies rule and fix formatter gaps\n\nAdd `type-only-dependencies` to RulesConfig (default: warn) so it is\nconfigurable like all other rules. Previously hardcoded to warn in\nhuman/SARIF output.\n\nAlso adds missing type_only_dependencies support to compact and SARIF\nformatters, deduplicates dependency formatting in human.rs, and\nextracts shared format_dependency helper in markdown.rs.",
          "timestamp": "2026-03-23T11:12:58+01:00",
          "tree_id": "d4a14d535e02845b2d8440f3066f06497e0664e6",
          "url": "https://github.com/fallow-rs/fallow/commit/dc388543a716872d2e4999785ecff85eea6e903e"
        },
        "date": 1774261032573,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 34263,
            "range": "± 355",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 957139,
            "range": "± 10910",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2047216,
            "range": "± 70302",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 11964887,
            "range": "± 107451",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 96828,
            "range": "± 2226",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2790,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 166613,
            "range": "± 1093",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 736787,
            "range": "± 4949",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1518108,
            "range": "± 31449",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 472348,
            "range": "± 14622",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3455833,
            "range": "± 26065",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2017862,
            "range": "± 5404",
            "unit": "ns/iter"
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
        "date": 1774261279688,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 39995,
            "range": "± 555",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1209190,
            "range": "± 11565",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2827997,
            "range": "± 109820",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17767536,
            "range": "± 144258",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 110916,
            "range": "± 871",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1978,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226496,
            "range": "± 7319",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1016377,
            "range": "± 31300",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1790561,
            "range": "± 29442",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 542113,
            "range": "± 4819",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4461184,
            "range": "± 42697",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2832225,
            "range": "± 6325",
            "unit": "ns/iter"
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
        "date": 1774268894775,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41912,
            "range": "± 564",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1227338,
            "range": "± 41265",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2906965,
            "range": "± 50075",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18530351,
            "range": "± 225871",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 103024,
            "range": "± 847",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2020,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 233112,
            "range": "± 2636",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1055867,
            "range": "± 17461",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1839199,
            "range": "± 15775",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 554926,
            "range": "± 4699",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4598669,
            "range": "± 51176",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2916296,
            "range": "± 34002",
            "unit": "ns/iter"
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
        "date": 1774273512931,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41951,
            "range": "± 1549",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1218205,
            "range": "± 18331",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2835031,
            "range": "± 31433",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17788320,
            "range": "± 91544",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 110231,
            "range": "± 1010",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2050,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 234101,
            "range": "± 9264",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1027486,
            "range": "± 15271",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1834717,
            "range": "± 38530",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 567608,
            "range": "± 11764",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4467532,
            "range": "± 48578",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2920696,
            "range": "± 20896",
            "unit": "ns/iter"
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
        "date": 1774276228144,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41126,
            "range": "± 467",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1225769,
            "range": "± 29511",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2817338,
            "range": "± 35491",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18032407,
            "range": "± 209017",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 109801,
            "range": "± 1847",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1975,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 227385,
            "range": "± 48612",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1022874,
            "range": "± 7848",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1812016,
            "range": "± 44567",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 553656,
            "range": "± 7808",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4494891,
            "range": "± 39445",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2862649,
            "range": "± 43608",
            "unit": "ns/iter"
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
        "date": 1774280677831,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41417,
            "range": "± 1969",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1239347,
            "range": "± 10866",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2863192,
            "range": "± 40430",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17868336,
            "range": "± 147889",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 111424,
            "range": "± 3663",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2196,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 224981,
            "range": "± 5681",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1012048,
            "range": "± 12112",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1798669,
            "range": "± 16030",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 547983,
            "range": "± 30788",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4447174,
            "range": "± 53316",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2858519,
            "range": "± 7118",
            "unit": "ns/iter"
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
        "date": 1774292657622,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41236,
            "range": "± 963",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1224738,
            "range": "± 12517",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2851073,
            "range": "± 45658",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17842719,
            "range": "± 111596",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 108243,
            "range": "± 5686",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2294,
            "range": "± 118",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226268,
            "range": "± 1990",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1015580,
            "range": "± 8295",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1806224,
            "range": "± 22620",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 547505,
            "range": "± 12752",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4456863,
            "range": "± 89993",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2822289,
            "range": "± 5635",
            "unit": "ns/iter"
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
        "date": 1774293595087,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 42541,
            "range": "± 1203",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1226945,
            "range": "± 22381",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2843345,
            "range": "± 37134",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18043133,
            "range": "± 309994",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 101481,
            "range": "± 1221",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1990,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226001,
            "range": "± 12779",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1014622,
            "range": "± 10466",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1797374,
            "range": "± 25433",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 548237,
            "range": "± 9890",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4467426,
            "range": "± 42292",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2808722,
            "range": "± 5566",
            "unit": "ns/iter"
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
          "id": "4a63117b8c4c8c3996151ef3988992bf70bcdc78",
          "message": "fix: add line/column numbers to duplicate export locations\n\nDuplicateExport.locations changes from Vec<PathBuf> to\nVec<DuplicateLocation> with path, line, and col fields. This fixes\nSARIF and LSP annotations that previously showed :1 for every\nduplicate export because no line information was available.\n\nThe export line/col data was already available in the detection code\nvia ExportSymbol.span but was dropped when only the path was stored.\n\nJSON output schema_version bumped from 1 to 2 (breaking: locations\nchanged from string array to object array).",
          "timestamp": "2026-03-23T21:59:59+01:00",
          "tree_id": "81d41047a78fd95272c116db942408162586aa23",
          "url": "https://github.com/fallow-rs/fallow/commit/4a63117b8c4c8c3996151ef3988992bf70bcdc78"
        },
        "date": 1774299875929,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 41538,
            "range": "± 671",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1248760,
            "range": "± 15038",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2898428,
            "range": "± 208780",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 19060548,
            "range": "± 319580",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 101783,
            "range": "± 430",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1980,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 224638,
            "range": "± 22145",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1014884,
            "range": "± 11399",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1794964,
            "range": "± 21746",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 559417,
            "range": "± 15186",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4502445,
            "range": "± 67790",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2830984,
            "range": "± 17642",
            "unit": "ns/iter"
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
        "date": 1774309600768,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46069,
            "range": "± 658",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1219290,
            "range": "± 26141",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2825116,
            "range": "± 53587",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18170877,
            "range": "± 359970",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 103971,
            "range": "± 1184",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2066,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226287,
            "range": "± 2361",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1019254,
            "range": "± 5985",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1832437,
            "range": "± 21387",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 545139,
            "range": "± 19727",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4460671,
            "range": "± 139715",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2875225,
            "range": "± 31879",
            "unit": "ns/iter"
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
        "date": 1774340111127,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45131,
            "range": "± 955",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1229813,
            "range": "± 24124",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2866782,
            "range": "± 30334",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18065183,
            "range": "± 311399",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 101430,
            "range": "± 695",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1985,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226531,
            "range": "± 8206",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1016504,
            "range": "± 16183",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1791521,
            "range": "± 19256",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 546726,
            "range": "± 6522",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4451855,
            "range": "± 50350",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2874355,
            "range": "± 6228",
            "unit": "ns/iter"
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
        "date": 1774341469125,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 39441,
            "range": "± 1126",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 965823,
            "range": "± 6387",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2080010,
            "range": "± 18501",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 12066349,
            "range": "± 131689",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 108914,
            "range": "± 2076",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2792,
            "range": "± 188",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 167788,
            "range": "± 4760",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 743539,
            "range": "± 3129",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1534764,
            "range": "± 11798",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 463076,
            "range": "± 11298",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3480195,
            "range": "± 14432",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2052340,
            "range": "± 9245",
            "unit": "ns/iter"
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
        "date": 1774343081244,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46737,
            "range": "± 1200",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1243165,
            "range": "± 59732",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2907047,
            "range": "± 51828",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 19420604,
            "range": "± 1257260",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 101205,
            "range": "± 620",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1997,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 245741,
            "range": "± 12591",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1027127,
            "range": "± 17785",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1810128,
            "range": "± 24224",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 572903,
            "range": "± 38360",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4587313,
            "range": "± 62719",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2869304,
            "range": "± 19159",
            "unit": "ns/iter"
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
        "date": 1774343983229,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 47017,
            "range": "± 2380",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1236249,
            "range": "± 18033",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2905418,
            "range": "± 29070",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 19692283,
            "range": "± 409849",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 104451,
            "range": "± 282",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1993,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226961,
            "range": "± 14542",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1019471,
            "range": "± 18810",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1810325,
            "range": "± 20227",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 555853,
            "range": "± 46625",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4532898,
            "range": "± 65872",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2843856,
            "range": "± 61565",
            "unit": "ns/iter"
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
        "date": 1774344217133,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46773,
            "range": "± 595",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1246375,
            "range": "± 16498",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2890710,
            "range": "± 128006",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18655526,
            "range": "± 437468",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 105017,
            "range": "± 549",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1983,
            "range": "± 157",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226373,
            "range": "± 8574",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1020197,
            "range": "± 13552",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1797727,
            "range": "± 20785",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 552287,
            "range": "± 16394",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4466833,
            "range": "± 39704",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2858572,
            "range": "± 7278",
            "unit": "ns/iter"
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
        "date": 1774354906768,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 39204,
            "range": "± 196",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 973265,
            "range": "± 16045",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2161859,
            "range": "± 46637",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 13304330,
            "range": "± 434735",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 104867,
            "range": "± 4088",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2767,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 169047,
            "range": "± 1687",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 757233,
            "range": "± 6852",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1555020,
            "range": "± 24800",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 478589,
            "range": "± 19648",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3677191,
            "range": "± 34658",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2067187,
            "range": "± 10492",
            "unit": "ns/iter"
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
        "date": 1774355149338,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 47045,
            "range": "± 958",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1230357,
            "range": "± 10978",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2885677,
            "range": "± 30511",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18048216,
            "range": "± 93474",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 105011,
            "range": "± 2140",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1998,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 222522,
            "range": "± 10078",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1001419,
            "range": "± 8446",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1786491,
            "range": "± 16821",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 538615,
            "range": "± 33320",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4386514,
            "range": "± 34613",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2791853,
            "range": "± 8773",
            "unit": "ns/iter"
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
        "date": 1774356189800,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46893,
            "range": "± 2067",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1246452,
            "range": "± 11572",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2943495,
            "range": "± 51775",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18828354,
            "range": "± 211610",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 104623,
            "range": "± 468",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1994,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 223291,
            "range": "± 2313",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1005038,
            "range": "± 30671",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1793388,
            "range": "± 18632",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 543142,
            "range": "± 10747",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4415225,
            "range": "± 32559",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2797546,
            "range": "± 10553",
            "unit": "ns/iter"
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
        "date": 1774357073107,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 39381,
            "range": "± 233",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 974065,
            "range": "± 25772",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2124374,
            "range": "± 30371",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 12727000,
            "range": "± 291189",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 97776,
            "range": "± 2648",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2786,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 167334,
            "range": "± 13374",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 744917,
            "range": "± 6191",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1562544,
            "range": "± 27787",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 466531,
            "range": "± 18783",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3520343,
            "range": "± 100800",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2046853,
            "range": "± 8945",
            "unit": "ns/iter"
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
        "date": 1774362614137,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 47394,
            "range": "± 803",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1227366,
            "range": "± 16471",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2882547,
            "range": "± 45080",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18540626,
            "range": "± 258914",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 104166,
            "range": "± 410",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1990,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 225041,
            "range": "± 3729",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1012138,
            "range": "± 9679",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1805912,
            "range": "± 204614",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 545454,
            "range": "± 7368",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4464836,
            "range": "± 46845",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2821229,
            "range": "± 7516",
            "unit": "ns/iter"
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
        "date": 1774364411373,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 47036,
            "range": "± 984",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1228143,
            "range": "± 20729",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2887645,
            "range": "± 31887",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18077209,
            "range": "± 106032",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 106704,
            "range": "± 1133",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2011,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 233468,
            "range": "± 17943",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1028908,
            "range": "± 21960",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1819063,
            "range": "± 21490",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 571149,
            "range": "± 12198",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4521658,
            "range": "± 60046",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2861975,
            "range": "± 9882",
            "unit": "ns/iter"
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
        "date": 1774366376458,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46205,
            "range": "± 2844",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1238354,
            "range": "± 20749",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2843791,
            "range": "± 29726",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18735080,
            "range": "± 809810",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 102056,
            "range": "± 523",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1981,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 229533,
            "range": "± 1396",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1023692,
            "range": "± 9525",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1791083,
            "range": "± 31278",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 548421,
            "range": "± 29729",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4456352,
            "range": "± 34888",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2864509,
            "range": "± 33013",
            "unit": "ns/iter"
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
        "date": 1774382253245,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46306,
            "range": "± 1704",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1221656,
            "range": "± 18168",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2835358,
            "range": "± 84107",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17965440,
            "range": "± 124511",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 106961,
            "range": "± 562",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1980,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 227686,
            "range": "± 6764",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1021936,
            "range": "± 8018",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1792406,
            "range": "± 53677",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 547706,
            "range": "± 8697",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4485015,
            "range": "± 40384",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2835239,
            "range": "± 3689",
            "unit": "ns/iter"
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
        "date": 1774391146523,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 38259,
            "range": "± 495",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1309609,
            "range": "± 11224",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 3071523,
            "range": "± 54286",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 19718676,
            "range": "± 206356",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 99988,
            "range": "± 454",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1972,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 186010,
            "range": "± 5685",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 826089,
            "range": "± 11913",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1667135,
            "range": "± 24764",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 519936,
            "range": "± 35501",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3787846,
            "range": "± 96260",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2247248,
            "range": "± 60654",
            "unit": "ns/iter"
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
        "date": 1774391380761,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45785,
            "range": "± 1368",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1236801,
            "range": "± 47631",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2884911,
            "range": "± 46700",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18024470,
            "range": "± 174858",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 108410,
            "range": "± 15513",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1991,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 225377,
            "range": "± 5187",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1013144,
            "range": "± 52426",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1804611,
            "range": "± 17922",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 544243,
            "range": "± 15254",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4476342,
            "range": "± 45639",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2828116,
            "range": "± 5953",
            "unit": "ns/iter"
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
        "date": 1774391917072,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45204,
            "range": "± 925",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1231768,
            "range": "± 50343",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2875394,
            "range": "± 19197",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18166189,
            "range": "± 289424",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 104459,
            "range": "± 968",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2000,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 202811,
            "range": "± 2523",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 904376,
            "range": "± 9735",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1700140,
            "range": "± 23027",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 510105,
            "range": "± 40162",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4198471,
            "range": "± 31449",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2512575,
            "range": "± 11107",
            "unit": "ns/iter"
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
        "date": 1774393023209,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45864,
            "range": "± 4982",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1238176,
            "range": "± 22228",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2889209,
            "range": "± 37304",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18750110,
            "range": "± 237173",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 103893,
            "range": "± 3928",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1998,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 202418,
            "range": "± 8130",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 906181,
            "range": "± 13991",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1702305,
            "range": "± 19639",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 516313,
            "range": "± 29525",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4212629,
            "range": "± 55096",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2500342,
            "range": "± 19624",
            "unit": "ns/iter"
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
        "date": 1774393341012,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46922,
            "range": "± 417",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1234759,
            "range": "± 27217",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2881790,
            "range": "± 20191",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17988415,
            "range": "± 99006",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 108896,
            "range": "± 1303",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1996,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 203113,
            "range": "± 2068",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 904597,
            "range": "± 12417",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1696942,
            "range": "± 18378",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 513224,
            "range": "± 19474",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4193066,
            "range": "± 29505",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2507620,
            "range": "± 6549",
            "unit": "ns/iter"
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
        "date": 1774429008803,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46993,
            "range": "± 809",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1228938,
            "range": "± 37826",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2843522,
            "range": "± 46852",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18589388,
            "range": "± 320571",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 110602,
            "range": "± 1251",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2012,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 234728,
            "range": "± 10108",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 935919,
            "range": "± 19333",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1740091,
            "range": "± 26851",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 518504,
            "range": "± 33612",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4350791,
            "range": "± 46389",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2545956,
            "range": "± 12548",
            "unit": "ns/iter"
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
        "date": 1774433408157,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 47695,
            "range": "± 255",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1232453,
            "range": "± 76419",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2876496,
            "range": "± 56741",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18130569,
            "range": "± 132268",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 107389,
            "range": "± 1025",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1997,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 205556,
            "range": "± 7913",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 910476,
            "range": "± 17108",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1707258,
            "range": "± 18006",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 517846,
            "range": "± 40672",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4238240,
            "range": "± 39467",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2537618,
            "range": "± 12687",
            "unit": "ns/iter"
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
        "date": 1774451472113,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46777,
            "range": "± 1354",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1268986,
            "range": "± 32439",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2928766,
            "range": "± 40652",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18898680,
            "range": "± 337880",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 108603,
            "range": "± 856",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1988,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 204467,
            "range": "± 3783",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 911135,
            "range": "± 18573",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1699298,
            "range": "± 18745",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 515347,
            "range": "± 5447",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4232681,
            "range": "± 56389",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2522658,
            "range": "± 8591",
            "unit": "ns/iter"
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
        "date": 1774461281767,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 47097,
            "range": "± 2580",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1241540,
            "range": "± 15711",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2895815,
            "range": "± 71659",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18311446,
            "range": "± 449709",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 107944,
            "range": "± 818",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1991,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 209223,
            "range": "± 3936",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 909198,
            "range": "± 11999",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1700422,
            "range": "± 29236",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 519503,
            "range": "± 16773",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4243154,
            "range": "± 56178",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2543694,
            "range": "± 20406",
            "unit": "ns/iter"
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
        "date": 1774461547051,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 47383,
            "range": "± 3816",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1226044,
            "range": "± 24995",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2851185,
            "range": "± 36477",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18186827,
            "range": "± 432004",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 108167,
            "range": "± 644",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1997,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 213220,
            "range": "± 4504",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 916690,
            "range": "± 17044",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1712962,
            "range": "± 69123",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 514004,
            "range": "± 15437",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4202998,
            "range": "± 28417",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2510358,
            "range": "± 5874",
            "unit": "ns/iter"
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
        "date": 1774464879776,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 47063,
            "range": "± 1540",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1239856,
            "range": "± 13765",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2851254,
            "range": "± 33903",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18140279,
            "range": "± 213308",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 105416,
            "range": "± 1146",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2009,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 202872,
            "range": "± 6908",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 906895,
            "range": "± 14320",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1711010,
            "range": "± 20281",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 518344,
            "range": "± 20300",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4218311,
            "range": "± 102748",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2509372,
            "range": "± 7632",
            "unit": "ns/iter"
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
        "date": 1774474062637,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45691,
            "range": "± 1373",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1225208,
            "range": "± 23394",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2870568,
            "range": "± 27187",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17979333,
            "range": "± 113043",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 106206,
            "range": "± 1420",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2013,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 204237,
            "range": "± 3821",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 907024,
            "range": "± 12155",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1697305,
            "range": "± 16028",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 511107,
            "range": "± 35379",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4201137,
            "range": "± 30718",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2502767,
            "range": "± 7964",
            "unit": "ns/iter"
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
        "date": 1774514101992,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46084,
            "range": "± 473",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1238176,
            "range": "± 33492",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2875957,
            "range": "± 36568",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18067591,
            "range": "± 199412",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 101785,
            "range": "± 1316",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1985,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 201338,
            "range": "± 2327",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 903336,
            "range": "± 13767",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1704208,
            "range": "± 28988",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 516232,
            "range": "± 31142",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4208317,
            "range": "± 34844",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2504563,
            "range": "± 9287",
            "unit": "ns/iter"
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
        "date": 1774515447508,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45157,
            "range": "± 817",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1205016,
            "range": "± 29967",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2653938,
            "range": "± 27837",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16469502,
            "range": "± 464393",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 107629,
            "range": "± 1183",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1983,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 202367,
            "range": "± 4540",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 905625,
            "range": "± 12750",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1712092,
            "range": "± 47314",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 511975,
            "range": "± 21200",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4212497,
            "range": "± 29409",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2498873,
            "range": "± 9672",
            "unit": "ns/iter"
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
        "date": 1774521520795,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46903,
            "range": "± 2312",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1240976,
            "range": "± 23924",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2894768,
            "range": "± 42546",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18183946,
            "range": "± 128541",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 101952,
            "range": "± 388",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1980,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 205874,
            "range": "± 7655",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 915061,
            "range": "± 17360",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1734423,
            "range": "± 45648",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 513975,
            "range": "± 33524",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4228195,
            "range": "± 41438",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2530058,
            "range": "± 17588",
            "unit": "ns/iter"
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
        "date": 1774523058913,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46525,
            "range": "± 859",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1241851,
            "range": "± 23436",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2920768,
            "range": "± 62559",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18782975,
            "range": "± 371306",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 98077,
            "range": "± 1480",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2004,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 206919,
            "range": "± 4343",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 920452,
            "range": "± 18713",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1714502,
            "range": "± 27249",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 521128,
            "range": "± 28670",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4282360,
            "range": "± 79141",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2554385,
            "range": "± 22831",
            "unit": "ns/iter"
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
          "id": "75229388f47b6d288e5fab2d733c63039964aef7",
          "message": "chore(deps): bump toml from 1.0.7+spec-1.1.0 to 1.1.0+spec-1.1.0 (#25)\n\nBumps [toml](https://github.com/toml-rs/toml) from 1.0.7+spec-1.1.0 to 1.1.0+spec-1.1.0.\n- [Commits](https://github.com/toml-rs/toml/compare/toml-v1.0.7...toml-v1.1.0)\n\n---\nupdated-dependencies:\n- dependency-name: toml\n  dependency-version: 1.1.0+spec-1.1.0\n  dependency-type: direct:production\n  update-type: version-update:semver-minor\n...\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-26T12:54:48+01:00",
          "tree_id": "20f4c42161172c71f4780fad1b141814bbaf4347",
          "url": "https://github.com/fallow-rs/fallow/commit/75229388f47b6d288e5fab2d733c63039964aef7"
        },
        "date": 1774526323965,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 47438,
            "range": "± 662",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1251527,
            "range": "± 23441",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2957338,
            "range": "± 155148",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18511573,
            "range": "± 701703",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 102647,
            "range": "± 1475",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1984,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 206319,
            "range": "± 8131",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 912892,
            "range": "± 58274",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1731362,
            "range": "± 26461",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 511557,
            "range": "± 7195",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4223723,
            "range": "± 42116",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2523640,
            "range": "± 12101",
            "unit": "ns/iter"
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
        "date": 1774537913869,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 48035,
            "range": "± 2695",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1273074,
            "range": "± 28363",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2977976,
            "range": "± 41730",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18713710,
            "range": "± 218334",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 107067,
            "range": "± 1506",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2066,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 207262,
            "range": "± 10491",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 924594,
            "range": "± 11296",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1750343,
            "range": "± 27337",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 523167,
            "range": "± 22712",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4303321,
            "range": "± 55266",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2585643,
            "range": "± 18907",
            "unit": "ns/iter"
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
        "date": 1774538725410,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46934,
            "range": "± 501",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1244814,
            "range": "± 26096",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2886108,
            "range": "± 39303",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 18287504,
            "range": "± 201101",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 101513,
            "range": "± 1827",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1976,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 204637,
            "range": "± 1751",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 909135,
            "range": "± 13936",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1712411,
            "range": "± 20618",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 529618,
            "range": "± 34810",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4222030,
            "range": "± 52648",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2519663,
            "range": "± 28905",
            "unit": "ns/iter"
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
        "date": 1774539041580,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46628,
            "range": "± 1018",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1182288,
            "range": "± 12821",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2637294,
            "range": "± 38025",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16460373,
            "range": "± 180372",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 108912,
            "range": "± 334",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2009,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 203923,
            "range": "± 4033",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 907630,
            "range": "± 10551",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1698529,
            "range": "± 12494",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 525671,
            "range": "± 14418",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4239050,
            "range": "± 50365",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2507147,
            "range": "± 14705",
            "unit": "ns/iter"
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
        "date": 1774554867644,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46236,
            "range": "± 1707",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1211232,
            "range": "± 23675",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2670048,
            "range": "± 40586",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16202265,
            "range": "± 412985",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 100152,
            "range": "± 572",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2064,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 202801,
            "range": "± 15487",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 905312,
            "range": "± 9380",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1702353,
            "range": "± 13558",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 517124,
            "range": "± 12578",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4196649,
            "range": "± 33853",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2502571,
            "range": "± 3200",
            "unit": "ns/iter"
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
        "date": 1774555721069,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 35272,
            "range": "± 305",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1147387,
            "range": "± 179804",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2556336,
            "range": "± 52406",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15660781,
            "range": "± 263613",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 107003,
            "range": "± 754",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2014,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 180067,
            "range": "± 8383",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 789814,
            "range": "± 3917",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1642722,
            "range": "± 20012",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 503788,
            "range": "± 11612",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3656133,
            "range": "± 39366",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2156647,
            "range": "± 8678",
            "unit": "ns/iter"
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
        "date": 1774595609518,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46141,
            "range": "± 1429",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1206941,
            "range": "± 22493",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2674880,
            "range": "± 72683",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16487876,
            "range": "± 152763",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 97784,
            "range": "± 489",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1993,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 207094,
            "range": "± 4720",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 911802,
            "range": "± 13588",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1715559,
            "range": "± 23857",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 516515,
            "range": "± 9811",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4219476,
            "range": "± 175639",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2507069,
            "range": "± 9303",
            "unit": "ns/iter"
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
        "date": 1774595918446,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 39219,
            "range": "± 462",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 935228,
            "range": "± 11602",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 1971143,
            "range": "± 26462",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 11313584,
            "range": "± 83518",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 97312,
            "range": "± 2557",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2749,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 160041,
            "range": "± 5346",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 726534,
            "range": "± 19121",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1499439,
            "range": "± 19449",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 462620,
            "range": "± 7416",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3430831,
            "range": "± 97847",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2001797,
            "range": "± 9556",
            "unit": "ns/iter"
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
        "date": 1774596400828,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46622,
            "range": "± 1598",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1180222,
            "range": "± 30761",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2729312,
            "range": "± 60866",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16727187,
            "range": "± 634541",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 98370,
            "range": "± 518",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1981,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 203205,
            "range": "± 23327",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 902988,
            "range": "± 34876",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1684435,
            "range": "± 35134",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 508963,
            "range": "± 25856",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4183895,
            "range": "± 25833",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2499179,
            "range": "± 12335",
            "unit": "ns/iter"
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
        "date": 1774597540658,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45829,
            "range": "± 1427",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1133298,
            "range": "± 95252",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2482494,
            "range": "± 30648",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15400068,
            "range": "± 364653",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 100248,
            "range": "± 855",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1968,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 203920,
            "range": "± 2434",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 907595,
            "range": "± 12048",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1702897,
            "range": "± 29578",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 514608,
            "range": "± 28112",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4252043,
            "range": "± 31836",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2517907,
            "range": "± 11369",
            "unit": "ns/iter"
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
        "date": 1774600642274,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 47585,
            "range": "± 1969",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1207997,
            "range": "± 23633",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2690287,
            "range": "± 46274",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 17197530,
            "range": "± 537497",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 100786,
            "range": "± 784",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2002,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 203876,
            "range": "± 1567",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 907283,
            "range": "± 10974",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1699563,
            "range": "± 22917",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 526439,
            "range": "± 12567",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4203555,
            "range": "± 42380",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2508285,
            "range": "± 11252",
            "unit": "ns/iter"
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
        "date": 1774604055349,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45265,
            "range": "± 306",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1118412,
            "range": "± 29068",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2509920,
            "range": "± 22736",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16013647,
            "range": "± 380034",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 102916,
            "range": "± 494",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2008,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 203442,
            "range": "± 5221",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 907484,
            "range": "± 11016",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1697598,
            "range": "± 41582",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 530586,
            "range": "± 12180",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4227654,
            "range": "± 41517",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2509205,
            "range": "± 15047",
            "unit": "ns/iter"
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
        "date": 1774611058264,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45078,
            "range": "± 2235",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1112587,
            "range": "± 28704",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2478206,
            "range": "± 32015",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15129582,
            "range": "± 91212",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 106060,
            "range": "± 449",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2004,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 211880,
            "range": "± 12133",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 916971,
            "range": "± 11102",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1701770,
            "range": "± 25691",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 514117,
            "range": "± 23354",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4190923,
            "range": "± 53404",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2543507,
            "range": "± 34509",
            "unit": "ns/iter"
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
        "date": 1774611291378,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45120,
            "range": "± 482",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1109077,
            "range": "± 28833",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2430298,
            "range": "± 54190",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 14840455,
            "range": "± 78981",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 109561,
            "range": "± 1586",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2008,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 207093,
            "range": "± 6072",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 920466,
            "range": "± 17852",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1705208,
            "range": "± 17886",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 513875,
            "range": "± 9914",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4206701,
            "range": "± 61743",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2563093,
            "range": "± 16197",
            "unit": "ns/iter"
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
        "date": 1774611856009,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 44806,
            "range": "± 290",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1106360,
            "range": "± 11299",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2409209,
            "range": "± 34863",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 14805930,
            "range": "± 254859",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 109198,
            "range": "± 1180",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1981,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 204530,
            "range": "± 5829",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 913037,
            "range": "± 188725",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1695927,
            "range": "± 22992",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 519009,
            "range": "± 7275",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4212879,
            "range": "± 35740",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2581521,
            "range": "± 18822",
            "unit": "ns/iter"
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
        "date": 1774618501299,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45718,
            "range": "± 1433",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1134386,
            "range": "± 13178",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2508647,
            "range": "± 60862",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15023901,
            "range": "± 247282",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 100273,
            "range": "± 375",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1973,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 205327,
            "range": "± 5285",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 914681,
            "range": "± 8636",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1709302,
            "range": "± 35970",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 530777,
            "range": "± 12876",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4235683,
            "range": "± 38499",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2536583,
            "range": "± 6925",
            "unit": "ns/iter"
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
        "date": 1774630113925,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45110,
            "range": "± 1959",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1113791,
            "range": "± 7982",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2452693,
            "range": "± 19150",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 14957931,
            "range": "± 194291",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 106858,
            "range": "± 681",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1994,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 216000,
            "range": "± 9410",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 934229,
            "range": "± 14365",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1732547,
            "range": "± 35794",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 509777,
            "range": "± 11233",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4262951,
            "range": "± 45151",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2602155,
            "range": "± 12894",
            "unit": "ns/iter"
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
        "date": 1774637275795,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 49260,
            "range": "± 880",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1231319,
            "range": "± 38981",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2715475,
            "range": "± 44705",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16523549,
            "range": "± 234053",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 112786,
            "range": "± 1301",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2180,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 226044,
            "range": "± 30040",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 1008310,
            "range": "± 22352",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1882153,
            "range": "± 24043",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 568454,
            "range": "± 36046",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4524298,
            "range": "± 93581",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2786413,
            "range": "± 35977",
            "unit": "ns/iter"
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
        "date": 1774642405796,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 43934,
            "range": "± 1251",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1120614,
            "range": "± 57185",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2490857,
            "range": "± 21847",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15122214,
            "range": "± 140983",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 109434,
            "range": "± 1031",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2215,
            "range": "± 145",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 205467,
            "range": "± 10358",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 914864,
            "range": "± 47676",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1709453,
            "range": "± 30291",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 565066,
            "range": "± 32659",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4304942,
            "range": "± 44515",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2520276,
            "range": "± 28146",
            "unit": "ns/iter"
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
        "date": 1774643187468,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45875,
            "range": "± 1013",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1130687,
            "range": "± 15421",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2519205,
            "range": "± 31214",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16765609,
            "range": "± 272621",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 101352,
            "range": "± 1641",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1986,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 205406,
            "range": "± 5811",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 911151,
            "range": "± 12452",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1708640,
            "range": "± 14907",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 521450,
            "range": "± 39452",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4304009,
            "range": "± 38256",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2520228,
            "range": "± 37889",
            "unit": "ns/iter"
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
        "date": 1774644846825,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 35781,
            "range": "± 207",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1146990,
            "range": "± 21066",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2575219,
            "range": "± 108791",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16282960,
            "range": "± 315998",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 97895,
            "range": "± 372",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1988,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 179481,
            "range": "± 1874",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 788546,
            "range": "± 8375",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1643274,
            "range": "± 20396",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 511935,
            "range": "± 36741",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3717155,
            "range": "± 38452",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2150315,
            "range": "± 9811",
            "unit": "ns/iter"
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
        "date": 1774645658432,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45020,
            "range": "± 515",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1124305,
            "range": "± 35534",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2500909,
            "range": "± 21416",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15510056,
            "range": "± 234351",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 103944,
            "range": "± 1402",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1996,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 236013,
            "range": "± 11904",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 943339,
            "range": "± 11024",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1754936,
            "range": "± 18127",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 524830,
            "range": "± 17022",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4341751,
            "range": "± 30795",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2561940,
            "range": "± 5787",
            "unit": "ns/iter"
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
        "date": 1774647190189,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 44891,
            "range": "± 770",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1645020,
            "range": "± 52912",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 3003921,
            "range": "± 60000",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15466970,
            "range": "± 108729",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 105874,
            "range": "± 688",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2072,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 203277,
            "range": "± 3267",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 909337,
            "range": "± 17977",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1710762,
            "range": "± 36504",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 528360,
            "range": "± 20304",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4203321,
            "range": "± 29193",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2514919,
            "range": "± 9081",
            "unit": "ns/iter"
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
        "date": 1774647630895,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46292,
            "range": "± 6614",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1631153,
            "range": "± 108181",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2997394,
            "range": "± 28840",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15453951,
            "range": "± 410668",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 107185,
            "range": "± 1801",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1985,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 203487,
            "range": "± 6307",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 908856,
            "range": "± 9742",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1708996,
            "range": "± 19026",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 519220,
            "range": "± 24043",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4200599,
            "range": "± 33446",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2520776,
            "range": "± 6665",
            "unit": "ns/iter"
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
        "date": 1774647859894,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 46513,
            "range": "± 780",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1640301,
            "range": "± 49165",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 3006125,
            "range": "± 26196",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15386104,
            "range": "± 115631",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 107005,
            "range": "± 629",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2003,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 205430,
            "range": "± 7201",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 913449,
            "range": "± 10444",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1710641,
            "range": "± 23483",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 514519,
            "range": "± 29079",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4221153,
            "range": "± 26761",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2528452,
            "range": "± 16157",
            "unit": "ns/iter"
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
        "date": 1774648159037,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 44687,
            "range": "± 391",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1619513,
            "range": "± 19987",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 3001334,
            "range": "± 35742",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15506714,
            "range": "± 145141",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 103161,
            "range": "± 1870",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2062,
            "range": "± 117",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 208605,
            "range": "± 10827",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 925514,
            "range": "± 32598",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1779579,
            "range": "± 57148",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 509738,
            "range": "± 41530",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4323247,
            "range": "± 59388",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2595688,
            "range": "± 75206",
            "unit": "ns/iter"
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
        "date": 1774649212431,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 44658,
            "range": "± 1190",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1655436,
            "range": "± 20763",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 3042167,
            "range": "± 32601",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16729799,
            "range": "± 425946",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 105848,
            "range": "± 619",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2019,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 204254,
            "range": "± 8541",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 911914,
            "range": "± 87110",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1711999,
            "range": "± 29419",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 517010,
            "range": "± 25765",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4295353,
            "range": "± 61078",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2532601,
            "range": "± 9966",
            "unit": "ns/iter"
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
        "date": 1774655004137,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45529,
            "range": "± 375",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1649141,
            "range": "± 45398",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 3006652,
            "range": "± 144085",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15398638,
            "range": "± 80962",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 101730,
            "range": "± 1483",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1979,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 211784,
            "range": "± 3237",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 918931,
            "range": "± 10954",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1729997,
            "range": "± 54603",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 519377,
            "range": "± 13411",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4266195,
            "range": "± 70110",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2536768,
            "range": "± 9182",
            "unit": "ns/iter"
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
        "date": 1774655677312,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 44801,
            "range": "± 425",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1623404,
            "range": "± 43359",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 3002617,
            "range": "± 30516",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15394104,
            "range": "± 134437",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 102522,
            "range": "± 1236",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1984,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 202991,
            "range": "± 4138",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 905256,
            "range": "± 13573",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1704787,
            "range": "± 52851",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 515825,
            "range": "± 31899",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4185987,
            "range": "± 36217",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2505002,
            "range": "± 10735",
            "unit": "ns/iter"
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
        "date": 1774655984035,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 44212,
            "range": "± 740",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1638680,
            "range": "± 103463",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2998587,
            "range": "± 26138",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15702105,
            "range": "± 228699",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 105203,
            "range": "± 1042",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2112,
            "range": "± 100",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 207045,
            "range": "± 4989",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 917804,
            "range": "± 41801",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1737908,
            "range": "± 27128",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 537022,
            "range": "± 36381",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4271642,
            "range": "± 46766",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2554865,
            "range": "± 65135",
            "unit": "ns/iter"
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
        "date": 1774656627827,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45364,
            "range": "± 534",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1619909,
            "range": "± 40157",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2996800,
            "range": "± 30277",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15796911,
            "range": "± 302669",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 101663,
            "range": "± 396",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1984,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 203325,
            "range": "± 5194",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 905922,
            "range": "± 11068",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1708224,
            "range": "± 19900",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 522652,
            "range": "± 9262",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4060242,
            "range": "± 55997",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2505874,
            "range": "± 7256",
            "unit": "ns/iter"
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
        "date": 1774656961525,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 44199,
            "range": "± 582",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1598879,
            "range": "± 37436",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2996941,
            "range": "± 27805",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15378173,
            "range": "± 133340",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 104850,
            "range": "± 1500",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1989,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 215052,
            "range": "± 3249",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 931639,
            "range": "± 9372",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1738099,
            "range": "± 22501",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 515408,
            "range": "± 6833",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4209286,
            "range": "± 25890",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2532189,
            "range": "± 3733",
            "unit": "ns/iter"
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
        "date": 1774700965372,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45562,
            "range": "± 876",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1633539,
            "range": "± 30051",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 3001606,
            "range": "± 96685",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15430995,
            "range": "± 66034",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 107098,
            "range": "± 628",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2032,
            "range": "± 242",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 208857,
            "range": "± 2484",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 923053,
            "range": "± 10790",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1731562,
            "range": "± 33087",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 519281,
            "range": "± 32457",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4223237,
            "range": "± 39479",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2545682,
            "range": "± 87492",
            "unit": "ns/iter"
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
        "date": 1774723040537,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 44747,
            "range": "± 1054",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1620199,
            "range": "± 93968",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2995414,
            "range": "± 34245",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15487210,
            "range": "± 231254",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 102352,
            "range": "± 1286",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1983,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 206930,
            "range": "± 5606",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 926846,
            "range": "± 13032",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1733020,
            "range": "± 37866",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 513018,
            "range": "± 16725",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4211927,
            "range": "± 45488",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2558848,
            "range": "± 45327",
            "unit": "ns/iter"
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
        "date": 1774723278071,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 44605,
            "range": "± 230",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1616156,
            "range": "± 38373",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2998335,
            "range": "± 105118",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15281602,
            "range": "± 100626",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 107009,
            "range": "± 2139",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1987,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 206108,
            "range": "± 42906",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 919403,
            "range": "± 13818",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1723983,
            "range": "± 23700",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 514228,
            "range": "± 12448",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4193572,
            "range": "± 42549",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2541188,
            "range": "± 9301",
            "unit": "ns/iter"
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
        "date": 1774723529970,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45554,
            "range": "± 775",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1641219,
            "range": "± 31499",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 3005583,
            "range": "± 22167",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15431768,
            "range": "± 178918",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 104197,
            "range": "± 1560",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1980,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 203081,
            "range": "± 3589",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 903503,
            "range": "± 13635",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1711078,
            "range": "± 16958",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 521053,
            "range": "± 23252",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4214981,
            "range": "± 162668",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2536836,
            "range": "± 11982",
            "unit": "ns/iter"
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
        "date": 1774723762561,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45344,
            "range": "± 956",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1606829,
            "range": "± 20461",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 3001879,
            "range": "± 25992",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15332380,
            "range": "± 97111",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 106493,
            "range": "± 379",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1981,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 203020,
            "range": "± 4563",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 906057,
            "range": "± 169593",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1714339,
            "range": "± 23546",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 518698,
            "range": "± 29554",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4202042,
            "range": "± 36936",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2497313,
            "range": "± 11556",
            "unit": "ns/iter"
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
        "date": 1774729206894,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 44513,
            "range": "± 489",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1623196,
            "range": "± 101713",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 3005422,
            "range": "± 39463",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 16274997,
            "range": "± 594291",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 112251,
            "range": "± 1272",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1997,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 204181,
            "range": "± 1801",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 911217,
            "range": "± 16939",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1733028,
            "range": "± 28650",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 517631,
            "range": "± 32903",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4271865,
            "range": "± 154665",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2523917,
            "range": "± 8965",
            "unit": "ns/iter"
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
        "date": 1774731992950,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 44043,
            "range": "± 1507",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1648655,
            "range": "± 22788",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 3004653,
            "range": "± 16914",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15331166,
            "range": "± 199070",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 102629,
            "range": "± 600",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2014,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 204920,
            "range": "± 4473",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 911319,
            "range": "± 37045",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1716605,
            "range": "± 46725",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 512591,
            "range": "± 27729",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4236549,
            "range": "± 28789",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2512828,
            "range": "± 4966",
            "unit": "ns/iter"
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
        "date": 1774732279811,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 44688,
            "range": "± 622",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1606430,
            "range": "± 37263",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2998884,
            "range": "± 38685",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15354230,
            "range": "± 70183",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 102240,
            "range": "± 3591",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1976,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 206937,
            "range": "± 2353",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 913456,
            "range": "± 17266",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1708327,
            "range": "± 16411",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 542479,
            "range": "± 10386",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4194789,
            "range": "± 37909",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2497537,
            "range": "± 14479",
            "unit": "ns/iter"
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
          "id": "b5d12c934e03f448e4beaeafb44b44f512194735",
          "message": "fix: track enum usage in Record<Enum, T> and keyof typeof mapped types",
          "timestamp": "2026-03-28T22:32:45+01:00",
          "tree_id": "e144545d50fbc8cf1c71a6dda73a59ebb0b7b670",
          "url": "https://github.com/fallow-rs/fallow/commit/b5d12c934e03f448e4beaeafb44b44f512194735"
        },
        "date": 1774734245147,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 44589,
            "range": "± 1630",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1622228,
            "range": "± 27033",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2984460,
            "range": "± 164832",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15346558,
            "range": "± 100569",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 101945,
            "range": "± 4435",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 1989,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 209920,
            "range": "± 8760",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 911045,
            "range": "± 10414",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1718191,
            "range": "± 80010",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 515291,
            "range": "± 38638",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4184756,
            "range": "± 44001",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2515065,
            "range": "± 11312",
            "unit": "ns/iter"
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
        "date": 1774734480093,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 45535,
            "range": "± 1980",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1637113,
            "range": "± 25032",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 3016537,
            "range": "± 66818",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 15671690,
            "range": "± 73975",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 105329,
            "range": "± 1724",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2004,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 206890,
            "range": "± 7752",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 915235,
            "range": "± 17139",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1727893,
            "range": "± 27313",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 514071,
            "range": "± 22186",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 4239052,
            "range": "± 41614",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2544995,
            "range": "± 13409",
            "unit": "ns/iter"
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
        "date": 1774735139750,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse_single_file",
            "value": 39676,
            "range": "± 361",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_10_files",
            "value": 1443173,
            "range": "± 71331",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_100_files",
            "value": 2480818,
            "range": "± 73711",
            "unit": "ns/iter"
          },
          {
            "name": "full_pipeline_1000_files",
            "value": 11876601,
            "range": "± 77801",
            "unit": "ns/iter"
          },
          {
            "name": "resolve_re_export_chains",
            "value": 98968,
            "range": "± 3111",
            "unit": "ns/iter"
          },
          {
            "name": "cache_round_trip",
            "value": 2727,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x500_identical",
            "value": 161531,
            "range": "± 2514",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x2000_identical",
            "value": 740912,
            "range": "± 6565",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_10x500_identical",
            "value": 1543277,
            "range": "± 24283",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_50x200_diverse",
            "value": 458132,
            "range": "± 14966",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_100x200_mixed",
            "value": 3666074,
            "range": "± 136390",
            "unit": "ns/iter"
          },
          {
            "name": "dupe_detect_2x5000_identical",
            "value": 2012311,
            "range": "± 10309",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}