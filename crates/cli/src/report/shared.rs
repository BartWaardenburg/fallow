//! Cross-surface strings shared between multiple report formats.
//!
//! Strings rendered by more than one output path are aliased here from the
//! crate that owns the wire format, so they cannot drift between, for example,
//! the JSON `actions[].note` and the human `Duplicate exports` section. Wording
//! is deliberately neutral (no JSON action names, no MCP tool names, no SARIF
//! rule IDs) so it reads naturally in every consuming surface.

/// Namespace-barrel orientation hint shared between the JSON `remove-duplicate`
/// action note and the human `Duplicate exports` section.
///
/// Aliases [`fallow_types::output_dead_code::NAMESPACE_BARREL_HINT`], the
/// literal the JSON action note is built from, so the two surfaces cannot
/// carry different wording.
///
/// The JSON note fires unconditionally on every duplicate-export finding; the
/// human path emits the same text once per section, gated on a high match ratio
/// so it stays useful in shadcn / Radix-clone projects and quiet otherwise. AI
/// agents discover the `add-to-config` action via the position-0 entry in
/// `actions[]`, not via this note.
pub(in crate::report) const NAMESPACE_BARREL_HINT: &str =
    fallow_types::output_dead_code::NAMESPACE_BARREL_HINT;
