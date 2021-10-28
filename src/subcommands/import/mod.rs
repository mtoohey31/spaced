mod anki;
mod mochi;

use crate::entities::frontmatter;
use chrono::{DateTime, Utc};
use std::fmt;

use std::path::Path;


#[derive(Debug)]
pub enum ImportError {
    IOError(std::io::Error),
    FrontmatterError(frontmatter::FrontmatterError),
    ZipError(zip::result::ZipError),
    JSONError(serde_json::Error),
    ParseError(std::num::ParseIntError),
    RusqliteError(rusqlite::Error),
    ValueError,
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::IOError(e) => e.fmt(f),
            ImportError::FrontmatterError(e) => e.fmt(f),
            ImportError::ZipError(e) => e.fmt(f),
            ImportError::JSONError(e) => e.fmt(f),
            ImportError::ParseError(e) => e.fmt(f),
            ImportError::RusqliteError(e) => e.fmt(f),
            ImportError::ValueError => write!(f, "ValueError"), // TODO: Determine how this should be formatted
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct Deck<'a> {
    name: &'a str,
    cards: Vec<Card>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct Card {
    created: DateTime<Utc>,
    updated: DateTime<Utc>,
    reviews: serde_yaml::Value,
    body: String,
}

pub fn import(matches: &clap::ArgMatches) {
    match matches.value_of("format").unwrap() {
        "mochi" => {
            mochi::import(
                &Path::new(matches.value_of("PATH").unwrap()),
                &Path::new(matches.value_of("OUT_DIR").unwrap()),
            )
            .unwrap();
        }
        "anki" => {
            anki::import(
                &Path::new(matches.value_of("PATH").unwrap()),
                &Path::new(matches.value_of("OUT_DIR").unwrap()),
            )
            .unwrap();
        }
        _ => panic!(), // Can't happen because clap will ensure one of the previous options is present
    }
}
