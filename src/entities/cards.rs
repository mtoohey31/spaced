use crate::entities::{algorithms, frontmatter};
use chrono::format::ParseError;
use chrono::{Date, NaiveDate, Utc};
use serde_yaml::Mapping;
use serde_yaml::Value;
use std::ffi::OsStr;
use std::fmt;
use std::path::{Component, Path};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug)]
pub enum ReviewHistoryError {
    DateParseError(ParseError),
    ValueError,
}

impl fmt::Display for ReviewHistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReviewHistoryError::DateParseError(e) => e.fmt(f),
            ReviewHistoryError::ValueError => write!(f, "ValueError"), // TODO: Determine how this should be formatted
        }
    }
}

pub fn get_cards(path: &str, algorithm: &str) -> Vec<DirEntry> {
    // TODO: Handle errors here
    WalkDir::new(path)
        .into_iter()
        .filter_map(|entry_result| {
            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => return None,
            };
            let canonical_path = match entry.path().canonicalize() {
                Ok(cp) => cp,
                Err(_) => return None,
            };
            if canonical_path
                .components()
                .find(|c| c == &Component::Normal(OsStr::new("cards")))
                .is_none()
            {
                return None;
            }
            let extension = match entry.path().extension() {
                Some(e) => e,
                None => return None,
            };
            if extension == "md" && review_time(entry.path(), algorithm) {
                Some(entry)
            } else {
                None
            }
        })
        .into_iter()
        .collect::<Vec<DirEntry>>()
}

fn review_time(path: &Path, algorithm: &str) -> bool {
    if algorithm == "all" {
        return true;
    }
    // TODO: Catch errors here
    let frontmatter = frontmatter::read_fm(path).unwrap();
    if let Some(archived) = frontmatter.get(&Value::String(String::from("archived"))) {
        // TODO: should I throw an error if archived is not a bool
        if archived.as_bool().unwrap_or(false) {
            return false;
        }
    }
    let mut review_history = read_review_history(frontmatter).unwrap();
    match algorithm {
        "leitner" => algorithms::leitner(&mut review_history),
        _ => panic!(), // Cannot occur because clap will block invalid algorithm arguments
    }
}

fn read_review_history(frontmatter: Mapping) -> Result<Vec<(Date<Utc>, bool)>, ReviewHistoryError> {
    match frontmatter
        .get(&Value::String(String::from("reviews")))
        .unwrap_or(&Value::Sequence(vec![]))
    {
        Value::Sequence(sequence) => {
            let mut review_history = Vec::new();
            for map in sequence {
                let date = match map.get("date") {
                    Some(value) => match value {
                        Value::String(string) => Date::from_utc(
                            match NaiveDate::parse_from_str(string, "%Y-%m-%d") {
                                Ok(d) => d,
                                Err(e) => return Err(ReviewHistoryError::DateParseError(e)),
                            },
                            Utc,
                        ),
                        _ => return Err(ReviewHistoryError::ValueError),
                    },
                    None => return Err(ReviewHistoryError::ValueError),
                };
                let remembered = match map.get("remembered") {
                    Some(value) => match value {
                        Value::Bool(b) => *b,
                        _ => return Err(ReviewHistoryError::ValueError),
                    },
                    None => return Err(ReviewHistoryError::ValueError),
                };
                review_history.push((date, remembered));
            }
            Ok(review_history)
        }
        Value::Null => Ok(vec![]),
        _ => return Err(ReviewHistoryError::ValueError),
    }
}

pub fn mark(path: &Path, remembered: bool) {
    let (mut mapping, body) = match frontmatter::read_fm_and_body(path) {
        Ok(fm) => fm,
        Err(e) => panic!("{}", e),
    };

    match mapping.get_mut(&Value::String(String::from("reviews"))) {
        Some(reviews) => match reviews {
            Value::Sequence(s) => {
                s.push(get_review_item(remembered));
                frontmatter::write_fm_and_body(path, Value::Mapping(mapping), body).unwrap();
            }
            Value::Null => {
                mapping.insert(
                    Value::String(String::from("reviews")),
                    get_review_item(remembered),
                );
                frontmatter::write_fm_and_body(path, Value::Mapping(mapping), body).unwrap();
            }
            _ => panic!("Unsupported frontmatter contents in {}", path.display()),
        },
        None => {
            mapping.insert(
                Value::String(String::from("reviews")),
                Value::Sequence(vec![get_review_item(remembered)]),
            );
            frontmatter::write_fm_and_body(path, Value::Mapping(mapping), body).unwrap();
        }
    }
}

pub fn unmark(path: &Path) {
    let (mut mapping, body) = match frontmatter::read_fm_and_body(path) {
        Ok(fm) => fm,
        Err(e) => panic!("{}", e),
    };

    match mapping.get_mut(&Value::String(String::from("reviews"))) {
        Some(reviews) => match reviews {
            Value::Sequence(s) => {
                s.pop();
                frontmatter::write_fm_and_body(path, Value::Mapping(mapping), body).unwrap();
            }
            Value::Null => panic!("Card has already been unmarked {}", path.display()),
            _ => panic!("Unsupported frontmatter contents in {}", path.display()),
        },
        None => {
            panic!("Card has already been unmarked {}", path.display())
        }
    }
}

pub fn mark_archived(path: &Path, archived: bool) {
    let (mut mapping, body) = match frontmatter::read_fm_and_body(path) {
        Ok(fm) => fm,
        Err(e) => panic!("{}", e),
    };

    match mapping.get_mut(&Value::String(String::from("archived"))) {
        Some(reviews) => match reviews {
            Value::Bool(b) => {
                *b = archived;
            }
            _ => panic!("Unsupported frontmatter contents in {}", path.display()),
        },
        None => {
            mapping.insert(
                Value::String(String::from("archived")),
                Value::Bool(archived),
            );
            frontmatter::write_fm_and_body(path, Value::Mapping(mapping), body).unwrap();
        }
    }
}

fn get_review_item(remembered: bool) -> serde_yaml::Value {
    let mut mapping = serde_yaml::Mapping::new();
    let today = Utc::today().format("%Y-%m-%d").to_string();
    mapping.insert(
        Value::String(String::from("date")),
        serde_yaml::Value::String(today),
    );
    mapping.insert(
        Value::String(String::from("remembered")),
        serde_yaml::Value::Bool(remembered),
    );
    Value::Mapping(mapping)
}
