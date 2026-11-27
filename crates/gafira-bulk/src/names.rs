//! The names of the files a run writes (spec 104 §10.2): safe on every
//! system, and never one already taken.

use std::collections::BTreeSet;

/// The longest name, in characters, before its extension.
const LONGEST: usize = 120;

/// The names Windows keeps for its devices.
const RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// `name` as the stem of a file on any system: the characters a system
/// forbids and the controls as `_`, no space or dot at its ends, at most
/// 120 characters and never a device name of Windows. It may be empty.
#[must_use]
pub fn sanitize(name: &str) -> String {
    let end = |s: &str| s.trim_end_matches([' ', '.']).to_owned();
    let mapped: String = name
        .trim()
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let mut out = end(&mapped);
    if out.chars().count() > LONGEST {
        out = end(&out.chars().take(LONGEST).collect::<String>());
    }
    let stem = out.split('.').next().unwrap_or_default().to_lowercase();
    if RESERVED.contains(&stem.as_str()) {
        out.insert(0, '_');
    }
    out
}

/// The names taken in a folder, compared without case, as systems do.
#[derive(Clone, Debug, Default)]
pub struct Names {
    taken: BTreeSet<String>,
}

impl Names {
    /// Marks `file` (with its extension) as taken.
    pub fn take(&mut self, file: &str) {
        self.taken.insert(file.to_lowercase());
    }

    /// A free name for `stem` with `extension`: as it is, or with `-2`,
    /// `-3`… after the stem; it is taken from then on. An empty stem is `_`.
    pub fn unique(&mut self, stem: &str, extension: &str) -> String {
        let stem = if stem.is_empty() { "_" } else { stem };
        let named = |suffix: &str| format!("{stem}{suffix}.{extension}");
        let mut name = named("");
        let mut n = 2;
        while self.taken.contains(&name.to_lowercase()) {
            name = named(&format!("-{n}"));
            n += 1;
        }
        self.take(&name);
        name
    }
}
