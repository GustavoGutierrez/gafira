use egui::Vec2;
use gafira_doc::PageSize;

use super::*;

const P: PageId = PageId::new(901);

fn set(guides: &[(GuideAxis, f64, bool)]) -> GuideSet {
    GuideSet(
        guides
            .iter()
            .map(|(axis, px, locked)| guide(*axis, *px, *locked))
            .collect(),
    )
}

fn page(guides: GuideSet) -> Page {
    let size = PageSize {
        width: Length::new(1000.0, Unit::Px),
        height: Length::new(800.0, Unit::Px),
    };
    let mut page = Page::new(P, String::new(), size);
    page.guides = guides;
    page
}

/// The page drawn at (100, 50) with zoom 0.5: 500 × 400 on screen.
fn drawn() -> Rect {
    Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(500.0, 400.0))
}

const V: GuideAxis = GuideAxis::Vertical;
const H: GuideAxis = GuideAxis::Horizontal;

// S44-08: a guide is held within 4 screen px of its line, the nearest one
// first; a vertical one only along its page, a locked one never.
#[test]
fn s44_08_the_pointer_holds_the_nearest_unlocked_guide() {
    let page = page(set(&[
        (V, 200.0, false),
        (V, 210.0, false),
        (H, 100.0, false),
        (V, 600.0, true),
    ]));
    let hit = |x: f32, y: f32| hit(&page, 96.0, drawn(), 0.5, Pos2::new(x, y));
    // x = 100 + 200 × 0.5 = 200 and 205 for the second.
    assert_eq!(hit(201.0, 300.0), Some(0));
    assert_eq!(hit(204.0, 300.0), Some(1), "nearer the second");
    assert_eq!(hit(196.0, 300.0), Some(0), "4 px away");
    assert_eq!(hit(195.5, 300.0), None, "4.5 px away");
    assert_eq!(hit(201.0, 460.0), None, "below its page");
    // y = 50 + 100 × 0.5 = 100, anywhere along the row.
    assert_eq!(hit(900.0, 102.0), Some(2));
    assert_eq!(hit(400.0, 300.0), None, "x = 400 is the locked guide");
}

// S44-08: a guide in mm is drawn and held where its px are.
#[test]
fn s44_08_a_guide_in_mm_is_where_its_px_are() {
    let mm = Guide {
        axis: V,
        position: Length::new(25.4, Unit::Mm),
        locked: false,
    };
    assert_eq!(guide_px(&mm, 96.0), Some(96.0));
    let page = page(GuideSet(vec![mm]));
    assert_eq!(
        hit(&page, 96.0, drawn(), 0.5, Pos2::new(148.0, 300.0)),
        Some(0)
    );
}

// S44-09: create, move and delete give the new set; the others keep their
// order and a moved guide its axis and lock.
#[test]
fn s44_09_create_move_and_delete_a_guide() {
    let guides = set(&[(V, 200.0, false), (H, 100.0, true)]);
    assert_eq!(
        added(&guides, H, 37.5),
        set(&[(V, 200.0, false), (H, 100.0, true), (H, 37.5, false)])
    );
    assert_eq!(
        moved(&guides, 1, 300.0),
        set(&[(V, 200.0, false), (H, 300.0, true)])
    );
    assert_eq!(moved(&guides, 5, 300.0), guides, "no such guide");
    assert_eq!(removed(&guides, 0), set(&[(H, 100.0, true)]));
    assert_eq!(removed(&guides, 1), set(&[(V, 200.0, false)]));
    assert_eq!(removed(&guides, 2), guides);
}

// S44-09: the pointer in px of page on the guide's axis, at any zoom.
#[test]
fn s44_09_the_pointer_in_px_of_page() {
    let at = Pos2::new(350.0, 150.0);
    assert_eq!(page_coordinate(V, drawn(), 0.5, at), 500.0);
    assert_eq!(page_coordinate(H, drawn(), 0.5, at), 200.0);
    assert_eq!(page_coordinate(V, drawn(), 2.0, at), 125.0);
}

// S44-10: a horizontal guide comes from and goes back to the top ruler,
// a vertical one to the left ruler.
#[test]
fn s44_10_each_axis_has_its_ruler() {
    let area = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(800.0, 600.0));
    assert_eq!(
        strip(H, area),
        Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(800.0, RULER))
    );
    assert_eq!(
        strip(V, area),
        Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(RULER, 600.0))
    );
}

// S44-10: a guide dropped between pages goes to the nearest page.
#[test]
fn s44_10_the_page_of_a_row() {
    let a = PageId::new(1);
    let b = PageId::new(2);
    let pages = [
        (
            a,
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(10.0, 100.0)),
        ),
        (
            b,
            Rect::from_min_max(Pos2::new(0.0, 150.0), Pos2::new(10.0, 250.0)),
        ),
    ];
    assert_eq!(page_at(&pages, 50.0), Some(a));
    assert_eq!(page_at(&pages, 200.0), Some(b));
    assert_eq!(page_at(&pages, 120.0), Some(a), "nearer a");
    assert_eq!(page_at(&pages, 140.0), Some(b), "nearer b");
    assert_eq!(page_at(&pages, 900.0), Some(b));
    assert_eq!(page_at(&[], 10.0), None);
}

// S44-11: the ruler numbers multiples of 1, 2 or 5 × 10^k at least 64 px
// apart, from the page's corner.
#[test]
fn s44_11_ruler_ticks() {
    // Zoom 1: 100 px steps from 0 at x = 50.
    assert_eq!(
        ticks(50.0, 1.0, 0.0, 300.0),
        vec![(50.0, 0), (150.0, 100), (250.0, 200)]
    );
    // Zoom 0.5: 64 screen px is 128 page px, so 200.
    assert_eq!(
        ticks(0.0, 0.5, -150.0, 150.0),
        vec![(-100.0, -200), (0.0, 0), (100.0, 200)]
    );
    // Zoom 4: 16 page px, so 20.
    assert_eq!(ticks(0.0, 4.0, 0.0, 100.0), vec![(0.0, 0), (80.0, 20)]);
}
