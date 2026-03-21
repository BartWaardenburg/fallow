window.BENCHMARK_DATA = {
  "lastUpdate": 1774079167934,
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
      }
    ]
  }
}