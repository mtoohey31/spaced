use serde_yaml::{Mapping, Value};
use std::fmt;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::path::Path;

#[derive(Debug)]
pub enum FrontmatterError {
    IOError(std::io::Error),
    YAMLError(serde_yaml::Error),
    ValueError,
}

impl fmt::Display for FrontmatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrontmatterError::IOError(e) => e.fmt(f),
            FrontmatterError::YAMLError(e) => e.fmt(f),
            FrontmatterError::ValueError => write!(f, "ValueError"), // TODO: Determine how this should be formatted
        }
    }
}

pub fn read_fm(path: &Path) -> Result<Value, FrontmatterError> {
    match File::open(path) {
        Ok(f) => {
            let reader = BufReader::new(f);
            let mut lines = reader.lines().into_iter();
            match lines.next() {
                Some(line_result) => match line_result {
                    Ok(line) => match line.as_str() {
                        "---" => {
                            let mut yaml_lines = Vec::new();
                            loop {
                                let line = lines.next();
                                match line {
                                    Some(line) => match line {
                                        Ok(line) => match line.as_str() {
                                            "---" => break,
                                            _ => yaml_lines.push(String::from("\n") + &line),
                                        },
                                        Err(e) => return Err(FrontmatterError::IOError(e)),
                                    },
                                    None => return Ok(Value::Mapping(Mapping::new())),
                                }
                            }
                            let deserialized_frontmatter: Result<Mapping, _> =
                                serde_yaml::from_str(&yaml_lines.join("\n"));
                            match deserialized_frontmatter {
                                Ok(fm) => Ok(Value::Mapping(fm)),
                                Err(e) => Err(FrontmatterError::YAMLError(e)),
                            }
                        }
                        _ => Ok(Value::Null),
                    },
                    Err(e) => Err(FrontmatterError::IOError(e)),
                },
                None => Ok(Value::Null),
            }
        }
        Err(e) => Err(FrontmatterError::IOError(e)),
    }
}

pub fn read_body(path: &Path) -> Result<String, FrontmatterError> {
    match File::open(path) {
        Ok(f) => {
            let reader = BufReader::new(f);
            let mut lines = reader.lines().into_iter();
            match lines.next() {
                Some(line_result) => match line_result {
                    Ok(line) => match line.as_str() {
                        "---" => {
                            let mut yaml_lines = Vec::new();
                            loop {
                                match lines.next() {
                                    Some(line) => match line {
                                        Ok(line) => match line.as_str() {
                                            "---" => break,
                                            _ => yaml_lines.push(String::from("\n") + &line),
                                        },
                                        Err(e) => return Err(FrontmatterError::IOError(e)),
                                    },
                                    None => {
                                        let mut body = yaml_lines;
                                        for line in lines {
                                            match line {
                                                Ok(l) => body.push(l),
                                                Err(e) => return Err(FrontmatterError::IOError(e)),
                                            }
                                        }
                                        return Ok(body.join("\n"));
                                    }
                                }
                            }
                            let mut body = vec![];
                            match lines.next() {
                                Some(l) => match l {
                                    Ok(l) => match l.as_ref() {
                                        "" => {}
                                        _ => {
                                            body.push(l);
                                        }
                                    },
                                    Err(e) => return Err(FrontmatterError::IOError(e)),
                                },
                                None => {}
                            }
                            for line in lines {
                                match line {
                                    Ok(l) => body.push(l),
                                    Err(e) => return Err(FrontmatterError::IOError(e)),
                                }
                            }
                            Ok(body.join("\n"))
                        }
                        first_line => {
                            let mut body = vec![String::from(first_line)];
                            for line in lines {
                                match line {
                                    Ok(l) => body.push(l),
                                    Err(e) => return Err(FrontmatterError::IOError(e)),
                                }
                            }
                            Ok(body.join("\n"))
                        }
                    },
                    Err(e) => Err(FrontmatterError::IOError(e)),
                },
                None => Ok(String::from("")),
            }
        }
        Err(e) => Err(FrontmatterError::IOError(e)),
    }
}
pub fn read_fm_and_body(path: &Path) -> Result<(Mapping, String), FrontmatterError> {
    match File::open(path) {
        Ok(f) => {
            let reader = BufReader::new(f);
            let mut lines = reader.lines().into_iter();
            match lines.next() {
                Some(line_result) => match line_result {
                    Ok(line) => match line.as_str() {
                        "---" => {
                            let mut yaml_lines = Vec::new();
                            loop {
                                match lines.next() {
                                    Some(line) => match line {
                                        Ok(line) => match line.as_str() {
                                            "---" => break,
                                            _ => yaml_lines.push(String::from("\n") + &line),
                                        },
                                        Err(e) => return Err(FrontmatterError::IOError(e)),
                                    },
                                    None => {
                                        let mut body = yaml_lines;
                                        for line in lines {
                                            match line {
                                                Ok(l) => body.push(l),
                                                Err(e) => return Err(FrontmatterError::IOError(e)),
                                            }
                                        }
                                        return Ok((Mapping::new(), body.join("\n")));
                                    }
                                }
                            }
                            let deserialized_frontmatter: Result<Mapping, _> =
                                serde_yaml::from_str(&yaml_lines.join("\n"));
                            let mut body = vec![];
                            match lines.next() {
                                Some(l) => match l {
                                    Ok(l) => match l.as_ref() {
                                        "" => {}
                                        _ => {
                                            body.push(l);
                                        }
                                    },
                                    Err(e) => return Err(FrontmatterError::IOError(e)),
                                },
                                None => {}
                            }
                            for line in lines {
                                match line {
                                    Ok(l) => body.push(l),
                                    Err(e) => return Err(FrontmatterError::IOError(e)),
                                }
                            }
                            match deserialized_frontmatter {
                                Ok(fm) => Ok((fm, body.join("\n"))),
                                Err(e) => Err(FrontmatterError::YAMLError(e)),
                            }
                        }
                        first_line => {
                            let mut body = vec![String::from(first_line)];
                            for line in lines {
                                match line {
                                    Ok(l) => body.push(l),
                                    Err(e) => return Err(FrontmatterError::IOError(e)),
                                }
                            }
                            Ok((Mapping::new(), body.join("\n")))
                        }
                    },
                    Err(e) => Err(FrontmatterError::IOError(e)),
                },
                None => Ok((Mapping::new(), String::from(""))),
            }
        }
        Err(e) => Err(FrontmatterError::IOError(e)),
    }
}

pub fn write_body(path: &Path, body: String) -> Result<(), FrontmatterError> {
    let mut file = match File::create(path) {
        Ok(f) => f,
        Err(e) => return Err(FrontmatterError::IOError(e)),
    };
    match write!(file, "{}", body) {
        Ok(_) => Ok(()),
        Err(e) => Err(FrontmatterError::IOError(e)),
    }
}

pub fn write_fm_and_body(path: &Path, fm: Value, body: String) -> Result<(), FrontmatterError> {
    let mut file = match File::create(path) {
        Ok(f) => f,
        Err(e) => return Err(FrontmatterError::IOError(e)),
    };
    let fm = match serde_yaml::to_string(&fm) {
        Ok(fm) => fm,
        Err(e) => return Err(FrontmatterError::YAMLError(e)),
    };
    match write!(file, "{}---\n\n{}", fm, body) {
        Ok(_) => Ok(()),
        Err(e) => Err(FrontmatterError::IOError(e)),
    }
}
