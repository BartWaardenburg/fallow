use std::collections::BTreeMap;

use super::super::FallowMcp;

const DESCRIPTION_FIXTURE: &str = include_str!("fixtures/tool-descriptions.json");
const SERVER_SOURCE: &str = include_str!("../mod.rs");

/// Live `tools/list` descriptions, per tool. This is one of the two channels
/// `tools/list` carries; the input schemas are the other, and
/// [`live_tool_schema_bytes`] budgets them.
fn live_tool_descriptions() -> BTreeMap<String, String> {
    let server = FallowMcp::new();
    server
        .tool_router
        .list_all()
        .iter()
        .map(|tool| {
            (
                tool.name.to_string(),
                tool.description.as_deref().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn descriptions_match(fixture: &str, live: &BTreeMap<String, String>) -> bool {
    serde_json::from_str::<BTreeMap<String, String>>(fixture)
        .is_ok_and(|expected| expected.eq(live))
}

#[test]
fn live_tool_descriptions_match_the_checked_fixture() {
    let live = live_tool_descriptions();
    assert!(
        descriptions_match(DESCRIPTION_FIXTURE, &live),
        "live MCP tool descriptions changed; update the checked fixture only for an intentional wire-contract change"
    );
}

#[test]
fn analyze_description_covers_supported_dependency_override_sources() {
    let live = live_tool_descriptions();
    let analyze = live.get("analyze").expect("analyze description");

    assert!(analyze.contains("top-level `package.json#overrides`"));
    assert!(analyze.contains("Bun `package.json#resolutions`"));
    assert!(!analyze.contains("unused pnpm dependency overrides"));
    assert!(!analyze.contains("misconfigured pnpm dependency overrides"));
}

#[test]
fn description_contract_detects_punctuation_and_whitespace_drift() {
    let live = BTreeMap::from([("example".to_owned(), "alpha beta".to_owned())]);

    assert!(!descriptions_match(r#"{"example":"alpha beta."}"#, &live));
    assert!(!descriptions_match(r#"{"example":"alpha  beta"}"#, &live));
}

#[test]
fn tool_attributes_take_descriptions_from_method_docs() {
    assert!(
        !SERVER_SOURCE.contains("description ="),
        "tool descriptions must live in method docs, not #[tool] arguments"
    );
}

/// Per-tool wire-description ceiling. Every `tools/list` byte is resident in
/// every agent session that connects, whether or not the tool is ever called,
/// so a description that grows without a budget is a permanent tax. Per-flag
/// payload shapes, unit vocabularies, and suppression placements belong in the
/// `fallow://tools/{name}` guide resource, which an agent reads once and only
/// for the tool it is about to call.
const MAX_TOOL_DESCRIPTION_BYTES: usize = 2_000;

/// Total wire-description bytes measured the last time this gate was re-pinned.
/// This is the ratchet's high-water mark, not the assertion: the target is
/// 35_000, reached by moving one tool's per-flag prose into its
/// `fallow://tools/{name}` guide at a time.
const RECORDED_TOTAL_DESCRIPTION_BYTES: usize = 54_433;

/// Deliberate headroom over [`RECORDED_TOTAL_DESCRIPTION_BYTES`].
///
/// Pinned to the exact live total, the gate failed on a one-word wording fix,
/// which reads as a break rather than as a budget and teaches the next
/// maintainer to raise the number reflexively. A kilobyte absorbs ordinary
/// rewording (a clarified sentence, a corrected flag name) while still
/// catching what the budget exists for: prose that grows by a paragraph.
///
/// It is spendable, and nothing reclaims it on its own. The re-pin check below
/// fires only when the live total drops [`TOTAL_REPIN_BYTES`] below the
/// recorded mark, so growth that stays inside this kilobyte is permanent until
/// the mark is re-pinned by hand. Re-pin it in the same change that spends part
/// of it, or the next author inherits headroom that is already gone.
const TOTAL_DESCRIPTION_SLACK_BYTES: usize = 1_024;

/// Total wire-description ceiling across every registered tool. The binding
/// constraint. A NEW capability is the one thing that may raise the recorded
/// mark by a whole description, and only by its own routing summary: the new
/// tool's description carries no per-flag detail (that goes straight into its
/// guide). Re-pinning after a change that spent slack is the other, smaller
/// reason the mark moves up.
const MAX_TOTAL_DESCRIPTION_BYTES: usize =
    RECORDED_TOTAL_DESCRIPTION_BYTES + TOTAL_DESCRIPTION_SLACK_BYTES;

/// How much unused headroom an exception may carry before the test asks for
/// the allowance to be lowered. Without this the list would keep stale numbers
/// and stop being a ratchet.
const MAX_EXCEPTION_SLACK_BYTES: usize = 128;

/// How far a live total may sit below its recorded mark before the gate asks
/// for a re-pin.
///
/// This is what keeps a budget a ratchet instead of a number that drifts: a
/// real reduction (one tool's prose moved into its guide) has to be banked, or
/// the bytes it freed become silent budget for the next description.
///
/// It is deliberately a small multiple of the per-tool ratchet
/// [`MAX_EXCEPTION_SLACK_BYTES`], not an order of magnitude above it: at
/// 4_096 bytes, thirty-two tools' worth of per-tool reclaim could be harvested
/// and spent without the gate ever asking, which is exactly the silent budget
/// the comment above claims to prevent. Four tools' worth is enough that
/// rewording never trips it, and small enough that a genuine harvest is banked
/// in the change that made it.
const TOTAL_REPIN_BYTES: usize = MAX_EXCEPTION_SLACK_BYTES * 4;

/// Tools allowed past [`MAX_TOOL_DESCRIPTION_BYTES`], each with the allowance
/// it may spend and the reason it earns one. Two kinds of entry live here.
///
/// Permanent: `code_execute` and `fix_apply`. For those two the prose IS the
/// safety boundary an agent reads before it acts, so a uniform 600- or
/// 800-byte ceiling is not defensible. `code_execute` states the sandbox
/// contract, the host-call allowlist, and the output and timeout bounds
/// before an agent runs JavaScript in this process; `fix_apply` states the
/// dry-run-first mutation contract, and it is the only tool that writes to
/// the project. `fix_apply`'s allowance last moved for a write fallow now
/// declines: a finding whose verdict rests on a source file the run did not
/// fully analyze. That is the one kind of growth this row exists to allow,
/// and it is not per-flag prose;
/// an agent that does not know a removal was withheld reads the run as a
/// clean no-op.
///
/// Temporary: every other row. Those descriptions still carry per-flag detail
/// that belongs in a `fallow://tools/{name}` guide; each is scheduled for the
/// same split `check_health` already had, and its allowance disappears with
/// that split.
const DESCRIPTION_BUDGET_EXCEPTIONS: &[(&str, usize)] = &[
    ("code_execute", 4_600),
    ("fix_apply", 4_200),
    ("check_health", 5_500),
    ("audit", 5_150),
    ("fix_preview", 2_750),
    ("analyze", 2_550),
    ("check_runtime_coverage", 2_350),
    ("impact", 2_250),
    ("security_candidates", 2_250),
    ("impact_all", 2_100),
];

fn description_allowance(tool: &str) -> usize {
    DESCRIPTION_BUDGET_EXCEPTIONS
        .iter()
        .find(|(name, _)| *name == tool)
        .map_or(MAX_TOOL_DESCRIPTION_BYTES, |(_, allowance)| *allowance)
}

#[test]
fn tool_descriptions_stay_within_their_byte_budget() {
    for (tool, description) in live_tool_descriptions() {
        let allowance = description_allowance(&tool);
        assert!(
            description.len() <= allowance,
            "{tool} wire description is {} bytes, over its {allowance}-byte budget; \
             move the per-flag detail into its fallow://tools/{tool} guide rather than \
             raising the number",
            description.len()
        );
    }
}

fn total_description_bytes() -> usize {
    live_tool_descriptions()
        .values()
        .map(std::string::String::len)
        .sum()
}

#[test]
fn total_tool_description_bytes_stay_within_budget() {
    let total = total_description_bytes();
    assert!(
        total <= MAX_TOTAL_DESCRIPTION_BYTES,
        "tools/list carries {total} description bytes, over the \
         {MAX_TOTAL_DESCRIPTION_BYTES}-byte budget every agent session pays on connect \
         ({RECORDED_TOTAL_DESCRIPTION_BYTES} recorded plus {TOTAL_DESCRIPTION_SLACK_BYTES} \
         slack); move per-flag detail into the tool's fallow://tools/{{name}} guide"
    );
}

#[test]
fn total_tool_description_budget_keeps_no_stale_headroom() {
    let total = total_description_bytes();
    assert!(
        RECORDED_TOTAL_DESCRIPTION_BYTES.saturating_sub(total) <= TOTAL_REPIN_BYTES,
        "tools/list is down to {total} description bytes but the ratchet still records \
         {RECORDED_TOTAL_DESCRIPTION_BYTES}; bank the win by setting \
         RECORDED_TOTAL_DESCRIPTION_BYTES to {total}, so the freed bytes are not \
         spendable by the next description"
    );
}

/// The smallest headroom that still lets a maintainer fix a word without the
/// total budget going red. One sentence rewritten is worth a couple of hundred
/// bytes; anything under that and the gate is a tripwire, not a budget.
const MIN_USABLE_TOTAL_HEADROOM_BYTES: usize = 256;

#[test]
fn total_description_budget_leaves_room_for_a_wording_fix() {
    let total = total_description_bytes();
    let headroom = MAX_TOTAL_DESCRIPTION_BYTES.saturating_sub(total);
    assert!(
        headroom >= MIN_USABLE_TOTAL_HEADROOM_BYTES,
        "the total description budget has {headroom} bytes of headroom; pinned this \
         tightly, a one-word wording fix fails the gate and reads as a break. Keep the \
         ceiling at RECORDED_TOTAL_DESCRIPTION_BYTES plus TOTAL_DESCRIPTION_SLACK_BYTES \
         rather than re-pinning it to the exact live total"
    );
}

#[test]
fn budget_exceptions_keep_no_stale_headroom() {
    let live = live_tool_descriptions();
    for (tool, allowance) in DESCRIPTION_BUDGET_EXCEPTIONS {
        let description = live
            .get(*tool)
            .unwrap_or_else(|| panic!("budget exception {tool} is not a registered tool"));
        assert!(
            allowance.saturating_sub(description.len()) <= MAX_EXCEPTION_SLACK_BYTES,
            "{tool} is {} bytes but its exception allows {allowance}; lower the allowance, \
             or drop the row when the description fits the {MAX_TOOL_DESCRIPTION_BYTES}-byte ceiling",
            description.len()
        );
    }
}

/// Serialized `tools/list` input-schema bytes, per tool.
///
/// The description budget above covers `tool.description` and nothing else,
/// which left the larger half of the payload ungoverned: a parameter with a
/// 500-byte doc comment cost 500 wire bytes and zero budget bytes, because
/// schemars renders a doc comment into the schema's `description`. Every
/// `tools/list` byte is resident in every agent session that connects, whether
/// or not the tool is ever called, so both channels are budgeted the same way.
fn live_tool_schema_bytes() -> BTreeMap<String, usize> {
    let server = FallowMcp::new();
    server
        .tool_router
        .list_all()
        .iter()
        .map(|tool| {
            (
                tool.name.to_string(),
                serde_json::to_string(&tool.input_schema)
                    .expect("input schema serializes")
                    .len(),
            )
        })
        .collect()
}

fn total_schema_bytes() -> usize {
    live_tool_schema_bytes().values().sum()
}

/// Total input-schema bytes measured the last time this gate was re-pinned.
/// The ratchet's high-water mark, not the assertion.
const RECORDED_TOTAL_SCHEMA_BYTES: usize = 78_897;

/// Deliberate headroom over [`RECORDED_TOTAL_SCHEMA_BYTES`], for the same
/// reason [`TOTAL_DESCRIPTION_SLACK_BYTES`] exists: pinned to the exact live
/// total, a clarified parameter sentence reads as a break rather than as a
/// budget. Schemas are shared across tools (one `workspace` sentence lands on
/// nearly every one of them), so a reworded shared parameter moves this total
/// by far more than one reworded description moves that one; the slack is
/// sized for a shared-parameter edit, not a single-tool one.
const TOTAL_SCHEMA_SLACK_BYTES: usize = 2_048;

/// Total input-schema ceiling across every registered tool.
const MAX_TOTAL_SCHEMA_BYTES: usize = RECORDED_TOTAL_SCHEMA_BYTES + TOTAL_SCHEMA_SLACK_BYTES;

#[test]
fn total_tool_schema_bytes_stay_within_budget() {
    let total = total_schema_bytes();
    assert!(
        total <= MAX_TOTAL_SCHEMA_BYTES,
        "tools/list carries {total} input-schema bytes, over the \
         {MAX_TOTAL_SCHEMA_BYTES}-byte budget every agent session pays on connect \
         ({RECORDED_TOTAL_SCHEMA_BYTES} recorded plus {TOTAL_SCHEMA_SLACK_BYTES} slack); \
         a parameter doc comment is wire text, so shorten it or drop the parameter"
    );
}

#[test]
fn total_tool_schema_budget_keeps_no_stale_headroom() {
    let total = total_schema_bytes();
    assert!(
        RECORDED_TOTAL_SCHEMA_BYTES.saturating_sub(total) <= TOTAL_REPIN_BYTES,
        "tools/list is down to {total} input-schema bytes but the ratchet still records \
         {RECORDED_TOTAL_SCHEMA_BYTES}; bank the win by setting RECORDED_TOTAL_SCHEMA_BYTES \
         to {total}, so the freed bytes are not spendable by the next parameter"
    );
}

/// The catalogue resource is the terse channel and the wire description is the
/// long one. `crates/mcp/src/tool_guides.rs` says a drift test holds that
/// ordering; this is that test.
#[test]
fn catalogue_lines_stay_shorter_than_the_wire_description() {
    let live = live_tool_descriptions();
    for tool in fallow_types::mcp_manifest::MCP_TOOLS {
        let wire = live
            .get(tool.name)
            .unwrap_or_else(|| panic!("{} is in the manifest but not registered", tool.name));
        assert!(
            tool.description.len() < wire.len(),
            "{}: the fallow://tools catalogue line is {} bytes and the tools/list description \
             is {}; the catalogue is the terse channel, so long prose belongs in the wire \
             description or in the tool's fallow://tools/{{name}} guide",
            tool.name,
            tool.description.len(),
            wire.len()
        );
    }
}
