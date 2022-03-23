use crossterm::{
    cursor,
    event::{read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::Stylize,
    terminal::{
        disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    env,
    error::Error,
    io::{self, Write},
    process::Command,
};
use walkdir::DirEntry;

use crate::entities::{cards, frontmatter};

const BOX_EMPTY: &str = " ";
const BOX_FULL: &str = "█";
const BOX_LEFT: [&str; 8] = [BOX_EMPTY, "▏", "▎", "▍", "▌", "▋", "▊", "▉"];
const BOX_RIGHT: [&str; 8] = [BOX_EMPTY, "▕", "🮇", "🮈", "▐", "🮉", "🮊", "🮋"];

enum UndoItem {
    MarkRemembered(DirEntry),
    MarkForgotten,
    MarkArchived(DirEntry),
    Skip,
}

// TODO: create a library and refactor the list of cards into a circular linked list for better
// performance
// TODO: handle foresable errors such as reading card bodies better
pub fn review(matches: Option<&clap::ArgMatches>) -> Result<(), Box<dyn Error>> {
    let (path, algorithm) = match matches {
        Some(m) => (
            m.value_of("PATH").unwrap_or("."),
            m.value_of("algorithm").unwrap_or("leitner"),
        ),
        _ => (".", "leitner"),
    };

    let mut cards = cards::get_cards(path, algorithm);

    if cards.len() == 0 {
        return Ok(());
    }

    let mut remembered = 0;
    let mut forgotten = 0;
    let mut component = 0;
    let card = frontmatter::read_body(&cards[0].path())?;
    let mut components: Vec<String> = card.split("\n---\n").map(|s| s.to_string()).collect();
    let mut undo_stack = Vec::new();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    print_progress(&mut stdout, remembered, forgotten, cards.len())?;
    print_card(&mut stdout, component, &components)?;
    stdout.flush()?;

    loop {
        match read()? {
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers { .. },
            }) => break,
            Event::Key(KeyEvent {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers { .. },
            }) => {
                if component == components.len() - 1 {
                    remembered += 1;
                    cards::mark(cards[0].path(), true);
                    undo_stack.push(UndoItem::MarkRemembered(cards.remove(0)));
                    if cards.len() == 0 {
                        break;
                    }

                    component = 0;
                    let card = frontmatter::read_body(&cards[0].path())?;
                    components = card.split("\n---\n").map(|s| s.to_string()).collect();
                    print_progress(&mut stdout, remembered, forgotten, cards.len())?;
                    print_card(&mut stdout, component, &components)?;
                } else {
                    component += 1;
                    print_card(&mut stdout, component, &components)?;
                }
                stdout.flush()?;
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('s'),
                modifiers: KeyModifiers { .. },
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers { .. },
            }) => {
                let card = cards.remove(0);
                cards.push(card);
                undo_stack.push(UndoItem::Skip);

                component = 0;
                let card = frontmatter::read_body(&cards[0].path())?;
                components = card.split("\n---\n").map(|s| s.to_string()).collect();
                print_progress(&mut stdout, remembered, forgotten, cards.len())?;
                print_card(&mut stdout, component, &components)?;
                stdout.flush()?;
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('f'),
                modifiers: KeyModifiers { .. },
            }) => {
                forgotten += 1;
                let card = cards.remove(0);
                cards::mark(card.path(), false);
                cards.push(card);
                undo_stack.push(UndoItem::MarkForgotten);

                component = 0;
                let card = frontmatter::read_body(&cards[0].path())?;
                components = card.split("\n---\n").map(|s| s.to_string()).collect();
                print_progress(&mut stdout, remembered, forgotten, cards.len())?;
                print_card(&mut stdout, component, &components)?;
                stdout.flush()?;
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers { .. },
            }) => {
                let card = cards.remove(0);
                if cards.len() == 0 {
                    break;
                }
                undo_stack.push(UndoItem::MarkArchived(card));

                component = 0;
                let card = frontmatter::read_body(&cards[0].path())?;
                components = card.split("\n---\n").map(|s| s.to_string()).collect();
                print_progress(&mut stdout, remembered, forgotten, cards.len())?;
                print_card(&mut stdout, component, &components)?;
                stdout.flush()?;
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('u'),
                modifiers: KeyModifiers { .. },
            }) => {
                if let Some(undo_item) = undo_stack.pop() {
                    match undo_item {
                        UndoItem::MarkRemembered(c) => {
                            remembered -= 1;
                            cards::unmark(c.path());
                            cards.insert(0, c);
                        }
                        UndoItem::MarkForgotten => {
                            forgotten -= 1;
                            let card = cards.pop().unwrap();
                            cards::unmark(card.path());
                            cards.insert(0, card);
                        }
                        UndoItem::MarkArchived(c) => {
                            cards::mark_archived(c.path(), false);
                            cards.insert(0, c);
                        }
                        UndoItem::Skip => {
                            let card = cards.pop().unwrap();
                            cards.insert(0, card);
                        }
                    }

                    component = 0;
                    let card = frontmatter::read_body(&cards[0].path())?;
                    components = card.split("\n---\n").map(|s| s.to_string()).collect();
                    print_progress(&mut stdout, remembered, forgotten, cards.len())?;
                    print_card(&mut stdout, component, &components)?;
                    stdout.flush()?;
                }
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers { .. },
            }) => {
                execute!(stdout, LeaveAlternateScreen, cursor::Show)?;
                disable_raw_mode()?;

                // TODO: extract this code and the code for editing notes into a helper module
                let editor = env::var("VISUAL")
                    .unwrap_or(env::var("EDITOR").expect("please set $VISUAL or $EDITOR"));
                Command::new(editor.clone())
                    .args([cards[0].path().as_os_str()])
                    .status()
                    .expect(&format!("failed to execute {}", editor));

                component = 0;
                let card = frontmatter::read_body(&cards[0].path())?;
                components = card.split("\n---\n").map(|s| s.to_string()).collect();

                enable_raw_mode()?;
                execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
                print_progress(&mut stdout, remembered, forgotten, cards.len())?;
                print_card(&mut stdout, component, &components)?;
                stdout.flush()?;
            }
            Event::Resize(..) => {
                print_progress(&mut stdout, remembered, forgotten, cards.len())?;
                print_card(&mut stdout, component, &components)?;
                stdout.flush()?;
            }
            _ => (),
        };
    }

    execute!(stdout, LeaveAlternateScreen, cursor::Show)?;
    disable_raw_mode()?;
    Ok(())
}

// TODO: add alternate implementation using configurable pandoc feature here with prettier printing
fn print_card(
    stdout: &mut io::Stdout,
    component: usize,
    components: &Vec<String>,
) -> Result<(), io::Error> {
    execute!(stdout, cursor::MoveTo(0, 1))?;
    execute!(stdout, Clear(ClearType::FromCursorDown))?;
    lazy_static! {
        static ref RE: Regex = Regex::new("<!--([^-]|-[^-]|--[^>])*-->(\r\n){0,2}").unwrap();
    }
    write!(
        stdout,
        "{}",
        RE.replace_all(
            &components[..component + 1]
                .join("\n---\n")
                .replace("\n", "\r\n"),
            ""
        )
    )
}

// TODO: turn this progress bar into its own crate with support for non-tui applications as well as
// different progress alignment and support for multiple data types such as time and storage units.
// if we want to get real fancy, we could support animations so it's even smoother
// check if there's already a good rust progress bar library, cause maybe I could contribute to
// that as an added double ended feature
fn print_progress(
    stdout: &mut io::Stdout,
    remembered: usize,
    forgotten: usize,
    incomplete: usize,
) -> Result<(), Box<dyn Error>> {
    let (cols, _) = size()?;
    let cols = cols as usize;

    execute!(stdout, cursor::MoveTo(0, 0))?;

    let r_bar_length = (remembered as f32 / (remembered + incomplete) as f32) * cols as f32;
    let r_floored_length = r_bar_length.floor();
    let r_remainder = r_bar_length - r_floored_length;
    let r_floored_length = r_floored_length as usize;

    let f_bar_length = (forgotten as f32 / (remembered + incomplete) as f32) * cols as f32;
    let f_floored_length = f_bar_length.floor();
    let f_remainder = f_bar_length - f_floored_length;
    let f_floored_length = f_floored_length as usize;

    let text = format!("{}/{}", remembered, (remembered + incomplete));
    let text_len = text.len();
    let text_pos = ((cols - text_len) as f32 / 2_f32).ceil() as usize;
    let text_range = text_pos..text_pos + text_len;

    if r_floored_length + f_floored_length >= cols - 1 {
        if r_floored_length < text_range.start {
            write!(
                stdout,
                "{}{}{}{}{}",
                BOX_FULL.repeat(r_floored_length).green(),
                BOX_LEFT[(r_remainder * 8_f32) as usize].green().on_red(),
                BOX_FULL
                    .repeat(text_range.start - r_floored_length - 1)
                    .red(),
                text.on_red(),
                BOX_FULL.repeat(cols - text_range.end).red()
            )?
        } else if text_range.contains(&r_floored_length) {
            let r_length = r_floored_length + (r_remainder.round() as usize);
            write!(
                stdout,
                "{}{}{}{}",
                BOX_FULL.repeat(text_range.start).green(),
                text[..r_length - text_range.start].on_green(),
                text[r_length - text_range.start..].on_red(),
                BOX_FULL.repeat(cols - text_range.end).red()
            )?
        } else {
            write!(
                stdout,
                "{}{}{}{}{}",
                BOX_FULL.repeat(text_range.start).green(),
                text.on_green(),
                BOX_FULL.repeat(r_floored_length - text_range.end).green(),
                BOX_LEFT[(r_remainder * 8_f32) as usize].green().on_red(),
                BOX_FULL.repeat(cols - r_floored_length - 1).red()
            )?
        }
    } else {
        if cols - f_floored_length <= text_range.start {
            write!(
                stdout,
                "{}{}{}{}{}{}{}",
                BOX_FULL.repeat(r_floored_length).green(),
                BOX_LEFT[(r_remainder * 8_f32) as usize].green(),
                BOX_EMPTY.repeat(cols - r_floored_length - f_floored_length - 2),
                BOX_RIGHT[(f_remainder * 8_f32) as usize].red(),
                BOX_FULL
                    .repeat(text_range.start - (cols - f_floored_length))
                    .red(),
                text.on_red(),
                BOX_FULL.repeat(cols - text_range.end).red(),
            )?;
        } else if text_range.contains(&(cols - f_floored_length - 1))
            && !text_range.contains(&r_floored_length)
        {
            let f_length = f_floored_length + (f_remainder.round() as usize);
            write!(
                stdout,
                "{}{}{}{}{}{}",
                BOX_FULL.repeat(r_floored_length).green(),
                BOX_LEFT[(r_remainder * 8_f32) as usize].green(),
                BOX_EMPTY.repeat(text_range.start - r_floored_length - 1),
                &text[..cols - f_length - text_range.start],
                text[cols - f_length - text_range.start..].on_red(),
                BOX_FULL.repeat(cols - text_range.end).red(),
            )?;
        } else if text_range.contains(&(cols - f_floored_length - 1))
            && text_range.contains(&r_floored_length)
        {
            let r_length = r_floored_length + (r_remainder.round() as usize);
            let f_length = f_floored_length + (f_remainder.round() as usize);
            write!(
                stdout,
                "{}{}{}{}{}",
                BOX_FULL.repeat(text_range.start).green(),
                text[..r_length - text_range.start].on_green(),
                &text[r_length - text_range.start..cols - f_length - text_range.start],
                text[cols - f_length - text_range.start..].on_red(),
                BOX_FULL.repeat(cols - text_range.end).red(),
            )?;
        } else if r_floored_length < text_range.start {
            write!(
                stdout,
                "{}{}{}{}{}{}{}",
                BOX_FULL.repeat(r_floored_length).green(),
                BOX_LEFT[(r_remainder * 8_f32) as usize].green(),
                BOX_EMPTY.repeat(text_range.start - r_floored_length - 1),
                text,
                BOX_EMPTY.repeat((cols - f_floored_length - 1) - text_range.end),
                BOX_RIGHT[(f_remainder * 8_f32) as usize].red(),
                BOX_FULL.repeat(f_floored_length).red()
            )?
        } else if text_range.contains(&r_floored_length) {
            let r_length = r_floored_length + (r_remainder.round() as usize);
            write!(
                stdout,
                "{}{}{}{}{}{}",
                BOX_FULL.repeat(text_range.start).green(),
                text[..r_length - text_range.start].on_green(),
                &text[r_length - text_range.start..],
                BOX_EMPTY.repeat((cols - f_floored_length - 1) - text_range.end),
                BOX_RIGHT[(f_remainder * 8_f32) as usize].red(),
                BOX_FULL.repeat(f_floored_length).red()
            )?
        } else {
            write!(
                stdout,
                "{}{}{}{}{}{}{}",
                BOX_FULL.repeat(text_range.start).green(),
                text.on_green(),
                BOX_FULL.repeat(r_floored_length - text_range.end).green(),
                BOX_LEFT[(r_remainder * 8_f32) as usize].green(),
                BOX_EMPTY.repeat(cols - r_floored_length - f_floored_length - 2),
                BOX_RIGHT[(f_remainder * 8_f32) as usize].red(),
                BOX_FULL.repeat(f_floored_length).red()
            )?
        }
    }

    Ok(())
}
