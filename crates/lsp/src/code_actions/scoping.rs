/// Find a suitable insert position at module scope above the first instance.
///
/// Walks backwards from `first_start_0based` looking for an empty line or a
/// line that starts at column 0 (module scope), to avoid inserting inside a
/// function body.
pub(super) fn find_insert_line(first_start_0based: u32, file_lines: &[&str]) -> u32 {
    let mut line = first_start_0based;
    while line > 0 {
        line -= 1;
        let content = file_lines.get(line as usize).copied().unwrap_or("");
        // An empty line or a line starting at column 0 (module scope) is a good insert point
        if content.is_empty() || (!content.starts_with(' ') && !content.starts_with('\t')) {
            break;
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_insert_line_stops_at_empty_line() {
        let lines = vec!["function a() {", "  return 1;", "}", "", "  const x = 1;"];
        // Searching backwards from line 4, should stop at empty line 3
        assert_eq!(find_insert_line(4, &lines), 3);
    }

    #[test]
    fn find_insert_line_stops_at_module_scope_line() {
        let lines = vec!["const a = 1;", "  indented code", "  more indented"];
        // Searching backwards from line 2, hits "  indented code" (starts with space),
        // then hits "const a = 1;" (no leading space/tab) at line 0
        assert_eq!(find_insert_line(2, &lines), 0);
    }

    #[test]
    fn find_insert_line_returns_0_when_all_indented() {
        let lines = vec!["  a", "  b", "  c"];
        // Searching backwards from line 2: line 1 is indented, line 0 is indented
        // Loop goes: line=1 (indented, continue), line=0 (indented, continue),
        // loop ends (line > 0 is false). Returns 0.
        assert_eq!(find_insert_line(2, &lines), 0);
    }

    #[test]
    fn find_insert_line_at_line_0_returns_0() {
        let lines = vec!["  something"];
        // first_start_0based is 0, while loop condition is line > 0, so loop never runs
        assert_eq!(find_insert_line(0, &lines), 0);
    }

    #[test]
    fn find_insert_line_stops_at_line_starting_with_text() {
        let lines = vec![
            "import { x } from 'y';",
            "export function foo() {",
            "  return x;",
            "}",
            "  // indented comment",
            "  const z = 1;",
        ];
        // From line 5, walk back: line 4 (indented), line 3 ("}" at col 0) => stop
        assert_eq!(find_insert_line(5, &lines), 3);
    }
}
