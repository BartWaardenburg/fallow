//! Nearest-name suggestion for refused selectors.
//!
//! `fallow_api::closest_match` answers the typo case: an edit distance of at
//! most two, which is what a slipped keystroke costs. It is deliberately silent
//! past that, so it cannot help the other common miss, a caller who spelled out
//! a name fallow abbreviates (`unused-dependencies` for `unused-deps`, seven
//! edits apart). This module scores that second case structurally: names here
//! are word lists, so it aligns the candidate's words against the caller's
//! rather than comparing the two strings as opaque byte runs.

/// One exactly shared word. The strongest signal a caller meant this name.
const WORD_WEIGHT: i64 = 8;

/// One word that is an abbreviation or extension of a caller word
/// (`deps`/`dependencies`, `helth`/`health`).
const PARTIAL_WORD_WEIGHT: i64 = 5;

/// Shortest leading run two words must share to count as the same word
/// abbreviated. Two is noise (`t` in `types` and `teleport`); three is a stem.
const MIN_PARTIAL_WORD_PREFIX: usize = 3;

/// Whole-token containment in either direction, which catches a caller who
/// named the distinctive half of a compound (`health` for `check_health`).
const SUBSTRING_WEIGHT: i64 = 4;

/// A candidate word the caller never named. A suggestion that introduces a
/// concept nobody asked about is a worse guess than one that does not, and
/// without this the longest name that happens to share a prefix wins:
/// `unused-dependencies` resolved to `unused-dependency-overrides` rather than
/// to `unused-deps`.
const EXTRA_WORD_PENALTY: i64 = 4;

/// The score a candidate needs before it is offered as a suggestion.
///
/// One exactly shared word, and nothing else, is the floor. Below it the only
/// remaining signals are accidental: two unrelated names share a leading letter
/// (`teleport` and `types`), and a wrong suggestion costs a caller more than no
/// suggestion.
pub const MIN_AFFINITY: usize = WORD_WEIGHT as usize;

/// How well `known` answers `token`, as a word-aligned score. Both sides split
/// on `-` and `_`, so the scorer works for snake_case tool names and
/// kebab-case selectors alike. Zero means "nothing in common worth saying".
#[must_use]
pub fn name_affinity(known: &str, token: &str) -> usize {
    let normalized = token.trim().to_ascii_lowercase();
    let token_words: Vec<&str> = split_name(&normalized).collect();

    let mut score = 0_i64;
    for word in split_name(known) {
        score += match best_word_match(word, &token_words) {
            WordMatch::Exact => WORD_WEIGHT,
            WordMatch::Partial => PARTIAL_WORD_WEIGHT,
            WordMatch::None => -EXTRA_WORD_PENALTY,
        };
    }
    if !normalized.is_empty() && (known.contains(&normalized) || normalized.contains(known)) {
        score += SUBSTRING_WEIGHT;
    }

    usize::try_from(score.max(0)).unwrap_or(0)
}

enum WordMatch {
    Exact,
    Partial,
    None,
}

fn best_word_match(word: &str, token_words: &[&str]) -> WordMatch {
    if token_words.contains(&word) {
        return WordMatch::Exact;
    }
    if token_words
        .iter()
        .any(|candidate| shared_prefix(word, candidate) >= MIN_PARTIAL_WORD_PREFIX)
    {
        return WordMatch::Partial;
    }
    WordMatch::None
}

fn shared_prefix(left: &str, right: &str) -> usize {
    left.bytes()
        .zip(right.bytes())
        .take_while(|(left, right)| left == right)
        .count()
}

fn split_name(name: &str) -> impl Iterator<Item = &str> {
    name.split(['-', '_']).filter(|word| !word.is_empty())
}

/// The candidates closest to `token`, best first, capped at `max` and filtered
/// by [`MIN_AFFINITY`]. Ties break on the candidate name so the list is stable.
///
/// When word alignment finds nothing, the typo matcher gets the last word: a
/// misspelling inside a word (`check_helth`) breaks the stem this scorer aligns
/// on, and edit distance is the right tool for it. The two are complementary,
/// so both callers get both without either duplicating the fallback.
pub fn nearest_names<'a, I>(token: &str, candidates: I, max: usize) -> Vec<&'a str>
where
    I: IntoIterator<Item = &'a str> + Clone,
{
    let mut scored: Vec<(usize, &'a str)> = candidates
        .clone()
        .into_iter()
        .filter_map(|candidate| {
            let score = name_affinity(candidate, token);
            (score >= MIN_AFFINITY).then_some((score, candidate))
        })
        .collect();
    if scored.is_empty() {
        return fallow_api::closest_match(token, candidates)
            .into_iter()
            .take(max)
            .collect();
    }
    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(right.1)));
    scored
        .into_iter()
        .take(max)
        .map(|(_, candidate)| candidate)
        .collect()
}

/// The single closest candidate, or `None` when nothing is close enough to say.
#[must_use]
pub fn nearest_name<'a, I>(token: &str, candidates: I) -> Option<&'a str>
where
    I: IntoIterator<Item = &'a str> + Clone,
{
    nearest_names(token, candidates, 1).into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The miss this module exists for: the long form of an abbreviated name is
    /// far outside edit distance two, so the typo matcher stays silent.
    #[test]
    fn a_long_form_selector_resolves_to_the_abbreviation() {
        let candidates = [
            "unused-files",
            "unused-exports",
            "unused-deps",
            "unused-dependency-overrides",
            "circular-deps",
        ];

        assert_eq!(
            nearest_name("unused-dependencies", candidates),
            Some("unused-deps"),
            "a longer name that merely shares a prefix must not outrank the abbreviation"
        );
        assert_eq!(
            fallow_api::closest_match("unused-dependencies", candidates),
            None,
            "the typo matcher is expected to stay silent here; that is why this scorer exists"
        );
    }

    #[test]
    fn a_novel_token_gets_no_suggestion() {
        let candidates = ["unused-files", "unused-exports", "types", "unused-deps"];

        assert_eq!(nearest_name("teleport", candidates), None);
    }

    /// A misspelled word inside an otherwise correct name breaks the stem word
    /// alignment needs, so the typo matcher has to answer it. The tool-guide
    /// resource relies on this.
    #[test]
    fn a_misspelled_word_still_resolves() {
        assert_eq!(name_affinity("check_health", "check_helth"), 4);
        assert_eq!(
            nearest_name("check_helth", ["check_health", "find_dupes"]),
            Some("check_health")
        );
    }

    /// Naming the distinctive half of a compound is a real request, not noise.
    #[test]
    fn a_partial_name_resolves_through_containment() {
        assert_eq!(
            nearest_name("health", ["check_health", "find_dupes"]),
            Some("check_health")
        );
    }

    #[test]
    fn suggestions_are_ranked_and_capped() {
        let candidates = ["unused-deps", "unused-exports", "unused-export-types"];

        assert_eq!(
            nearest_names("unused-export", candidates, 2),
            vec!["unused-exports", "unused-export-types"]
        );
    }
}
