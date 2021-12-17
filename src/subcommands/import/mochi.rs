use super::{Card, Deck};
use crate::entities::frontmatter;
use crate::error::ValueError as VE;
use chrono::DateTime;
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
        .ok_or(VE::from("missing `~:decks` key".to_string()))?
        .as_array()
        .ok_or(VE::from("`~:decks` key was not of type array".to_string()))?;

    let mut child_decks = HashMap::new();
    let mut root_decks = HashMap::new();

    for deck in decks {
        let deck = deck
            .as_object()
            .ok_or(VE::from("deck was not of type object".to_string()))?;
        let name = deck
            .get("~:name")
            .ok_or(VE::from("missing `~:name` key".to_string()))?
            .as_str()
            .ok_or(VE::from("`~:name` key was not of type string".to_string()))?;
        println!("{}", name);
        let id = deck
            .get("~:id")
            .ok_or(VE::from(format!("missing `~:id` key in deck {}", name)))?
            .as_str()
            .ok_or(VE::from(format!(
                "`~:id` key was not of type string in deck {}",
                name
            )))?;
        let parent_id = deck.get("~:parent-id");
        let cards = parse_cards(
            deck.get("~:cards")
                .ok_or(VE::from(format!("missing `~:cards` key in deck {}", name)))?
                .as_object()
                .ok_or(VE::from(format!(
                    "`~:cards` key was not of type object in deck {}",
                    name
                )))?
                .get("~#list")
                .ok_or(VE::from(format!(
                    "missing `~:cards.~#list` key in deck {}",
                    name
                )))?
                .as_array()
                .ok_or(VE::from(format!(
                    "`~:cards.~#list` key was not of type array in deck {}",
                    name
                )))?,
        )?;

        if let Some(parent_id) = parent_id {
            child_decks.insert(
                id,
                ChildDeck {
                    parent_deck: ParentDeck {
                        deck: Deck { name, cards },
                        children: vec![],
                    },
                    parent_id: parent_id.as_str().ok_or(VE::from(format!(
                        "`~:parent-id` key was not of type string in deck {}",
                        name
                    )))?,
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
            .ok_or(VE::from(format!("parent id {} not found", parent_id)))?
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
    let created = timestamp_to_date(
        json_card
            .get("~:created-at")
            .ok_or(VE::from("missing `~:created-at` key in card".to_string()))?
            .as_str()
            .ok_or(VE::from(
                "`~:created-at` key was not of type string in card".to_string(),
            ))?,
    )?;
    // let updated = timestamp_to_date(
    //     json_card
    //         .get("~:updated-at")
    //         .ok_or(VE::from("missing `~:updated-at` key in card".to_string()))?
    //         .as_str()
    //         .ok_or(VE::from(
    //             "`~:updated-at` key was not of type string in card".to_string(),
    //         ))?,
    // )?;
    let reviews = parse_reviews(
        json_card
            .get("~:reviews")
            .ok_or(VE::from("missing `~:reviews` key in card".to_string()))?
            .as_array()
            .ok_or(VE::from(
                "`~:reviews` key was not of type array in card".to_string(),
            ))?,
    )?;
    let body = json_card
        .get("~:content")
        .ok_or(VE::from("missing `~:content` key in card".to_string()))?
        .as_str()
        .ok_or(VE::from(
            "`~:content` key was not of type string in card".to_string(),
        ))?
        .to_string();
    Ok(Card {
        created,
        // updated,
        reviews,
        body,
    })
}

fn parse_reviews(json_reviews: &Vec<Value>) -> Result<serde_yaml::Value, Box<dyn Error>> {
    let mut reviews = Vec::new();
    for review in json_reviews {
        let review = review.as_object().ok_or(VE::from(
            "review was not of type string in card".to_string(),
        ))?;
        let mut mapping = serde_yaml::Mapping::new();
        mapping.insert(
            serde_yaml::Value::String(String::from("date")),
            serde_yaml::Value::String(
                timestamp_to_date(
                    (review
                        .get("~:date")
                        .ok_or(VE::from("missing `~:date` key in review".to_string()))?
                        .as_str()
                        .ok_or(VE::from(
                            "`~:date` key was not of type string in review".to_string(),
                        )))?,
                )?
                .format("%Y-%m-%d")
                .to_string(),
            ),
        );
        mapping.insert(
            serde_yaml::Value::String(String::from("remembered")),
            serde_yaml::Value::Bool(
                review
                    .get("~:remembered?")
                    .ok_or(VE::from(
                        "missing `~:remembered?` key in review".to_string(),
                    ))?
                    .as_bool()
                    .ok_or(VE::from(
                        "`~:remembered?` key was not of type boolean in review".to_string(),
                    ))?,
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
                _ => return Err(Box::new(e)),
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

fn timestamp_to_date(timestamp_string: &str) -> Result<DateTime<Utc>, Box<dyn Error>> {
    Ok(Utc::now()
        .timezone()
        .timestamp_millis(timestamp_string[2..].parse()?))
}
