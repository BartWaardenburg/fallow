use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};

// Re-export all public types so existing `use ... tokenize::X` paths continue to work.
pub use super::token_types::{
    FileTokens, KeywordType, OperatorType, PunctuationType, SourceToken, TokenKind,
};
use super::token_visitor::TokenExtractor;

/// Tokenize a source file into a sequence of normalized tokens.
///
/// For Vue/Svelte SFC files, extracts `<script>` blocks first and tokenizes
/// their content, mirroring the main analysis pipeline's SFC handling.
/// For Astro files, extracts frontmatter. For MDX files, extracts import/export statements.
///
/// When `strip_types` is true, TypeScript type annotations, interfaces, and type
/// aliases are stripped from the token stream. This enables cross-language clone
/// detection between `.ts` and `.js` files.
pub fn tokenize_file(path: &Path, source: &str) -> FileTokens {
    tokenize_file_inner(path, source, false)
}

/// Tokenize a source file with optional type stripping for cross-language detection.
pub fn tokenize_file_cross_language(path: &Path, source: &str, strip_types: bool) -> FileTokens {
    tokenize_file_inner(path, source, strip_types)
}

fn tokenize_file_inner(path: &Path, source: &str, strip_types: bool) -> FileTokens {
    use crate::extract::{
        extract_astro_frontmatter, extract_mdx_statements, extract_sfc_scripts, is_sfc_file,
    };

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // For Vue/Svelte SFCs, extract and tokenize `<script>` blocks.
    if is_sfc_file(path) {
        let scripts = extract_sfc_scripts(source);
        let mut all_tokens = Vec::new();

        for script in &scripts {
            let source_type = match (script.is_typescript, script.is_jsx) {
                (true, true) => SourceType::tsx(),
                (true, false) => SourceType::ts(),
                (false, true) => SourceType::jsx(),
                (false, false) => SourceType::mjs(),
            };
            let allocator = Allocator::default();
            let parser_return = Parser::new(&allocator, &script.body, source_type).parse();

            let mut extractor = TokenExtractor::with_strip_types(strip_types);
            extractor.visit_program(&parser_return.program);

            // Adjust token spans to reference positions in the full SFC source
            // rather than the extracted script block.
            let offset = script.byte_offset as u32;
            for token in &mut extractor.tokens {
                token.span = Span::new(token.span.start + offset, token.span.end + offset);
            }
            all_tokens.extend(extractor.tokens);
        }

        let line_count = source.lines().count().max(1);
        return FileTokens {
            tokens: all_tokens,
            source: source.to_string(),
            line_count,
        };
    }

    // For Astro files, extract and tokenize frontmatter.
    if ext == "astro" {
        if let Some(script) = extract_astro_frontmatter(source) {
            let allocator = Allocator::default();
            let parser_return = Parser::new(&allocator, &script.body, SourceType::ts()).parse();

            let mut extractor = TokenExtractor::with_strip_types(strip_types);
            extractor.visit_program(&parser_return.program);

            let offset = script.byte_offset as u32;
            for token in &mut extractor.tokens {
                token.span = Span::new(token.span.start + offset, token.span.end + offset);
            }

            let line_count = source.lines().count().max(1);
            return FileTokens {
                tokens: extractor.tokens,
                source: source.to_string(),
                line_count,
            };
        }
        // No frontmatter — return empty tokens.
        let line_count = source.lines().count().max(1);
        return FileTokens {
            tokens: Vec::new(),
            source: source.to_string(),
            line_count,
        };
    }

    // For MDX files, extract and tokenize import/export statements.
    if ext == "mdx" {
        let statements = extract_mdx_statements(source);
        if !statements.is_empty() {
            let allocator = Allocator::default();
            let parser_return = Parser::new(&allocator, &statements, SourceType::jsx()).parse();

            let mut extractor = TokenExtractor::with_strip_types(strip_types);
            extractor.visit_program(&parser_return.program);

            let line_count = source.lines().count().max(1);
            return FileTokens {
                tokens: extractor.tokens,
                source: source.to_string(),
                line_count,
            };
        }
        let line_count = source.lines().count().max(1);
        return FileTokens {
            tokens: Vec::new(),
            source: source.to_string(),
            line_count,
        };
    }

    // CSS/SCSS files are not JS/TS — skip tokenization for duplication detection.
    if ext == "css" || ext == "scss" {
        let line_count = source.lines().count().max(1);
        return FileTokens {
            tokens: Vec::new(),
            source: source.to_string(),
            line_count,
        };
    }

    let source_type = SourceType::from_path(path).unwrap_or_default();
    let allocator = Allocator::default();
    let parser_return = Parser::new(&allocator, source, source_type).parse();

    let mut extractor = TokenExtractor::with_strip_types(strip_types);
    extractor.visit_program(&parser_return.program);

    // If parsing produced very few tokens relative to source size (likely parse errors
    // from Flow types or JSX in .js files), retry with JSX/TSX source type as a fallback.
    if extractor.tokens.len() < 5 && source.len() > 100 && !source_type.is_jsx() {
        let jsx_type = if source_type.is_typescript() {
            SourceType::tsx()
        } else {
            SourceType::jsx()
        };
        let allocator2 = Allocator::default();
        let retry_return = Parser::new(&allocator2, source, jsx_type).parse();
        let mut retry_extractor = TokenExtractor::with_strip_types(strip_types);
        retry_extractor.visit_program(&retry_return.program);
        if retry_extractor.tokens.len() > extractor.tokens.len() {
            extractor = retry_extractor;
        }
    }

    let line_count = source.lines().count().max(1);

    FileTokens {
        tokens: extractor.tokens,
        source: source.to_string(),
        line_count,
    }
}

#[cfg(test)]
mod tests;
