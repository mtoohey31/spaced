mod anki;
mod mochi;

use chrono::{DateTime, Utc};

use std::path::Path;

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
