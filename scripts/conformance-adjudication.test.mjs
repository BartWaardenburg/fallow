import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const AGGREGATE = join(REPO_ROOT, "tests/conformance/aggregate.py");
const REGISTER = join(REPO_ROOT, "tests/conformance/adjudications.json");

/** One per-project report in the shape compare.py emits. */
const projectReport = (fallowOnly, knipOnly = []) => ({
  summary: {
    fallow_total: fallowOnly.length,
    knip_total: knipOnly.length,
    agreed: 1,
    fallow_only: fallowOnly.length,
    knip_only: knipOnly.length,
    agreement_pct: 50.0,
  },
  by_type: {},
  details: { agreed: [], fallow_only: fallowOnly, knip_only: knipOnly },
});

const row = (file, name, type = "unused_exports") => ({ file, name, type });

const runAggregate = (reportsDir, registerPath) =>
  spawnSync("python3", [AGGREGATE, reportsDir, "--adjudications", registerPath], {
    encoding: "utf8",
  });

const withFixture = (fn) => {
  const dir = mkdtempSync(join(tmpdir(), "conformance-adjudication-"));
  try {
    return fn(dir);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
};

const REGISTER_SHELL = {
  schema_version: 1,
  reviewed_on: "2026-09-07",
  method: "test fixture",
  knip_version: "6.32.0",
  corpus_refs: { preact: "10.25.4" },
  verdicts: {
    fallow_wrong: "fallow is wrong",
    knip_wrong: "knip is wrong",
    model_difference: "both defensible",
  },
  causes: { resolve: "resolution", analysis: "analysis" },
  record_fields: [
    "project",
    "issue_type",
    "path",
    "export_name",
    "verdict",
    "cause",
    "knip_version",
    "corpus_ref",
    "reviewed_on",
    "issue",
  ],
  adjudications: [],
};

const record = (overrides) => ({
  project: "preact",
  issue_type: "unused_exports",
  path: "src/index.ts",
  export_name: "unusedThing",
  verdict: "fallow_wrong",
  cause: "resolve",
  knip_version: "6.32.0",
  corpus_ref: "10.25.4",
  reviewed_on: "2026-09-07",
  issue: "https://github.com/fallow-rs/fallow/issues/1",
  ...overrides,
});

test("every disagreement row lands in an adjudication bucket", () => {
  withFixture((dir) => {
    writeFileSync(
      join(dir, "preact-report.json"),
      JSON.stringify(
        projectReport(
          [row("src/index.ts", "unusedThing"), row("src/other.ts", "another")],
          [row("src/knip.ts", "knipOnlyThing")],
        ),
      ),
    );
    const registerPath = join(dir, "register.json");
    writeFileSync(
      registerPath,
      JSON.stringify({
        ...REGISTER_SHELL,
        adjudications: [
          record({}),
          record({
            path: "src/knip.ts",
            export_name: "knipOnlyThing",
            verdict: "knip_wrong",
            cause: "analysis",
          }),
        ],
      }),
    );

    const result = runAggregate(dir, registerPath);
    assert.equal(result.status, 0, result.stderr);
    const report = JSON.parse(result.stdout);

    assert.equal(report.summary.adjudicated_fallow_wrong, 1);
    assert.equal(report.summary.adjudicated_knip_wrong, 1);
    assert.equal(report.summary.adjudicated_model_difference, 0);
    // src/other.ts:another is nobody's verdict yet.
    assert.equal(report.summary.unadjudicated, 1);
    // The buckets partition the disagreements, so nothing is silently dropped.
    assert.equal(
      report.summary.adjudicated_fallow_wrong +
        report.summary.adjudicated_knip_wrong +
        report.summary.adjudicated_model_difference +
        report.summary.unadjudicated,
      report.summary.fallow_only + report.summary.knip_only,
    );
    // Per-project rows carry the same counters.
    assert.equal(report.projects.preact.unadjudicated, 1);
    assert.equal(report.projects.preact.adjudicated_fallow_wrong, 1);
  });
});

test("an adjudication for another project does not credit this one", () => {
  withFixture((dir) => {
    writeFileSync(
      join(dir, "preact-report.json"),
      JSON.stringify(projectReport([row("src/index.ts", "unusedThing")])),
    );
    const registerPath = join(dir, "register.json");
    writeFileSync(
      registerPath,
      JSON.stringify({
        ...REGISTER_SHELL,
        adjudications: [record({ project: "vite" })],
      }),
    );

    const result = runAggregate(dir, registerPath);
    assert.equal(result.status, 0, result.stderr);
    const report = JSON.parse(result.stdout);
    assert.equal(report.summary.adjudicated_fallow_wrong, 0);
    assert.equal(report.summary.unadjudicated, 1);
  });
});

test("an unknown verdict, unknown cause, or missing field fails loudly", () => {
  withFixture((dir) => {
    writeFileSync(
      join(dir, "preact-report.json"),
      JSON.stringify(projectReport([row("src/index.ts", "unusedThing")])),
    );

    const cases = [
      [{ verdict: "fallow_is_a_bit_off" }, /unknown verdict/],
      [{ cause: "vibes" }, /unknown cause/],
    ];
    for (const [overrides, pattern] of cases) {
      const registerPath = join(dir, "register.json");
      writeFileSync(
        registerPath,
        JSON.stringify({ ...REGISTER_SHELL, adjudications: [record(overrides)] }),
      );
      const result = runAggregate(dir, registerPath);
      assert.equal(result.status, 1);
      assert.match(result.stderr, pattern);
    }

    const incomplete = record({});
    delete incomplete.issue;
    const registerPath = join(dir, "register.json");
    writeFileSync(registerPath, JSON.stringify({ ...REGISTER_SHELL, adjudications: [incomplete] }));
    const result = runAggregate(dir, registerPath);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /missing issue/);
  });
});

test("the committed register parses and declares the three verdicts", () => {
  withFixture((dir) => {
    writeFileSync(
      join(dir, "preact-report.json"),
      JSON.stringify(projectReport([row("src/index.ts", "unusedThing")])),
    );
    const result = runAggregate(dir, REGISTER);
    assert.equal(result.status, 0, result.stderr);
    const report = JSON.parse(result.stdout);
    // Nothing adjudicated yet: the whole disagreement set is the backlog.
    assert.equal(report.summary.unadjudicated, 1);
    assert.match(result.stderr, /Unadjudicated: 1/);
  });
});
