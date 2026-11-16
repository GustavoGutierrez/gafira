//! The versions a project keeps (02 §7.1 «Versiones», D-71, spec 031): a
//! named version is the person's and is never removed; a save adds an
//! automatic one when the newest automatic one is more than
//! [`AUTO_EVERY_SECS`] old, and only the [`AUTO_KEPT`] newest automatic
//! ones stay.

use std::sync::Arc;

use gafira_doc::Document;
use gafira_ports::common::UnixTime;
use gafira_ports::repository::{Version, VersionId, VersionInfo, VersionKind};

/// How old the newest automatic version may get before a save adds another:
/// 30 min (02 §7.1, confirmed by D-71).
pub(crate) const AUTO_EVERY_SECS: i64 = 30 * 60;

/// How many automatic versions a project keeps: 20 (02 §7.1, D-71).
pub(crate) const AUTO_KEPT: usize = 20;

/// A version of `document` made now, with the next free id.
fn made(
    versions: &[Version],
    name: String,
    kind: VersionKind,
    document: &Arc<Document>,
    now: UnixTime,
) -> Version {
    let id = versions.iter().map(|v| v.info.id.0).max().unwrap_or(0) + 1;
    Version {
        info: VersionInfo {
            id: VersionId(id),
            name,
            kind,
            created: now,
            app: env!("CARGO_PKG_VERSION").to_owned(),
            schema: document.schema_version(),
        },
        document: Arc::clone(document),
    }
}

/// `versions` with a named version of `document` called `name` at the end.
pub(crate) fn with_named(
    versions: &[Version],
    name: &str,
    document: &Arc<Document>,
    now: UnixTime,
) -> Vec<Version> {
    let mut kept = versions.to_vec();
    kept.push(made(
        versions,
        name.trim().to_owned(),
        VersionKind::Named,
        document,
        now,
    ));
    kept
}

/// The versions a save of `document` at `now` writes: `versions`, plus an
/// automatic version of `document` when there is no automatic one yet or
/// the newest is more than [`AUTO_EVERY_SECS`] old, minus the oldest
/// automatic ones beyond [`AUTO_KEPT`]. Named ones always stay, in place.
pub(crate) fn on_save(
    versions: &[Version],
    document: &Arc<Document>,
    now: UnixTime,
) -> Vec<Version> {
    let newest_auto = versions
        .iter()
        .filter(|v| v.info.kind == VersionKind::Auto)
        .map(|v| v.info.created)
        .max();
    let due = newest_auto.is_none_or(|at| now.0.saturating_sub(at.0) > AUTO_EVERY_SECS);
    let mut kept = versions.to_vec();
    if due {
        kept.push(made(
            versions,
            String::new(),
            VersionKind::Auto,
            document,
            now,
        ));
    }
    let autos = kept
        .iter()
        .filter(|v| v.info.kind == VersionKind::Auto)
        .count();
    let mut drop = autos.saturating_sub(AUTO_KEPT);
    // The list runs oldest first, so the first automatic ones go.
    kept.retain(|v| {
        if drop > 0 && v.info.kind == VersionKind::Auto {
            drop -= 1;
            false
        } else {
            true
        }
    });
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_700_000_000;

    fn doc() -> Arc<Document> {
        Arc::new(Document::default())
    }

    fn kinds(versions: &[Version]) -> Vec<(u64, VersionKind)> {
        versions
            .iter()
            .map(|v| (v.info.id.0, v.info.kind))
            .collect()
    }

    #[test]
    fn s31_05_the_first_save_makes_an_automatic_version() {
        let versions = on_save(&[], &doc(), UnixTime(T0));
        assert_eq!(kinds(&versions), [(1, VersionKind::Auto)]);
        let info = &versions[0].info;
        assert_eq!(info.created, UnixTime(T0));
        assert_eq!(info.name, "");
        assert_eq!(info.app, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.schema, Document::SCHEMA_VERSION);
    }

    #[test]
    fn s31_05_within_30_minutes_no_new_automatic_version() {
        let first = on_save(&[], &doc(), UnixTime(T0));
        let at_30 = on_save(&first, &doc(), UnixTime(T0 + AUTO_EVERY_SECS));
        assert_eq!(kinds(&at_30), [(1, VersionKind::Auto)], "exactly 30 min");
        let after = on_save(&first, &doc(), UnixTime(T0 + AUTO_EVERY_SECS + 1));
        assert_eq!(
            kinds(&after),
            [(1, VersionKind::Auto), (2, VersionKind::Auto)]
        );
        assert_eq!(after[1].info.created, UnixTime(T0 + AUTO_EVERY_SECS + 1));
    }

    #[test]
    fn s31_05_a_named_version_does_not_count_as_the_last_automatic() {
        let named = with_named(&[], "  Final  ", &doc(), UnixTime(T0));
        assert_eq!(named[0].info.name, "Final");
        assert_eq!(named[0].info.kind, VersionKind::Named);
        let saved = on_save(&named, &doc(), UnixTime(T0 + 1));
        assert_eq!(
            kinds(&saved),
            [(1, VersionKind::Named), (2, VersionKind::Auto)]
        );
    }

    #[test]
    fn s31_05_only_the_20_newest_automatic_versions_stay_and_named_ones_never_go() {
        let mut versions = with_named(&[], "keep", &doc(), UnixTime(T0));
        for n in 1..=25 {
            versions = on_save(&versions, &doc(), UnixTime(T0 + n * (AUTO_EVERY_SECS + 1)));
        }
        let autos: Vec<u64> = versions
            .iter()
            .filter(|v| v.info.kind == VersionKind::Auto)
            .map(|v| v.info.id.0)
            .collect();
        assert_eq!(autos, (7..=26).collect::<Vec<_>>(), "the 20 newest");
        assert_eq!(kinds(&versions[..1]), [(1, VersionKind::Named)]);
        assert_eq!(versions.len(), 21);
    }

    #[test]
    fn s31_05_ids_are_never_reused_after_pruning() {
        let mut versions = Vec::new();
        for n in 0..=AUTO_KEPT as i64 {
            versions = on_save(&versions, &doc(), UnixTime(T0 + n * (AUTO_EVERY_SECS + 1)));
        }
        let named = with_named(&versions, "x", &doc(), UnixTime(T0));
        assert_eq!(named.last().expect("one").info.id, VersionId(22));
    }
}
