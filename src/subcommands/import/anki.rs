use super::{Card, Deck};
use crate::entities::frontmatter;
use crate::error::ValueError as VE;
use chrono::{TimeZone, Utc};
use rusqlite::{params, Connection, OpenFlags, Result};
use serde_yaml::{Mapping, Sequence, Value};
use std::collections::HashMap;
use std::error::Error;
use std::fs::create_dir;
use std::fs::File;
use std::io::{prelude::*, Read};
use std::path::Path;

// TODO: Use model to construct format intelligently
struct Model {}

// TODO: Support media files
pub fn import(path: &Path, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut collection;
    if archive.file_names().any(|s| s == "collection.anki21") {
        collection = archive.by_name("collection.anki21")?;
    } else {
        collection = archive.by_name("collection.anki2")?;
    }
    let mut bytes = Vec::new();
    collection.read_to_end(&mut bytes)?;
    let mut tmp_file = File::create("/tmp/collection")?;
    tmp_file.write_all(&bytes)?;

    let conn = Connection::open_with_flags(
        "/tmp/collection",
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_SHARED_CACHE,
    )?;

    let mut statement = conn.prepare("SELECT decks, models FROM col")?;

    let query_result = statement
        .query_row::<(Result<String, _>, Result<String, _>), _, _>(params![], |row| {
            Ok((row.get(0), row.get(1)))
        })?;
    let deck_info = serde_json::from_str::<serde_json::Value>(&(query_result.0?))?;
    let deck_info = deck_info.as_object().ok_or(VE::new())?;
    let deck_models = serde_json::from_str::<serde_json::Value>(&(query_result.1?))?;
    let deck_models = deck_models.as_object().ok_or(VE::new())?;
    let mut models = HashMap::new();
    for _model in deck_models {
        models.insert("", Model {});
    }
    let mut decks = Vec::new();
    for (id, data) in deck_info.iter() {
        decks.push(Deck {
            name: data
                .get("name")
                .ok_or(VE::new())?
                .as_str()
                .ok_or(VE::new())?,
            cards: get_cards(id, &conn, &models)?,
        })
    }

    write_decks(decks, out_dir)
}

fn get_cards(
    did: &str,
    conn: &rusqlite::Connection,
    _models: &HashMap<&str, Model>,
) -> Result<Vec<Card>, Box<dyn Error>> {
    let mut statement = conn.prepare(
        &(String::from(
            "SELECT cards.id, notes.flds, notes.mod
FROM (SELECT * FROM cards WHERE cards.did=",
        ) + did
            + ") as cards
LEFT JOIN notes
ON cards.nid=notes.id"),
    )?;

    let card_rows = statement
        .query_map([], |row| Ok((row.get(0), row.get(1), row.get(2))))?
        .collect::<Vec<Result<(Result<isize, _>, Result<String, _>, Result<i64, _>), _>>>();
    let mut cards = Vec::new();
    for row in card_rows {
        let row = row.unwrap(); // Safe because we explicitly Ok'd the row in the query map
        let id = row.0?;
        let body = row
            .1?
            .split("\u{1f}")
            .collect::<Vec<&str>>()
            .join("\n\n---\n\n");
        let modified = row.2?;
        cards.push(Card {
            created: Utc::now(),
            updated: Utc::now().timezone().timestamp_millis(modified * 1000),
            reviews: get_reviews(id, conn)?,
            body,
        });
    }
    Ok(cards)
}

fn get_reviews(cid: isize, conn: &rusqlite::Connection) -> Result<Value, Box<dyn Error>> {
    let mut statement = conn.prepare(
        &(String::from(
            "SELECT id, ease
FROM revlog
WHERE revlog.cid=",
        ) + &cid.to_string()),
    )?;
    let review_rows = statement
        .query_map(params![], |row| Ok((row.get(0), row.get(1))))?
        .collect::<Vec<Result<(Result<i64, _>, Result<isize, _>), _>>>();
    let mut reviews = Sequence::new();
    for row in review_rows.into_iter() {
        let row = row.unwrap(); // Safe because we explicitly Ok'd the row in the query map
        let timestamp = row.0?;
        let ease = timestamp.clone();
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

fn write_decks(decks: Vec<Deck>, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    for deck in decks {
        let deck_out_dir = out_dir.join(deck.name);
        match create_dir(deck_out_dir.clone()) {
            Ok(_) => {}
            Err(e) => match e.raw_os_error() {
                Some(17) => {}
                _ => return Err(Box::new(e)),
            },
        }

        let mut i: i32 = 0;

        for card in deck.cards {
            i += 1;
            let mut frontmatter = serde_yaml::Mapping::new();
            frontmatter.insert(
                serde_yaml::Value::String(String::from("reviews")),
                card.reviews,
            );

            frontmatter::write_fm_and_body(
                &deck_out_dir.join(String::from("card") + &i.to_string() + ".md"),
                serde_yaml::Value::Mapping(frontmatter),
                String::from(card.body),
            )?;
        }
    }

    Ok(())
}
