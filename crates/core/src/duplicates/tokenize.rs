use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_ast_visit::walk;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use oxc_syntax::scope::ScopeFlags;

/// A single token extracted from the AST with its source location.
#[derive(Debug, Clone)]
pub struct SourceToken {
    /// The kind of token.
    pub kind: TokenKind,
    /// Byte offset into the source file.
    pub span: Span,
}

/// Normalized token types for clone detection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Keywords
    Keyword(KeywordType),
    // Identifiers -- value is the actual name (blinded in semantic mode)
    Identifier(String),
    // Literals
    StringLiteral(String),
    NumericLiteral(String),
    BooleanLiteral(bool),
    NullLiteral,
    TemplateLiteral,
    RegExpLiteral,
    // Operators
    Operator(OperatorType),
    // Punctuation / delimiters
    Punctuation(PunctuationType),
}

/// JavaScript/TypeScript keyword types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeywordType {
    Var,
    Let,
    Const,
    Function,
    Return,
    If,
    Else,
    For,
    While,
    Do,
    Switch,
    Case,
    Break,
    Continue,
    Default,
    Throw,
    Try,
    Catch,
    Finally,
    New,
    Delete,
    Typeof,
    Instanceof,
    In,
    Of,
    Void,
    This,
    Super,
    Class,
    Extends,
    Import,
    Export,
    From,
    As,
    Async,
    Await,
    Yield,
    Static,
    Get,
    Set,
    Type,
    Interface,
    Enum,
    Implements,
    Abstract,
    Declare,
    Readonly,
    Keyof,
    Satisfies,
}

/// Operator categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatorType {
    Assign,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Exp,
    Eq,
    NEq,
    StrictEq,
    StrictNEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    Not,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    BitwiseNot,
    ShiftLeft,
    ShiftRight,
    UnsignedShiftRight,
    NullishCoalescing,
    OptionalChaining,
    Spread,
    Ternary,
    Arrow,
    Comma,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    ExpAssign,
    AndAssign,
    OrAssign,
    NullishAssign,
    BitwiseAndAssign,
    BitwiseOrAssign,
    BitwiseXorAssign,
    ShiftLeftAssign,
    ShiftRightAssign,
    UnsignedShiftRightAssign,
    Increment,
    Decrement,
    Instanceof,
    In,
}

/// Punctuation / delimiter types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PunctuationType {
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Semicolon,
    Colon,
    Dot,
}

/// Result of tokenizing a source file.
#[derive(Debug, Clone)]
pub struct FileTokens {
    /// The extracted token sequence.
    pub tokens: Vec<SourceToken>,
    /// Source text (needed for extracting fragments).
    pub source: String,
    /// Total number of lines in the source.
    pub line_count: usize,
}

/// Create a 1-byte span at the given byte position.
///
/// Used for synthetic punctuation tokens (`(`, `)`, `,`, `.`) that don't
/// have their own AST span. Using the parent expression's full span would
/// inflate clone line ranges, especially in chained method calls.
const fn point_span(pos: u32) -> Span {
    Span::new(pos, pos + 1)
}

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

/// AST visitor that extracts a flat sequence of normalized tokens.
struct TokenExtractor {
    tokens: Vec<SourceToken>,
    /// When true, skip TypeScript type annotations, interfaces, and type aliases
    /// to enable cross-language clone detection between .ts and .js files.
    strip_types: bool,
}

impl TokenExtractor {
    const fn with_strip_types(strip_types: bool) -> Self {
        Self {
            tokens: Vec::new(),
            strip_types,
        }
    }

    fn push(&mut self, kind: TokenKind, span: Span) {
        self.tokens.push(SourceToken { kind, span });
    }

    fn push_keyword(&mut self, kw: KeywordType, span: Span) {
        self.push(TokenKind::Keyword(kw), span);
    }

    fn push_op(&mut self, op: OperatorType, span: Span) {
        self.push(TokenKind::Operator(op), span);
    }

    fn push_punc(&mut self, p: PunctuationType, span: Span) {
        self.push(TokenKind::Punctuation(p), span);
    }
}

impl<'a> Visit<'a> for TokenExtractor {
    // ── Statements ──────────────────────────────────────────

    fn visit_variable_declaration(&mut self, decl: &VariableDeclaration<'a>) {
        let kw = match decl.kind {
            VariableDeclarationKind::Var => KeywordType::Var,
            VariableDeclarationKind::Let => KeywordType::Let,
            VariableDeclarationKind::Const => KeywordType::Const,
            VariableDeclarationKind::Using | VariableDeclarationKind::AwaitUsing => {
                KeywordType::Const
            }
        };
        self.push_keyword(kw, decl.span);
        walk::walk_variable_declaration(self, decl);
    }

    fn visit_return_statement(&mut self, stmt: &ReturnStatement<'a>) {
        self.push_keyword(KeywordType::Return, stmt.span);
        walk::walk_return_statement(self, stmt);
    }

    fn visit_if_statement(&mut self, stmt: &IfStatement<'a>) {
        self.push_keyword(KeywordType::If, stmt.span);
        self.push_punc(PunctuationType::OpenParen, stmt.span);
        self.visit_expression(&stmt.test);
        self.push_punc(PunctuationType::CloseParen, stmt.span);
        self.visit_statement(&stmt.consequent);
        if let Some(alt) = &stmt.alternate {
            self.push_keyword(KeywordType::Else, stmt.span);
            self.visit_statement(alt);
        }
    }

    fn visit_for_statement(&mut self, stmt: &ForStatement<'a>) {
        self.push_keyword(KeywordType::For, stmt.span);
        self.push_punc(PunctuationType::OpenParen, stmt.span);
        walk::walk_for_statement(self, stmt);
        self.push_punc(PunctuationType::CloseParen, stmt.span);
    }

    fn visit_for_in_statement(&mut self, stmt: &ForInStatement<'a>) {
        self.push_keyword(KeywordType::For, stmt.span);
        self.push_punc(PunctuationType::OpenParen, stmt.span);
        self.visit_for_statement_left(&stmt.left);
        self.push_keyword(KeywordType::In, stmt.span);
        self.visit_expression(&stmt.right);
        self.push_punc(PunctuationType::CloseParen, stmt.span);
        self.visit_statement(&stmt.body);
    }

    fn visit_for_of_statement(&mut self, stmt: &ForOfStatement<'a>) {
        self.push_keyword(KeywordType::For, stmt.span);
        self.push_punc(PunctuationType::OpenParen, stmt.span);
        self.visit_for_statement_left(&stmt.left);
        self.push_keyword(KeywordType::Of, stmt.span);
        self.visit_expression(&stmt.right);
        self.push_punc(PunctuationType::CloseParen, stmt.span);
        self.visit_statement(&stmt.body);
    }

    fn visit_while_statement(&mut self, stmt: &WhileStatement<'a>) {
        self.push_keyword(KeywordType::While, stmt.span);
        self.push_punc(PunctuationType::OpenParen, stmt.span);
        walk::walk_while_statement(self, stmt);
        self.push_punc(PunctuationType::CloseParen, stmt.span);
    }

    fn visit_do_while_statement(&mut self, stmt: &DoWhileStatement<'a>) {
        self.push_keyword(KeywordType::Do, stmt.span);
        walk::walk_do_while_statement(self, stmt);
    }

    fn visit_switch_statement(&mut self, stmt: &SwitchStatement<'a>) {
        self.push_keyword(KeywordType::Switch, stmt.span);
        self.push_punc(PunctuationType::OpenParen, stmt.span);
        walk::walk_switch_statement(self, stmt);
        self.push_punc(PunctuationType::CloseParen, stmt.span);
    }

    fn visit_switch_case(&mut self, case: &SwitchCase<'a>) {
        if case.test.is_some() {
            self.push_keyword(KeywordType::Case, case.span);
        } else {
            self.push_keyword(KeywordType::Default, case.span);
        }
        self.push_punc(PunctuationType::Colon, case.span);
        walk::walk_switch_case(self, case);
    }

    fn visit_break_statement(&mut self, stmt: &BreakStatement<'a>) {
        self.push_keyword(KeywordType::Break, stmt.span);
    }

    fn visit_continue_statement(&mut self, stmt: &ContinueStatement<'a>) {
        self.push_keyword(KeywordType::Continue, stmt.span);
    }

    fn visit_throw_statement(&mut self, stmt: &ThrowStatement<'a>) {
        self.push_keyword(KeywordType::Throw, stmt.span);
        walk::walk_throw_statement(self, stmt);
    }

    fn visit_try_statement(&mut self, stmt: &TryStatement<'a>) {
        self.push_keyword(KeywordType::Try, stmt.span);
        walk::walk_try_statement(self, stmt);
    }

    fn visit_catch_clause(&mut self, clause: &CatchClause<'a>) {
        self.push_keyword(KeywordType::Catch, clause.span);
        walk::walk_catch_clause(self, clause);
    }

    fn visit_block_statement(&mut self, block: &BlockStatement<'a>) {
        self.push_punc(PunctuationType::OpenBrace, block.span);
        walk::walk_block_statement(self, block);
        self.push_punc(PunctuationType::CloseBrace, block.span);
    }

    // ── Expressions ─────────────────────────────────────────

    fn visit_identifier_reference(&mut self, ident: &IdentifierReference<'a>) {
        self.push(TokenKind::Identifier(ident.name.to_string()), ident.span);
    }

    fn visit_binding_identifier(&mut self, ident: &BindingIdentifier<'a>) {
        self.push(TokenKind::Identifier(ident.name.to_string()), ident.span);
    }

    fn visit_string_literal(&mut self, lit: &StringLiteral<'a>) {
        self.push(TokenKind::StringLiteral(lit.value.to_string()), lit.span);
    }

    fn visit_numeric_literal(&mut self, lit: &NumericLiteral<'a>) {
        let raw_str = lit
            .raw
            .as_ref()
            .map_or_else(|| lit.value.to_string(), |r| r.to_string());
        self.push(TokenKind::NumericLiteral(raw_str), lit.span);
    }

    fn visit_boolean_literal(&mut self, lit: &BooleanLiteral) {
        self.push(TokenKind::BooleanLiteral(lit.value), lit.span);
    }

    fn visit_null_literal(&mut self, lit: &NullLiteral) {
        self.push(TokenKind::NullLiteral, lit.span);
    }

    fn visit_template_literal(&mut self, lit: &TemplateLiteral<'a>) {
        self.push(TokenKind::TemplateLiteral, lit.span);
        walk::walk_template_literal(self, lit);
    }

    fn visit_reg_exp_literal(&mut self, lit: &RegExpLiteral<'a>) {
        self.push(TokenKind::RegExpLiteral, lit.span);
    }

    fn visit_this_expression(&mut self, expr: &ThisExpression) {
        self.push_keyword(KeywordType::This, expr.span);
    }

    fn visit_super(&mut self, expr: &Super) {
        self.push_keyword(KeywordType::Super, expr.span);
    }

    fn visit_array_expression(&mut self, expr: &ArrayExpression<'a>) {
        self.push_punc(PunctuationType::OpenBracket, expr.span);
        walk::walk_array_expression(self, expr);
        self.push_punc(PunctuationType::CloseBracket, expr.span);
    }

    fn visit_object_expression(&mut self, expr: &ObjectExpression<'a>) {
        self.push_punc(PunctuationType::OpenBrace, expr.span);
        walk::walk_object_expression(self, expr);
        self.push_punc(PunctuationType::CloseBrace, expr.span);
    }

    fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
        self.visit_expression(&expr.callee);
        // Use point spans for synthetic punctuation to avoid inflating clone
        // ranges when call expressions are chained (expr.span covers the
        // entire chain, not just this call's parentheses).
        let open = point_span(expr.callee.span().end);
        self.push_punc(PunctuationType::OpenParen, open);
        for arg in &expr.arguments {
            self.visit_argument(arg);
            let comma = point_span(arg.span().end);
            self.push_op(OperatorType::Comma, comma);
        }
        let close = point_span(expr.span.end.saturating_sub(1));
        self.push_punc(PunctuationType::CloseParen, close);
    }

    fn visit_new_expression(&mut self, expr: &NewExpression<'a>) {
        self.push_keyword(KeywordType::New, expr.span);
        self.visit_expression(&expr.callee);
        let open = point_span(expr.callee.span().end);
        self.push_punc(PunctuationType::OpenParen, open);
        for arg in &expr.arguments {
            self.visit_argument(arg);
            let comma = point_span(arg.span().end);
            self.push_op(OperatorType::Comma, comma);
        }
        let close = point_span(expr.span.end.saturating_sub(1));
        self.push_punc(PunctuationType::CloseParen, close);
    }

    fn visit_static_member_expression(&mut self, expr: &StaticMemberExpression<'a>) {
        self.visit_expression(&expr.object);
        // Use point span at the dot position (right after the object).
        let dot = point_span(expr.object.span().end);
        self.push_punc(PunctuationType::Dot, dot);
        self.push(
            TokenKind::Identifier(expr.property.name.to_string()),
            expr.property.span,
        );
    }

    fn visit_computed_member_expression(&mut self, expr: &ComputedMemberExpression<'a>) {
        self.visit_expression(&expr.object);
        let open = point_span(expr.object.span().end);
        self.push_punc(PunctuationType::OpenBracket, open);
        self.visit_expression(&expr.expression);
        let close = point_span(expr.span.end.saturating_sub(1));
        self.push_punc(PunctuationType::CloseBracket, close);
    }

    fn visit_assignment_expression(&mut self, expr: &AssignmentExpression<'a>) {
        self.visit_assignment_target(&expr.left);
        let op = match expr.operator {
            AssignmentOperator::Assign => OperatorType::Assign,
            AssignmentOperator::Addition => OperatorType::AddAssign,
            AssignmentOperator::Subtraction => OperatorType::SubAssign,
            AssignmentOperator::Multiplication => OperatorType::MulAssign,
            AssignmentOperator::Division => OperatorType::DivAssign,
            AssignmentOperator::Remainder => OperatorType::ModAssign,
            AssignmentOperator::Exponential => OperatorType::ExpAssign,
            AssignmentOperator::LogicalAnd => OperatorType::AndAssign,
            AssignmentOperator::LogicalOr => OperatorType::OrAssign,
            AssignmentOperator::LogicalNullish => OperatorType::NullishAssign,
            AssignmentOperator::BitwiseAnd => OperatorType::BitwiseAndAssign,
            AssignmentOperator::BitwiseOR => OperatorType::BitwiseOrAssign,
            AssignmentOperator::BitwiseXOR => OperatorType::BitwiseXorAssign,
            AssignmentOperator::ShiftLeft => OperatorType::ShiftLeftAssign,
            AssignmentOperator::ShiftRight => OperatorType::ShiftRightAssign,
            AssignmentOperator::ShiftRightZeroFill => OperatorType::UnsignedShiftRightAssign,
        };
        self.push_op(op, expr.span);
        self.visit_expression(&expr.right);
    }

    fn visit_binary_expression(&mut self, expr: &BinaryExpression<'a>) {
        self.visit_expression(&expr.left);
        let op = match expr.operator {
            BinaryOperator::Addition => OperatorType::Add,
            BinaryOperator::Subtraction => OperatorType::Sub,
            BinaryOperator::Multiplication => OperatorType::Mul,
            BinaryOperator::Division => OperatorType::Div,
            BinaryOperator::Remainder => OperatorType::Mod,
            BinaryOperator::Exponential => OperatorType::Exp,
            BinaryOperator::Equality => OperatorType::Eq,
            BinaryOperator::Inequality => OperatorType::NEq,
            BinaryOperator::StrictEquality => OperatorType::StrictEq,
            BinaryOperator::StrictInequality => OperatorType::StrictNEq,
            BinaryOperator::LessThan => OperatorType::Lt,
            BinaryOperator::GreaterThan => OperatorType::Gt,
            BinaryOperator::LessEqualThan => OperatorType::LtEq,
            BinaryOperator::GreaterEqualThan => OperatorType::GtEq,
            BinaryOperator::BitwiseAnd => OperatorType::BitwiseAnd,
            BinaryOperator::BitwiseOR => OperatorType::BitwiseOr,
            BinaryOperator::BitwiseXOR => OperatorType::BitwiseXor,
            BinaryOperator::ShiftLeft => OperatorType::ShiftLeft,
            BinaryOperator::ShiftRight => OperatorType::ShiftRight,
            BinaryOperator::ShiftRightZeroFill => OperatorType::UnsignedShiftRight,
            BinaryOperator::Instanceof => OperatorType::Instanceof,
            BinaryOperator::In => OperatorType::In,
        };
        self.push_op(op, expr.span);
        self.visit_expression(&expr.right);
    }

    fn visit_logical_expression(&mut self, expr: &LogicalExpression<'a>) {
        self.visit_expression(&expr.left);
        let op = match expr.operator {
            LogicalOperator::And => OperatorType::And,
            LogicalOperator::Or => OperatorType::Or,
            LogicalOperator::Coalesce => OperatorType::NullishCoalescing,
        };
        self.push_op(op, expr.span);
        self.visit_expression(&expr.right);
    }

    fn visit_unary_expression(&mut self, expr: &UnaryExpression<'a>) {
        let op = match expr.operator {
            UnaryOperator::UnaryPlus => OperatorType::Add,
            UnaryOperator::UnaryNegation => OperatorType::Sub,
            UnaryOperator::LogicalNot => OperatorType::Not,
            UnaryOperator::BitwiseNot => OperatorType::BitwiseNot,
            UnaryOperator::Typeof => {
                self.push_keyword(KeywordType::Typeof, expr.span);
                walk::walk_unary_expression(self, expr);
                return;
            }
            UnaryOperator::Void => {
                self.push_keyword(KeywordType::Void, expr.span);
                walk::walk_unary_expression(self, expr);
                return;
            }
            UnaryOperator::Delete => {
                self.push_keyword(KeywordType::Delete, expr.span);
                walk::walk_unary_expression(self, expr);
                return;
            }
        };
        self.push_op(op, expr.span);
        walk::walk_unary_expression(self, expr);
    }

    fn visit_update_expression(&mut self, expr: &UpdateExpression<'a>) {
        let op = match expr.operator {
            UpdateOperator::Increment => OperatorType::Increment,
            UpdateOperator::Decrement => OperatorType::Decrement,
        };
        if expr.prefix {
            self.push_op(op, expr.span);
        }
        walk::walk_update_expression(self, expr);
        if !expr.prefix {
            self.push_op(op, expr.span);
        }
    }

    fn visit_conditional_expression(&mut self, expr: &ConditionalExpression<'a>) {
        self.visit_expression(&expr.test);
        self.push_op(OperatorType::Ternary, expr.span);
        self.visit_expression(&expr.consequent);
        self.push_punc(PunctuationType::Colon, expr.span);
        self.visit_expression(&expr.alternate);
    }

    fn visit_arrow_function_expression(&mut self, expr: &ArrowFunctionExpression<'a>) {
        if expr.r#async {
            self.push_keyword(KeywordType::Async, expr.span);
        }
        let params_span = expr.params.span;
        self.push_punc(PunctuationType::OpenParen, point_span(params_span.start));
        for param in &expr.params.items {
            self.visit_binding_pattern(&param.pattern);
            self.push_op(OperatorType::Comma, point_span(param.span.end));
        }
        self.push_punc(
            PunctuationType::CloseParen,
            point_span(params_span.end.saturating_sub(1)),
        );
        self.push_op(OperatorType::Arrow, point_span(params_span.end));
        walk::walk_arrow_function_expression(self, expr);
    }

    fn visit_yield_expression(&mut self, expr: &YieldExpression<'a>) {
        self.push_keyword(KeywordType::Yield, expr.span);
        walk::walk_yield_expression(self, expr);
    }

    fn visit_await_expression(&mut self, expr: &AwaitExpression<'a>) {
        self.push_keyword(KeywordType::Await, expr.span);
        walk::walk_await_expression(self, expr);
    }

    fn visit_spread_element(&mut self, elem: &SpreadElement<'a>) {
        self.push_op(OperatorType::Spread, elem.span);
        walk::walk_spread_element(self, elem);
    }

    fn visit_sequence_expression(&mut self, expr: &SequenceExpression<'a>) {
        for (i, sub_expr) in expr.expressions.iter().enumerate() {
            if i > 0 {
                self.push_op(OperatorType::Comma, expr.span);
            }
            self.visit_expression(sub_expr);
        }
    }

    // ── Functions ──────────────────────────────────────────

    fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
        if func.r#async {
            self.push_keyword(KeywordType::Async, func.span);
        }
        self.push_keyword(KeywordType::Function, func.span);
        if let Some(id) = &func.id {
            self.push(TokenKind::Identifier(id.name.to_string()), id.span);
        }
        let params_span = func.params.span;
        self.push_punc(PunctuationType::OpenParen, point_span(params_span.start));
        for param in &func.params.items {
            self.visit_binding_pattern(&param.pattern);
            self.push_op(OperatorType::Comma, point_span(param.span.end));
        }
        self.push_punc(
            PunctuationType::CloseParen,
            point_span(params_span.end.saturating_sub(1)),
        );
        walk::walk_function(self, func, flags);
    }

    // ── Classes ─────────────────────────────────────────────

    fn visit_class(&mut self, class: &Class<'a>) {
        self.push_keyword(KeywordType::Class, class.span);
        if let Some(id) = &class.id {
            self.push(TokenKind::Identifier(id.name.to_string()), id.span);
        }
        if class.super_class.is_some() {
            self.push_keyword(KeywordType::Extends, class.span);
        }
        walk::walk_class(self, class);
    }

    // ── Import/Export ───────────────────────────────────────

    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        // Skip `import type { ... } from '...'` when stripping types
        if self.strip_types && decl.import_kind.is_type() {
            return;
        }
        self.push_keyword(KeywordType::Import, decl.span);
        walk::walk_import_declaration(self, decl);
        self.push_keyword(KeywordType::From, decl.span);
        self.push(
            TokenKind::StringLiteral(decl.source.value.to_string()),
            decl.source.span,
        );
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        // Skip `export type { ... }` when stripping types
        if self.strip_types && decl.export_kind.is_type() {
            return;
        }
        self.push_keyword(KeywordType::Export, decl.span);
        walk::walk_export_named_declaration(self, decl);
    }

    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        self.push_keyword(KeywordType::Export, decl.span);
        self.push_keyword(KeywordType::Default, decl.span);
        walk::walk_export_default_declaration(self, decl);
    }

    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        self.push_keyword(KeywordType::Export, decl.span);
        self.push_keyword(KeywordType::From, decl.span);
        self.push(
            TokenKind::StringLiteral(decl.source.value.to_string()),
            decl.source.span,
        );
    }

    // ── TypeScript declarations ────────────────────────────

    fn visit_ts_interface_declaration(&mut self, decl: &TSInterfaceDeclaration<'a>) {
        if self.strip_types {
            return; // Skip entire interface when stripping types
        }
        self.push_keyword(KeywordType::Interface, decl.span);
        walk::walk_ts_interface_declaration(self, decl);
    }

    fn visit_ts_interface_body(&mut self, body: &TSInterfaceBody<'a>) {
        self.push_punc(PunctuationType::OpenBrace, body.span);
        walk::walk_ts_interface_body(self, body);
        self.push_punc(PunctuationType::CloseBrace, body.span);
    }

    fn visit_ts_type_alias_declaration(&mut self, decl: &TSTypeAliasDeclaration<'a>) {
        if self.strip_types {
            return; // Skip entire type alias when stripping types
        }
        self.push_keyword(KeywordType::Type, decl.span);
        walk::walk_ts_type_alias_declaration(self, decl);
    }

    fn visit_ts_module_declaration(&mut self, decl: &TSModuleDeclaration<'a>) {
        if self.strip_types && decl.declare {
            return; // Skip `declare module` / `declare namespace` when stripping types
        }
        walk::walk_ts_module_declaration(self, decl);
    }

    fn visit_ts_enum_declaration(&mut self, decl: &TSEnumDeclaration<'a>) {
        self.push_keyword(KeywordType::Enum, decl.span);
        walk::walk_ts_enum_declaration(self, decl);
    }

    fn visit_ts_enum_body(&mut self, body: &TSEnumBody<'a>) {
        self.push_punc(PunctuationType::OpenBrace, body.span);
        walk::walk_ts_enum_body(self, body);
        self.push_punc(PunctuationType::CloseBrace, body.span);
    }

    fn visit_ts_property_signature(&mut self, sig: &TSPropertySignature<'a>) {
        walk::walk_ts_property_signature(self, sig);
        self.push_punc(PunctuationType::Semicolon, sig.span);
    }

    fn visit_ts_type_annotation(&mut self, ann: &TSTypeAnnotation<'a>) {
        if self.strip_types {
            return; // Skip parameter/return type annotations when stripping types
        }
        self.push_punc(PunctuationType::Colon, ann.span);
        walk::walk_ts_type_annotation(self, ann);
    }

    fn visit_ts_type_parameter_declaration(&mut self, decl: &TSTypeParameterDeclaration<'a>) {
        if self.strip_types {
            return; // Skip generic type parameters when stripping types
        }
        walk::walk_ts_type_parameter_declaration(self, decl);
    }

    fn visit_ts_type_parameter_instantiation(&mut self, inst: &TSTypeParameterInstantiation<'a>) {
        if self.strip_types {
            return; // Skip generic type arguments when stripping types
        }
        walk::walk_ts_type_parameter_instantiation(self, inst);
    }

    fn visit_ts_as_expression(&mut self, expr: &TSAsExpression<'a>) {
        self.visit_expression(&expr.expression);
        if !self.strip_types {
            self.push_keyword(KeywordType::As, expr.span);
            self.visit_ts_type(&expr.type_annotation);
        }
    }

    fn visit_ts_satisfies_expression(&mut self, expr: &TSSatisfiesExpression<'a>) {
        self.visit_expression(&expr.expression);
        if !self.strip_types {
            self.push_keyword(KeywordType::Satisfies, expr.span);
            self.visit_ts_type(&expr.type_annotation);
        }
    }

    fn visit_ts_non_null_expression(&mut self, expr: &TSNonNullExpression<'a>) {
        self.visit_expression(&expr.expression);
        // The `!` postfix is stripped when stripping types (it's a type assertion)
    }

    fn visit_identifier_name(&mut self, ident: &IdentifierName<'a>) {
        self.push(TokenKind::Identifier(ident.name.to_string()), ident.span);
    }

    fn visit_ts_string_keyword(&mut self, it: &TSStringKeyword) {
        self.push(TokenKind::Identifier("string".to_string()), it.span);
    }

    fn visit_ts_number_keyword(&mut self, it: &TSNumberKeyword) {
        self.push(TokenKind::Identifier("number".to_string()), it.span);
    }

    fn visit_ts_boolean_keyword(&mut self, it: &TSBooleanKeyword) {
        self.push(TokenKind::Identifier("boolean".to_string()), it.span);
    }

    fn visit_ts_any_keyword(&mut self, it: &TSAnyKeyword) {
        self.push(TokenKind::Identifier("any".to_string()), it.span);
    }

    fn visit_ts_void_keyword(&mut self, it: &TSVoidKeyword) {
        self.push(TokenKind::Identifier("void".to_string()), it.span);
    }

    fn visit_ts_null_keyword(&mut self, it: &TSNullKeyword) {
        self.push(TokenKind::NullLiteral, it.span);
    }

    fn visit_ts_undefined_keyword(&mut self, it: &TSUndefinedKeyword) {
        self.push(TokenKind::Identifier("undefined".to_string()), it.span);
    }

    fn visit_ts_never_keyword(&mut self, it: &TSNeverKeyword) {
        self.push(TokenKind::Identifier("never".to_string()), it.span);
    }

    fn visit_ts_unknown_keyword(&mut self, it: &TSUnknownKeyword) {
        self.push(TokenKind::Identifier("unknown".to_string()), it.span);
    }

    // ── JSX ─────────────────────────────────────────────────

    fn visit_jsx_opening_element(&mut self, elem: &JSXOpeningElement<'a>) {
        self.push_punc(PunctuationType::OpenBracket, elem.span);
        walk::walk_jsx_opening_element(self, elem);
        self.push_punc(PunctuationType::CloseBracket, elem.span);
    }

    fn visit_jsx_closing_element(&mut self, elem: &JSXClosingElement<'a>) {
        self.push_punc(PunctuationType::OpenBracket, elem.span);
        walk::walk_jsx_closing_element(self, elem);
        self.push_punc(PunctuationType::CloseBracket, elem.span);
    }

    fn visit_jsx_identifier(&mut self, ident: &JSXIdentifier<'a>) {
        self.push(TokenKind::Identifier(ident.name.to_string()), ident.span);
    }

    fn visit_jsx_spread_attribute(&mut self, attr: &JSXSpreadAttribute<'a>) {
        self.push_op(OperatorType::Spread, attr.span);
        walk::walk_jsx_spread_attribute(self, attr);
    }

    // ── Misc ────────────────────────────────────────────────

    fn visit_variable_declarator(&mut self, decl: &VariableDeclarator<'a>) {
        self.visit_binding_pattern(&decl.id);
        if let Some(init) = &decl.init {
            self.push_op(OperatorType::Assign, decl.span);
            self.visit_expression(init);
        }
        self.push_punc(PunctuationType::Semicolon, decl.span);
    }

    fn visit_expression_statement(&mut self, stmt: &ExpressionStatement<'a>) {
        walk::walk_expression_statement(self, stmt);
        self.push_punc(PunctuationType::Semicolon, stmt.span);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tokenize(code: &str) -> Vec<SourceToken> {
        let path = PathBuf::from("test.ts");
        tokenize_file(&path, code).tokens
    }

    #[test]
    fn tokenize_variable_declaration() {
        let tokens = tokenize("const x = 42;");
        assert!(!tokens.is_empty());
        // Should have: const, x (identifier), = (assign), 42 (numeric), ;
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Keyword(KeywordType::Const)
        ));
    }

    #[test]
    fn tokenize_function_declaration() {
        let tokens = tokenize("function foo() { return 1; }");
        assert!(!tokens.is_empty());
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Keyword(KeywordType::Function)
        ));
    }

    #[test]
    fn tokenize_arrow_function() {
        let tokens = tokenize("const f = (a, b) => a + b;");
        assert!(!tokens.is_empty());
        let has_arrow = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Arrow)));
        assert!(has_arrow, "Should contain arrow operator");
    }

    #[test]
    fn tokenize_if_else() {
        let tokens = tokenize("if (x) { y; } else { z; }");
        assert!(!tokens.is_empty());
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Keyword(KeywordType::If)
        ));
        let has_else = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Else)));
        assert!(has_else, "Should contain else keyword");
    }

    #[test]
    fn tokenize_class() {
        let tokens = tokenize("class Foo extends Bar { }");
        assert!(!tokens.is_empty());
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Keyword(KeywordType::Class)
        ));
        let has_extends = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Extends)));
        assert!(has_extends, "Should contain extends keyword");
    }

    #[test]
    fn tokenize_string_literal() {
        let tokens = tokenize("const s = \"hello\";");
        let has_string = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::StringLiteral(s) if s == "hello"));
        assert!(has_string, "Should contain string literal");
    }

    #[test]
    fn tokenize_boolean_literal() {
        let tokens = tokenize("const b = true;");
        let has_bool = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::BooleanLiteral(true)));
        assert!(has_bool, "Should contain boolean literal");
    }

    #[test]
    fn tokenize_null_literal() {
        let tokens = tokenize("const n = null;");
        let has_null = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::NullLiteral));
        assert!(has_null, "Should contain null literal");
    }

    #[test]
    fn tokenize_empty_file() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn tokenize_ts_interface() {
        let tokens = tokenize("interface Foo { bar: string; baz: number; }");
        let has_interface = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Interface)));
        assert!(has_interface, "Should contain interface keyword");
        let has_bar = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(name) if name == "bar"));
        assert!(has_bar, "Should contain property name 'bar'");
        let has_string = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(name) if name == "string"));
        assert!(has_string, "Should contain type 'string'");
        // Should have enough tokens for clone detection
        assert!(
            tokens.len() >= 10,
            "Interface should produce sufficient tokens, got {}",
            tokens.len()
        );
    }

    #[test]
    fn tokenize_ts_type_alias() {
        let tokens = tokenize("type Result = { ok: boolean; error: string; }");
        let has_type = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Type)));
        assert!(has_type, "Should contain type keyword");
    }

    #[test]
    fn tokenize_ts_enum() {
        let tokens = tokenize("enum Color { Red, Green, Blue }");
        let has_enum = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Enum)));
        assert!(has_enum, "Should contain enum keyword");
        let has_red = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(name) if name == "Red"));
        assert!(has_red, "Should contain enum member 'Red'");
    }

    fn tokenize_tsx(code: &str) -> Vec<SourceToken> {
        let path = PathBuf::from("test.tsx");
        tokenize_file(&path, code).tokens
    }

    fn tokenize_cross_language(code: &str) -> Vec<SourceToken> {
        let path = PathBuf::from("test.ts");
        tokenize_file_cross_language(&path, code, true).tokens
    }

    #[test]
    fn tokenize_jsx_element() {
        let tokens =
            tokenize_tsx("const x = <div className=\"foo\"><Button onClick={handler} /></div>;");
        let has_div = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(name) if name == "div"));
        assert!(has_div, "Should contain JSX element name 'div'");
        let has_classname = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(name) if name == "className"));
        assert!(has_classname, "Should contain JSX attribute 'className'");
        let brackets = tokens
            .iter()
            .filter(|t| {
                matches!(
                    t.kind,
                    TokenKind::Punctuation(PunctuationType::OpenBracket)
                        | TokenKind::Punctuation(PunctuationType::CloseBracket)
                )
            })
            .count();
        assert!(
            brackets >= 4,
            "Should contain JSX angle brackets, got {brackets}"
        );
    }

    // ── Cross-language type stripping tests ──────────────────────

    #[test]
    fn strip_types_removes_parameter_type_annotations() {
        let ts_tokens = tokenize("function foo(x: string) { return x; }");
        let stripped = tokenize_cross_language("function foo(x: string) { return x; }");

        // The stripped version should have fewer tokens (no `: string`)
        assert!(
            stripped.len() < ts_tokens.len(),
            "Stripped tokens ({}) should be fewer than full tokens ({})",
            stripped.len(),
            ts_tokens.len()
        );

        // Should NOT contain type-annotation colon or the type name
        let has_colon_before_string = ts_tokens.windows(2).any(|w| {
            matches!(w[0].kind, TokenKind::Punctuation(PunctuationType::Colon))
                && matches!(&w[1].kind, TokenKind::Identifier(n) if n == "string")
        });
        assert!(has_colon_before_string, "Original should have `: string`");

        // Stripped version should match JS version
        let js_tokens = {
            let path = PathBuf::from("test.js");
            tokenize_file(&path, "function foo(x) { return x; }").tokens
        };
        assert_eq!(
            stripped.len(),
            js_tokens.len(),
            "Stripped TS should produce same token count as JS"
        );
    }

    #[test]
    fn strip_types_removes_return_type_annotations() {
        let stripped = tokenize_cross_language("function foo(): string { return 'hello'; }");
        // Should NOT contain the return type annotation
        let has_string_type = stripped.iter().enumerate().any(|(i, t)| {
            matches!(&t.kind, TokenKind::Identifier(n) if n == "string")
                && i > 0
                && matches!(
                    stripped[i - 1].kind,
                    TokenKind::Punctuation(PunctuationType::Colon)
                )
        });
        assert!(
            !has_string_type,
            "Stripped version should not have return type annotation"
        );
    }

    #[test]
    fn strip_types_removes_interface_declarations() {
        let stripped = tokenize_cross_language("interface Foo { bar: string; }\nconst x = 42;");
        // Should NOT contain interface keyword
        let has_interface = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Interface)));
        assert!(
            !has_interface,
            "Stripped version should not contain interface declaration"
        );
        // Should still contain the const declaration
        let has_const = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Const)));
        assert!(has_const, "Should still contain const keyword");
    }

    #[test]
    fn strip_types_removes_type_alias_declarations() {
        let stripped = tokenize_cross_language("type Result = string | number;\nconst x = 42;");
        let has_type = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Type)));
        assert!(!has_type, "Stripped version should not contain type alias");
        let has_const = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Const)));
        assert!(has_const, "Should still contain const keyword");
    }

    #[test]
    fn strip_types_preserves_runtime_code() {
        let stripped =
            tokenize_cross_language("const x: number = 42;\nif (x > 0) { console.log(x); }");
        // Should have const, x, =, 42, if, x, >, 0, console, log, x, etc.
        let has_const = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Const)));
        let has_if = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::If)));
        let has_42 = stripped
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::NumericLiteral(n) if n == "42"));
        assert!(has_const, "Should preserve const");
        assert!(has_if, "Should preserve if");
        assert!(has_42, "Should preserve numeric literal");
    }

    #[test]
    fn strip_types_preserves_enums() {
        // Enums have runtime semantics, so they should NOT be stripped
        let stripped = tokenize_cross_language("enum Color { Red, Green, Blue }");
        let has_enum = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Enum)));
        assert!(
            has_enum,
            "Enums should be preserved (they have runtime semantics)"
        );
    }

    #[test]
    fn strip_types_removes_import_type() {
        let stripped = tokenize_cross_language("import type { Foo } from './foo';\nconst x = 42;");
        // Should NOT contain import keyword from the type-only import
        let import_count = stripped
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Import)))
            .count();
        assert_eq!(import_count, 0, "import type should be stripped");
        // Should still contain the const declaration
        let has_const = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Const)));
        assert!(has_const, "Runtime code should be preserved");
    }

    #[test]
    fn strip_types_preserves_value_imports() {
        let stripped = tokenize_cross_language("import { foo } from './foo';\nconst x = foo();");
        let has_import = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Import)));
        assert!(has_import, "Value imports should be preserved");
    }

    #[test]
    fn strip_types_removes_export_type() {
        let stripped = tokenize_cross_language("export type { Foo };\nconst x = 42;");
        // The export type should be stripped
        let export_count = stripped
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Export)))
            .count();
        assert_eq!(export_count, 0, "export type should be stripped");
    }

    #[test]
    fn strip_types_removes_declare_module() {
        let stripped = tokenize_cross_language(
            "declare module 'foo' { export function bar(): void; }\nconst x = 42;",
        );
        // Should not contain function keyword from the declare block
        let has_function_keyword = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Function)));
        assert!(
            !has_function_keyword,
            "declare module contents should be stripped"
        );
        let has_const = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Const)));
        assert!(has_const, "Runtime code should be preserved");
    }
}
