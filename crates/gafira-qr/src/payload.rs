//! The text a code encodes, by kind of content (S80-03, S80-04).

use std::fmt::Write as _;

use gafira_doc::{QrCoin, QrContact, QrContactFormat, QrContent, QrWifiSecurity};

/// A field of a content, for its message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Url,
    Text,
    Email,
    Phone,
    WorkPhone,
    Ssid,
    Password,
    Name,
    Title,
    Start,
    End,
    Latitude,
    Longitude,
    Address,
    Amount,
}

/// What is wrong with a content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentError {
    Empty(Field),
    Invalid(Field),
}

/// Everything wrong with `content`, one a field, for the form (S80-17).
#[must_use]
pub fn problems(content: &QrContent) -> Vec<ContentError> {
    use Field as F;
    let checks: Vec<Result<(), ContentError>> = match content {
        QrContent::Url { url } | QrContent::Social { url } => vec![address(url).map(drop)],
        QrContent::Text { text } => vec![needed(text, F::Text).map(drop)],
        QrContent::Email { address, .. } => vec![mailbox(address).map(drop)],
        QrContent::Phone { number } | QrContent::Sms { number, .. } => {
            vec![phone(number, F::Phone).map(drop)]
        }
        QrContent::Wifi {
            ssid,
            password,
            security,
            ..
        } => vec![
            if ssid.is_empty() {
                Err(ContentError::Empty(F::Ssid))
            } else {
                Ok(())
            },
            wifi_key(password, *security),
        ],
        QrContent::Contact(c) => {
            // The optional fields are checked when they hold something.
            let optional = |text: &str, check: &dyn Fn(&str) -> Result<String, ContentError>| {
                if text.trim().is_empty() {
                    Ok(())
                } else {
                    check(text).map(drop)
                }
            };
            vec![
                shown_name(c).map(drop),
                optional(&c.phone, &|t| phone(t, F::Phone)),
                optional(&c.work_phone, &|t| phone(t, F::WorkPhone)),
                optional(&c.email, &|t| mailbox(t).map(str::to_owned)),
                optional(&c.url, &address),
            ]
        }
        QrContent::Event {
            title, start, end, ..
        } => {
            let (from, to) = (moment(start, F::Start), moment(end, F::End));
            let order = match (&from, &to) {
                (Ok(from), Ok(to)) if to < from => Err(ContentError::Invalid(F::End)),
                _ => Ok(()),
            };
            vec![
                needed(title, F::Title).map(drop),
                from.map(drop),
                to.map(drop),
                order,
            ]
        }
        QrContent::Location {
            latitude,
            longitude,
        } => vec![
            degrees(latitude, 90.0, F::Latitude).map(drop),
            degrees(longitude, 180.0, F::Longitude).map(drop),
        ],
        QrContent::Crypto {
            coin,
            address,
            amount,
        } => vec![
            wallet(address).map(drop),
            if *coin == QrCoin::Ethereum {
                Ok(())
            } else {
                coins(amount).map(drop)
            },
        ],
    };
    checks.into_iter().filter_map(Result::err).collect()
}

/// The text of `content`, ready to encode.
pub fn payload(content: &QrContent) -> Result<String, ContentError> {
    if let Some(first) = problems(content).into_iter().next() {
        return Err(first);
    }
    match content {
        QrContent::Url { url } | QrContent::Social { url } => address(url),
        QrContent::Text { text } => {
            if text.trim().is_empty() {
                return Err(ContentError::Empty(Field::Text));
            }
            Ok(text.clone())
        }
        QrContent::Email {
            address,
            subject,
            body,
        } => email(address, subject, body),
        QrContent::Phone { number } => Ok(format!("tel:{}", phone(number, Field::Phone)?)),
        QrContent::Sms { number, text } => {
            Ok(format!("SMSTO:{}:{text}", phone(number, Field::Phone)?))
        }
        QrContent::Wifi {
            ssid,
            password,
            security,
            hidden,
        } => wifi(ssid, password, *security, *hidden),
        QrContent::Contact(contact) => card(contact),
        QrContent::Event {
            title,
            place,
            start,
            end,
        } => event(title, place, start, end),
        QrContent::Location {
            latitude,
            longitude,
        } => {
            let lat = degrees(latitude, 90.0, Field::Latitude)?;
            let lon = degrees(longitude, 180.0, Field::Longitude)?;
            Ok(format!("geo:{lat},{lon}"))
        }
        QrContent::Crypto {
            coin,
            address,
            amount,
        } => crypto(*coin, address, amount),
    }
}

/// `text` without the blanks around it, which must hold something.
fn needed(text: &str, field: Field) -> Result<&str, ContentError> {
    let text = text.trim();
    if text.is_empty() {
        Err(ContentError::Empty(field))
    } else {
        Ok(text)
    }
}

/// Schemes that need no host.
const PLAIN_SCHEMES: [&str; 8] = [
    "mailto", "tel", "sms", "smsto", "geo", "bitcoin", "magnet", "urn",
];
/// Schemes whose address starts with a host.
const HOST_SCHEMES: [&str; 5] = ["http", "https", "ftp", "ftps", "sftp"];

fn address(url: &str) -> Result<String, ContentError> {
    let url = needed(url, Field::Url)?;
    let wrong = Err(ContentError::Invalid(Field::Url));
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return wrong;
    }
    if let Some((scheme, rest)) = url.split_once("://") {
        let scheme = scheme.to_ascii_lowercase();
        let right = if HOST_SCHEMES.contains(&scheme.as_str()) {
            has_host(rest)
        } else {
            // `htp://` is a slip, `maps://` the address of an app.
            is_scheme(&scheme)
                && !rest.is_empty()
                && ["http", "https"].iter().all(|of| slips(&scheme, of) > 1)
        };
        return if right { Ok(url.to_owned()) } else { wrong };
    }
    // `mailto:…` has a scheme; `example.com:8080` has a port.
    if let Some((scheme, rest)) = url.split_once(':')
        && is_scheme(scheme)
        && !scheme.contains('.')
        && !rest.is_empty()
    {
        let scheme = scheme.to_ascii_lowercase();
        let known = PLAIN_SCHEMES.contains(&scheme.as_str());
        let port = rest.starts_with(|c: char| c.is_ascii_digit());
        if HOST_SCHEMES.contains(&scheme.as_str()) {
            return wrong;
        }
        if known || (!port && !rest.starts_with('/')) {
            return Ok(url.to_owned());
        }
    }
    if has_host(url) {
        Ok(format!("https://{url}"))
    } else {
        wrong
    }
}

/// Whether `text` (an address after its scheme) starts with a host that can
/// exist, and a port in range if it has one.
fn has_host(text: &str) -> bool {
    let end = text.find(['/', '?', '#']).unwrap_or(text.len());
    let authority = &text[..end];
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let (host, port) = match host.strip_prefix('[') {
        // An IPv6 address: hexadecimal groups and colons.
        Some(rest) => {
            let Some((inside, after)) = rest.split_once(']') else {
                return false;
            };
            let groups = inside.contains(':')
                && inside
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.');
            if !groups {
                return false;
            }
            if after.is_empty() {
                return true;
            }
            ("localhost", after.strip_prefix(':').or(Some("")))
        }
        None => match host.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (host, None),
        },
    };
    port.is_none_or(|port| port.parse::<u16>().is_ok_and(|p| p > 0))
        && (host.eq_ignore_ascii_case("localhost") || is_domain(host))
}

/// Whether `host` is an IPv4 address or a name with a top-level domain.
fn is_domain(host: &str) -> bool {
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() == 4 && labels.iter().all(|l| l.parse::<u8>().is_ok()) {
        return true;
    }
    let well_formed = labels.iter().all(|label| {
        (1..=63).contains(&label.len())
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_alphanumeric() || c == '-')
    });
    let top = labels.last().copied().unwrap_or_default();
    labels.len() >= 2
        && well_formed
        && top.chars().count() >= 2
        && !top.chars().all(|c| c.is_ascii_digit())
}

/// How many letters to add, take or change to turn `a` into `b`.
fn slips(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.chars().enumerate() {
        let mut corner = row[0];
        row[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let kept = corner + usize::from(x != *y);
            corner = row[j + 1];
            row[j + 1] = kept.min(row[j] + 1).min(corner + 1);
        }
    }
    row[b.len()]
}

fn is_scheme(text: &str) -> bool {
    text.starts_with(|c: char| c.is_ascii_alphabetic())
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
}

/// An email address.
fn mailbox(address: &str) -> Result<&str, ContentError> {
    let address = needed(address, Field::Email)?;
    let well_formed = address.split_once('@').is_some_and(|(name, host)| {
        !name.is_empty()
            && !name.starts_with('.')
            && !name.ends_with('.')
            && !name.contains("..")
            && is_domain(host)
    }) && !address
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || "?&#,;:<>\"()[]".contains(c));
    if well_formed {
        Ok(address)
    } else {
        Err(ContentError::Invalid(Field::Email))
    }
}

fn email(address: &str, subject: &str, body: &str) -> Result<String, ContentError> {
    let mut out = format!("mailto:{}", mailbox(address)?);
    let mut mark = '?';
    for (name, value) in [("subject", subject), ("body", body)] {
        if !value.is_empty() {
            let _ = write!(out, "{mark}{name}={}", percent(value));
            mark = '&';
        }
    }
    Ok(out)
}

/// RFC 3986 percent-encoding; a line break is CRLF, as RFC 6068 asks.
fn percent(text: &str) -> String {
    let mut out = String::new();
    for byte in text.replace("\r\n", "\n").replace('\n', "\r\n").bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

fn phone(number: &str, field: Field) -> Result<String, ContentError> {
    let number = needed(number, field)?;
    let plain: String = number
        .chars()
        .filter(|c| !c.is_whitespace() && !"()-.".contains(*c))
        .collect();
    let digits = plain.strip_prefix('+').unwrap_or(&plain);
    if !(3..=15).contains(&digits.len()) || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(ContentError::Invalid(field));
    }
    Ok(plain)
}

/// `text` with a backslash before each of `reserved`.
fn escaped(text: &str, reserved: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if reserved.contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The key of a network: WPA takes 8 to 63 characters, or 64 hexadecimal
/// digits.
fn wifi_key(key: &str, security: QrWifiSecurity) -> Result<(), ContentError> {
    if security == QrWifiSecurity::Open {
        return Ok(());
    }
    if key.is_empty() {
        return Err(ContentError::Empty(Field::Password));
    }
    let length = key.chars().count();
    let hexadecimal = length == 64 && key.chars().all(|c| c.is_ascii_hexdigit());
    if security == QrWifiSecurity::Wpa && !(8..=63).contains(&length) && !hexadecimal {
        return Err(ContentError::Invalid(Field::Password));
    }
    Ok(())
}

fn wifi(
    ssid: &str,
    password: &str,
    security: QrWifiSecurity,
    hidden: bool,
) -> Result<String, ContentError> {
    // Blanks may be part of a name or a key: nothing is trimmed.
    if ssid.is_empty() {
        return Err(ContentError::Empty(Field::Ssid));
    }
    let reserved = "\\;,:\"";
    let kind = match security {
        QrWifiSecurity::Open => "nopass",
        QrWifiSecurity::Wep => "WEP",
        QrWifiSecurity::Wpa => "WPA",
    };
    let mut out = format!("WIFI:T:{kind};S:{};", escaped(ssid, reserved));
    if security != QrWifiSecurity::Open {
        wifi_key(password, security)?;
        let _ = write!(out, "P:{};", escaped(password, reserved));
    }
    if hidden {
        out.push_str("H:true;");
    }
    out.push(';');
    Ok(out)
}

/// A value of a vCard or a calendar (RFC 2426, RFC 5545).
fn line(text: &str) -> String {
    escaped(&text.trim().replace("\r\n", "\n"), "\\;,").replace('\n', "\\n")
}

/// The name a contact is shown with: its own, or its organisation's.
fn shown_name(c: &QrContact) -> Result<String, ContentError> {
    let own = [c.first_name.trim(), c.last_name.trim()];
    if own.iter().all(|part| part.is_empty()) {
        return needed(&c.organization, Field::Name).map(str::to_owned);
    }
    let parts: Vec<&str> = own.into_iter().filter(|part| !part.is_empty()).collect();
    Ok(parts.join(" "))
}

fn card(c: &QrContact) -> Result<String, ContentError> {
    let shown = shown_name(c)?;
    let place = [&c.street, &c.city, &c.region, &c.postcode, &c.country];
    let has_place = place.iter().any(|part| !part.trim().is_empty());
    match c.format {
        QrContactFormat::Vcard => {
            let mut out = format!(
                "BEGIN:VCARD\r\nVERSION:3.0\r\nN:{};{};;;\r\nFN:{}\r\n",
                line(&c.last_name),
                line(&c.first_name),
                line(&shown)
            );
            for (name, value) in [
                ("ORG", &c.organization),
                ("TITLE", &c.job),
                ("TEL;TYPE=CELL", &c.phone),
                ("TEL;TYPE=WORK", &c.work_phone),
                ("EMAIL;TYPE=INTERNET", &c.email),
                ("URL", &c.url),
            ] {
                if !value.trim().is_empty() {
                    let _ = write!(out, "{name}:{}\r\n", line(value));
                }
            }
            if has_place {
                let parts = place.map(|part| line(part)).join(";");
                let _ = write!(out, "ADR:;;{parts}\r\n");
            }
            out.push_str("END:VCARD\r\n");
            Ok(out)
        }
        QrContactFormat::Mecard => {
            let value = |text: &str| escaped(&text.trim().replace(['\r', '\n'], " "), "\\;,:\"");
            let name = match (c.last_name.trim(), c.first_name.trim()) {
                ("", "") => value(&shown),
                ("", first) => value(first),
                (last, "") => value(last),
                (last, first) => format!("{},{}", value(last), value(first)),
            };
            let mut out = format!("MECARD:N:{name};");
            for (name, text) in [
                ("ORG", &c.organization),
                ("TEL", &c.phone),
                ("TEL", &c.work_phone),
                ("EMAIL", &c.email),
                ("URL", &c.url),
            ] {
                if !text.trim().is_empty() {
                    let _ = write!(out, "{name}:{};", value(text));
                }
            }
            if has_place {
                let parts = place.map(|part| value(part)).join(",");
                let _ = write!(out, "ADR:,,{parts};");
            }
            out.push(';');
            Ok(out)
        }
    }
}

/// `YYYY-MM-DD HH:MM` (or with a `T`) as `YYYYMMDDTHHMM00`.
fn moment(text: &str, field: Field) -> Result<String, ContentError> {
    let text = needed(text, field)?;
    let wrong = ContentError::Invalid(field);
    let b = text.as_bytes();
    let shaped = b.len() == 16
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            10 => *c == b' ' || *c == b'T',
            13 => *c == b':',
            _ => c.is_ascii_digit(),
        });
    if !shaped {
        return Err(wrong);
    }
    let number = |from: usize, to: usize| text[from..to].parse::<u32>().unwrap_or(0);
    let (year, month, day) = (number(0, 4), number(5, 7), number(8, 10));
    let (hour, minute) = (number(11, 13), number(14, 16));
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if !(1..=days).contains(&day) || hour > 23 || minute > 59 {
        return Err(wrong);
    }
    Ok(format!(
        "{year:04}{month:02}{day:02}T{hour:02}{minute:02}00"
    ))
}

fn event(title: &str, place: &str, start: &str, end: &str) -> Result<String, ContentError> {
    let title = needed(title, Field::Title)?;
    let start = moment(start, Field::Start)?;
    let end = moment(end, Field::End)?;
    if end < start {
        return Err(ContentError::Invalid(Field::End));
    }
    let mut out = format!(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Gafira//QR//ES\r\nBEGIN:VEVENT\r\nSUMMARY:{}\r\n",
        line(title)
    );
    if !place.trim().is_empty() {
        let _ = write!(out, "LOCATION:{}\r\n", line(place));
    }
    let _ = write!(
        out,
        "DTSTART:{start}\r\nDTEND:{end}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
    );
    Ok(out)
}

/// Degrees with a decimal point, in `-limit..=limit`.
fn degrees(text: &str, limit: f64, field: Field) -> Result<String, ContentError> {
    let text = needed(text, field)?.replace(',', ".");
    match text.parse::<f64>() {
        Ok(value) if (-limit..=limit).contains(&value) => Ok(format!("{value}")),
        _ => Err(ContentError::Invalid(field)),
    }
}

fn wallet(address: &str) -> Result<&str, ContentError> {
    let address = needed(address, Field::Address)?;
    if address.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(address)
    } else {
        Err(ContentError::Invalid(Field::Address))
    }
}

/// An amount with a decimal point; `None` when it was left empty.
fn coins(amount: &str) -> Result<Option<String>, ContentError> {
    let amount = amount.trim().replace(',', ".");
    if amount.is_empty() {
        return Ok(None);
    }
    match amount.parse::<f64>() {
        Ok(value) if value.is_finite() && value > 0.0 => Ok(Some(amount)),
        _ => Err(ContentError::Invalid(Field::Amount)),
    }
}

fn crypto(coin: QrCoin, address: &str, amount: &str) -> Result<String, ContentError> {
    let address = wallet(address)?;
    let scheme = match coin {
        QrCoin::Bitcoin => "bitcoin",
        QrCoin::BitcoinCash => "bitcoincash",
        QrCoin::Ethereum => "ethereum",
        QrCoin::Litecoin => "litecoin",
        QrCoin::Dash => "dash",
    };
    // Ethereum counts its amount in another unit: only the address goes.
    let amount = if coin == QrCoin::Ethereum {
        None
    } else {
        coins(amount)?
    };
    Ok(match amount {
        Some(amount) => format!("{scheme}:{address}?amount={amount}"),
        None => format!("{scheme}:{address}"),
    })
}
