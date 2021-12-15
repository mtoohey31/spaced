use super::{Card, Deck};
use crate::entities::frontmatter;
use crate::error::ValueError as VE;
use chrono::{TimeZone, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
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
pub fn import(path: &Path, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut file = archive.by_name("data.json")?;
    let mut data_string = String::new();
    file.read_to_string(&mut data_string)?;
    let export: serde_json::map::Map<String, serde_json::value::Value> =
        serde_json::from_str(&data_string)?;
    let decks = export
        .get("~:decks")
        .ok_or(VE::new())?
        .as_array()
        .ok_or(VE::new())?;

    let mut child_decks = HashMap::new();
    let mut root_decks = HashMap::new();

    for deck in decks {
        let deck = deck.as_object().ok_or(VE::new())?;
        let name = deck
            .get("~:name")
            .ok_or(VE::new())?
            .as_str()
            .ok_or(VE::new())?;
        println!("{}", name);
        let id = deck
            .get("~:id")
            .ok_or(VE::new())?
            .as_str()
            .ok_or(VE::new())?;
        let parent_id = deck.get("~:parent-id").ok_or(VE::new())?.as_str();
        let cards = parse_cards(
            deck.get("~:cards")
                .ok_or(VE::new())?
                .as_object()
                .ok_or(VE::new())?
                .get("~#list")
                .ok_or(VE::new())?
                .as_array()
                .ok_or(VE::new())?,
        )?;

        if let Some(parent_id) = parent_id {
            child_decks.insert(
                id,
                ChildDeck {
                    parent_deck: ParentDeck {
                        deck: Deck { name, cards },
                        children: vec![],
                    },
                    parent_id,
                },
            );
        } else {
            root_decks.insert(
                id,
                ParentDeck {
                    deck: Deck { name, cards },
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
        let child = child_decks.remove(&child_id).unwrap(); // Guaruanteed to exist since we just got the ids from the deck
        let parent_id = child.parent_id;
        match root_decks.get_mut(parent_id) {
            Some(p) => {
                p.children.push(child.into_deck());
                continue;
            }
            None => {}
        }
        child_decks
            .get_mut(parent_id)
            .ok_or(VE::new())?
            .children
            .push(child.into_deck());
    }

    write_decks(
        root_decks.into_values().collect::<Vec<ParentDeck>>(),
        out_dir,
    )
}

fn parse_cards(json_cards: &Vec<Value>) -> Result<Vec<Card>, Box<dyn Error>> {
    let mut cards = Vec::new();
    for card in json_cards {
        cards.push(parse_card(card)?);
    }
    Ok(cards)
}

fn parse_card<'a>(json_card: &'a Value) -> Result<Card, Box<dyn Error>> {
    let created = Utc::now().timezone().timestamp_millis(
        json_card
            .get("~:created-at")
            .ok_or(VE::new())?
            .as_object()
            .ok_or(VE::new())?
            .get("~#dt")
            .ok_or(VE::new())?
            .as_i64()
            .ok_or(VE::new())?,
    );
    let updated = Utc::now().timezone().timestamp_millis(
        json_card
            .get("~:updated-at")
            .ok_or(VE::new())?
            .as_object()
            .ok_or(VE::new())?
            .get("~#dt")
            .ok_or(VE::new())?
            .as_i64()
            .ok_or(VE::new())?,
    );
    let reviews = parse_reviews(
        json_card
            .get("~:reviews")
            .ok_or(VE::new())?
            .as_array()
            .ok_or(VE::new())?,
    )?;
    let body = json_card
        .get("~:content")
        .ok_or(VE::new())?
        .as_str()
        .ok_or(VE::new())?
        .to_string();
    Ok(Card {
        created,
        updated,
        reviews,
        body,
    })
}

fn parse_reviews(json_reviews: &Vec<Value>) -> Result<serde_yaml::Value, Box<dyn Error>> {
    let mut reviews = Vec::new();
    for review in json_reviews {
        let review = review.as_object().ok_or(VE::new())?;
        let mut mapping = serde_yaml::Mapping::new();
        mapping.insert(
            serde_yaml::Value::String(String::from("date")),
            match review.get("~:date") {
                Some(Value::String(s)) => {
                    let i: i64 = s[2..].parse()?;
                    let date = Utc::now().timezone().timestamp_millis(i);
                    serde_yaml::Value::String(date.format("%Y-%m-%d").to_string())
                }
                _ => return Err(Box::new(VE::new())),
            },
        );
        mapping.insert(
            serde_yaml::Value::String(String::from("remembered")),
            serde_yaml::Value::Bool(
                review
                    .get("~:remembered?")
                    .ok_or(VE::new())?
                    .as_bool()
                    .ok_or(VE::new())?,
            ),
        );
        reviews.push(serde_yaml::Value::Mapping(mapping));
    }
    Ok(serde_yaml::Value::Sequence(reviews))
}

fn write_decks(decks: Vec<ParentDeck>, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    for deck in decks {
        let deck_out_dir = out_dir.join(deck.name);
        match create_dir(deck_out_dir.clone()) {
            Ok(_) => {}
            Err(e) => match e.raw_os_error() {
                Some(17) => {}
                _ => return Err(Box::new(VE::new())),
            },
        }

        let mut i: i32 = 0;

        for card in deck.deck.cards {
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

        write_decks(deck.children, &deck_out_dir)?;
    }

    Ok(())
}
