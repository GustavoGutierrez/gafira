//! How a number of a workbook is shown (S82-01).
//!
//! What is shown as a number reads back as that number with
//! `gafira_table::value`, so the two spellings it cannot tell apart are kept
//! out: a lone thousands sign goes, and three decimals get one more zero.

/// The marks of a language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Locale {
    /// `1234,5` in place of `1234.5`.
    pub comma: bool,
    pub yes: &'static str,
    pub no: &'static str,
}

/// The first serial past the 31st of December of 9999.
const END_OF_DATES: f64 = 2_958_466.0;

/// The most decimals shown.
const MAX_DECIMALS: usize = 15;

/// The longest code a workbook of Excel holds, in bytes of UTF-8.
const MAX_CODE: usize = 1024;

/// A piece of a format code.
#[derive(Clone, Debug, PartialEq)]
enum Token {
    /// `0`, `#` or `?`.
    Digit(char),
    Point,
    Comma,
    Percent,
    Literal(String),
    General,
    /// Letters of a date, all alike, in lower case.
    Date(char, usize),
    /// `[h]`, `[m]`, `[s]`: the whole time, not what is left of a day.
    Elapsed(char),
    /// `AM/PM` or `A/P`.
    Half,
    /// A date or time as the system writes it.
    SystemDate,
    SystemTime,
    /// `E+`: a number is shown in full instead.
    Scientific,
    /// `@`: the number as it is.
    Text,
}

/// A section of a format code.
#[derive(Clone, Debug, Default)]
struct Section {
    tokens: Vec<Token>,
    condition: Option<(String, f64)>,
}

/// `value` as the format `code` shows it.
#[must_use]
pub fn show(value: f64, code: &str, date1904: bool, locale: Locale) -> String {
    if !value.is_finite() {
        return String::new();
    }
    if code.is_empty() || code.len() > MAX_CODE || code.eq_ignore_ascii_case("general") {
        return general(value, locale);
    }
    let sections = sections(code);
    let Some((section, value)) = choose(&sections, value) else {
        return general(value, locale);
    };
    let is_date = section.tokens.iter().any(|t| {
        matches!(
            t,
            Token::Date(..) | Token::Elapsed(_) | Token::SystemDate | Token::SystemTime
        )
    });
    let shown = if is_date {
        date(value, &section.tokens, date1904)
    } else {
        Some(number(value, &section.tokens, locale))
    };
    shown.unwrap_or_else(|| general(value, locale))
}

/// The section that shows `value`, and the value it shows: a section of the
/// negative numbers writes its own sign.
fn choose(sections: &[Section], value: f64) -> Option<(&Section, f64)> {
    // The fourth section is the one of words.
    let sections = &sections[..sections.len().min(3)];
    if sections.iter().any(|s| s.condition.is_some()) {
        let holds = |s: &&Section| match &s.condition {
            None => true,
            Some((op, limit)) => match op.as_str() {
                "<" => value < *limit,
                "<=" => value <= *limit,
                ">" => value > *limit,
                ">=" => value >= *limit,
                "=" => value == *limit,
                _ => value != *limit,
            },
        };
        return sections.iter().find(holds).map(|s| (s, value));
    }
    match sections {
        [] => None,
        [all] => Some((all, value)),
        [_, negative, ..] if value < 0.0 => Some((negative, -value)),
        [_, _, zero] if value == 0.0 => Some((zero, value)),
        [positive, ..] => Some((positive, value)),
    }
}

/// The sections of a format code.
fn sections(code: &str) -> Vec<Section> {
    let mut all = vec![Section::default()];
    let chars: Vec<char> = code.chars().collect();
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        i += 1;
        let Some(section) = all.last_mut() else { break };
        let literal = |section: &mut Section, text: String| {
            section.tokens.push(Token::Literal(text));
        };
        match c {
            ';' => all.push(Section::default()),
            '"' => {
                let text: String = chars[i..].iter().take_while(|c| **c != '"').collect();
                i += text.chars().count() + 1;
                literal(section, text);
            }
            '\\' | '!' => {
                if let Some(next) = chars.get(i) {
                    literal(section, next.to_string());
                }
                i += 1;
            }
            // Room as wide as the next character.
            '_' => {
                literal(section, " ".to_owned());
                i += 1;
            }
            // A character repeated to fill the cell.
            '*' => i += 1,
            '[' => {
                let text: String = chars[i..].iter().take_while(|c| **c != ']').collect();
                i += text.chars().count() + 1;
                bracket(section, &text);
            }
            '0' | '#' | '?' => section.tokens.push(Token::Digit(c)),
            '.' => section.tokens.push(Token::Point),
            ',' => section.tokens.push(Token::Comma),
            '%' => section.tokens.push(Token::Percent),
            '@' => section.tokens.push(Token::Text),
            'E' | 'e' if matches!(chars.get(i), Some('+' | '-')) => {
                i += 1;
                section.tokens.push(Token::Scientific);
            }
            'G' | 'g' if starts(&chars[i - 1..], "general") => {
                i += 6;
                section.tokens.push(Token::General);
            }
            'A' | 'a' if starts(&chars[i - 1..], "am/pm") => {
                i += 4;
                section.tokens.push(Token::Half);
            }
            'A' | 'a' if starts(&chars[i - 1..], "a/p") => {
                i += 2;
                section.tokens.push(Token::Half);
            }
            'y' | 'Y' | 'm' | 'M' | 'd' | 'D' | 'h' | 'H' | 's' | 'S' | 'e' => {
                let letter = if c == 'e' {
                    'y'
                } else {
                    c.to_ascii_lowercase()
                };
                let more = chars[i..]
                    .iter()
                    .take_while(|n| n.eq_ignore_ascii_case(&c))
                    .count();
                i += more;
                let count = if c == 'e' { 4 } else { more + 1 };
                section.tokens.push(Token::Date(letter, count));
            }
            _ => literal(section, c.to_string()),
        }
    }
    all
}

fn starts(chars: &[char], word: &str) -> bool {
    chars.len() >= word.len()
        && chars
            .iter()
            .zip(word.chars())
            .all(|(a, b)| a.to_ascii_lowercase() == b)
}

/// What `[…]` says: an elapsed time, a currency, a condition or a colour.
fn bracket(section: &mut Section, text: &str) {
    let lower = text.to_ascii_lowercase();
    if let Some(rest) = text.strip_prefix('$') {
        let (sign, place) = rest.split_once('-').unwrap_or((rest, ""));
        match place.to_ascii_lowercase().as_str() {
            "f800" | "x-sysdate" => section.tokens.push(Token::SystemDate),
            "f400" | "x-systime" => section.tokens.push(Token::SystemTime),
            _ if !sign.is_empty() => section.tokens.push(Token::Literal(sign.to_owned())),
            _ => {}
        }
        return;
    }
    if let Some(letter @ ('h' | 'm' | 's')) = lower.chars().next()
        && lower.chars().all(|c| c == letter)
    {
        section.tokens.push(Token::Elapsed(letter));
        return;
    }
    let limit = text.trim_start_matches(['<', '>', '=']);
    let op = &text[..text.len() - limit.len()];
    if !op.is_empty()
        && let Ok(limit) = limit.trim().parse()
    {
        section.condition = Some((op.to_owned(), limit));
    }
}

/// `value` rounded to `decimals`, half away from zero as a spreadsheet does,
/// from its fifteen digits; `shift` moves the decimal point right first.
/// Its whole and decimal digits, without a sign.
fn rounded(value: f64, decimals: usize, shift: i64) -> (String, String) {
    let written = format!("{:.14e}", value.abs());
    let (digits, exponent) = written.split_once('e').unwrap_or((&written, "0"));
    let exponent = exponent.parse::<i64>().unwrap_or(0) + shift;
    let mut digits: Vec<u8> = digits
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|d| d - b'0')
        .collect();
    // How many of the digits stay.
    let keep = exponent + 1 + i64::try_from(decimals).unwrap_or(0);
    let mut exponent = exponent;
    if let Ok(keep) = usize::try_from(keep) {
        if keep < digits.len() {
            let up = digits[keep] >= 5;
            digits.truncate(keep);
            if up {
                let mut at = keep;
                loop {
                    if at == 0 {
                        digits.insert(0, 1);
                        exponent += 1;
                        break;
                    }
                    at -= 1;
                    if digits[at] < 9 {
                        digits[at] += 1;
                        break;
                    }
                    digits[at] = 0;
                }
            }
        }
    } else {
        digits.clear();
    }
    // The digit at a power of ten.
    let at = |power: i64| -> char {
        let digit = usize::try_from(exponent - power)
            .ok()
            .and_then(|i| digits.get(i))
            .copied()
            .unwrap_or(0);
        char::from(b'0' + digit)
    };
    let whole: String = (0..=exponent.max(0)).rev().map(at).collect();
    let decimals: String = (1..=i64::try_from(decimals).unwrap_or(0))
        .map(|n| at(-n))
        .collect();
    (whole, decimals)
}

/// One more zero where three decimals could be thousands.
fn unambiguous(whole: &str, decimals: &mut String) {
    if decimals.len() == 3 && whole.len() <= 3 && whole != "0" {
        decimals.push('0');
    }
}

/// `value` as a cell with no format shows it: in full, in the eleven
/// characters a spreadsheet gives it.
fn general(value: f64, locale: Locale) -> String {
    let size = value.abs();
    if size != 0.0 && !(1e-9..1e15).contains(&size) {
        let written = format!("{:.5e}", value);
        let (digits, exponent) = written.split_once('e').unwrap_or((&written, "0"));
        let digits = digits.trim_end_matches('0').trim_end_matches('.');
        let digits = if locale.comma {
            digits.replace('.', ",")
        } else {
            digits.to_owned()
        };
        let sign = if exponent.starts_with('-') { "" } else { "+" };
        return format!("{digits}E{sign}{exponent}");
    }
    let whole = rounded(value, 0, 0).0.len();
    let room = 10_usize.saturating_sub(whole).min(9);
    let (whole, mut decimals) = rounded(value, room, 0);
    decimals.truncate(decimals.trim_end_matches('0').len());
    unambiguous(&whole, &mut decimals);
    let zero = whole == "0" && decimals.is_empty();
    let sign = if value < 0.0 && !zero { "-" } else { "" };
    let point = match (decimals.is_empty(), locale.comma) {
        (true, _) => "",
        (false, true) => ",",
        (false, false) => ".",
    };
    format!("{sign}{whole}{point}{decimals}")
}

/// `value` as a section of digits shows it.
fn number(value: f64, tokens: &[Token], locale: Locale) -> String {
    let digit = |t: &Token| matches!(t, Token::Digit(_));
    // A fraction has digits on both sides of a bar.
    let fraction = tokens
        .iter()
        .position(|t| *t == Token::Literal("/".to_owned()))
        .is_some_and(|at| tokens[..at].iter().any(digit) && tokens[at..].iter().any(digit));
    let as_is = fraction
        || tokens
            .iter()
            .any(|t| matches!(t, Token::Scientific | Token::General | Token::Text));
    let Some(first) = tokens.iter().position(digit).filter(|_| !as_is) else {
        let mut out = String::new();
        let mut written = false;
        for token in tokens {
            match token {
                Token::General | Token::Text if !written => {
                    out.push_str(&general(value, locale));
                    written = true;
                }
                Token::Literal(text) if !fraction => out.push_str(text),
                Token::Percent => out.push('%'),
                _ => {}
            }
        }
        if as_is && !written {
            return general(value, locale);
        }
        return out.trim().to_owned();
    };
    let point = tokens.iter().position(|t| *t == Token::Point);
    let last = tokens.iter().rposition(digit).unwrap_or(first);
    let end = point.unwrap_or(last + 1);
    // `.00` has no digit before its point.
    let first = first.min(end);
    let grouped = tokens[first..end]
        .iter()
        .enumerate()
        .any(|(i, t)| *t == Token::Comma && tokens[first + i..end].iter().any(digit));
    let thousands = tokens[last + 1..]
        .iter()
        .chain(&tokens[first..end])
        .rev()
        .filter(|t| !matches!(t, Token::Literal(_) | Token::Percent))
        .take_while(|t| **t == Token::Comma)
        .count();
    let thousands = thousands.min(5);
    let percents = tokens
        .iter()
        .filter(|t| **t == Token::Percent)
        .count()
        .min(3);
    let shift =
        i64::try_from(percents * 2).unwrap_or(0) - i64::try_from(thousands * 3).unwrap_or(0);
    let places: Vec<char> = point
        .map(|at| &tokens[at..])
        .unwrap_or_default()
        .iter()
        .filter_map(|t| match t {
            Token::Digit(c) => Some(*c),
            _ => None,
        })
        .take(MAX_DECIMALS)
        .collect();
    let (whole, mut decimals) = rounded(value, places.len(), shift);
    // The places that show nothing in place of a last zero.
    while decimals.ends_with('0') && places.get(decimals.len() - 1) != Some(&'0') {
        decimals.pop();
    }
    let zero = whole.bytes().chain(decimals.bytes()).all(|d| d == b'0');
    let places = tokens[first..end].iter().filter(|t| digit(t)).count();
    let least = tokens[first..end]
        .iter()
        .position(|t| *t == Token::Digit('0'))
        .map_or(0, |at| {
            tokens[first + at..end].iter().filter(|t| digit(t)).count()
        });
    // `#` shows no zero; a decimal keeps one before its point.
    let whole = if whole == "0" && least == 0 && decimals.is_empty() {
        String::new()
    } else {
        format!("{whole:0>least$}")
    };
    unambiguous(&whole, &mut decimals);
    // Seven digits have two signs, which can only be of thousands.
    let grouped = grouped && (!decimals.is_empty() || whole.len() > 6);
    let mut out = String::new();
    let mut left = places;
    let mut written = 0;
    let (group, mark) = if locale.comma { ('.', ',') } else { (',', '.') };
    let put = |out: &mut String, count: usize, written: &mut usize| {
        for c in whole.chars().skip(*written).take(count) {
            out.push(c);
            *written += 1;
            let rest = whole.len() - *written;
            if grouped && rest > 0 && rest.is_multiple_of(3) {
                out.push(group);
            }
        }
    };
    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::Digit(_) if i < end => {
                // The first place takes the digits that have none.
                left -= 1;
                let count = whole.len().saturating_sub(left + written);
                put(&mut out, count, &mut written);
            }
            Token::Point if point == Some(i) => {
                put(&mut out, whole.len(), &mut written);
                if !decimals.is_empty() {
                    if out.is_empty() && places == 0 && whole.is_empty() {
                        out.push('0');
                    }
                    out.push(mark);
                    out.push_str(&decimals);
                }
            }
            Token::Point => out.push(mark),
            Token::Percent => out.push('%'),
            Token::Literal(text) => out.push_str(text),
            _ => {}
        }
    }
    let out = out.trim();
    // A section with only the sign of the positive ones gets its own.
    if value < 0.0 && !zero {
        format!("-{out}")
    } else {
        out.to_owned()
    }
}

/// The year, month and day of a day number; the 29th of February of 1900 is
/// the one a spreadsheet made up.
fn civil(day: i64, date1904: bool) -> (i64, i64, i64) {
    let day = if date1904 {
        day + 1462
    } else {
        match day {
            0 => return (1900, 1, 0),
            60 => return (1900, 2, 29),
            1..=59 => day + 1,
            _ => day,
        }
    };
    // Days from the first of March of the year 0: 1899-12-30 is 693 899.
    let z = day + 693_899;
    let era = z.div_euclid(146_097);
    let of_era = z.rem_euclid(146_097);
    let year = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year + year / 4 - year / 100);
    let month = (5 * of_year + 2) / 153;
    let d = of_year - (153 * month + 2) / 5 + 1;
    let m = if month < 10 { month + 3 } else { month - 9 };
    (year + era * 400 + i64::from(m <= 2), m, d)
}

/// `value` as a section of a date or a time shows it; `None` out of the
/// dates there are.
fn date(value: f64, tokens: &[Token], date1904: bool) -> Option<String> {
    if !(0.0..END_OF_DATES).contains(&value) {
        return None;
    }
    // The decimals of the seconds, `ss.00`.
    let places = tokens
        .iter()
        .position(|t| *t == Token::Point)
        .filter(|at| *at > 0 && matches!(tokens[at - 1], Token::Date('s', _) | Token::Elapsed('s')))
        .map_or(0, |at| {
            tokens[at + 1..]
                .iter()
                .take_while(|t| matches!(t, Token::Digit(_)))
                .count()
                .min(3)
        });
    let unit = [1_i64, 10, 100, 1000][places];
    // Below 2^53: the cast is exact.
    let total = (value * 86_400.0 * unit as f64).round() as i64;
    let (seconds, part) = (total / unit, total % unit);
    let (day, time) = (seconds / 86_400, seconds % 86_400);
    let (y, m, d) = civil(day, date1904);
    let (hour, minute, second) = (time / 3600, time / 60 % 60, time % 60);
    let half = tokens.contains(&Token::Half);
    let two = |n: i64, count: usize| {
        if count >= 2 {
            format!("{n:02}")
        } else {
            n.to_string()
        }
    };
    let mut out = String::new();
    // Whether the last piece was one of hours or seconds: `m` is minutes
    // next to them.
    let mut after_hour = false;
    let mut swallow = false;
    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::Literal(_) | Token::Comma if swallow => {}
            Token::Literal(text) => out.push_str(text),
            Token::Comma => out.push(','),
            // A bar between the parts; the point of the decimals is below.
            Token::Point if places > 0 => out.push_str(&format!(".{part:0places$}")),
            Token::Point => out.push('.'),
            Token::Half => out.push_str(if hour < 12 { "AM" } else { "PM" }),
            Token::SystemDate => {
                out.push_str(&format!("{y:04}-{m:02}-{d:02}"));
                return Some(out.trim().to_owned());
            }
            Token::SystemTime => {
                out.push_str(&format!("{hour}:{minute:02}:{second:02}"));
                return Some(out.trim().to_owned());
            }
            Token::Elapsed(letter) => {
                swallow = false;
                after_hour = *letter == 'h';
                let n = match letter {
                    'h' => seconds / 3600,
                    'm' => seconds / 60,
                    _ => seconds,
                };
                out.push_str(&n.to_string());
            }
            Token::Date(letter, count) => {
                let before_second = tokens[i + 1..]
                    .iter()
                    .find(|t| matches!(t, Token::Date(..) | Token::Elapsed(_)))
                    .is_some_and(|t| matches!(t, Token::Date('s', _) | Token::Elapsed('s')));
                swallow = false;
                match letter {
                    'y' if *count <= 2 => out.push_str(&format!("{:02}", y % 100)),
                    'y' => out.push_str(&format!("{y:04}")),
                    'm' if *count <= 2 && (after_hour || before_second) => {
                        out.push_str(&two(minute, *count));
                    }
                    'm' => out.push_str(&two(m, *count)),
                    // The name of the day says nothing the date does not.
                    'd' if *count > 2 => swallow = true,
                    'd' => out.push_str(&two(d, *count)),
                    'h' if half => out.push_str(&two((hour + 11) % 12 + 1, *count)),
                    'h' => out.push_str(&two(hour, *count)),
                    _ => out.push_str(&two(second, *count)),
                }
                after_hour = *letter == 'h';
            }
            _ => {}
        }
    }
    Some(out.trim().to_owned())
}
