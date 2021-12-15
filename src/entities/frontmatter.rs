use serde_yaml::Mapping;
use serde_yaml::Value;
use std::error::Error;
use std::fs::File;
use std::io::{prelude::*, BufReader, Lines};
use std::iter::Peekable;
use std::path::Path;

type Consumable = Peekable<Lines<BufReader<File>>>;

pub fn read_fm(path: &Path) -> Result<Mapping, Box<dyn Error>> {
    consume_fm(&mut produce_consumable(path)?)
}

fn produce_consumable(path: &Path) -> Result<Consumable, Box<dyn Error>> {
    let reader = BufReader::new(File::open(path)?);
    Ok(reader.lines().into_iter().peekable())
}

fn consume_fm(line_iter: &mut Consumable) -> Result<Mapping, Box<dyn Error>> {
    Ok(match consume_fm_text(line_iter)? {
        Some(s) => serde_yaml::from_str(&s)?,
        None => serde_yaml::Mapping::new(),
    })
}

fn consume_fm_text(line_iter: &mut Consumable) -> Result<Option<String>, Box<dyn Error>> {
    if let Some(_) = line_iter.next_if(|first| {
        if let Ok(first) = first {
            first == "---"
        } else {
            false
        }
    }) {
        Ok(Some(
            line_iter
                .map_while(|line| {
                    if let Ok(line) = line {
                        if line == "---" {
                            return None;
                        }
                        Some(line)
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()
                .join("\n"),
        ))
    } else {
        Ok(None)
    }
}

pub fn read_body(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut consumable = produce_consumable(path)?;
    consume_fm_text(&mut consumable)?;
    consume_rest_text(consumable)
}

fn consume_rest_text(line_iter: Consumable) -> Result<String, Box<dyn Error>> {
    Ok(line_iter.collect::<Result<Vec<String>, _>>()?.join("\n"))
}

pub fn read_fm_and_body(path: &Path) -> Result<(Mapping, String), Box<dyn Error>> {
    let mut consumable = produce_consumable(path)?;
    Ok((consume_fm(&mut consumable)?, consume_rest_text(consumable)?))
}

pub fn write_body(path: &Path, body: String) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(path)?;
    Ok(write!(file, "{}", body)?)
}

pub fn write_fm_and_body(path: &Path, fm: Value, body: String) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(path)?;
    let fm = serde_yaml::to_string(&fm)?;
    Ok(write!(file, "{}---\n\n{}", fm, body)?)
}
