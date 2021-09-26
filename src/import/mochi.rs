use super::{Card, Deck, ImportError};
use crate::frontmatter;
use chrono::{TimeZone, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::create_dir;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct ParentDeck<'a> {
    deck: Deck<'a>,
    children: Vec<ParentDeck<'a>>,
}

impl<'a> std::ops::Deref for ParentDeck<'a> {
    type Target = Deck<'a>;
    fn deref(&self) -> &Self::Target {
        &self.deck
    }
}

impl<'a> std::ops::DerefMut for ParentDeck<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.deck
    }
}
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct ChildDeck<'a> {
    parent_deck: ParentDeck<'a>,
    parent_id: &'a str,
}

impl<'a> std::ops::Deref for ChildDeck<'a> {
    type Target = ParentDeck<'a>;
    fn deref(&self) -> &Self::Target {
        &self.parent_deck
    }
}

impl<'a> std::ops::DerefMut for ChildDeck<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.parent_deck
    }
}

impl<'a> ChildDeck<'a> {
    fn into_deck(self) -> ParentDeck<'a> {
        self.parent_deck
    }
}

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
    let mut file = match archive.by_name("data.json") {
        Ok(f) => f,
        Err(e) => return Err(ImportError::ZipError(e)),
    };
    let mut data_string = String::new();
    match file.read_to_string(&mut data_string) {
        Ok(_) => {}
        Err(e) => return Err(ImportError::IOError(e)),
    }
    let export = match serde_json::from_str(&data_string) {
        Ok(v) => match v {
            Value::Object(o) => o,
            _ => return Err(ImportError::ValueError),
        },
        Err(e) => return Err(ImportError::JSONError(e)),
    };
    let decks = match export.get("~:decks") {
        Some(v) => match v {
            Value::Array(a) => a,
            _ => return Err(ImportError::ValueError),
        },
        None => return Err(ImportError::ValueError),
    };

    let mut child_decks = HashMap::new();
    let mut root_decks = HashMap::new();

    for deck in decks {
        let deck = match deck {
            Value::Object(a) => a,
            _ => return Err(ImportError::ValueError),
        };
        let name = match deck.get("~:name") {
            Some(v) => match v {
                Value::String(s) => s,
                _ => return Err(ImportError::ValueError),
            },
            None => return Err(ImportError::ValueError),
        };
        let id = match deck.get("~:id") {
            Some(v) => match v {
                Value::String(s) => s.as_str(),
                _ => return Err(ImportError::ValueError),
            },
            None => return Err(ImportError::ValueError),
        };
        let parent_id = match deck.get("~:parent-id") {
            Some(v) => match v {
                Value::String(s) => Some(s.as_str()),
                _ => return Err(ImportError::ValueError),
            },
            None => None,
        };
        let cards = match deck.get("~:cards") {
            Some(v) => match v {
                Value::Object(o) => match o.get("~#list") {
                    Some(v) => match v {
                        Value::Array(a) => parse_cards(a)?,
                        _ => return Err(ImportError::ValueError),
                    },
                    None => return Err(ImportError::ValueError),
                },
                _ => return Err(ImportError::ValueError),
            },
            None => return Err(ImportError::ValueError),
        };

        if let Some(parent_id) = parent_id {
            child_decks.insert(
                id,
                ChildDeck {
                    parent_deck: ParentDeck {
                        deck: Deck {
                            name: name,
                            cards: cards,
                        },
                        children: vec![],
                    },
                    parent_id: parent_id,
                },
            );
        } else {
            root_decks.insert(
                id,
                ParentDeck {
                    deck: Deck {
                        name: name,
                        cards: cards,
                    },
                    children: vec![],
                },
            );
        }
    }

    let mut child_deck_ids = Vec::new();
    for child_id in child_decks.keys() {
        child_deck_ids.push(child_id.clone());
    }

    for child_id in child_deck_ids {
        let child = child_decks.remove(child_id).unwrap(); // Guaruanteed to exist since we just got the ids from the deck
        let parent_id = child.parent_id;
        match root_decks.get_mut(parent_id) {
            Some(p) => {
                p.children.push(child.into_deck());
                continue;
            }
            None => {}
        }
        match child_decks.get_mut(parent_id) {
            Some(p) => {
                p.children.push(child.into_deck());
            }
            None => return Err(ImportError::ValueError),
        }
    }

    write_decks(
        root_decks.into_values().collect::<Vec<ParentDeck>>(),
        out_dir,
    )
}

fn parse_cards(json_cards: &Vec<Value>) -> Result<Vec<Card>, ImportError> {
    let mut cards = Vec::new();
    for card in json_cards {
        cards.push(match parse_card(card) {
            Ok(c) => c,
            Err(e) => return Err(e),
        });
    }
    Ok(cards)
}

fn parse_card<'a>(json_card: &'a Value) -> Result<Card, ImportError> {
    let created = match json_card.get("~:created-at") {
        Some(v) => match v {
            Value::Object(o) => match o.get("~#dt") {
                Some(v) => match v {
                    Value::Number(n) => match n.as_i64() {
                        Some(i) => Utc::now().timezone().timestamp_millis(i),
                        None => return Err(ImportError::ValueError),
                    },
                    _ => return Err(ImportError::ValueError),
                },
                None => return Err(ImportError::ValueError),
            },
            _ => return Err(ImportError::ValueError),
        },
        None => return Err(ImportError::ValueError),
    };
    let updated = match json_card.get("~:updated-at") {
        Some(v) => match v {
            Value::Object(o) => match o.get("~#dt") {
                Some(v) => match v {
                    Value::Number(n) => match n.as_i64() {
                        Some(i) => Utc::now().timezone().timestamp_millis(i),
                        None => return Err(ImportError::ValueError),
                    },
                    _ => return Err(ImportError::ValueError),
                },
                None => return Err(ImportError::ValueError),
            },
            _ => return Err(ImportError::ValueError),
        },
        None => return Err(ImportError::ValueError),
    };
    let reviews = match json_card.get("~:reviews") {
        Some(v) => match v {
            Value::Array(a) => match parse_reviews(a) {
                Ok(r) => r,
                Err(e) => return Err(e),
            },
            _ => return Err(ImportError::ValueError),
        },
        None => return Err(ImportError::ValueError),
    };
    let body = match json_card.get("~:content") {
        Some(v) => match v {
            Value::String(s) => String::from(s),
            _ => return Err(ImportError::ValueError),
        },
        None => return Err(ImportError::ValueError),
    };
    Ok(Card {
        created: created,
        updated: updated,
        reviews: reviews,
        body: body,
    })
}

fn parse_reviews(json_reviews: &Vec<Value>) -> Result<serde_yaml::Value, ImportError> {
    let mut reviews = Vec::new();
    for review in json_reviews {
        let review = match review {
            Value::Object(o) => o,
            _ => return Err(ImportError::ValueError),
        };
        let mut mapping = serde_yaml::Mapping::new();
        mapping.insert(
            serde_yaml::Value::String(String::from("date")),
            match review.get("~:date") {
                Some(v) => match v {
                    Value::String(s) => {
                        let i: i64 = match s[2..].parse() {
                            Ok(i) => i,
                            Err(e) => return Err(ImportError::ParseError(e)),
                        };
                        let date = Utc::now().timezone().timestamp_millis(i);
                        serde_yaml::Value::String(date.format("%Y-%m-%d").to_string())
                    }
                    _ => return Err(ImportError::ValueError),
                },
                None => return Err(ImportError::ValueError),
            },
        );
        mapping.insert(
            serde_yaml::Value::String(String::from("remembered")),
            match review.get("~:remembered?") {
                Some(v) => match v {
                    Value::Bool(b) => serde_yaml::Value::Bool(*b),
                    _ => return Err(ImportError::ValueError),
                },
                None => return Err(ImportError::ValueError),
            },
        );
        reviews.push(serde_yaml::Value::Mapping(mapping));
    }
    Ok(serde_yaml::Value::Sequence(reviews))
}

fn write_decks(decks: Vec<ParentDeck>, out_dir: &Path) -> Result<(), ImportError> {
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

        for card in deck.deck.cards {
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

        write_decks(deck.children, &deck_out_dir)?;
    }

    Ok(())
}
