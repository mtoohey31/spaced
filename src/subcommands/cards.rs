use crate::entities::frontmatter;
use serde_yaml::Value;
use std::io::{self, Write};
use termion::{color, style};

use crate::entities::cards;

pub fn cards(matches: Option<&clap::ArgMatches>) {
    match matches {
        Some(m) => match m.subcommand_name() {
            Some("clear-history") | None => {
                cards_clear_history(m.subcommand_matches("clear-history"))
            }
            _ => {} // Cannot occur since no other subcommands are specified in ../cli.yaml
        },
        None => cards_clear_history(None),
    }
}

fn cards_clear_history(matches: Option<&clap::ArgMatches>) {
    let (path, no_confirm) = match matches {
        Some(m) => (
            m.value_of("PATH").unwrap_or("."),
            m.is_present("no-confirm"),
        ),
        _ => (".", false),
    };

    let cards = cards::get_cards(path, "all");

    if cards.len() == 0 {
        eprintln!("No cards found to review");
        return;
    }

    if !no_confirm {
        println!(
            "{}{}# Warning, history will be cleared for...{}{}\n",
            style::Bold,
            color::Fg(color::Red),
            style::Reset,
            color::Fg(color::Reset)
        );
    }

    for entry in cards.clone() {
        println!("{}", entry.path().display());
    }

    loop {
        print!("Proceed? [y/N] ");
        let _ = io::stdout().flush();
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            // TODO: use lowercase conversion here
            Ok(_) => match input.as_ref() {
                "Y\n" | "y\n" | "YES\n" | "Yes\n" | "yes\n" => break,
                "\n" | "N\n" | "n\n" | "NO\n" | "No\n" | "no\n" => return,
                _ => {
                    eprintln!(
                        "{}Invalid input, please try again{}",
                        color::Fg(color::Red),
                        color::Fg(color::Reset)
                    );
                    continue;
                }
            },
            Err(e) => panic!("{}", e),
        };
    }

    for entry in cards {
        let (mut mapping, body) = match frontmatter::read_fm_and_body(entry.path()) {
            Ok(fm) => fm,
            Err(e) => panic!("{}", e),
        };

        mapping.remove(&Value::String(String::from("reviews")));

        if mapping.len() > 0 {
            match frontmatter::write_fm_and_body(entry.path(), Value::Mapping(mapping), body) {
                Ok(_) => {}
                Err(e) => panic!("{}", e),
            };
        } else {
            match frontmatter::write_body(entry.path(), body) {
                Ok(_) => {}
                Err(e) => panic!("{}", e),
            };
        }
    }
}
