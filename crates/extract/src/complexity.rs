//! Cyclomatic and cognitive complexity computation via Oxc AST visitor.
//!
//! Computes both metrics in a single AST traversal using a function scope stack.
//! Each function/method/arrow gets its own independent complexity frame.
//!
//! **Cyclomatic complexity** (McCabe): 1 + number of decision points per function.
//! Counts `if`, `for`, `while`, `do`, `case`, `catch`, `?:`, `&&`, `||`, `??`,
//! `&&=`/`||=`/`??=`, and `?.`.
//!
//! **Cognitive complexity** (SonarSource): structural increments with nesting penalty.
//! Counts control flow breaks weighted by nesting depth. Boolean operator sequences
//! add +1 per operator kind change. Optional chaining (`?.`) is NOT counted (Principle 3).

use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_ast_visit::walk;
use oxc_semantic::ScopeFlags;
use oxc_span::Span;

use fallow_types::extract::FunctionComplexity;

/// Per-function state on the scope stack.
struct FunctionFrame {
    name: String,
    span: Span,
    cyclomatic: u16,
    cognitive: u16,
    nesting_level: u16,
    /// Track the last logical operator for cognitive boolean sequence detection.
    last_logical_operator: Option<LogicalOperator>,
}

/// AST visitor that computes per-function complexity metrics.
pub struct ComplexityVisitor {
    stack: Vec<FunctionFrame>,
    pub results: Vec<FunctionComplexity>,
    /// Line offsets for byte-offset to line/col conversion.
    line_offsets: Vec<u32>,
    /// Name override from a parent node (e.g., method name from `MethodDefinition`,
    /// variable name from `const foo = function() {}`).
    pending_name: Option<String>,
}

impl ComplexityVisitor {
    pub fn new(line_offsets: Vec<u32>) -> Self {
        Self {
            stack: Vec::new(),
            results: Vec::new(),
            line_offsets,
            pending_name: None,
        }
    }

    fn push_function(&mut self, name: String, span: Span) {
        self.stack.push(FunctionFrame {
            name,
            span,
            cyclomatic: 1, // base complexity
            cognitive: 0,
            nesting_level: 0,
            last_logical_operator: None,
        });
    }

    fn pop_function(&mut self) {
        if let Some(frame) = self.stack.pop() {
            let (line, col) = fallow_types::extract::byte_offset_to_line_col(
                &self.line_offsets,
                frame.span.start,
            );
            let end_line =
                fallow_types::extract::byte_offset_to_line_col(&self.line_offsets, frame.span.end)
                    .0;
            self.results.push(FunctionComplexity {
                name: frame.name,
                line,
                col,
                cyclomatic: frame.cyclomatic,
                cognitive: frame.cognitive,
                line_count: end_line.saturating_sub(line) + 1,
            });
        }
    }

    /// Increment cyclomatic complexity for the current function.
    fn inc_cyclomatic(&mut self) {
        if let Some(frame) = self.stack.last_mut() {
            frame.cyclomatic = frame.cyclomatic.saturating_add(1);
        }
    }

    /// Increment cognitive complexity: +1 structural + nesting penalty.
    fn inc_cognitive_with_nesting(&mut self) {
        if let Some(frame) = self.stack.last_mut() {
            frame.cognitive = frame.cognitive.saturating_add(1 + frame.nesting_level);
        }
    }

    /// Increment cognitive complexity: flat +1 (no nesting penalty).
    fn inc_cognitive_flat(&mut self) {
        if let Some(frame) = self.stack.last_mut() {
            frame.cognitive = frame.cognitive.saturating_add(1);
        }
    }

    /// Increase nesting level for the current function.
    fn inc_nesting(&mut self) {
        if let Some(frame) = self.stack.last_mut() {
            frame.nesting_level = frame.nesting_level.saturating_add(1);
        }
    }

    /// Decrease nesting level for the current function.
    fn dec_nesting(&mut self) {
        if let Some(frame) = self.stack.last_mut() {
            frame.nesting_level = frame.nesting_level.saturating_sub(1);
        }
    }

    /// Handle a logical expression for cognitive complexity.
    /// Sequences of the same operator get +1 total; each operator change adds +1.
    fn handle_logical_operator(&mut self, op: LogicalOperator) {
        if let Some(frame) = self.stack.last_mut() {
            match frame.last_logical_operator {
                None => {
                    // First operator in a sequence
                    frame.cognitive = frame.cognitive.saturating_add(1);
                    frame.last_logical_operator = Some(op);
                }
                Some(prev) if prev == op => {
                    // Same operator, no increment
                }
                Some(_) => {
                    // Operator changed
                    frame.cognitive = frame.cognitive.saturating_add(1);
                    frame.last_logical_operator = Some(op);
                }
            }
        }
    }

    /// Reset the logical operator tracking (end of a logical expression chain).
    fn reset_logical_operator(&mut self) {
        if let Some(frame) = self.stack.last_mut() {
            frame.last_logical_operator = None;
        }
    }

    /// Check if a node is the direct child of a `LogicalExpression`.
    /// Used to avoid resetting the logical operator tracker in the middle of a chain.
    fn is_nested_logical(expr: &Expression<'_>) -> bool {
        matches!(expr, Expression::LogicalExpression(_))
    }
}

impl<'a> Visit<'a> for ComplexityVisitor {
    // ── Function boundaries ─────────────────────────────────────

    fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
        // Prefer the function's own name (func.id) over pending_name from parent
        let name = func
            .id
            .as_ref()
            .map(|id| {
                self.pending_name.take(); // consume to avoid leaking
                id.name.to_string()
            })
            .or_else(|| self.pending_name.take())
            .unwrap_or_else(|| "<anonymous>".to_string());

        // Nested function increases enclosing scope's nesting
        let is_nested = !self.stack.is_empty();
        if is_nested {
            self.inc_nesting();
        }

        self.push_function(name, func.span);
        walk::walk_function(self, func, flags);
        self.pop_function();

        if is_nested {
            self.dec_nesting();
        }
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        let name = self
            .pending_name
            .take()
            .unwrap_or_else(|| "<arrow>".to_string());

        // Nested arrow increases enclosing scope's nesting
        let is_nested = !self.stack.is_empty();
        if is_nested {
            self.inc_nesting();
        }

        self.push_function(name, arrow.span);
        walk::walk_arrow_function_expression(self, arrow);
        self.pop_function();

        if is_nested {
            self.dec_nesting();
        }
    }

    // ── Name capture from parent nodes ──────────────────────────

    fn visit_method_definition(&mut self, method: &MethodDefinition<'a>) {
        // Capture method name for the inner Function node
        if let Some(name) = method.key.static_name() {
            self.pending_name = Some(name.to_string());
        }
        walk::walk_method_definition(self, method);
        self.pending_name = None;
    }

    fn visit_variable_declarator(&mut self, decl: &VariableDeclarator<'a>) {
        // Capture `const foo = function() {}` or `const foo = () => {}`
        if let Some(id) = decl.id.get_binding_identifier() {
            self.pending_name = Some(id.name.to_string());
        }
        walk::walk_variable_declarator(self, decl);
        self.pending_name = None;
    }

    fn visit_property_definition(&mut self, prop: &PropertyDefinition<'a>) {
        // Capture class property initializers: `foo = () => {}`
        if let Some(name) = prop.key.static_name() {
            self.pending_name = Some(name.to_string());
        }
        walk::walk_property_definition(self, prop);
        self.pending_name = None;
    }

    fn visit_object_property(&mut self, prop: &ObjectProperty<'a>) {
        // Capture object method shorthand: `{ foo() {} }` and object arrow properties: `{ foo: () => {} }`
        if let Some(name) = prop.key.static_name() {
            self.pending_name = Some(name.to_string());
        }
        walk::walk_object_property(self, prop);
        self.pending_name = None;
    }

    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        // Capture `export default function() {}` as "default"
        self.pending_name = Some("default".to_string());
        walk::walk_export_default_declaration(self, decl);
        self.pending_name = None;
    }

    // ── Structural complexity (both cyclomatic + cognitive) ──────

    fn visit_if_statement(&mut self, stmt: &IfStatement<'a>) {
        // Cyclomatic: +1 for each if (including else-if)
        self.inc_cyclomatic();

        // Cognitive: +1 + nesting for `if`, but `else if` gets flat +1
        // We check if this IfStatement is the alternate of its parent IfStatement
        // by tracking whether we're already inside an else-if chain.
        // Since we can't easily check parent, we always add with nesting for `if`.
        // The `else if` case is handled below when we see the alternate.
        self.inc_cognitive_with_nesting();

        // Visit the test expression
        self.visit_expression(&stmt.test);

        // Visit consequent with increased nesting
        self.inc_nesting();
        self.visit_statement(&stmt.consequent);
        self.dec_nesting();

        // Handle alternate (else / else-if)
        if let Some(alternate) = &stmt.alternate {
            match alternate {
                Statement::IfStatement(else_if) => {
                    // `else if`: cognitive gets flat +1 (no nesting penalty),
                    // but the recursive visit_if_statement will handle it.
                    // We DON'T call inc_cognitive_with_nesting again here —
                    // the recursive call does it. But we need to counteract it
                    // since else-if should be flat. We handle this by visiting
                    // the else-if without increasing nesting.
                    self.visit_if_statement(else_if);
                    // Note: cyclomatic +1 happens in the recursive call above.
                    // For cognitive, the recursive call adds +1 + nesting, but
                    // SonarSource says else-if should be flat +1. To fix this,
                    // we subtract the nesting penalty that was added.
                    if let Some(frame) = self.stack.last_mut() {
                        // Undo the nesting penalty from the recursive call
                        frame.cognitive = frame.cognitive.saturating_sub(frame.nesting_level);
                    }
                }
                _ => {
                    // `else`: cognitive gets flat +1
                    self.inc_cognitive_flat();
                    self.inc_nesting();
                    self.visit_statement(alternate);
                    self.dec_nesting();
                }
            }
        }
    }

    fn visit_for_statement(&mut self, stmt: &ForStatement<'a>) {
        self.inc_cyclomatic();
        self.inc_cognitive_with_nesting();
        if let Some(init) = &stmt.init {
            self.visit_for_statement_init(init);
        }
        if let Some(test) = &stmt.test {
            self.visit_expression(test);
        }
        if let Some(update) = &stmt.update {
            self.visit_expression(update);
        }
        self.inc_nesting();
        self.visit_statement(&stmt.body);
        self.dec_nesting();
    }

    fn visit_for_in_statement(&mut self, stmt: &ForInStatement<'a>) {
        self.inc_cyclomatic();
        self.inc_cognitive_with_nesting();
        self.visit_for_statement_left(&stmt.left);
        self.visit_expression(&stmt.right);
        self.inc_nesting();
        self.visit_statement(&stmt.body);
        self.dec_nesting();
    }

    fn visit_for_of_statement(&mut self, stmt: &ForOfStatement<'a>) {
        self.inc_cyclomatic();
        self.inc_cognitive_with_nesting();
        self.visit_for_statement_left(&stmt.left);
        self.visit_expression(&stmt.right);
        self.inc_nesting();
        self.visit_statement(&stmt.body);
        self.dec_nesting();
    }

    fn visit_while_statement(&mut self, stmt: &WhileStatement<'a>) {
        self.inc_cyclomatic();
        self.inc_cognitive_with_nesting();
        self.visit_expression(&stmt.test);
        self.inc_nesting();
        self.visit_statement(&stmt.body);
        self.dec_nesting();
    }

    fn visit_do_while_statement(&mut self, stmt: &DoWhileStatement<'a>) {
        self.inc_cyclomatic();
        self.inc_cognitive_with_nesting();
        self.inc_nesting();
        self.visit_statement(&stmt.body);
        self.dec_nesting();
        self.visit_expression(&stmt.test);
    }

    fn visit_switch_statement(&mut self, stmt: &SwitchStatement<'a>) {
        // Cognitive: +1 for the switch (not per-case), with nesting
        self.inc_cognitive_with_nesting();
        self.visit_expression(&stmt.discriminant);
        self.inc_nesting();
        for case in &stmt.cases {
            self.visit_switch_case(case);
        }
        self.dec_nesting();
    }

    fn visit_switch_case(&mut self, case: &SwitchCase<'a>) {
        // Cyclomatic: +1 per case (classic variant), not for default
        if case.test.is_some() {
            self.inc_cyclomatic();
        }
        // Cognitive: no increment per case (handled by switch)
        walk::walk_switch_case(self, case);
    }

    fn visit_catch_clause(&mut self, clause: &CatchClause<'a>) {
        self.inc_cyclomatic();
        self.inc_cognitive_with_nesting();
        self.inc_nesting();
        walk::walk_catch_clause(self, clause);
        self.dec_nesting();
    }

    // ── Conditional expression (ternary) ────────────────────────

    fn visit_conditional_expression(&mut self, expr: &ConditionalExpression<'a>) {
        self.inc_cyclomatic();
        self.inc_cognitive_with_nesting();
        self.visit_expression(&expr.test);
        self.inc_nesting();
        self.visit_expression(&expr.consequent);
        self.visit_expression(&expr.alternate);
        self.dec_nesting();
    }

    // ── Logical expressions ─────────────────────────────────────

    fn visit_logical_expression(&mut self, expr: &LogicalExpression<'a>) {
        // Cyclomatic: +1 per logical operator
        self.inc_cyclomatic();

        // Cognitive: sequence-based. Same operator = no increment, operator change = +1.
        self.handle_logical_operator(expr.operator);

        // Visit left side — if it's also a logical expression, the recursive call handles it
        self.visit_expression(&expr.left);

        // Visit right side
        self.visit_expression(&expr.right);

        // Reset tracker if the right side is NOT a logical expression
        // (meaning we've exited the chain)
        if !Self::is_nested_logical(&expr.right) {
            // Only reset if we're the outermost logical expression
            if !Self::is_nested_logical(&expr.left) {
                self.reset_logical_operator();
            }
        }
    }

    // ── Assignment expressions with logical operators ───────────

    fn visit_assignment_expression(&mut self, expr: &AssignmentExpression<'a>) {
        // Cyclomatic: +1 for &&=, ||=, ??=
        if matches!(
            expr.operator,
            AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalNullish
        ) {
            self.inc_cyclomatic();
        }
        walk::walk_assignment_expression(self, expr);
    }

    // ── Optional chaining ───────────────────────────────────────

    fn visit_chain_expression(&mut self, expr: &ChainExpression<'a>) {
        // Cyclomatic: +1 per optional chain link
        match &expr.expression {
            ChainElement::CallExpression(call) => {
                if call.optional {
                    self.inc_cyclomatic();
                }
            }
            ChainElement::StaticMemberExpression(member) => {
                if member.optional {
                    self.inc_cyclomatic();
                }
            }
            ChainElement::ComputedMemberExpression(member) => {
                if member.optional {
                    self.inc_cyclomatic();
                }
            }
            ChainElement::PrivateFieldExpression(field) => {
                if field.optional {
                    self.inc_cyclomatic();
                }
            }
            ChainElement::TSNonNullExpression(_) => {}
        }
        // Cognitive: NOT counted (Principle 3 — shorthand that reduces cognitive load)
        walk::walk_chain_expression(self, expr);
    }

    // ── Break/continue with label ───────────────────────────────

    fn visit_break_statement(&mut self, stmt: &BreakStatement<'a>) {
        if stmt.label.is_some() {
            self.inc_cognitive_flat();
        }
        walk::walk_break_statement(self, stmt);
    }

    fn visit_continue_statement(&mut self, stmt: &ContinueStatement<'a>) {
        if stmt.label.is_some() {
            self.inc_cognitive_flat();
        }
        walk::walk_continue_statement(self, stmt);
    }
}

/// Compute per-function complexity metrics from a parsed Oxc program.
pub fn compute_complexity(
    program: &Program<'_>,
    line_offsets: Vec<u32>,
) -> Vec<FunctionComplexity> {
    let mut visitor = ComplexityVisitor::new(line_offsets);

    // Push a module-level frame for top-level code
    // (we don't report this, but it serves as a catch-all for top-level expressions)
    visitor.visit_program(program);

    visitor.results
}

#[cfg(all(test, not(miri)))]
mod tests {
    use super::*;
    use fallow_types::extract::compute_line_offsets;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    fn analyze(source: &str) -> Vec<FunctionComplexity> {
        let allocator = Allocator::default();
        let source_type = SourceType::tsx();
        let parser_return = Parser::new(&allocator, source, source_type).parse();
        let line_offsets = compute_line_offsets(source);
        compute_complexity(&parser_return.program, line_offsets)
    }

    fn find_fn<'a>(results: &'a [FunctionComplexity], name: &str) -> &'a FunctionComplexity {
        results
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("function '{name}' not found in results: {results:?}"))
    }

    // ── Cyclomatic complexity ───────────────────────────────────

    #[test]
    fn empty_function_has_cyclomatic_1() {
        let results = analyze("function foo() {}");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 1);
    }

    #[test]
    fn if_statement_adds_1() {
        let results = analyze("function foo(x) { if (x) { return 1; } return 0; }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn if_else_if_else_adds_2() {
        let results = analyze(
            "function foo(x) { if (x > 0) { return 1; } else if (x < 0) { return -1; } else { return 0; } }",
        );
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 3); // 1 + if + else-if
    }

    #[test]
    fn for_loop_adds_1() {
        let results = analyze("function foo() { for (let i = 0; i < 10; i++) {} }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn while_loop_adds_1() {
        let results = analyze("function foo() { while (true) { break; } }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn switch_case_adds_per_case() {
        let results = analyze(
            "function foo(x) { switch (x) { case 1: break; case 2: break; default: break; } }",
        );
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 3); // 1 + case1 + case2 (default doesn't count)
    }

    #[test]
    fn catch_adds_1() {
        let results = analyze("function foo() { try { } catch (e) { } }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn ternary_adds_1() {
        let results = analyze("function foo(x) { return x ? 1 : 0; }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn logical_and_adds_1() {
        let results = analyze("function foo(a, b) { return a && b; }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn logical_or_adds_1() {
        let results = analyze("function foo(a, b) { return a || b; }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn nullish_coalescing_adds_1() {
        let results = analyze("function foo(a) { return a ?? 'default'; }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn logical_assignment_adds_1() {
        let results = analyze("function foo(a) { a &&= true; a ||= false; a ??= null; }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 4); // 1 + 3 assignments
    }

    #[test]
    fn do_while_adds_1() {
        let results = analyze("function foo() { do { } while (true); }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn for_of_adds_1() {
        let results = analyze("function foo(arr) { for (const x of arr) { } }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn for_in_adds_1() {
        let results = analyze("function foo(obj) { for (const k in obj) { } }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn optional_chaining_adds_1() {
        let results = analyze("function foo(obj) { return obj?.value; }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2); // 1 + ?.
    }

    #[test]
    fn optional_chaining_computed_member_adds_1() {
        let results = analyze("function foo(obj) { return obj?.[0]; }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2); // 1 + ?.[]
    }

    #[test]
    fn optional_chaining_not_cognitive() {
        // Optional chaining increments cyclomatic but NOT cognitive (Principle 3)
        let results = analyze("function foo(obj) { return obj?.a?.b?.c; }");
        let f = find_fn(&results, "foo");
        assert!(
            f.cyclomatic > 1,
            "optional chaining should increment cyclomatic"
        );
        assert_eq!(
            f.cognitive, 0,
            "optional chaining should NOT increment cognitive"
        );
    }

    #[test]
    fn complex_function_cyclomatic() {
        let results = analyze(
            r"function complex(x, y) {
                if (x > 0) {
                    for (let i = 0; i < x; i++) {
                        if (y && i > 5) {
                            return true;
                        }
                    }
                } else if (x < 0) {
                    while (y) {
                        y--;
                    }
                }
                return x ? true : false;
            }",
        );
        let f = find_fn(&results, "complex");
        // 1 + if + for + if + && + else-if + while + ternary = 8
        assert_eq!(f.cyclomatic, 8);
    }

    // ── Cognitive complexity ─────────────────────────────────────

    #[test]
    fn empty_function_has_cognitive_0() {
        let results = analyze("function foo() {}");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cognitive, 0);
    }

    #[test]
    fn simple_if_cognitive_1() {
        let results = analyze("function foo(x) { if (x) { return 1; } }");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cognitive, 1); // +1 for if (nesting=0)
    }

    #[test]
    fn nested_if_cognitive_with_nesting() {
        let results = analyze("function foo(x, y) { if (x) { if (y) { return 1; } } }");
        let f = find_fn(&results, "foo");
        // outer if: +1 (nesting=0)
        // inner if: +1+1 (nesting=1)
        assert_eq!(f.cognitive, 3);
    }

    #[test]
    fn if_else_cognitive() {
        let results = analyze("function foo(x) { if (x) { return 1; } else { return 0; } }");
        let f = find_fn(&results, "foo");
        // if: +1, else: +1 (flat)
        assert_eq!(f.cognitive, 2);
    }

    #[test]
    fn if_else_if_else_cognitive() {
        let results = analyze(
            "function foo(x) { if (x > 0) { return 1; } else if (x < 0) { return -1; } else { return 0; } }",
        );
        let f = find_fn(&results, "foo");
        // if: +1, else if: +1 (flat), else: +1 (flat)
        assert_eq!(f.cognitive, 3);
    }

    #[test]
    fn boolean_sequence_same_operator() {
        let results = analyze("function foo(a, b, c) { return a && b && c; }");
        let f = find_fn(&results, "foo");
        // Same operator throughout: +1
        assert_eq!(f.cognitive, 1);
    }

    #[test]
    fn boolean_sequence_mixed_operators() {
        let results = analyze("function foo(a, b, c) { return a && b || c; }");
        let f = find_fn(&results, "foo");
        // && then ||: +1 + +1 = 2
        assert_eq!(f.cognitive, 2);
    }

    #[test]
    fn for_loop_increases_nesting() {
        let results =
            analyze("function foo(arr) { for (const x of arr) { if (x) { return x; } } }");
        let f = find_fn(&results, "foo");
        // for: +1 (nesting=0), if: +1+1 (nesting=1)
        assert_eq!(f.cognitive, 3);
    }

    #[test]
    fn switch_cognitive_1() {
        let results = analyze("function foo(x) { switch (x) { case 1: break; case 2: break; } }");
        let f = find_fn(&results, "foo");
        // switch: +1 (not per-case)
        assert_eq!(f.cognitive, 1);
    }

    #[test]
    fn nested_function_resets_nesting() {
        let results = analyze(
            r"function outer(x) {
                if (x) {
                    const inner = () => {
                        if (x) { return 1; }
                    };
                }
            }",
        );
        let outer = find_fn(&results, "outer");
        let inner = find_fn(&results, "inner");
        // outer: if +1 (nesting=0)
        assert_eq!(outer.cognitive, 1);
        // inner: if +1 (nesting=0, reset for new function)
        assert_eq!(inner.cognitive, 1);
    }

    #[test]
    fn break_with_label_adds_1() {
        let results = analyze("function foo() { outer: for (;;) { break outer; } }");
        let f = find_fn(&results, "foo");
        // for: +1 cognitive, break label: +1 flat, for: +1 cyclomatic
        assert!(f.cognitive >= 2);
    }

    #[test]
    fn arrow_function_tracked() {
        let results = analyze("const foo = (x) => x > 0 ? 1 : 0;");
        assert!(!results.is_empty());
        let f = &results[0];
        assert_eq!(f.name, "foo"); // captures variable name
        assert_eq!(f.cyclomatic, 2); // 1 + ternary
    }

    #[test]
    fn line_count_computed() {
        let results =
            analyze("function foo() {\n  const a = 1;\n  const b = 2;\n  return a + b;\n}");
        let f = find_fn(&results, "foo");
        assert_eq!(f.line_count, 5);
    }

    #[test]
    fn deeply_nested_cognitive() {
        let results = analyze(
            r"function deep(a, b, c, d) {
                if (a) {           // +1 (n=0) = 1
                    for (;;) {     // +1+1 (n=1) = 3
                        if (b) {   // +1+2 (n=2) = 6
                            while (c) { // +1+3 (n=3) = 10
                                if (d) {} // +1+4 (n=4) = 15
                            }
                        }
                    }
                }
            }",
        );
        let f = find_fn(&results, "deep");
        assert_eq!(f.cognitive, 15);
    }

    // ── Function naming ─────────────────────────────────────────

    #[test]
    fn object_method_shorthand_named() {
        let results = analyze("const obj = { foo(x) { if (x) {} } };");
        let f = find_fn(&results, "foo");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn object_arrow_property_named() {
        let results = analyze("const obj = { bar: (x) => x ? 1 : 0 };");
        let f = find_fn(&results, "bar");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn class_method_named() {
        let results = analyze("class Foo { parse(x) { if (x) {} } }");
        let f = find_fn(&results, "parse");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn export_default_function_named() {
        let results = analyze("export default function() { if (true) {} }");
        let f = find_fn(&results, "default");
        assert_eq!(f.cyclomatic, 2);
    }

    #[test]
    fn export_default_named_function_keeps_name() {
        let results = analyze("export default function myFn() { if (true) {} }");
        let f = find_fn(&results, "myFn");
        assert_eq!(f.cyclomatic, 2);
    }
}
