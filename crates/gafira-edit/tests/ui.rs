//! ARQ-30 evidence: the fixtures in `tests/ui/` act as a crate outside
//! `gafira-edit`. Mutating a `Document` without going through `History`
//! must not compile; an outside `Command` run by `History` must.

#[test]
fn mutation_needs_the_edit_capability() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass_outside_command.rs");
    cases.compile_fail("tests/ui/fail_history_document_mut.rs");
    cases.compile_fail("tests/ui/fail_assign_through_history.rs");
    cases.compile_fail("tests/ui/fail_assign_through_ref.rs");
    cases.compile_fail("tests/ui/fail_edit_without_token.rs");
    cases.compile_fail("tests/ui/fail_edit_on_shared_ref.rs");
    cases.compile_fail("tests/ui/fail_write_private_field.rs");
    cases.compile_fail("tests/ui/fail_mut_page_from_shared_doc.rs");
    cases.compile_fail("tests/ui/fail_mut_node_from_shared_page.rs");
}
