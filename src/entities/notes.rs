use crate::entities::frontmatter::read_fm;
use crate::error::ValueError;
use serde_yaml::Value;
use std::error::Error;
use std::ffi::OsStr;
use std::path::{Component, Path};
use walkdir::{DirEntry, WalkDir};

pub fn get_notes(path: &str, all: bool) -> Vec<DirEntry> {
    // TODO: Handle errors here
    WalkDir::new(path)
        .into_iter()
        .filter_map(|entry_result| match entry_result {
            Ok(entry) => match entry.path().canonicalize() {
                Ok(cp) => {
                    if cp
                        .components()
                        .find(|c| c == &Component::Normal(OsStr::new("notes")))
                        .is_some()
                    {
                        match entry.path().extension() {
                            Some(extension_option) => match extension_option.to_str() {
                                Some(extension) => {
                                    if extension == "md" {
                                        if all || !is_spaced(entry.path()).unwrap() {
                                            Some(entry)
                                        } else {
                                            None
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

pub fn is_spaced(path: &Path) -> Result<bool, Box<dyn Error>> {
    match read_fm(path) {
        Ok(fm) => match fm.get(&Value::String(String::from("spaced"))) {
            Some(spaced) => spaced.as_bool().ok_or(Box::new(ValueError::new())),
            None => Ok(false),
        },
        Err(e) => Err(e),
    }
}
