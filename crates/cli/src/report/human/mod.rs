mod check;
mod cross_ref;
mod dupes;
mod health;
mod perf;
mod traces;

pub(super) use check::*;
pub(super) use cross_ref::*;
pub(super) use dupes::*;
pub(super) use health::*;
pub(super) use perf::*;
pub(super) use traces::*;

use std::path::Path;

use colored::Colorize;

use super::{Level, relative_path, split_dir_filename};

/// Maximum items shown per flat section (unused files, deps, etc.).
pub(super) const MAX_FLAT_ITEMS: usize = 10;

/// Format a path with dimmed directory and bold filename.
pub(super) fn format_path(path_str: &str) -> String {
    let (dir, filename) = split_dir_filename(path_str);
    format!("{}{}", dir.dimmed(), filename.bold())
}

/// Format a number with thousands separators (e.g., 5433 → "5,433").
pub(super) fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}

/// Build a colored section header with bullet, title, and count.
pub(super) fn build_section_header(title: &str, count: usize, level: Level) -> String {
    let label = format!("{title} ({count})");
    match level {
        Level::Warn => format!("{} {}", "\u{25cf}".yellow(), label.yellow().bold()),
        Level::Info => format!("{} {}", "\u{25cf}".cyan(), label.cyan().bold()),
        Level::Error => format!("{} {}", "\u{25cf}".red(), label.red().bold()),
    }
}

/// Section footer: description + docs URL (with anchor to specific section).
fn section_footer_text(title: &str) -> Option<(&'static str, &'static str)> {
    match title {
        "Unused files" => Some((
            "Files not imported or referenced by any entry point",
            "https://docs.fallow.tools/explanations/dead-code#unused-files",
        )),
        "Unused exports" => Some((
            "Exported symbols not imported by any reachable file",
            "https://docs.fallow.tools/explanations/dead-code#unused-exports",
        )),
        "Unused type exports" => Some((
            "Exported types/interfaces not imported by any reachable file",
            "https://docs.fallow.tools/explanations/dead-code#unused-types",
        )),
        "Unused dependencies" | "Unused devDependencies" | "Unused optionalDependencies" => Some((
            "Packages in package.json not imported anywhere in the project",
            "https://docs.fallow.tools/explanations/dead-code#unused-dependencies",
        )),
        "Unused enum members" => Some((
            "Enum variants never referenced outside their declaration",
            "https://docs.fallow.tools/explanations/dead-code#unused-enum-members",
        )),
        "Unused class members" => Some((
            "Class methods/properties never referenced outside their class",
            "https://docs.fallow.tools/explanations/dead-code#unused-class-members",
        )),
        "Unresolved imports" => Some((
            "Import specifiers that could not be resolved to a file",
            "https://docs.fallow.tools/explanations/dead-code#unresolved-imports",
        )),
        "Unlisted dependencies" => Some((
            "Packages imported in code but missing from package.json",
            "https://docs.fallow.tools/explanations/dead-code#unlisted-dependencies",
        )),
        "Duplicate exports" => Some((
            "Same export name defined in multiple files",
            "https://docs.fallow.tools/explanations/dead-code#duplicate-exports",
        )),
        "Circular dependencies" => Some((
            "Files that import each other, forming dependency cycles",
            "https://docs.fallow.tools/explanations/dead-code#circular-dependencies",
        )),
        t if t.starts_with("Type-only") => Some((
            "Dependencies only used for type imports, safe to move to devDependencies",
            "https://docs.fallow.tools/explanations/dead-code#type-only-dependencies",
        )),
        _ => None,
    }
}

/// Push a dimmed section footer line: description — docs_url
pub(super) fn push_section_footer(lines: &mut Vec<String>, title: &str) {
    if let Some((desc, url)) = section_footer_text(title) {
        lines.push(format!("  {}", format!("{desc} \u{2014} {url}").dimmed()));
    }
}

/// Build items grouped by file path, sorted by count descending, with truncation.
pub(super) fn build_grouped_by_file<'a, T>(
    lines: &mut Vec<String>,
    items: &'a [T],
    root: &Path,
    get_path: impl Fn(&'a T) -> &'a Path,
    format_detail: &impl Fn(&T) -> String,
    max_files: usize,
    max_items_per_file: usize,
) {
    // Group items by file path, preserving indices
    let mut file_groups: Vec<(String, Vec<usize>)> = Vec::new();
    let mut file_map: rustc_hash::FxHashMap<String, usize> = rustc_hash::FxHashMap::default();

    for (i, item) in items.iter().enumerate() {
        let file_str = relative_path(get_path(item), root).display().to_string();
        if let Some(&group_idx) = file_map.get(&file_str) {
            file_groups[group_idx].1.push(i);
        } else {
            file_map.insert(file_str.clone(), file_groups.len());
            file_groups.push((file_str, vec![i]));
        }
    }

    // Sort files by item count descending, alphabetical tiebreaker
    file_groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));

    let total_files = file_groups.len();
    let shown_files = total_files.min(max_files);

    for (file_str, indices) in &file_groups[..shown_files] {
        let count_tag = if indices.len() > 1 {
            format!(" ({})", indices.len()).dimmed().to_string()
        } else {
            String::new()
        };
        lines.push(format!("  {}{}", format_path(file_str), count_tag));

        let shown_items = indices.len().min(max_items_per_file);
        for &i in &indices[..shown_items] {
            lines.push(format!("    {}", format_detail(&items[i])));
        }
        if indices.len() > max_items_per_file {
            lines.push(format!(
                "    {}",
                format!("... and {} more", indices.len() - max_items_per_file).dimmed()
            ));
        }
    }

    if total_files > max_files {
        let hidden_files = total_files - max_files;
        let hidden_items: usize = file_groups[max_files..]
            .iter()
            .map(|(_, indices)| indices.len())
            .sum();
        lines.push(format!(
            "  {}",
            format!(
                "... and {} more in {} file{}",
                hidden_items,
                hidden_files,
                if hidden_files == 1 { "" } else { "s" }
            )
            .dimmed()
        ));
    }
}

/// Strip ANSI escape sequences from a string, leaving only the printable text.
#[cfg(test)]
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until 'm' (end of SGR sequence)
            for inner in chars.by_ref() {
                if inner == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Utility function tests ──

    #[test]
    fn thousands_zero() {
        assert_eq!(thousands(0), "0");
    }

    #[test]
    fn thousands_small() {
        assert_eq!(thousands(999), "999");
    }

    #[test]
    fn thousands_boundary() {
        assert_eq!(thousands(1000), "1,000");
    }

    #[test]
    fn thousands_large() {
        assert_eq!(thousands(1_000_000), "1,000,000");
    }

    #[test]
    fn thousands_irregular() {
        assert_eq!(thousands(12345), "12,345");
    }

    #[test]
    fn format_path_with_directory() {
        let result = strip_ansi(&format_path("src/components/Button.tsx"));
        assert!(result.ends_with("Button.tsx"));
        assert!(result.contains("src/components/"));
    }

    #[test]
    fn format_path_no_directory() {
        let result = strip_ansi(&format_path("index.ts"));
        assert_eq!(result, "index.ts");
    }

    // ── strip_ansi utility ──

    #[test]
    fn strip_ansi_removes_color_codes() {
        let colored_str = "hello".red().bold().to_string();
        assert_eq!(strip_ansi(&colored_str), "hello");
    }

    #[test]
    fn strip_ansi_preserves_plain_text() {
        assert_eq!(strip_ansi("plain text"), "plain text");
    }

    #[test]
    fn strip_ansi_handles_empty_string() {
        assert_eq!(strip_ansi(""), "");
    }

    // ── Section header tests ──

    #[test]
    fn section_header_uses_bullet_indicator() {
        let header = build_section_header("Test section", 3, Level::Error);
        let text = strip_ansi(&header);
        assert!(text.contains("\u{25cf}"));
        assert!(text.contains("Test section (3)"));
    }

    #[test]
    fn section_header_formats_for_all_levels() {
        for level in [Level::Error, Level::Warn, Level::Info] {
            let header = build_section_header("Items", 7, level);
            let text = strip_ansi(&header);
            assert!(
                text.contains("Items (7)"),
                "Missing title for level {level:?}"
            );
        }
    }
}
