/// Compute a deterministic fingerprint hash from key fields.
#[must_use]
pub fn fingerprint_hash(parts: &[&str]) -> String {
    fallow_output::codeclimate_fingerprint_hash(parts)
}

#[cfg(test)]
#[must_use]
pub fn finding_fingerprint(rule_id: &str, path: &str, snippet: &str, col: u32) -> String {
    fallow_output::sarif_finding_fingerprint(rule_id, path, snippet, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_for_whitespace_only_snippet_changes() {
        let a = finding_fingerprint(
            "fallow/unused-export",
            "src/a.ts",
            "  export const x = 1;  ",
            14,
        );
        let b = finding_fingerprint(
            "fallow/unused-export",
            "src/a.ts",
            "\nexport const x = 1;\n",
            14,
        );
        assert_eq!(a, b);
    }

    /// A one-line re-export barrel puts several findings of the same rule on the
    /// same line, so the snippet alone cannot tell them apart.
    #[test]
    fn two_findings_sharing_a_line_get_different_fingerprints() {
        let snippet = "export { alpha, beta } from './m';";
        assert_ne!(
            finding_fingerprint("fallow/unused-export", "src/barrel.ts", snippet, 10),
            finding_fingerprint("fallow/unused-export", "src/barrel.ts", snippet, 17)
        );
    }

    #[test]
    fn fingerprint_parts_are_separated() {
        assert_ne!(
            fingerprint_hash(&["ab", "c"]),
            fingerprint_hash(&["a", "bc"])
        );
    }
}
