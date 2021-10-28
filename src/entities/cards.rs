use crate::entities::{algorithms, frontmatter};
use chrono::format::ParseError;
use chrono::{Date, NaiveDate, Utc};
use serde_yaml::Value;
use std::ffi::OsStr;
use std::fmt;
use std::path::{Component, Path};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug)]
pub enum ReviewHistoryError {
    FrontmatterError(frontmatter::FrontmatterError),
    DateParseError(ParseError),
    ValueError,
}

impl fmt::Display for ReviewHistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReviewHistoryError::FrontmatterError(e) => e.fmt(f),
            ReviewHistoryError::DateParseError(e) => e.fmt(f),
            ReviewHistoryError::ValueError => write!(f, "ValueError"), // TODO: Determine how this should be formatted
        }
    }
}

pub fn get_cards(path: &str, algorithm: &str) -> Vec<DirEntry> {
    // TODO: Handle errors here
    WalkDir::new(path)
        .into_iter()
        .filter_map(|entry_result| match entry_result {
            Ok(entry) => match entry.path().canonicalize() {
                Ok(cp) => {
                    if cp
                        .components()
                        .find(|c| c == &Component::Normal(OsStr::new("cards")))
                        .is_some()
                    {
                        match entry.path().extension() {
                            Some(extension_option) => match extension_option.to_str() {
                                Some(extension) => {
                                    if extension == "md" {
                                        match review_time(entry.path(), algorithm) {
                                            true => Some(entry),
                                            false => None,
                                        }
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            },
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                Err(_) => None,
            },
            Err(_) => None,
        })
        .into_iter()
        .collect::<Vec<DirEntry>>()
}

fn review_time(path: &Path, algorithm: &str) -> bool {
    match algorithm {
        "all" => true,
        _ => {
            // TODO: Catch errors here
            let mut review_history = read_review_history(path).unwrap();
            match algorithm {
                "leitner" => algorithms::leitner(&mut review_history),
                _ => panic!(), // Cannot occur because clap will block invalid algorithm arguments
            }
        }
    }
}

fn read_review_history(path: &Path) -> Result<Vec<(Date<Utc>, bool)>, ReviewHistoryError> {
    let frontmatter = match frontmatter::read_fm(path) {
        Ok(fm) => fm,
        Err(e) => return Err(ReviewHistoryError::FrontmatterError(e)),
    };
    match frontmatter
        .get("reviews")
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
