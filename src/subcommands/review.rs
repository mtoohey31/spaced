use std::cell::RefCell;
use std::io;
use std::time::SystemTime;
use termion::{
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

use crate::entities::{cards, frontmatter};

enum UndoItem {
    Mark(DirEntry, bool),
    MarkArchived(DirEntry, bool),
    Skip,
}

pub fn review(matches: Option<&clap::ArgMatches>) {
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

    // TODO: refactor into atomic integers
    let remembered_cards = RefCell::new(0);
    let forgotten_cards = RefCell::new(0);
    let curr_side = RefCell::new(0);
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
                    sides = sides
                        .drain(0..(*curr_side.borrow() + 1))
                        .into_iter()
                        .collect();
                    // TODO: Parse markdown here
                    // TODO: Center markdown and be smart about where it breaks to keep borders on
                    // all sides as close to equal as possible
                    // TODO: Handle scrolling and scroll to bottom by default
                    let card = Paragraph::new(sides.join("\n---\n"))
                        .block(Block::default().borders(topbottomless));

                    let hint_string;
                    if *curr_side.borrow() + 1 == *num_sides.borrow() {
                        hint_string = "[f]orgot [space] remembered [l] skip [a]rchive [q]uit";
                    } else {
                        hint_string = "[f]orgot [space] flip [l] skip [a]rchive [q]uit";
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
                        *curr_side.borrow_mut() = 0;
                        let card = cards.remove(0);
                        cards.push(card);
                        undo_stack.push(UndoItem::Skip);
                    }
                    Key::Char(' ') => {
                        if *curr_side.borrow() + 1 == *num_sides.borrow() {
                            cards::mark(cards[0].path(), true);
                            *curr_side.borrow_mut() = 0;
                            *remembered_cards.borrow_mut() += 1;
                            undo_stack.push(UndoItem::Mark(cards.remove(0), true));
                        } else {
                            *curr_side.borrow_mut() += 1;
                        }
                    }
                    Key::Char('f') => {
                        cards::mark(cards[0].path(), false);
                        *curr_side.borrow_mut() = 0;
                        *forgotten_cards.borrow_mut() += 1;
                        undo_stack.push(UndoItem::Mark(cards.remove(0), false));
                    }
                    Key::Char('a') => {
                        cards::mark_archived(cards[0].path(), true);
                        *curr_side.borrow_mut() = 0;
                        undo_stack.push(UndoItem::MarkArchived(cards.remove(0), false));
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
                            UndoItem::MarkArchived(entry, archived) => {
                                cards::mark_archived(entry.path(), !archived);
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