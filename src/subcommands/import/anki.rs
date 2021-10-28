use super::{Card, Deck, ImportError};
use crate::entities::frontmatter;
use chrono::{TimeZone, Utc};
use rusqlite::{params, Connection, OpenFlags, Result};
use serde_yaml::{Mapping, Sequence, Value};
use std::collections::HashMap;
use std::fs::create_dir;
use std::fs::File;
use std::io::{prelude::*, Read};
use std::path::Path;

// TODO: Use model to construct format intelligently
struct Model {}

// TODO: Support media files
pub fn import(path: &Path, out_dir: &Path) -> Result<(), ImportError> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return Err(ImportError::IOError(e)),
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(e) => return Err(ImportError::ZipError(e)),
    };
    let mut collection;
    if archive.file_names().any(|s| s == "collection.anki21") {
        collection = match archive.by_name("collection.anki21") {
            Ok(c) => c,
            Err(e) => return Err(ImportError::ZipError(e)),
        };
    } else {
        collection = match archive.by_name("collection.ank21") {
            Ok(c) => c,
            Err(e) => return Err(ImportError::ZipError(e)),
        };
    }
    let mut bytes = Vec::new();
    match collection.read_to_end(&mut bytes) {
        Ok(_) => {}
        Err(e) => return Err(ImportError::IOError(e)),
    };
    let mut tmp_file = match File::create("/tmp/collection") {
        Ok(f) => f,
        Err(e) => return Err(ImportError::IOError(e)),
    };
    match tmp_file.write_all(&bytes) {
        Ok(_) => {}
        Err(e) => return Err(ImportError::IOError(e)),
    }

    let conn = match Connection::open_with_flags(
        "/tmp/collection",
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_SHARED_CACHE,
    ) {
        Ok(c) => c,
        Err(e) => return Err(ImportError::RusqliteError(e)),
    };

    let mut statement = match conn.prepare("SELECT decks, models FROM col") {
        Ok(s) => s,
        Err(e) => return Err(ImportError::RusqliteError(e)),
    };

    let (deck_info, deck_models) = match statement
        .query_row::<(Result<String, _>, Result<String, _>), _, _>(params![], |row| {
            Ok((row.get(0), row.get(1)))
        }) {
        Ok(r) => (
            match serde_json::from_str::<serde_json::Value>(
                &(match r.0 {
                    Ok(v) => v,
                    Err(e) => return Err(ImportError::RusqliteError(e)),
                }),
            ) {
                Ok(json) => match json {
                    serde_json::Value::Object(o) => o,
                    _ => return Err(ImportError::ValueError),
                },
                Err(e) => return Err(ImportError::JSONError(e)),
            },
            match serde_json::from_str::<serde_json::Value>(
                &(match r.1 {
                    Ok(v) => v,
                    Err(e) => return Err(ImportError::RusqliteError(e)),
                }),
            ) {
                Ok(json) => match json {
                    serde_json::Value::Object(o) => o,
                    _ => return Err(ImportError::ValueError),
                },
                Err(e) => return Err(ImportError::JSONError(e)),
            },
        ),
        Err(e) => return Err(ImportError::RusqliteError(e)),
    };
    let mut models = HashMap::new();
    for _model in deck_models {
        models.insert("", Model {});
    }
    let mut decks = Vec::new();
    for (id, data) in deck_info.iter() {
        decks.push(Deck {
            name: match data.get("name") {
                Some(n) => match n {
                    serde_json::Value::String(s) => s,
                    _ => return Err(ImportError::ValueError),
                },
                None => return Err(ImportError::ValueError),
            },
            cards: get_cards(id, &conn, &models)?,
        })
    }

    write_decks(decks, out_dir)
}

fn get_cards(
    did: &str,
    conn: &rusqlite::Connection,
    _models: &HashMap<&str, Model>,
) -> Result<Vec<Card>, ImportError> {
    let mut statement = match conn.prepare(
        &(String::from(
            "SELECT cards.id, notes.flds, notes.mod
FROM (SELECT * FROM cards WHERE cards.did=",
        ) + did
            + ") as cards
LEFT JOIN notes
ON cards.nid=notes.id"),
    ) {
        Ok(s) => s,
        Err(e) => return Err(ImportError::RusqliteError(e)),
    };

    let card_rows = match statement.query_map([], |row| Ok((row.get(0), row.get(1), row.get(2)))) {
        Ok(r) => {
            r.collect::<Vec<Result<(Result<isize, _>, Result<String, _>, Result<i64, _>), _>>>()
        }
        Err(e) => return Err(ImportError::RusqliteError(e)),
    };
    let mut cards = Vec::new();
    for row in card_rows {
        let row = row.unwrap(); // Safe because we explicitly Ok'd the row in the query map
        let id = match row.0 {
            Ok(id) => id,
            Err(e) => return Err(ImportError::RusqliteError(e)),
        };
        let body = match row.1 {
            Ok(s) => s.split("\u{1f}").collect::<Vec<&str>>().join("\n\n---\n\n"),
            Err(e) => return Err(ImportError::RusqliteError(e)),
        };
        let modified = match row.2 {
            Ok(m) => m,
            Err(e) => return Err(ImportError::RusqliteError(e)),
        };
        cards.push(Card {
            created: Utc::now(),
            updated: Utc::now().timezone().timestamp_millis(modified * 1000),
            reviews: get_reviews(id, conn)?,
            body: body,
        });
    }
    Ok(cards)
}

fn get_reviews(cid: isize, conn: &rusqlite::Connection) -> Result<Value, ImportError> {
    let mut statement = match conn.prepare(
        &(String::from(
            "SELECT id, ease
FROM revlog
WHERE revlog.cid=",
        ) + &cid.to_string()),
    ) {
        Ok(s) => s,
        Err(e) => return Err(ImportError::RusqliteError(e)),
    };
    let review_rows = match statement.query_map(params![], |row| Ok((row.get(0), row.get(1)))) {
        Ok(r) => r.collect::<Vec<Result<(Result<i64, _>, Result<isize, _>), _>>>(),
        Err(e) => return Err(ImportError::RusqliteError(e)),
    };
    let mut reviews = Sequence::new();
    for row in review_rows {
        let row = row.unwrap(); // Safe because we explicitly Ok'd the row in the query map
        let timestamp = match row.0 {
            Ok(d) => d,
            Err(e) => return Err(ImportError::RusqliteError(e)),
        };
        let ease = match row.0 {
            Ok(d) => d,
            Err(e) => return Err(ImportError::RusqliteError(e)),
        };
        let mut mapping = Mapping::new();
        mapping.insert(
            Value::String(String::from("date")),
            Value::String(String::from(
                Utc::now()
                    .timezone()
                    .timestamp_millis(timestamp)
                    .format("%Y-%m-%d")
                    .to_string(),
            )),
        );
        mapping.insert(
            Value::String(String::from("remembered")),
            Value::Bool(ease > 2),
        );
        reviews.push(Value::Mapping(mapping));
    }
    Ok(Value::Sequence(reviews))
}

fn write_decks(decks: Vec<Deck>, out_dir: &Path) -> Result<(), ImportError> {
    for deck in decks {
        let deck_out_dir = out_dir.join(deck.name);
        match create_dir(deck_out_dir.clone()) {
            Ok(_) => {}
            Err(e) => match e.raw_os_error() {
                Some(17) => {}
                _ => return Err(ImportError::IOError(e)),
            },
        }

        let mut i = 0;

        for card in deck.cards {
            i += 1;
            let mut frontmatter = serde_yaml::Mapping::new();
            frontmatter.insert(
                serde_yaml::Value::String(String::from("reviews")),
                card.reviews,
            );

            match frontmatter::write_fm_and_body(
                &deck_out_dir.join(String::from("card") + &i.to_string() + ".md"),
                serde_yaml::Value::Mapping(frontmatter),
                String::from(card.body),
            ) {
                Ok(_) => {}
                Err(e) => return Err(ImportError::FrontmatterError(e)),
            };
        }
    }

    Ok(())
}
