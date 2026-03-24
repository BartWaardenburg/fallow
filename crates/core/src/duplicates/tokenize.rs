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

/// TypeScript/JavaScript keyword types.
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

    // ── File type dispatch tests ─────────────────────────────────

    #[test]
    fn tokenize_vue_sfc_extracts_script_block() {
        let vue_source = r#"<template><div>Hello</div></template>
<script lang="ts">
import { ref } from 'vue';
const count = ref(0);
</script>"#;
        let path = PathBuf::from("Component.vue");
        let result = tokenize_file(&path, vue_source);
        assert!(!result.tokens.is_empty(), "Vue SFC should produce tokens");
        let has_import = result
            .tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Import)));
        assert!(has_import, "Should tokenize import in <script> block");
        let has_const = result
            .tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Const)));
        assert!(has_const, "Should tokenize const in <script> block");
    }

    #[test]
    fn tokenize_svelte_sfc_extracts_script_block() {
        let svelte_source = r"<script>
let count = 0;
function increment() { count += 1; }
</script>
<button on:click={increment}>{count}</button>";
        let path = PathBuf::from("Component.svelte");
        let result = tokenize_file(&path, svelte_source);
        assert!(
            !result.tokens.is_empty(),
            "Svelte SFC should produce tokens"
        );
        let has_let = result
            .tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Let)));
        assert!(has_let, "Should tokenize let in <script> block");
        let has_function = result
            .tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Function)));
        assert!(has_function, "Should tokenize function in <script> block");
    }

    #[test]
    fn tokenize_vue_sfc_adjusts_span_offsets() {
        let vue_source = "<template><div/></template>\n<script>\nconst x = 1;\n</script>";
        let path = PathBuf::from("Test.vue");
        let result = tokenize_file(&path, vue_source);
        // The script body starts after "<template><div/></template>\n<script>\n"
        let script_body_offset = vue_source.find("const x").unwrap() as u32;
        // All token spans should reference positions in the full SFC source,
        // not positions within the extracted script body.
        for token in &result.tokens {
            assert!(
                token.span.start >= script_body_offset,
                "Token span start ({}) should be >= script body offset ({})",
                token.span.start,
                script_body_offset
            );
            // Verify span text is recoverable from the full source
            let text = &vue_source[token.span.start as usize..token.span.end as usize];
            assert!(
                !text.is_empty(),
                "Token span should recover non-empty text from full SFC source"
            );
        }
    }

    #[test]
    fn tokenize_astro_extracts_frontmatter() {
        let astro_source = "---\nimport { Layout } from '../layouts/Layout.astro';\nconst title = 'Home';\n---\n<Layout title={title}><h1>Hello</h1></Layout>";
        let path = PathBuf::from("page.astro");
        let result = tokenize_file(&path, astro_source);
        assert!(
            !result.tokens.is_empty(),
            "Astro frontmatter should produce tokens"
        );
        let has_import = result
            .tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Import)));
        assert!(has_import, "Should tokenize import in frontmatter");
    }

    #[test]
    fn tokenize_astro_without_frontmatter_returns_empty() {
        let astro_source = "<html><body>Hello</body></html>";
        let path = PathBuf::from("page.astro");
        let result = tokenize_file(&path, astro_source);
        assert!(
            result.tokens.is_empty(),
            "Astro without frontmatter should produce no tokens"
        );
    }

    #[test]
    fn tokenize_astro_adjusts_span_offsets() {
        let astro_source = "---\nconst x = 1;\n---\n<div/>";
        let path = PathBuf::from("page.astro");
        let result = tokenize_file(&path, astro_source);
        assert!(!result.tokens.is_empty());
        // "---\n" is 4 bytes — spans should be offset from there
        for token in &result.tokens {
            assert!(
                token.span.start >= 4,
                "Token span start ({}) should be offset into the full astro source",
                token.span.start
            );
        }
    }

    #[test]
    fn tokenize_mdx_extracts_imports_and_exports() {
        let mdx_source = "import { Button } from './Button';\nexport const meta = { title: 'Hello' };\n\n# Hello World\n\n<Button>Click me</Button>";
        let path = PathBuf::from("page.mdx");
        let result = tokenize_file(&path, mdx_source);
        assert!(
            !result.tokens.is_empty(),
            "MDX should produce tokens from imports/exports"
        );
        let has_import = result
            .tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Import)));
        assert!(has_import, "Should tokenize import in MDX");
        let has_export = result
            .tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Export)));
        assert!(has_export, "Should tokenize export in MDX");
    }

    #[test]
    fn tokenize_mdx_without_statements_returns_empty() {
        let mdx_source = "# Just Markdown\n\nNo imports or exports here.";
        let path = PathBuf::from("page.mdx");
        let result = tokenize_file(&path, mdx_source);
        assert!(
            result.tokens.is_empty(),
            "MDX without imports/exports should produce no tokens"
        );
    }

    #[test]
    fn tokenize_css_returns_empty() {
        let css_source = ".foo { color: red; }\n.bar { font-size: 16px; }";
        let path = PathBuf::from("styles.css");
        let result = tokenize_file(&path, css_source);
        assert!(
            result.tokens.is_empty(),
            "CSS files should produce no tokens"
        );
        assert!(result.line_count >= 1);
    }

    #[test]
    fn tokenize_scss_returns_empty() {
        let scss_source = "$color: red;\n.foo { color: $color; }";
        let path = PathBuf::from("styles.scss");
        let result = tokenize_file(&path, scss_source);
        assert!(
            result.tokens.is_empty(),
            "SCSS files should produce no tokens"
        );
    }

    // ── Line count and FileTokens metadata ──────────────────────

    #[test]
    fn file_tokens_line_count_matches_source() {
        let source = "const x = 1;\nconst y = 2;\nconst z = 3;";
        let path = PathBuf::from("test.ts");
        let result = tokenize_file(&path, source);
        assert_eq!(result.line_count, 3);
        assert_eq!(result.source, source);
    }

    #[test]
    fn file_tokens_line_count_minimum_is_one() {
        let path = PathBuf::from("test.ts");
        let result = tokenize_file(&path, "");
        assert_eq!(result.line_count, 1, "Empty file should have line_count 1");
    }

    // ── JSX fallback retry path ─────────────────────────────────

    #[test]
    fn js_file_with_jsx_retries_as_jsx() {
        // A .js file containing JSX should trigger the fallback retry with JSX source type.
        // The initial parse as plain JS will fail on JSX, producing few tokens.
        // The retry as JSX should succeed and produce more tokens.
        let jsx_code = r#"
function App() {
    return (
        <div className="app">
            <h1>Hello World</h1>
            <p>Welcome to the app</p>
        </div>
    );
}
"#;
        let path = PathBuf::from("app.js");
        let result = tokenize_file(&path, jsx_code);
        // If the retry works, we should see JSX angle brackets
        let has_brackets = result
            .tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::OpenBracket)));
        assert!(
            has_brackets,
            "JSX fallback retry should produce JSX tokens from .js file"
        );
    }

    // ── Statement tokenization ──────────────────────────────────

    #[test]
    fn tokenize_for_in_statement() {
        let tokens = tokenize("for (const key in obj) { console.log(key); }");
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Keyword(KeywordType::For)
        ));
        let has_in = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::In)));
        assert!(has_in, "Should contain 'in' keyword");
    }

    #[test]
    fn tokenize_for_of_statement() {
        let tokens = tokenize("for (const item of items) { process(item); }");
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Keyword(KeywordType::For)
        ));
        let has_of = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Of)));
        assert!(has_of, "Should contain 'of' keyword");
    }

    #[test]
    fn tokenize_while_statement() {
        let tokens = tokenize("while (x > 0) { x--; }");
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Keyword(KeywordType::While)
        ));
        let has_gt = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Gt)));
        assert!(has_gt, "Should contain greater-than operator");
    }

    #[test]
    fn tokenize_do_while_statement() {
        let tokens = tokenize("do { x++; } while (x < 10);");
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Keyword(KeywordType::Do)
        ));
        // The visitor only emits `Do` -- the `while` part is implicit in the AST walk.
        // Verify the body and condition are tokenized:
        let has_increment = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Increment)));
        assert!(has_increment, "do-while body should contain increment");
        let has_lt = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Lt)));
        assert!(has_lt, "do-while condition should contain < operator");
    }

    #[test]
    fn tokenize_switch_case_default() {
        let tokens = tokenize("switch (x) { case 1: break; case 2: break; default: return; }");
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Keyword(KeywordType::Switch)
        ));
        let case_count = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Case)))
            .count();
        assert_eq!(case_count, 2, "Should have two case keywords");
        let has_default = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Default)));
        assert!(has_default, "Should have default keyword");
        let has_break = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Break)));
        assert!(has_break, "Should have break keyword");
        // Colons after case/default
        let colon_count = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::Colon)))
            .count();
        assert!(
            colon_count >= 3,
            "Should have at least 3 colons (case, case, default), got {colon_count}"
        );
    }

    #[test]
    fn tokenize_continue_statement() {
        let tokens = tokenize("for (let i = 0; i < 10; i++) { if (i === 5) continue; }");
        let has_continue = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Continue)));
        assert!(has_continue, "Should contain continue keyword");
    }

    #[test]
    fn tokenize_try_catch_finally() {
        let tokens = tokenize("try { foo(); } catch (e) { bar(); } finally { baz(); }");
        let has_try = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Try)));
        let has_catch = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Catch)));
        let has_finally = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Finally)));
        assert!(has_try, "Should contain try keyword");
        assert!(has_catch, "Should contain catch keyword");
        // No visit_finally_clause override — finally keyword is not emitted as a token.
        // The finally block's braces and contents are still visited via walk.
        assert!(
            !has_finally,
            "Finally keyword is not emitted (no visitor override)"
        );
    }

    #[test]
    fn tokenize_throw_statement() {
        let tokens = tokenize("throw new Error('fail');");
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Keyword(KeywordType::Throw)
        ));
        let has_new = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::New)));
        assert!(has_new, "Should contain new keyword");
    }

    // ── Expression tokenization ─────────────────────────────────

    #[test]
    fn tokenize_this_expression() {
        let tokens = tokenize("const x = this.foo;");
        let has_this = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::This)));
        assert!(has_this, "Should contain this keyword");
    }

    #[test]
    fn tokenize_super_expression() {
        let tokens = tokenize("class Child extends Parent { constructor() { super(); } }");
        let has_super = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Super)));
        assert!(has_super, "Should contain super keyword");
    }

    #[test]
    fn tokenize_array_expression() {
        let tokens = tokenize("const arr = [1, 2, 3];");
        let open_bracket = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::OpenBracket)));
        let close_bracket = tokens.iter().any(|t| {
            matches!(
                t.kind,
                TokenKind::Punctuation(PunctuationType::CloseBracket)
            )
        });
        assert!(open_bracket, "Should contain open bracket");
        assert!(close_bracket, "Should contain close bracket");
    }

    #[test]
    fn tokenize_object_expression() {
        let tokens = tokenize("const obj = { a: 1, b: 2 };");
        let open_brace = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::OpenBrace)))
            .count();
        let close_brace = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::CloseBrace)))
            .count();
        assert!(open_brace >= 1, "Should have open brace for object");
        assert!(close_brace >= 1, "Should have close brace for object");
    }

    #[test]
    fn tokenize_computed_member_expression() {
        let tokens = tokenize("const x = obj[key];");
        // Should have open and close brackets around the computed member
        let open_bracket = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::OpenBracket)));
        let close_bracket = tokens.iter().any(|t| {
            matches!(
                t.kind,
                TokenKind::Punctuation(PunctuationType::CloseBracket)
            )
        });
        assert!(
            open_bracket,
            "Should contain open bracket for computed member"
        );
        assert!(
            close_bracket,
            "Should contain close bracket for computed member"
        );
    }

    #[test]
    fn tokenize_static_member_expression() {
        let tokens = tokenize("const x = obj.prop;");
        let has_dot = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::Dot)));
        let has_prop = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(n) if n == "prop"));
        assert!(has_dot, "Should contain dot for member access");
        assert!(has_prop, "Should contain property name 'prop'");
    }

    #[test]
    fn tokenize_new_expression() {
        let tokens = tokenize("const d = new Date(2024, 1, 1);");
        let has_new = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::New)));
        assert!(has_new, "Should contain new keyword");
        let has_date = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(n) if n == "Date"));
        assert!(has_date, "Should contain identifier 'Date'");
    }

    #[test]
    fn tokenize_template_literal() {
        let tokens = tokenize("const s = `hello ${name}`;");
        let has_template = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::TemplateLiteral));
        assert!(has_template, "Should contain template literal token");
    }

    #[test]
    fn tokenize_regex_literal() {
        let tokens = tokenize("const re = /foo[a-z]+/gi;");
        let has_regex = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::RegExpLiteral));
        assert!(has_regex, "Should contain regex literal token");
    }

    #[test]
    fn tokenize_conditional_ternary_expression() {
        let tokens = tokenize("const x = a ? b : c;");
        let has_ternary = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Ternary)));
        let has_colon = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::Colon)));
        assert!(has_ternary, "Should contain ternary operator");
        assert!(has_colon, "Should contain colon for ternary");
    }

    #[test]
    fn tokenize_sequence_expression() {
        let tokens = tokenize("for (let i = 0, j = 10; i < j; i++, j--) {}");
        let comma_count = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Comma)))
            .count();
        assert!(
            comma_count >= 1,
            "Sequence expression should produce comma operators"
        );
    }

    #[test]
    fn tokenize_spread_element() {
        let tokens = tokenize("const arr = [...other, 1, 2];");
        let has_spread = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Spread)));
        assert!(has_spread, "Should contain spread operator");
    }

    #[test]
    fn tokenize_yield_expression() {
        let tokens = tokenize("function* gen() { yield 42; }");
        let has_yield = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Yield)));
        assert!(has_yield, "Should contain yield keyword");
    }

    #[test]
    fn tokenize_await_expression() {
        let tokens = tokenize("async function run() { const x = await fetch(); }");
        let has_async = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Async)));
        let has_await = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Await)));
        assert!(has_async, "Should contain async keyword");
        assert!(has_await, "Should contain await keyword");
    }

    #[test]
    fn tokenize_async_arrow_function() {
        let tokens = tokenize("const f = async () => { await fetch(); };");
        let has_async = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Async)));
        let has_arrow = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Arrow)));
        assert!(has_async, "Should contain async keyword before arrow");
        assert!(has_arrow, "Should contain arrow operator");
    }

    // ── Operator coverage ───────────────────────────────────────

    #[test]
    fn tokenize_all_binary_operators() {
        let code = r"
const a = 1 + 2;
const b = 3 - 4;
const c = 5 * 6;
const d = 7 / 8;
const e = 9 % 10;
const f = 2 ** 3;
const g = a == b;
const h = a != b;
const i = a === b;
const j = a !== b;
const k = a < b;
const l = a > b;
const m = a <= b;
const n = a >= b;
const o = a & b;
const p = a | b;
const q = a ^ b;
const r = a << b;
const s = a >> b;
const t = a >>> b;
const u = a instanceof Object;
const v = 'key' in obj;
";
        let tokens = tokenize(code);
        let ops: Vec<&OperatorType> = tokens
            .iter()
            .filter_map(|t| match &t.kind {
                TokenKind::Operator(op) => Some(op),
                _ => None,
            })
            .collect();
        assert!(ops.contains(&&OperatorType::Add));
        assert!(ops.contains(&&OperatorType::Sub));
        assert!(ops.contains(&&OperatorType::Mul));
        assert!(ops.contains(&&OperatorType::Div));
        assert!(ops.contains(&&OperatorType::Mod));
        assert!(ops.contains(&&OperatorType::Exp));
        assert!(ops.contains(&&OperatorType::Eq));
        assert!(ops.contains(&&OperatorType::NEq));
        assert!(ops.contains(&&OperatorType::StrictEq));
        assert!(ops.contains(&&OperatorType::StrictNEq));
        assert!(ops.contains(&&OperatorType::Lt));
        assert!(ops.contains(&&OperatorType::Gt));
        assert!(ops.contains(&&OperatorType::LtEq));
        assert!(ops.contains(&&OperatorType::GtEq));
        assert!(ops.contains(&&OperatorType::BitwiseAnd));
        assert!(ops.contains(&&OperatorType::BitwiseOr));
        assert!(ops.contains(&&OperatorType::BitwiseXor));
        assert!(ops.contains(&&OperatorType::ShiftLeft));
        assert!(ops.contains(&&OperatorType::ShiftRight));
        assert!(ops.contains(&&OperatorType::UnsignedShiftRight));
        assert!(ops.contains(&&OperatorType::Instanceof));
        assert!(ops.contains(&&OperatorType::In));
    }

    #[test]
    fn tokenize_logical_operators() {
        let tokens = tokenize("const x = a && b || c ?? d;");
        let ops: Vec<&OperatorType> = tokens
            .iter()
            .filter_map(|t| match &t.kind {
                TokenKind::Operator(op) => Some(op),
                _ => None,
            })
            .collect();
        assert!(ops.contains(&&OperatorType::And));
        assert!(ops.contains(&&OperatorType::Or));
        assert!(ops.contains(&&OperatorType::NullishCoalescing));
    }

    #[test]
    fn tokenize_assignment_operators() {
        let code = r"
x = 1;
x += 1;
x -= 1;
x *= 1;
x /= 1;
x %= 1;
x **= 1;
x &&= true;
x ||= true;
x ??= 1;
x &= 1;
x |= 1;
x ^= 1;
x <<= 1;
x >>= 1;
x >>>= 1;
";
        let tokens = tokenize(code);
        let ops: Vec<&OperatorType> = tokens
            .iter()
            .filter_map(|t| match &t.kind {
                TokenKind::Operator(op) => Some(op),
                _ => None,
            })
            .collect();
        assert!(ops.contains(&&OperatorType::Assign));
        assert!(ops.contains(&&OperatorType::AddAssign));
        assert!(ops.contains(&&OperatorType::SubAssign));
        assert!(ops.contains(&&OperatorType::MulAssign));
        assert!(ops.contains(&&OperatorType::DivAssign));
        assert!(ops.contains(&&OperatorType::ModAssign));
        assert!(ops.contains(&&OperatorType::ExpAssign));
        assert!(ops.contains(&&OperatorType::AndAssign));
        assert!(ops.contains(&&OperatorType::OrAssign));
        assert!(ops.contains(&&OperatorType::NullishAssign));
        assert!(ops.contains(&&OperatorType::BitwiseAndAssign));
        assert!(ops.contains(&&OperatorType::BitwiseOrAssign));
        assert!(ops.contains(&&OperatorType::BitwiseXorAssign));
        assert!(ops.contains(&&OperatorType::ShiftLeftAssign));
        assert!(ops.contains(&&OperatorType::ShiftRightAssign));
        assert!(ops.contains(&&OperatorType::UnsignedShiftRightAssign));
    }

    #[test]
    fn tokenize_unary_operators() {
        let code = "const a = +x; const b = -x; const c = !x; const d = ~x;";
        let tokens = tokenize(code);
        let ops: Vec<&OperatorType> = tokens
            .iter()
            .filter_map(|t| match &t.kind {
                TokenKind::Operator(op) => Some(op),
                _ => None,
            })
            .collect();
        // Unary plus maps to Add, unary minus to Sub
        assert!(
            ops.contains(&&OperatorType::Add),
            "Should have unary plus (mapped to Add)"
        );
        assert!(
            ops.contains(&&OperatorType::Sub),
            "Should have unary minus (mapped to Sub)"
        );
        assert!(ops.contains(&&OperatorType::Not), "Should have logical not");
        assert!(
            ops.contains(&&OperatorType::BitwiseNot),
            "Should have bitwise not"
        );
    }

    #[test]
    fn tokenize_typeof_void_delete_as_keywords() {
        let tokens = tokenize("typeof x; void 0; delete obj.key;");
        let has_typeof = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Typeof)));
        let has_void = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Void)));
        let has_delete = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Delete)));
        assert!(has_typeof, "typeof should be a keyword token");
        assert!(has_void, "void should be a keyword token");
        assert!(has_delete, "delete should be a keyword token");
    }

    #[test]
    fn tokenize_prefix_and_postfix_update() {
        let tokens = tokenize("++x; x--;");
        let first_increment_idx = tokens
            .iter()
            .position(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Increment)));
        let has_decrement = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Decrement)));
        assert!(
            first_increment_idx.is_some(),
            "Should have increment operator"
        );
        assert!(has_decrement, "Should have decrement operator");

        // Prefix ++x: the operator appears before the identifier
        let first_x_idx = tokens
            .iter()
            .position(|t| matches!(&t.kind, TokenKind::Identifier(n) if n == "x"))
            .unwrap();
        assert!(
            first_increment_idx.unwrap() < first_x_idx,
            "Prefix ++ should appear before identifier"
        );
    }

    // ── TypeScript-specific syntax ──────────────────────────────

    #[test]
    fn tokenize_ts_as_expression() {
        let tokens = tokenize("const x = value as string;");
        let has_as = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::As)));
        assert!(has_as, "Should contain 'as' keyword");
    }

    #[test]
    fn tokenize_ts_satisfies_expression() {
        let tokens = tokenize("const config = {} satisfies Config;");
        let has_satisfies = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Satisfies)));
        assert!(has_satisfies, "Should contain 'satisfies' keyword");
    }

    #[test]
    fn tokenize_ts_non_null_assertion() {
        let ts_tokens = tokenize("const x = value!.toString();");
        // The non-null assertion (!) is NOT emitted as a separate token.
        // It just visits the inner expression.
        let has_value = ts_tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(n) if n == "value"));
        assert!(has_value, "Should contain 'value' identifier");
    }

    #[test]
    fn tokenize_ts_generic_type_parameters() {
        let tokens = tokenize("function identity<T>(x: T): T { return x; }");
        // Without stripping types, the generic parameter T should appear
        let t_count = tokens
            .iter()
            .filter(|t| matches!(&t.kind, TokenKind::Identifier(n) if n == "T"))
            .count();
        assert!(
            t_count >= 1,
            "Generic type parameter T should appear in tokens"
        );
    }

    #[test]
    fn tokenize_ts_type_keywords() {
        let tokens = tokenize(
            "type T = string | number | boolean | any | void | null | undefined | never | unknown;",
        );
        let idents: Vec<&String> = tokens
            .iter()
            .filter_map(|t| match &t.kind {
                TokenKind::Identifier(name) => Some(name),
                _ => None,
            })
            .collect();
        assert!(idents.contains(&&"string".to_string()));
        assert!(idents.contains(&&"number".to_string()));
        assert!(idents.contains(&&"boolean".to_string()));
        assert!(idents.contains(&&"any".to_string()));
        assert!(idents.contains(&&"void".to_string()));
        assert!(idents.contains(&&"undefined".to_string()));
        assert!(idents.contains(&&"never".to_string()));
        assert!(idents.contains(&&"unknown".to_string()));
        // null is a NullLiteral, not an identifier
        let has_null = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::NullLiteral));
        assert!(has_null, "null keyword should produce NullLiteral token");
    }

    #[test]
    fn tokenize_ts_property_signatures_in_interface() {
        let tokens = tokenize("interface Foo { bar: string; baz: number; }");
        // Property signatures end with semicolons
        let semicolons = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::Semicolon)))
            .count();
        assert!(
            semicolons >= 2,
            "Interface property signatures should produce semicolons, got {semicolons}"
        );
    }

    #[test]
    fn tokenize_ts_enum_with_initializers() {
        let tokens = tokenize("enum Status { Active = 'ACTIVE', Inactive = 'INACTIVE' }");
        let has_enum = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Enum)));
        assert!(has_enum);
        let has_active_str = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::StringLiteral(s) if s == "ACTIVE"));
        assert!(has_active_str, "Should contain string initializer 'ACTIVE'");
    }

    // ── Cross-language type stripping (advanced) ────────────────

    #[test]
    fn strip_types_removes_generic_type_parameters() {
        let stripped = tokenize_cross_language("function identity<T>(x: T): T { return x; }");
        let js_tokens = {
            let path = PathBuf::from("test.js");
            tokenize_file(&path, "function identity(x) { return x; }").tokens
        };
        assert_eq!(
            stripped.len(),
            js_tokens.len(),
            "Stripped TS with generics should match JS token count: stripped={}, js={}",
            stripped.len(),
            js_tokens.len()
        );
    }

    #[test]
    fn strip_types_removes_generic_type_arguments() {
        let stripped = tokenize_cross_language("const x = new Map<string, number>();");
        // <string, number> should be stripped
        let has_string_ident = stripped
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(n) if n == "string"));
        // "string" as a type argument should be removed, but "Map" should remain
        let has_map = stripped
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(n) if n == "Map"));
        assert!(has_map, "Map identifier should be preserved");
        // In strip mode the type args are removed
        assert!(
            !has_string_ident,
            "Type argument 'string' should be stripped"
        );
    }

    #[test]
    fn strip_types_removes_as_expression() {
        let stripped = tokenize_cross_language("const x = value as string;");
        let has_as = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::As)));
        assert!(!has_as, "'as' expression should be stripped");
    }

    #[test]
    fn strip_types_removes_satisfies_expression() {
        let stripped = tokenize_cross_language("const config = {} satisfies Config;");
        let has_satisfies = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Satisfies)));
        assert!(!has_satisfies, "'satisfies' expression should be stripped");
    }

    #[test]
    fn strip_types_ts_and_js_produce_identical_token_kinds() {
        let ts_code = r#"
function greet(name: string, age: number): string {
    const msg: string = `Hello ${name}`;
    if (age > 18) {
        return msg;
    }
    return "too young";
}
"#;
        let js_code = r#"
function greet(name, age) {
    const msg = `Hello ${name}`;
    if (age > 18) {
        return msg;
    }
    return "too young";
}
"#;
        let stripped = tokenize_cross_language(ts_code);
        let js_tokens = {
            let path = PathBuf::from("test.js");
            tokenize_file(&path, js_code).tokens
        };

        assert_eq!(
            stripped.len(),
            js_tokens.len(),
            "Stripped TS and JS should produce same number of tokens"
        );

        // Verify token kinds match one by one
        for (i, (ts_tok, js_tok)) in stripped.iter().zip(js_tokens.iter()).enumerate() {
            assert_eq!(
                ts_tok.kind, js_tok.kind,
                "Token {i} mismatch: TS={:?}, JS={:?}",
                ts_tok.kind, js_tok.kind
            );
        }
    }

    #[test]
    fn strip_types_removes_export_type_but_keeps_export_value() {
        let stripped =
            tokenize_cross_language("export type { Foo };\nexport { bar };\nexport const x = 1;");
        let export_count = stripped
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Export)))
            .count();
        // export type is stripped, but export { bar } and export const x = 1 remain
        assert_eq!(
            export_count, 2,
            "Should have 2 value exports, got {export_count}"
        );
    }

    // ── JSX/TSX tokenization ────────────────────────────────────

    #[test]
    fn tokenize_jsx_fragment() {
        let tokens = tokenize_tsx("const x = <><div>Hello</div></>;");
        // Fragments produce opening and closing bracket tokens
        let bracket_count = tokens
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
            bracket_count >= 4,
            "JSX fragment should produce bracket tokens, got {bracket_count}"
        );
    }

    #[test]
    fn tokenize_jsx_spread_attribute() {
        let tokens = tokenize_tsx("const x = <div {...props}>Hello</div>;");
        let has_spread = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Spread)));
        assert!(
            has_spread,
            "JSX spread attribute should produce spread operator"
        );
    }

    #[test]
    fn tokenize_jsx_expression_container() {
        let tokens = tokenize_tsx("const x = <div>{count > 0 ? 'yes' : 'no'}</div>;");
        let has_ternary = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Ternary)));
        assert!(
            has_ternary,
            "Expression in JSX should be tokenized (ternary)"
        );
    }

    // ── ES module patterns ──────────────────────────────────────

    #[test]
    fn tokenize_import_declaration() {
        let tokens = tokenize("import { foo, bar } from './module';");
        let has_import = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Import)));
        let has_from = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::From)));
        let has_source = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::StringLiteral(s) if s == "./module"));
        assert!(has_import, "Should contain import keyword");
        assert!(has_from, "Should contain from keyword");
        assert!(has_source, "Should contain module source string");
    }

    #[test]
    fn tokenize_export_default_declaration() {
        let tokens = tokenize("export default function() { return 42; }");
        let has_export = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Export)));
        let has_default = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Default)));
        assert!(has_export, "Should contain export keyword");
        assert!(has_default, "Should contain default keyword");
    }

    #[test]
    fn tokenize_export_all_declaration() {
        let tokens = tokenize("export * from './module';");
        let has_export = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Export)));
        let has_from = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::From)));
        let has_source = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::StringLiteral(s) if s == "./module"));
        assert!(has_export, "export * should have export keyword");
        assert!(has_from, "export * should have from keyword");
        assert!(has_source, "export * should have source string");
    }

    #[test]
    fn tokenize_dynamic_import() {
        let tokens = tokenize("const mod = await import('./module');");
        let has_import = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Import)));
        let has_await = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Await)));
        // Dynamic import() is an expression — no visit_import_expression override,
        // so no Import keyword is emitted (only static import declarations emit it).
        assert!(
            !has_import,
            "Dynamic import() should not produce Import keyword"
        );
        assert!(has_await, "Should contain await keyword");
    }

    // ── Edge cases ──────────────────────────────────────────────

    #[test]
    fn tokenize_only_comments() {
        let tokens = tokenize("// This is a comment\n/* block comment */\n");
        assert!(
            tokens.is_empty(),
            "File with only comments should produce no tokens"
        );
    }

    #[test]
    fn tokenize_deeply_nested_structure() {
        let code = "const x = { a: { b: { c: { d: { e: 1 } } } } };";
        let tokens = tokenize(code);
        let open_braces = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::OpenBrace)))
            .count();
        let close_braces = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::CloseBrace)))
            .count();
        assert_eq!(
            open_braces, close_braces,
            "Nested structure should have balanced braces"
        );
        assert!(
            open_braces >= 5,
            "Should have at least 5 levels of braces, got {open_braces}"
        );
    }

    #[test]
    fn tokenize_chained_method_calls_uses_point_spans() {
        let tokens = tokenize("arr.filter(x => x > 0).map(x => x * 2).reduce((a, b) => a + b, 0);");
        // Verify that call expression parentheses use point spans (not the full chain span).
        // The dots should be at point spans just after each object expression ends.
        let dots: Vec<&SourceToken> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::Dot)))
            .collect();
        assert!(
            dots.len() >= 3,
            "Chained calls should produce dots, got {}",
            dots.len()
        );
        // Point spans should be small (1 byte)
        for dot in &dots {
            assert_eq!(
                dot.span.end - dot.span.start,
                1,
                "Dot should use point span"
            );
        }
    }

    #[test]
    fn tokenize_expression_statement_appends_semicolon() {
        let tokens = tokenize("foo();");
        let last = tokens.last().unwrap();
        assert!(
            matches!(
                last.kind,
                TokenKind::Punctuation(PunctuationType::Semicolon)
                    | TokenKind::Punctuation(PunctuationType::CloseParen)
                    | TokenKind::Operator(OperatorType::Comma)
            ),
            "Expression statement should end with semicolon or related punctuation"
        );
        let has_semicolon = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::Semicolon)));
        assert!(
            has_semicolon,
            "Expression statement should produce a semicolon"
        );
    }

    #[test]
    fn tokenize_variable_declarator_with_no_initializer() {
        let tokens = tokenize("let x;");
        let has_let = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Let)));
        let has_x = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(n) if n == "x"));
        // Should NOT have an assign operator since there's no initializer
        let has_assign = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Assign)));
        assert!(has_let, "Should have let keyword");
        assert!(has_x, "Should have identifier x");
        assert!(
            !has_assign,
            "Uninitialized declarator should not have assign operator"
        );
    }

    #[test]
    fn tokenize_using_declaration_maps_to_const() {
        // TC39 `using` declaration should map to Const keyword
        let tokens = tokenize("{ using resource = getResource(); }");
        let has_const = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Const)));
        assert!(
            has_const,
            "`using` declaration should be mapped to Const keyword"
        );
    }

    #[test]
    fn tokenize_block_statement_produces_braces() {
        let tokens = tokenize("{ const x = 1; }");
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Punctuation(PunctuationType::OpenBrace)
        ));
        let last = tokens.last().unwrap();
        assert!(
            matches!(
                last.kind,
                TokenKind::Punctuation(PunctuationType::CloseBrace)
            ),
            "Block should end with close brace"
        );
    }

    #[test]
    fn tokenize_class_without_name_and_no_extends() {
        let tokens = tokenize("const C = class { };");
        let has_class = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Class)));
        let has_extends = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Extends)));
        assert!(has_class, "Should have class keyword");
        assert!(
            !has_extends,
            "Anonymous class without extends should not have extends keyword"
        );
    }

    #[test]
    fn tokenize_function_without_name() {
        let tokens = tokenize("const f = function() { return 1; };");
        let has_function = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Function)));
        assert!(has_function, "Should have function keyword");
    }

    #[test]
    fn tokenize_ts_interface_body_has_braces() {
        let tokens = tokenize("interface I { x: number; }");
        let open_braces = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::OpenBrace)))
            .count();
        let close_braces = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::CloseBrace)))
            .count();
        assert!(open_braces >= 1, "Interface body should have open brace");
        assert_eq!(
            open_braces, close_braces,
            "Interface body braces should be balanced"
        );
    }

    #[test]
    fn tokenize_ts_enum_body_has_braces() {
        let tokens = tokenize("enum E { A, B }");
        let open_braces = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::OpenBrace)))
            .count();
        assert!(open_braces >= 1, "Enum body should have open brace");
    }

    #[test]
    fn tokenize_ts_module_declaration_not_stripped_when_not_declare() {
        // A non-declare namespace should not be stripped even when strip_types is true
        let tokens = tokenize("namespace Foo { export const x = 1; }");
        let has_const = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Const)));
        assert!(
            has_const,
            "Non-declare namespace contents should be preserved"
        );
    }

    #[test]
    fn cross_language_preserves_non_declare_namespace() {
        let stripped = tokenize_cross_language("namespace Foo { export const x = 1; }");
        let has_const = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Const)));
        assert!(
            has_const,
            "Non-declare namespace contents should be preserved in cross-language mode"
        );
    }

    #[test]
    fn tokenize_for_statement_with_all_clauses() {
        let tokens = tokenize("for (let i = 0; i < 10; i++) { console.log(i); }");
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Keyword(KeywordType::For)
        ));
        let has_open_paren = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::OpenParen)));
        let has_close_paren = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::CloseParen)));
        assert!(has_open_paren, "For statement should have open paren");
        assert!(has_close_paren, "For statement should have close paren");
    }

    #[test]
    fn tokenize_cross_language_produces_correct_metadata() {
        let path = PathBuf::from("test.ts");
        let source = "const x: number = 1;\nconst y: string = 'hello';";
        let result = tokenize_file_cross_language(&path, source, true);
        assert_eq!(result.line_count, 2);
        assert_eq!(result.source, source);
        assert!(!result.tokens.is_empty());
    }

    #[test]
    fn strip_types_removes_complex_generics() {
        let stripped = tokenize_cross_language(
            "function merge<T extends object, U extends object>(a: T, b: U): T & U { return Object.assign(a, b); }",
        );
        let js_tokens = {
            let path = PathBuf::from("test.js");
            tokenize_file(
                &path,
                "function merge(a, b) { return Object.assign(a, b); }",
            )
            .tokens
        };
        assert_eq!(
            stripped.len(),
            js_tokens.len(),
            "Complex generics should be fully stripped: stripped={}, js={}",
            stripped.len(),
            js_tokens.len()
        );
    }

    #[test]
    fn tokenize_ts_conditional_type_without_strip() {
        let tokens = tokenize("type IsString<T> = T extends string ? true : false;");
        let has_type = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Type)));
        assert!(has_type, "Should contain type keyword");
        // The 'extends' in a conditional type is part of TSConditionalType AST,
        // not a class extends clause. The tokenizer walks the type which produces
        // identifiers (T, string) and the ternary operator/colon from the conditional.
        let has_true_bool = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::BooleanLiteral(true)));
        let has_false_bool = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::BooleanLiteral(false)));
        assert!(
            has_true_bool,
            "Conditional type should contain true literal"
        );
        assert!(
            has_false_bool,
            "Conditional type should contain false literal"
        );
    }

    #[test]
    fn strip_types_removes_conditional_type() {
        let stripped = tokenize_cross_language(
            "type IsString<T> = T extends string ? true : false;\nconst x = 1;",
        );
        let has_type = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Type)));
        assert!(!has_type, "Conditional type alias should be fully stripped");
    }

    #[test]
    fn tokenize_vue_sfc_with_cross_language_stripping() {
        let vue_source = r#"<template><div/></template>
<script lang="ts">
import type { Ref } from 'vue';
import { ref } from 'vue';
const count: Ref<number> = ref(0);
</script>"#;
        let path = PathBuf::from("Component.vue");
        let result = tokenize_file_cross_language(&path, vue_source, true);
        // import type should be stripped
        let import_count = result
            .tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Import)))
            .count();
        assert_eq!(
            import_count, 1,
            "import type should be stripped, leaving only 1 value import, got {import_count}"
        );
    }

    #[test]
    fn tokenize_no_extension_uses_default_source_type() {
        let path = PathBuf::from("Makefile");
        // Files without a recognized extension should still not panic
        let result = tokenize_file(&path, "const x = 1;");
        // May or may not produce tokens depending on how SourceType handles unknown extensions
        // The important thing is no panic
        assert!(result.line_count >= 1);
    }

    #[test]
    fn point_span_is_one_byte() {
        let span = point_span(42);
        assert_eq!(span.start, 42);
        assert_eq!(span.end, 43);
    }

    #[test]
    fn tokenize_call_expression_with_arguments() {
        let tokens = tokenize("foo(1, 'hello', true);");
        let has_open_paren = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::OpenParen)));
        let has_close_paren = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::CloseParen)));
        let comma_count = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Comma)))
            .count();
        assert!(has_open_paren, "Call should have open paren");
        assert!(has_close_paren, "Call should have close paren");
        assert!(
            comma_count >= 3,
            "3 arguments should produce at least 3 commas (one per arg), got {comma_count}"
        );
    }

    #[test]
    fn tokenize_new_expression_with_arguments() {
        let tokens = tokenize("new Foo(1, 2);");
        let has_new = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::New)));
        let comma_count = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Comma)))
            .count();
        assert!(has_new);
        assert!(
            comma_count >= 2,
            "2 arguments should produce at least 2 commas, got {comma_count}"
        );
    }

    #[test]
    fn tokenize_arrow_function_params_produce_commas() {
        let tokens = tokenize("const f = (a, b, c) => a;");
        let comma_count = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Comma)))
            .count();
        assert!(
            comma_count >= 3,
            "Arrow function with 3 params should produce at least 3 commas, got {comma_count}"
        );
    }

    #[test]
    fn tokenize_function_params_produce_commas() {
        let tokens = tokenize("function f(a, b) { return a + b; }");
        let comma_count = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Operator(OperatorType::Comma)))
            .count();
        assert!(
            comma_count >= 2,
            "Function with 2 params should produce at least 2 commas, got {comma_count}"
        );
    }

    #[test]
    fn tokenize_switch_with_open_close_parens() {
        let tokens = tokenize("switch (x) { case 1: break; }");
        let has_open_paren = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::OpenParen)));
        let has_close_paren = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::CloseParen)));
        assert!(
            has_open_paren,
            "Switch should have open paren for discriminant"
        );
        assert!(
            has_close_paren,
            "Switch should have close paren for discriminant"
        );
    }

    #[test]
    fn tokenize_while_has_parens_around_condition() {
        let tokens = tokenize("while (true) { break; }");
        let has_open_paren = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::OpenParen)));
        let has_close_paren = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::CloseParen)));
        assert!(has_open_paren, "While should have open paren");
        assert!(has_close_paren, "While should have close paren");
    }

    #[test]
    fn tokenize_for_in_has_parens() {
        let tokens = tokenize("for (const k in obj) {}");
        let open_parens = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::OpenParen)))
            .count();
        let close_parens = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::CloseParen)))
            .count();
        assert!(open_parens >= 1, "for-in should have open paren");
        assert!(close_parens >= 1, "for-in should have close paren");
    }

    #[test]
    fn tokenize_for_of_has_parens() {
        let tokens = tokenize("for (const v of arr) {}");
        let open_parens = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::OpenParen)))
            .count();
        let close_parens = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::CloseParen)))
            .count();
        assert!(open_parens >= 1, "for-of should have open paren");
        assert!(close_parens >= 1, "for-of should have close paren");
    }

    #[test]
    fn strip_types_removes_ts_type_annotation_colon() {
        // Verify that the colon from type annotations is also stripped
        let stripped = tokenize_cross_language("const x: number = 1;");
        let colon_count = stripped
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Punctuation(PunctuationType::Colon)))
            .count();
        assert_eq!(
            colon_count, 0,
            "Type annotation colons should be stripped, got {colon_count}"
        );
    }

    #[test]
    fn tokenize_ts_as_const() {
        let tokens = tokenize("const colors = ['red', 'green', 'blue'] as const;");
        let has_as = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::As)));
        assert!(has_as, "as const should produce 'as' keyword");
        // The declaration 'const' is emitted as a keyword; the 'const' in 'as const'
        // is visited as a TS type (TSTypeOperator), not as a keyword.
        let has_const_decl = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::Const)));
        assert!(
            has_const_decl,
            "Should have Const keyword for the declaration"
        );
    }

    #[test]
    fn strip_types_removes_as_const() {
        let stripped = tokenize_cross_language("const colors = ['red', 'green', 'blue'] as const;");
        let has_as = stripped
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Keyword(KeywordType::As)));
        assert!(
            !has_as,
            "'as const' should be stripped in cross-language mode"
        );
    }
}
