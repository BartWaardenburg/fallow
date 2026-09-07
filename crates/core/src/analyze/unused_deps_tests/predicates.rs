#[test]
fn production_exclude_globset_is_reused_without_changing_matches() {
    let first = super::super::production_exclude_globset().expect("static patterns compile");
    let second = super::super::production_exclude_globset().expect("cached patterns remain valid");

    assert!(std::ptr::eq(first, second));
    assert!(first.is_match("src/app.test.ts"));
    assert!(first.is_match("vitest.config.ts"));
    assert!(!first.is_match("src/app.ts"));
}
