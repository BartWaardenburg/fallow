use std::collections::BTreeMap;

use super::super::FallowMcp;

const DESCRIPTION_FIXTURE: &str = include_str!("fixtures/tool-descriptions.json");
const SERVER_SOURCE: &str = include_str!("../mod.rs");

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
const RECORDED_TOTAL_DESCRIPTION_BYTES: usize = 52_276;

/// Deliberate headroom over [`RECORDED_TOTAL_DESCRIPTION_BYTES`].
///
/// Pinned to the exact live total, the gate failed on a one-word wording fix,
/// which reads as a break rather than as a budget and teaches the next
/// maintainer to raise the number reflexively. A kilobyte absorbs ordinary
/// rewording (a clarified sentence, a corrected flag name) while still
/// catching what the budget exists for: prose that grows by a paragraph. It is
/// not a spending allowance, because the re-pin check below reclaims it.
const TOTAL_DESCRIPTION_SLACK_BYTES: usize = 1_024;

/// Total wire-description ceiling across every registered tool. The binding
/// constraint. A NEW capability is the one thing that may raise the recorded
/// mark, and only by its own routing summary: the new tool's description
/// carries no per-flag detail (that goes straight into its guide).
const MAX_TOTAL_DESCRIPTION_BYTES: usize =
    RECORDED_TOTAL_DESCRIPTION_BYTES + TOTAL_DESCRIPTION_SLACK_BYTES;

/// How far the live total may sit below the recorded mark before the gate asks
/// for a re-pin.
///
/// This is what keeps the budget a ratchet instead of a number that drifts: a
/// real reduction (one tool's prose moved into its guide) has to be banked, or
/// the bytes it freed become silent budget for the next description. The
/// tolerance is deliberately several times [`TOTAL_DESCRIPTION_SLACK_BYTES`],
/// so rewording never trips it and only a genuine harvest does.
const TOTAL_DESCRIPTION_REPIN_BYTES: usize = 4_096;

/// How much unused headroom an exception may carry before the test asks for
/// the allowance to be lowered. Without this the list would keep stale numbers
/// and stop being a ratchet.
const MAX_EXCEPTION_SLACK_BYTES: usize = 128;

/// Tools allowed past [`MAX_TOOL_DESCRIPTION_BYTES`], each with the allowance
/// it may spend and the reason it earns one. Two kinds of entry live here.
///
/// Permanent: `code_execute` and `fix_apply`. For those two the prose IS the
/// safety boundary an agent reads before it acts, so a uniform 600- or
/// 800-byte ceiling is not defensible. `code_execute` states the sandbox
/// contract, the host-call allowlist, and the output and timeout bounds
/// before an agent runs JavaScript in this process; `fix_apply` states the
/// dry-run-first mutation contract, and it is the only tool that writes to
/// the project.
///
/// Temporary: every other row. Those descriptions still carry per-flag detail
/// that belongs in a `fallow://tools/{name}` guide; each is scheduled for the
/// same split `check_health` already had, and its allowance disappears with
/// that split.
const DESCRIPTION_BUDGET_EXCEPTIONS: &[(&str, usize)] = &[
    ("code_execute", 4_600),
    ("fix_apply", 3_800),
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
        RECORDED_TOTAL_DESCRIPTION_BYTES.saturating_sub(total) <= TOTAL_DESCRIPTION_REPIN_BYTES,
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
