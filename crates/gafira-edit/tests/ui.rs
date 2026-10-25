//! ARQ-30 evidence: the fixtures in `tests/ui/` act as a crate outside
//! `gafira-edit`. Mutating a `Document` without going through `History`
//! must not compile; an outside `Command` run by `History` must.

#[test]
fn mutation_needs_the_edit_capability() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass_outside_command.rs");
}
