//! `History` reports the dirty set of the entry it installed (design P14,
//! 02 §5.1, §4.3; ARQ-30).

use std::time::Duration;

use gafira_doc::{Document, PageId, PageSize};
use gafira_edit::{
    Applied, CommandError, DirtySet, GestureId, History, InsertPage, Property, SetProperty,
};
use gafira_geom::{Length, Unit};

const NAME: &str = "page";

fn size() -> PageSize {
    PageSize {
        width: Length::new(10.0, Unit::Px),
        height: Length::new(10.0, Unit::Px),
    }
}

/// Appends one page called [`NAME`]; its id is known only after `apply`
/// (`touches` is empty before, XP4).
fn append_page(history: &mut History) -> PageId {
    let len = history.document().pages().len();
    let command = InsertPage::new(len, NAME.to_owned(), size());
    let applied = history.execute(Box::new(command), Duration::ZERO);
    assert_eq!(applied, Ok(Applied::Changed));
    history.document().pages()[len].id
}

fn rename(page: PageId, name: &str, gesture: Option<GestureId>) -> Box<SetProperty> {
    Box::new(SetProperty::page(
        page,
        Property::PageName(name.to_owned()),
        gesture,
    ))
}

fn pages(ids: &[PageId]) -> DirtySet {
    DirtySet {
        pages: ids.iter().copied().collect(),
        nodes: Default::default(),
    }
}

fn touches(history: &History) -> (Option<DirtySet>, Option<DirtySet>) {
    (history.newest_touches(), history.undone_touches())
}

/// Two pages added and the second undone: both stacks hold one entry.
fn both_stacks() -> (History, PageId, PageId) {
    let mut history = History::new(Document::default()).unwrap();
    let first = append_page(&mut history);
    let second = append_page(&mut history);
    assert!(history.undo().unwrap().is_some());
    (history, first, second)
}

// ARQ-30
#[test]
fn ac1_after_a_changed_execute_the_newest_set_is_the_applied_one() {
    let mut history = History::new(Document::default()).unwrap();
    let page = append_page(&mut history);
    assert_eq!(touches(&history), (Some(pages(&[page])), None));
}

// ARQ-30
#[test]
fn ac2_undo_reports_the_undone_entry_and_the_new_top() {
    let mut history = History::new(Document::default()).unwrap();
    let first = append_page(&mut history);
    let second = append_page(&mut history);

    assert!(history.undo().unwrap().is_some());
    assert_eq!(
        touches(&history),
        (Some(pages(&[first])), Some(pages(&[second])))
    );
    assert!(history.undo().unwrap().is_some());
    assert_eq!(touches(&history), (None, Some(pages(&[first]))));
}

// ARQ-30
#[test]
fn ac3_redo_reports_the_redone_entry_and_the_next_redo_entry() {
    let mut history = History::new(Document::default()).unwrap();
    let ids = [0; 3].map(|_| append_page(&mut history));
    while history.undo().unwrap().is_some() {}

    let expected = [
        (Some(pages(&[ids[0]])), Some(pages(&[ids[1]]))),
        (Some(pages(&[ids[1]])), Some(pages(&[ids[2]]))),
        (Some(pages(&[ids[2]])), None),
    ];
    for want in expected {
        assert!(history.redo().unwrap().is_some());
        assert_eq!(touches(&history), want);
    }
}

// ARQ-30
#[test]
fn ac4_a_merged_gesture_reports_the_merged_entry() {
    let mut history = History::new(Document::default()).unwrap();
    append_page(&mut history);
    let second = append_page(&mut history);

    let gesture = history.begin_gesture();
    for (i, name) in ["a", "b", "c"].into_iter().enumerate() {
        let at = Duration::from_secs(i as u64 + 1);
        let applied = history.execute(rename(second, name, Some(gesture)), at);
        assert_eq!(applied, Ok(Applied::Changed));
    }
    history.end_gesture();

    assert_eq!(history.len(), 3, "two inserts and one merged rename");
    assert_eq!(touches(&history), (Some(pages(&[second])), None));
}

// ARQ-30
#[test]
fn ac5_an_empty_history_reports_nothing() {
    let history = History::new(Document::default()).unwrap();
    assert_eq!(touches(&history), (None, None));
}

// ARQ-30
#[test]
fn ac5_an_unchanged_execute_leaves_both_sets() {
    let (mut history, first, second) = both_stacks();
    let applied = history.execute(rename(first, NAME, None), Duration::ZERO);
    assert_eq!(applied, Ok(Applied::Unchanged));
    assert_eq!(
        touches(&history),
        (Some(pages(&[first])), Some(pages(&[second])))
    );
}

// ARQ-30
#[test]
fn ac6_a_command_that_fails_to_apply_leaves_both_sets() {
    let (mut history, first, second) = both_stacks();
    let out_of_range = InsertPage::new(9, NAME.to_owned(), size());
    let result = history.execute(Box::new(out_of_range), Duration::ZERO);
    assert_eq!(
        result,
        Err(CommandError::IndexOutOfRange { index: 9, len: 1 })
    );
    assert_eq!(
        touches(&history),
        (Some(pages(&[first])), Some(pages(&[second])))
    );
}
