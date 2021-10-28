use crate::frontmatter::{read_fm, FrontmatterError};
use serde_yaml::Value;
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
                                        if all || !spaced(entry.path()).unwrap() {
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

pub fn spaced(path: &Path) -> Result<bool, FrontmatterError> {
    match read_fm(path) {
        Ok(fm) => match fm.get("spaced") {
            Some(spaced) => match spaced {
                Value::Bool(b) => Ok(*b),
                _ => Err(FrontmatterError::ValueError),
            },
            None => Ok(false),
        },
        Err(e) => Err(e),
    }
}
