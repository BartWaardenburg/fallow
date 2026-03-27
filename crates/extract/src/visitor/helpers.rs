//! Standalone helper functions for AST extraction.
//!
//! These functions don't require visitor state and operate purely on AST nodes.

use oxc_ast::ast::*;

use crate::{MemberInfo, MemberKind};

/// Extract class members (methods and properties) from a class declaration.
pub fn extract_class_members(class: &Class<'_>) -> Vec<MemberInfo> {
    let mut members = Vec::new();
    for element in &class.body.body {
        match element {
            ClassElement::MethodDefinition(method) => {
                if let Some(name) = method.key.static_name() {
                    let name_str = name.to_string();
                    // Skip constructor, private, and protected methods
                    if name_str != "constructor"
                        && !matches!(
                            method.accessibility,
                            Some(
                                oxc_ast::ast::TSAccessibility::Private
                                    | oxc_ast::ast::TSAccessibility::Protected
                            )
                        )
                    {
                        members.push(MemberInfo {
                            name: name_str,
                            kind: MemberKind::ClassMethod,
                            span: method.span,
                            has_decorator: !method.decorators.is_empty(),
                        });
                    }
                }
            }
            ClassElement::PropertyDefinition(prop) => {
                if let Some(name) = prop.key.static_name()
                    && !matches!(
                        prop.accessibility,
                        Some(
                            oxc_ast::ast::TSAccessibility::Private
                                | oxc_ast::ast::TSAccessibility::Protected
                        )
                    )
                {
                    members.push(MemberInfo {
                        name: name.to_string(),
                        kind: MemberKind::ClassProperty,
                        span: prop.span,
                        has_decorator: !prop.decorators.is_empty(),
                    });
                }
            }
            _ => {}
        }
    }
    members
}

/// Check if an argument expression is `import.meta.url`.
pub(super) fn is_meta_url_arg(arg: &Argument<'_>) -> bool {
    if let Argument::StaticMemberExpression(member) = arg
        && member.property.name == "url"
        && matches!(member.object, Expression::MetaProperty(_))
    {
        return true;
    }
    false
}

/// Extract static prefix and optional suffix from a binary addition chain.
pub(super) fn extract_concat_parts(
    expr: &BinaryExpression<'_>,
) -> Option<(String, Option<String>)> {
    let prefix = extract_leading_string(&expr.left)?;
    let suffix = extract_trailing_string(&expr.right);
    Some((prefix, suffix))
}

fn extract_leading_string(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::StringLiteral(lit) => Some(lit.value.to_string()),
        Expression::BinaryExpression(bin)
            if bin.operator == oxc_ast::ast::BinaryOperator::Addition =>
        {
            extract_leading_string(&bin.left)
        }
        _ => None,
    }
}

fn extract_trailing_string(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::StringLiteral(lit) => {
            let s = lit.value.to_string();
            if s.is_empty() { None } else { Some(s) }
        }
        _ => None,
    }
}

/// Convert a simple regex extension filter pattern to a glob suffix.
///
/// Handles common `require.context()` patterns like:
/// - `\.vue$` → `".vue"`
/// - `\.tsx?$` → uses `".ts"` / `".tsx"` via glob `".{ts,tsx}"`
/// - `\.(js|ts)$` → `".{js,ts}"`
/// - `\.(js|jsx|ts|tsx)$` → `".{js,jsx,ts,tsx}"`
///
/// Returns `None` for patterns that are too complex to convert.
pub(super) fn regex_pattern_to_suffix(pattern: &str) -> Option<String> {
    // Strip leading `^` or `.*` anchors (they don't affect extension matching)
    let p = pattern.strip_prefix('^').unwrap_or(pattern);
    let p = p.strip_prefix(".*").unwrap_or(p);

    // Must start with `\.` (escaped dot for extension)
    let p = p.strip_prefix("\\.")?;

    // Must end with `$`
    let p = p.strip_suffix('$')?;

    // Pattern: `ext?` — e.g., `tsx?` → {ts,tsx}
    if let Some(base) = p.strip_suffix('?') {
        // base must be simple alphanumeric (e.g., "tsx" from "tsx?")
        if base.chars().all(|c| c.is_ascii_alphanumeric()) && !base.is_empty() {
            let without_last = &base[..base.len() - 1];
            if without_last.is_empty() {
                // Single char like `x?` → matches "" or "x", too ambiguous
                return None;
            }
            return Some(format!(".{{{without_last},{base}}}"));
        }
        return None;
    }

    // Pattern: `(ext1|ext2|...)` — e.g., `(js|ts)` → {js,ts}
    if let Some(inner) = p.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        let exts: Vec<&str> = inner.split('|').collect();
        if exts
            .iter()
            .all(|e| e.chars().all(|c| c.is_ascii_alphanumeric()) && !e.is_empty())
        {
            return Some(format!(".{{{}}}", exts.join(",")));
        }
        return None;
    }

    // Pattern: simple extension like `vue`, `json`, `css`
    if p.chars().all(|c| c.is_ascii_alphanumeric()) && !p.is_empty() {
        return Some(format!(".{p}"));
    }

    None
}

/// Check if a name is a well-known JavaScript/DOM built-in constructor.
///
/// Used to avoid creating spurious instance bindings for `new URL()`, `new Map()`,
/// etc. These are never user-exported classes and would only create noise in the
/// member access tracking pipeline.
pub(super) fn is_builtin_constructor(name: &str) -> bool {
    matches!(
        name,
        "Array"
            | "ArrayBuffer"
            | "Blob"
            | "Boolean"
            | "DataView"
            | "Date"
            | "Error"
            | "EvalError"
            | "Event"
            | "Float32Array"
            | "Float64Array"
            | "FormData"
            | "Headers"
            | "Int8Array"
            | "Int16Array"
            | "Int32Array"
            | "Map"
            | "Number"
            | "Object"
            | "Promise"
            | "Proxy"
            | "RangeError"
            | "ReferenceError"
            | "RegExp"
            | "Request"
            | "Response"
            | "Set"
            | "SharedArrayBuffer"
            | "String"
            | "SyntaxError"
            | "TypeError"
            | "URIError"
            | "URL"
            | "URLSearchParams"
            | "Uint8Array"
            | "Uint8ClampedArray"
            | "Uint16Array"
            | "Uint32Array"
            | "WeakMap"
            | "WeakRef"
            | "WeakSet"
            | "Worker"
            | "AbortController"
            | "ReadableStream"
            | "WritableStream"
            | "TransformStream"
            | "TextEncoder"
            | "TextDecoder"
            | "MutationObserver"
            | "IntersectionObserver"
            | "ResizeObserver"
            | "PerformanceObserver"
            | "MessageChannel"
            | "BroadcastChannel"
            | "WebSocket"
            | "XMLHttpRequest"
            | "EventEmitter"
            | "Buffer"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── regex_pattern_to_suffix ──────────────────────────────

    #[test]
    fn regex_suffix_with_caret_anchor() {
        // Leading `^` should be stripped — result matches bare pattern
        assert_eq!(
            regex_pattern_to_suffix(r"^\.vue$"),
            Some(".vue".to_string())
        );
        assert_eq!(
            regex_pattern_to_suffix(r"^\.json$"),
            Some(".json".to_string())
        );
    }

    #[test]
    fn regex_suffix_with_dotstar_anchor() {
        // Leading `.*` should be stripped
        assert_eq!(
            regex_pattern_to_suffix(r".*\.css$"),
            Some(".css".to_string())
        );
    }

    #[test]
    fn regex_suffix_with_both_anchors() {
        // Both `^` and `.*` as prefix
        assert_eq!(
            regex_pattern_to_suffix(r"^.*\.ts$"),
            Some(".ts".to_string())
        );
    }

    #[test]
    fn regex_suffix_single_char_optional_returns_none() {
        // `\.x?$` — single char base "x" minus last char = "" which is too ambiguous
        assert_eq!(regex_pattern_to_suffix(r"\.x?$"), None);
    }

    #[test]
    fn regex_suffix_two_char_optional() {
        // `\.ts?$` — base "ts" minus last = "t", result: .{t,ts}
        assert_eq!(
            regex_pattern_to_suffix(r"\.ts?$"),
            Some(".{t,ts}".to_string())
        );
    }

    #[test]
    fn regex_suffix_no_dollar_sign_returns_none() {
        // Missing trailing `$` should return None
        assert_eq!(regex_pattern_to_suffix(r"\.vue"), None);
    }

    #[test]
    fn regex_suffix_no_escaped_dot_returns_none() {
        // Missing `\.` prefix should return None
        assert_eq!(regex_pattern_to_suffix(r"vue$"), None);
    }

    #[test]
    fn regex_suffix_empty_alternation_returns_none() {
        // Empty group `()` should return None (no extensions)
        assert_eq!(regex_pattern_to_suffix(r"\.()$"), None);
    }

    #[test]
    fn regex_suffix_alternation_with_special_chars_returns_none() {
        // Special characters in alternation group
        assert_eq!(regex_pattern_to_suffix(r"\.(j.s|ts)$"), None);
    }

    #[test]
    fn regex_suffix_complex_wildcard_returns_none() {
        assert_eq!(regex_pattern_to_suffix(r"\..+$"), None);
        assert_eq!(regex_pattern_to_suffix(r"\.[a-z]+$"), None);
    }

    // ── is_builtin_constructor ───────────────────────────────

    #[test]
    fn builtin_constructors_recognized() {
        assert!(is_builtin_constructor("Array"));
        assert!(is_builtin_constructor("Map"));
        assert!(is_builtin_constructor("Set"));
        assert!(is_builtin_constructor("WeakMap"));
        assert!(is_builtin_constructor("WeakSet"));
        assert!(is_builtin_constructor("Promise"));
        assert!(is_builtin_constructor("URL"));
        assert!(is_builtin_constructor("URLSearchParams"));
        assert!(is_builtin_constructor("RegExp"));
        assert!(is_builtin_constructor("Date"));
        assert!(is_builtin_constructor("Error"));
        assert!(is_builtin_constructor("TypeError"));
        assert!(is_builtin_constructor("Request"));
        assert!(is_builtin_constructor("Response"));
        assert!(is_builtin_constructor("Headers"));
        assert!(is_builtin_constructor("FormData"));
        assert!(is_builtin_constructor("Blob"));
        assert!(is_builtin_constructor("AbortController"));
        assert!(is_builtin_constructor("ReadableStream"));
        assert!(is_builtin_constructor("WritableStream"));
        assert!(is_builtin_constructor("TransformStream"));
        assert!(is_builtin_constructor("TextEncoder"));
        assert!(is_builtin_constructor("TextDecoder"));
        assert!(is_builtin_constructor("Worker"));
        assert!(is_builtin_constructor("WebSocket"));
        assert!(is_builtin_constructor("EventEmitter"));
        assert!(is_builtin_constructor("Buffer"));
        assert!(is_builtin_constructor("MutationObserver"));
        assert!(is_builtin_constructor("IntersectionObserver"));
        assert!(is_builtin_constructor("ResizeObserver"));
        assert!(is_builtin_constructor("MessageChannel"));
        assert!(is_builtin_constructor("BroadcastChannel"));
    }

    #[test]
    fn user_defined_classes_not_builtin() {
        assert!(!is_builtin_constructor("MyService"));
        assert!(!is_builtin_constructor("UserRepository"));
        assert!(!is_builtin_constructor("AppController"));
        assert!(!is_builtin_constructor("DatabaseConnection"));
        assert!(!is_builtin_constructor("Logger"));
        assert!(!is_builtin_constructor("Config"));
        assert!(!is_builtin_constructor(""));
    }

    #[test]
    fn builtin_names_are_case_sensitive() {
        assert!(!is_builtin_constructor("array"));
        assert!(!is_builtin_constructor("map"));
        assert!(!is_builtin_constructor("url"));
        assert!(!is_builtin_constructor("MAP"));
        assert!(!is_builtin_constructor("ARRAY"));
    }
}
