//! The text of each kind of content (S80-03, S80-04).

use gafira_doc::{QrCoin, QrContact, QrContactFormat, QrContent, QrWifiSecurity as Security};
use gafira_qr::{ContentError, Field, payload, problems};

fn s(text: &str) -> String {
    text.to_owned()
}

fn url(text: &str) -> Result<String, ContentError> {
    payload(&QrContent::Url { url: s(text) })
}

#[test]
fn s80_03_url_takes_a_scheme() {
    assert_eq!(
        url("https://example.com/a?b=c"),
        Ok(s("https://example.com/a?b=c"))
    );
    assert_eq!(url(" example.com "), Ok(s("https://example.com")));
    assert_eq!(
        url("example.com:8080/a"),
        Ok(s("https://example.com:8080/a"))
    );
    assert_eq!(url("mailto:a@b.c"), Ok(s("mailto:a@b.c")));
    assert_eq!(url(" "), Err(ContentError::Empty(Field::Url)));
    assert_eq!(url("una frase"), Err(ContentError::Invalid(Field::Url)));
    assert_eq!(url("https://"), Err(ContentError::Invalid(Field::Url)));
    let social = QrContent::Social {
        url: s("example.com/ana"),
    };
    assert_eq!(payload(&social), Ok(s("https://example.com/ana")));
}

#[test]
fn s80_03_text_is_kept() {
    let text = |t: &str| payload(&QrContent::Text { text: s(t) });
    assert_eq!(text("  año\n2 "), Ok(s("  año\n2 ")));
    assert_eq!(text(" \n"), Err(ContentError::Empty(Field::Text)));
}

#[test]
fn s80_03_mailto_percent_encodes() {
    let mail = |address: &str, subject: &str, body: &str| {
        payload(&QrContent::Email {
            address: s(address),
            subject: s(subject),
            body: s(body),
        })
    };
    assert_eq!(
        mail("ana@example.com", "", ""),
        Ok(s("mailto:ana@example.com"))
    );
    assert_eq!(
        mail("ana@example.com", "Hola & adiós", "a=b\nc"),
        Ok(s(
            "mailto:ana@example.com?subject=Hola%20%26%20adi%C3%B3s&body=a%3Db%0D%0Ac"
        ))
    );
    assert_eq!(
        mail("ana@example.com", "", "x"),
        Ok(s("mailto:ana@example.com?body=x"))
    );
    assert_eq!(mail("", "", ""), Err(ContentError::Empty(Field::Email)));
    for wrong in ["ana", "@b.c", "a@", "a b@c.d", "a@b@c", "a?x@b.c"] {
        assert_eq!(
            mail(wrong, "", ""),
            Err(ContentError::Invalid(Field::Email)),
            "{wrong}"
        );
    }
}

#[test]
fn s80_03_phone_and_sms() {
    let phone = |n: &str| payload(&QrContent::Phone { number: s(n) });
    assert_eq!(phone("+34 (600) 00-00.00"), Ok(s("tel:+34600000000")));
    assert_eq!(phone(""), Err(ContentError::Empty(Field::Phone)));
    for wrong in ["+", "60a", "6+0"] {
        assert_eq!(
            phone(wrong),
            Err(ContentError::Invalid(Field::Phone)),
            "{wrong}"
        );
    }
    let sms = |t: &str| {
        payload(&QrContent::Sms {
            number: s("600 00"),
            text: s(t),
        })
    };
    assert_eq!(sms("hola: qué tal"), Ok(s("SMSTO:60000:hola: qué tal")));
    assert_eq!(sms(""), Ok(s("SMSTO:60000:")));
}

#[test]
fn s80_03_wifi_escapes_the_reserved_characters() {
    let wifi = |ssid: &str, password: &str, security, hidden| {
        payload(&QrContent::Wifi {
            ssid: s(ssid),
            password: s(password),
            security,
            hidden,
        })
    };
    assert_eq!(
        wifi("Cafe;Uno", "a:b\\c,d\"e", Security::Wpa, false),
        Ok(s("WIFI:T:WPA;S:Cafe\\;Uno;P:a\\:b\\\\c\\,d\\\"e;;"))
    );
    assert_eq!(
        wifi("x", "left", Security::Open, false),
        Ok(s("WIFI:T:nopass;S:x;;"))
    );
    assert_eq!(
        wifi("x", "y", Security::Wep, true),
        Ok(s("WIFI:T:WEP;S:x;P:y;H:true;;"))
    );
    assert_eq!(
        wifi("", "y", Security::Wpa, true),
        Err(ContentError::Empty(Field::Ssid))
    );
    assert_eq!(
        wifi("x", "", Security::Wpa, true),
        Err(ContentError::Empty(Field::Password))
    );
}

fn contact(format: QrContactFormat) -> QrContact {
    QrContact {
        format,
        first_name: s("Ana;María"),
        last_name: s("Ruiz"),
        email: s("ana@example.com"),
        street: s("Calle 1, 2"),
        city: s("Cali"),
        ..QrContact::default()
    }
}

#[test]
fn s80_03_vcard_escapes_and_uses_crlf() {
    let text = payload(&QrContent::Contact(Box::new(contact(
        QrContactFormat::Vcard,
    ))));
    assert_eq!(
        text,
        Ok(s(
            "BEGIN:VCARD\r\nVERSION:3.0\r\nN:Ruiz;Ana\\;María;;;\r\nFN:Ana\\;María Ruiz\r\n\
EMAIL;TYPE=INTERNET:ana@example.com\r\nADR:;;Calle 1\\, 2;Cali;;;\r\nEND:VCARD\r\n"
        ))
    );
    let mut only_org = QrContact::default();
    assert_eq!(
        payload(&QrContent::Contact(Box::new(only_org.clone()))),
        Err(ContentError::Empty(Field::Name))
    );
    only_org.organization = s("Uno\nDos");
    assert_eq!(
        payload(&QrContent::Contact(Box::new(only_org))),
        Ok(s(
            "BEGIN:VCARD\r\nVERSION:3.0\r\nN:;;;;\r\nFN:Uno\\nDos\r\nORG:Uno\\nDos\r\nEND:VCARD\r\n"
        ))
    );
}

#[test]
fn s80_03_mecard_escapes() {
    let text = payload(&QrContent::Contact(Box::new(contact(
        QrContactFormat::Mecard,
    ))));
    assert_eq!(
        text,
        Ok(s(
            "MECARD:N:Ruiz,Ana\\;María;EMAIL:ana@example.com;ADR:,,Calle 1\\, 2,Cali,,,;;"
        ))
    );
}

#[test]
fn s80_03_event_is_a_calendar() {
    let event = |title: &str, start: &str, end: &str| {
        payload(&QrContent::Event {
            title: s(title),
            place: s("Sala 1"),
            start: s(start),
            end: s(end),
        })
    };
    assert_eq!(
        event("Feria, 2026", "2026-11-22 13:34", "2026-11-22T15:00"),
        Ok(s(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Gafira//QR//ES\r\nBEGIN:VEVENT\r\n\
SUMMARY:Feria\\, 2026\r\nLOCATION:Sala 1\r\nDTSTART:20261122T133400\r\nDTEND:20261122T150000\r\n\
END:VEVENT\r\nEND:VCALENDAR\r\n"
        ))
    );
    let (a, b) = ("2026-11-22 13:34", "2026-11-22 13:33");
    assert_eq!(event("", a, a), Err(ContentError::Empty(Field::Title)));
    assert_eq!(event("x", "", a), Err(ContentError::Empty(Field::Start)));
    assert!(event("x", a, a).is_ok());
    assert_eq!(event("x", a, b), Err(ContentError::Invalid(Field::End)));
    for wrong in [
        "2026-02-30 10:00",
        "2026-13-01 10:00",
        "2026-1-1 1:1",
        "2026-01-01 24:00",
        "hoy",
    ] {
        assert_eq!(
            event("x", wrong, a),
            Err(ContentError::Invalid(Field::Start)),
            "{wrong}"
        );
    }
    assert!(event("x", "2028-02-29 00:00", "2028-02-29 23:59").is_ok());
}

#[test]
fn s80_03_geo_limits() {
    let geo = |a: &str, b: &str| {
        payload(&QrContent::Location {
            latitude: s(a),
            longitude: s(b),
        })
    };
    assert_eq!(geo("4,60971", " -74.08175"), Ok(s("geo:4.60971,-74.08175")));
    assert_eq!(geo("-90", "180"), Ok(s("geo:-90,180")));
    assert_eq!(
        geo("90.1", "0"),
        Err(ContentError::Invalid(Field::Latitude))
    );
    assert_eq!(
        geo("0", "-180.1"),
        Err(ContentError::Invalid(Field::Longitude))
    );
    assert_eq!(geo("", "0"), Err(ContentError::Empty(Field::Latitude)));
    assert_eq!(geo("NaN", "0"), Err(ContentError::Invalid(Field::Latitude)));
}

#[test]
fn s80_03_crypto() {
    let pay = |coin, address: &str, amount: &str| {
        payload(&QrContent::Crypto {
            coin,
            address: s(address),
            amount: s(amount),
        })
    };
    assert_eq!(
        pay(QrCoin::Bitcoin, "1abc", "0,5"),
        Ok(s("bitcoin:1abc?amount=0.5"))
    );
    assert_eq!(pay(QrCoin::BitcoinCash, "q1", ""), Ok(s("bitcoincash:q1")));
    assert_eq!(pay(QrCoin::Ethereum, "0xab", "2"), Ok(s("ethereum:0xab")));
    assert_eq!(
        pay(QrCoin::Litecoin, "L", "1"),
        Ok(s("litecoin:L?amount=1"))
    );
    assert_eq!(pay(QrCoin::Dash, "X", "1"), Ok(s("dash:X?amount=1")));
    assert_eq!(
        pay(QrCoin::Dash, "", ""),
        Err(ContentError::Empty(Field::Address))
    );
    assert_eq!(
        pay(QrCoin::Dash, "a&b", ""),
        Err(ContentError::Invalid(Field::Address))
    );
    for wrong in ["x", "-1", "0"] {
        assert_eq!(
            pay(QrCoin::Dash, "X", wrong),
            Err(ContentError::Invalid(Field::Amount))
        );
    }
}

// S80-17: a web address needs a host that can exist.
#[test]
fn s80_17_a_mistyped_url_is_wrong() {
    for wrong in [
        "asd",
        "https://asd",
        "http:/example.com",
        "https//example.com",
        "htp://example.com",
        "example..com",
        ".example.com",
        "example.com.",
        "example.c",
        "-a.com",
        "a_b.com",
        "https://example.com:99999",
        "https://example.com:x",
        "exam ple.com",
        "https://user@",
        "ejemplo,com",
    ] {
        assert_eq!(
            url(wrong),
            Err(ContentError::Invalid(Field::Url)),
            "{wrong}"
        );
    }
    for right in [
        "https://example.com",
        "http://localhost:3000/a",
        "https://192.168.0.1/admin",
        "https://[::1]:8080/",
        "https://sub.example.co.uk/a?b=c#d",
        "https://camión.com",
        "https://example.com?x=1",
        "ftp://example.com",
        "tel:123",
    ] {
        assert_eq!(url(right), Ok(s(right)), "{right}");
    }
    assert_eq!(url("www.example.com"), Ok(s("https://www.example.com")));
}

// S80-17: so does an email address.
#[test]
fn s80_17_a_mistyped_email_is_wrong() {
    let mail = |address: &str| {
        problems(&QrContent::Email {
            address: s(address),
            subject: s(""),
            body: s(""),
        })
    };
    for wrong in [
        "ana@example",
        "ana@example.c",
        "ana@.com",
        "ana..b@example.com",
        ".a@b.co",
        "a.@b.co",
        "ana@ex ample.com",
    ] {
        assert_eq!(
            mail(wrong),
            vec![ContentError::Invalid(Field::Email)],
            "{wrong}"
        );
    }
    for right in ["ana@example.com", "a.b+c@sub.example.co"] {
        assert_eq!(mail(right), vec![], "{right}");
    }
}

// S80-17: a phone has 3 to 15 digits; a WPA key, 8 to 63 characters or 64
// hexadecimal digits.
#[test]
fn s80_17_lengths() {
    let phone = |n: &str| problems(&QrContent::Phone { number: s(n) });
    for wrong in ["12", "1234567890123456"] {
        assert_eq!(phone(wrong), vec![ContentError::Invalid(Field::Phone)]);
    }
    for right in ["112", "123456789012345"] {
        assert_eq!(phone(right), vec![]);
    }
    let wpa = |key: &str| {
        problems(&QrContent::Wifi {
            ssid: s("x"),
            password: s(key),
            security: Security::Wpa,
            hidden: false,
        })
    };
    for wrong in [
        "1234567".to_owned(),
        "a".repeat(64),
        "a".repeat(65),
        "z".repeat(64),
    ] {
        let expected = if wrong == "a".repeat(64) {
            vec![]
        } else {
            vec![ContentError::Invalid(Field::Password)]
        };
        assert_eq!(wpa(&wrong), expected, "{wrong}");
    }
    assert_eq!(wpa("12345678"), vec![]);
    assert_eq!(wpa(&"a".repeat(63)), vec![]);
}

// S80-17: every field at fault is told at once.
#[test]
fn s80_17_every_problem_is_told() {
    use ContentError::{Empty, Invalid};
    let wifi = QrContent::Wifi {
        ssid: s(""),
        password: s(""),
        security: Security::Wpa,
        hidden: false,
    };
    assert_eq!(
        problems(&wifi),
        vec![Empty(Field::Ssid), Empty(Field::Password)]
    );
    let event = QrContent::Event {
        title: s(""),
        place: s(""),
        start: s("ayer"),
        end: s(""),
    };
    assert_eq!(
        problems(&event),
        vec![
            Empty(Field::Title),
            Invalid(Field::Start),
            Empty(Field::End)
        ]
    );
    let here = QrContent::Location {
        latitude: s("100"),
        longitude: s(""),
    };
    assert_eq!(
        problems(&here),
        vec![Invalid(Field::Latitude), Empty(Field::Longitude)]
    );
    let pay = QrContent::Crypto {
        coin: QrCoin::Dash,
        address: s(""),
        amount: s("x"),
    };
    assert_eq!(
        problems(&pay),
        vec![Empty(Field::Address), Invalid(Field::Amount)]
    );
    // The optional fields of a contact are checked when written.
    let contact = QrContact {
        phone: s("x"),
        work_phone: s("y"),
        email: s("ana@"),
        url: s("asd"),
        ..QrContact::default()
    };
    assert_eq!(
        problems(&QrContent::Contact(Box::new(contact))),
        vec![
            Empty(Field::Name),
            Invalid(Field::Phone),
            Invalid(Field::WorkPhone),
            Invalid(Field::Email),
            Invalid(Field::Url),
        ]
    );
    // What has no problems makes a text, and the other way round.
    let sms = QrContent::Sms {
        number: s("112"),
        text: s(""),
    };
    assert_eq!(problems(&sms), vec![]);
    assert!(payload(&sms).is_ok());
}
