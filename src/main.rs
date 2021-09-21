// TODO: Display errors to users instead of panicking
// TODO: Add comments to a bunch of stuff, and look into how to properly document rust functions
// TODO: Add vim config snippet to fold frontmatter to docs
// TODO: Support anki and mochi imports, with review history if possible
// (https://github.com/zip-rs/zip/blob/7edf2489d5cff8b80f02ee6fc5febf3efd0a9442/examples/extract.rs, https://crates.io/crates/edn-rs)
extern crate clap;
use clap::App;
use clap::Arg;
use clap::SubCommand;
use serde_yaml::Value;
use std::cell::RefCell;
use std::fs::metadata;
use std::io::{self, Write};
use std::time::SystemTime;
use termion::{
    color,
    event::Key,
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::AlternateScreen,
    style,
};
use tui::{
    backend::TermionBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, Gauge, Paragraph},
    Terminal,
};
use walkdir::DirEntry;

mod algorithms;
mod cards;
mod frontmatter;
mod notes;

fn main() {
    let matches = App::new("spaced")
        .version("0.1.0")
        .author("mtoohey31 <mtoohey31@users.noreply.github.com>")
        .about("Spaced repetition in YAML")
        .subcommand(
            SubCommand::with_name("cards")
                .alias("c")
                .about("Handle cards")
                .subcommand(
                    SubCommand::with_name("clear-history")
                        .alias("c")
                        .about("Clear review history")
                        .arg(
                            Arg::with_name("no-confirm")
                                .short("y")
                                .long("no-confirm")
                                .help("Do not ask for confirmation"),
                        )
                        .arg(Arg::with_name("PATH").index(1)),
                ),
        )
        .subcommand(
            SubCommand::with_name("notes")
                .alias("n")
                .about("Handle notes")
                .arg(
                    Arg::with_name("all")
                        .short("a")
                        .long("all")
                        .help("Show all notes"),
                ),
        )
        .subcommand(
            SubCommand::with_name("review")
                .alias("r")
                .about("Review cards")
                .arg(
                    Arg::with_name("algorithm")
                        .short("a")
                        .long("algorithm")
                        .takes_value(true)
                        .possible_values(&[
                            "all",
                            "leitner",
                            // "half-life" // duolingo
                            // "super-memo"
                        ]),
                )
                .arg(Arg::with_name("PATHS").index(1)),
        )
        .get_matches();
    match matches.subcommand_name() {
        Some("cards") => cards(matches.subcommand_matches("cards")),
        Some("notes") => notes(matches.subcommand_matches("notes")),
        Some("review") | None => review(matches.subcommand_matches("review")),
        _ => {} // Cannot occur since no other subcommands are specified in ../cli.yaml
    }
}

fn cards(matches: Option<&clap::ArgMatches>) {
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

fn notes(matches: Option<&clap::ArgMatches>) {
    let (path, all) = match matches {
        Some(m) => (m.value_of("PATH").unwrap_or("."), m.is_present("all")),
        _ => (".", false),
    };

    let mut entries = notes::get_notes(path, all);

    entries.sort_by(|a, b| {
        let a_time = match metadata(a.path()) {
            Ok(m) => m.created().unwrap_or(std::time::SystemTime::now()),
            _ => std::time::SystemTime::now(),
        };
        let b_time = match metadata(b.path()) {
            Ok(m) => m.created().unwrap_or(std::time::SystemTime::now()),
            _ => std::time::SystemTime::now(),
        };
        a_time.cmp(&b_time)
    });

    for entry in entries {
        println!("{}", entry.path().display());
    }
}

enum UndoItem {
    Mark(DirEntry, bool),
    Skip,
}

fn review(matches: Option<&clap::ArgMatches>) {
    let (path, algorithm) = match matches {
        Some(m) => (
            m.value_of("PATH").unwrap_or("."),
            m.value_of("algorithm").unwrap_or("leitner"),
        ),
        _ => (".", "leitner"),
    };

    let mut cards = cards::get_cards(path, algorithm);
    let mut undo_stack = Vec::new();

    if cards.len() == 0 {
        eprintln!("No cards found to review");
        return;
    }

    let remembered_cards = RefCell::new(0);
    let forgotten_cards = RefCell::new(0);
    let curr_side = RefCell::new(1);
    let num_sides = RefCell::new(0);

    let start_time = SystemTime::now();

    {
        let stdin = io::stdin();
        let stdout = io::stdout().into_raw_mode().unwrap();
        let stdout = MouseTerminal::from(stdout);
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut bottomless = Borders::ALL;
        bottomless.remove(Borders::BOTTOM);
        let mut topbottomless = Borders::ALL;
        topbottomless.remove(Borders::TOP);
        topbottomless.remove(Borders::BOTTOM);
        let mut topless = Borders::ALL;
        topless.remove(Borders::TOP);

        let mut draw = |cards: &Vec<DirEntry>| {
            terminal
                .draw(|frame| {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(
                            [
                                Constraint::Length(2),
                                Constraint::Length(frame.size().height - 4),
                                Constraint::Length(2),
                            ]
                            .as_ref(),
                        )
                        .split(frame.size());

                    let numerator = *remembered_cards.borrow() + *forgotten_cards.borrow();
                    let denominator = cards.len() + numerator;
                    let progress = Gauge::default()
                        .block(Block::default().title(" spaced ").borders(bottomless))
                        .ratio(numerator as f64 / denominator as f64)
                        .gauge_style(tui::style::Style::default().fg(Color::Green))
                        .label(format!("{}/{}", numerator, denominator,));

                    let dir_entry: &DirEntry = &cards[0];
                    // TODO: Handle errors here
                    let body = frontmatter::read_body(dir_entry.path()).unwrap();
                    let mut sides = body.split("\n---\n").collect::<Vec<&str>>();
                    *num_sides.borrow_mut() = sides.len();
                    sides = sides.drain(0..*curr_side.borrow()).into_iter().collect();
                    // TODO: Parse markdown here
                    // TODO: Center markdown and be smart about where it breaks to keep borders on
                    // all sides as close to equal as possible
                    // TODO: Handle scrolling and scroll to bottom by default
                    let card = Paragraph::new(sides.join("\n---\n"))
                        .block(Block::default().borders(topbottomless));

                    let hint_string;
                    if *curr_side.borrow() == *num_sides.borrow() {
                        hint_string = "[f]orgot [space] remembered [l] skip [q]uit";
                    } else {
                        hint_string = "[f]orgot [space] flip [l] skip [q]uit";
                    }
                    let hints = Paragraph::new(hint_string)
                        .block(Block::default().borders(topless))
                        .alignment(Alignment::Center);

                    frame.render_widget(progress, chunks[0]);
                    frame.render_widget(card, chunks[1]);
                    frame.render_widget(hints, chunks[2]);
                })
                .unwrap()
        };

        draw(&cards);

        for event in stdin.events() {
            match event.unwrap() {
                termion::event::Event::Key(key) => match key {
                    Key::Char('q') => break,
                    Key::Char('e') => {} // TODO: Implement this
                    Key::Char('l') => {
                        *curr_side.borrow_mut() = 1;
                        let card = cards.remove(0);
                        cards.push(card);
                        undo_stack.push(UndoItem::Skip);
                    }
                    Key::Char(' ') => {
                        if *curr_side.borrow() == *num_sides.borrow() {
                            cards::mark(cards[0].path(), true);
                            *curr_side.borrow_mut() = 1;
                            *remembered_cards.borrow_mut() += 1;
                            undo_stack.push(UndoItem::Mark(cards.remove(0), true));
                        } else {
                            *curr_side.borrow_mut() += 1;
                        }
                    }
                    Key::Char('f') => {
                        cards::mark(cards[0].path(), false);
                        *curr_side.borrow_mut() = 1;
                        *forgotten_cards.borrow_mut() += 1;
                        undo_stack.push(UndoItem::Mark(cards.remove(0), false));
                    }
                    Key::Char('u') => match undo_stack.pop() {
                        Some(undo_item) => match undo_item {
                            UndoItem::Mark(entry, remembered) => {
                                cards::unmark(entry.path());
                                if remembered {
                                    *remembered_cards.borrow_mut() -= 1;
                                } else {
                                    *forgotten_cards.borrow_mut() -= 1;
                                }
                                cards.insert(0, entry);
                            }
                            UndoItem::Skip => {
                                let card = match cards.pop() {
                                    Some(c) => c,
                                    None => panic!("Mismatched cards and undo stack"),
                                };
                                cards.insert(0, card);
                            }
                        },
                        None => {} // TODO: Inform user
                    },
                    _ => (),
                },
                _ => (),
            }
            if cards.len() == 0 {
                break;
            }

            draw(&cards);
        }
    }

    // TODO: Color code facts by how good they are
    println!("{}# Recap{}\n", style::Bold, style::Reset);
    let total = *remembered_cards.borrow() + *forgotten_cards.borrow();
    match start_time.elapsed() {
        Ok(e) => {
            let elapsed = e.as_secs_f64();
            let duration: f64;
            let unit: &str;
            if elapsed > 60_f64 {
                if elapsed > 3600_f64 {
                    duration = (elapsed / 360_f64).round() / 10_f64;
                    if duration == 1_f64 {
                        unit = "hour";
                    } else {
                        unit = "hours";
                    }
                } else {
                    duration = (elapsed / 6_f64).round() / 10_f64;
                    if duration == 1_f64 {
                        unit = "minute";
                    } else {
                        unit = "minutes";
                    }
                }
            } else {
                duration = (10_f64 * elapsed).round() / 10_f64;
                if duration == 1_f64 {
                    unit = "second";
                } else {
                    unit = "seconds";
                }
            }
            println!("• Reviewed {} cards in {} {}", total, duration, unit)
        }
        Err(_) => println!("• Reviewed {} cards", total),
    }
    println!(
        "• Remembered {}% of cards",
        (1000_f64 * *remembered_cards.borrow() as f64 / total.max(1) as f64).round() / 10_f64
    );
}