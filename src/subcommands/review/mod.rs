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
mod converters;

#[cfg(feature = "pandoc")]
const BASE16_THEME_BYTES: &[u8; 814] = include_bytes!("../../../assets/base16.themedump");

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
// TODO: make sure that write! and execute! are buffering and not actually writing until flush is
// called
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

#[cfg(feature = "pandoc")]
fn print_card(
    stdout: &mut io::Stdout,
    component: usize,
    components: &Vec<String>,
) -> Result<(), Box<dyn Error>> {
    use pandoc::{InputFormat, InputKind, OutputFormat, OutputKind};
    use pandoc_types::definition::IterBlocks;

    lazy_static! {
        static ref RE: Regex = Regex::new("<!--([^-]|-[^-]|--[^>])*-->\n{0,2}").unwrap();
    }

    let mut pandoc = pandoc::new();
    pandoc.set_input(InputKind::Pipe(
        RE.replace_all(&components[..component + 1].join("\n---\n"), "")
            .to_string(),
    ));
    pandoc.set_output(OutputKind::Pipe);
    pandoc.set_input_format(InputFormat::Markdown, Vec::new());
    pandoc.set_output_format(OutputFormat::Json, Vec::new());
    let ast = match pandoc.execute()? {
        pandoc::PandocOutput::ToBuffer(s) => {
            serde_json::from_str::<pandoc_types::definition::Pandoc>(&s).unwrap()
        }
        _ => panic!(),
    };

    execute!(stdout, cursor::MoveTo(0, 1))?;
    execute!(stdout, Clear(ClearType::FromCursorDown))?;
    write!(
        stdout,
        "{}",
        ast.iter_blocks()
            .map(stringify_pandoc_block)
            .intersperse_with(|| Ok("\n\n".to_string()))
            .collect::<Result<String, _>>()?
            .replace("\n", "\r\n")
    )?;
    Ok(())
}

// TODO: get rid of comments in this version too
#[cfg(feature = "pandoc")]
fn stringify_pandoc_block<'a>(
    block: &'a pandoc_types::definition::Block,
) -> Result<String, Box<dyn Error>> {
    use crossterm::style::Color;
    use pandoc_types::definition::{Block, ListNumberDelim, ListNumberStyle};
    use septem::Roman;
    use syntect::{
        dumps::from_binary,
        easy::HighlightLines,
        highlighting::{Style, Theme},
        parsing::SyntaxSet,
    };

    match block {
        Block::Plain(inline) => inline
            .iter()
            .map(stringify_pandoc_inline)
            .collect::<Result<String, _>>(),
        Block::Para(inline) => inline
            .iter()
            .map(stringify_pandoc_inline)
            .collect::<Result<String, _>>(),
        Block::LineBlock(lines) => lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(stringify_pandoc_inline)
                    .collect::<Result<String, _>>()
            })
            .collect::<Result<String, _>>(),
        Block::CodeBlock(attr, s) => {
            lazy_static! {
                static ref PS: SyntaxSet = SyntaxSet::load_defaults_newlines();
                static ref BASE16_THEME: Theme = from_binary(BASE16_THEME_BYTES);
            }
            let syntax = match attr.classes.first() {
                Some(lang) => PS
                    .syntaxes()
                    .iter()
                    .rev()
                    .find(|s| *lang == s.name.to_lowercase())
                    .unwrap_or(PS.find_syntax_plain_text()),
                None => PS.find_syntax_plain_text(),
            };
            // TODO: figure out how to do this without all the theme file rigamorale
            let mut h = HighlightLines::new(syntax, &BASE16_THEME);
            let ranges: Vec<(Style, &str)> = h.highlight(s, &PS);
            Ok(ranges
                .iter()
                .map(|(style, str)| {
                    match style.foreground.r {
                        0x00 => str.black(),
                        0x01 => str.red(),
                        0x02 => str.green(),
                        0x03 => str.yellow(),
                        0x04 => str.blue(),
                        0x05 => str.magenta(),
                        0x06 => str.cyan(),
                        0x07 => str.white(),
                        n => str.with(Color::AnsiValue(n)),
                    }
                    .to_string()
                })
                .collect::<String>())
        }
        Block::RawBlock(_, s) => Ok(s.to_string()),
        Block::BlockQuote(blocks) => blocks
            .iter()
            .map(stringify_pandoc_block)
            .collect::<Result<String, _>>()
            .map(|s| (format!("> {}", s.replace("\n", "\n> "))).dim().to_string()), // TODO: check if dim does anything noticeable
        Block::OrderedList(attr, items) => {
            // TODO: handle different attribute styles and pad things based on the widest marker
            let delim = match attr.delim {
                ListNumberDelim::DefaultDelim | ListNumberDelim::Period => ("", ". "),
                ListNumberDelim::OneParen => ("", ") "),
                ListNumberDelim::TwoParens => ("(", ") "),
            };
            let index_formatter: fn(i32) -> String = match attr.style {
                ListNumberStyle::DefaultStyle
                | ListNumberStyle::Example // example is included here because we don't have enough information to do anything else
                | ListNumberStyle::Decimal => |n| format!("{}", n),
                // TODO: investigate how pandoc handles lengths greater than 26
                ListNumberStyle::LowerAlpha => {
                    |n| String::from_utf8_lossy(&[97 + (n - 1 % 26) as u8]).to_string()
                }
                ListNumberStyle::UpperAlpha => {
                    |n| String::from_utf8_lossy(&[65 + (n - 1 % 26) as u8]).to_string()
                }
                ListNumberStyle::LowerRoman => |n| Roman::from(n as u32).unwrap().to_lowercase(),
                ListNumberStyle::UpperRoman => |n| Roman::from(n as u32).unwrap().to_string(),
            };
            let markers = (attr.start_number..attr.start_number + items.len() as i32)
                .map(|n| delim.0.to_owned() + &index_formatter(n) + delim.1);
            // NOTE: unwrapping here is safe because a list with no items is no l ist at all
            // TODO: use this before markers too
            let max_marker_len = markers.clone().map(|m| m.chars().count()).max().unwrap();
            markers
                .zip(items)
                .map(|(marker, blocks)| {
                    blocks
                        .iter()
                        .map(stringify_pandoc_block)
                        .intersperse_with(|| Ok("\n\n".to_string()))
                        .collect::<Result<String, _>>()
                        .map(|s| {
                            marker.clone()
                                + &" ".repeat(max_marker_len - marker.chars().count())
                                + &s.replace(
                                    "\n",
                                    &("\n".to_string() + &" ".repeat(max_marker_len)),
                                )
                        })
                })
                .intersperse_with(|| Ok("\n".to_string()))
                .collect::<Result<String, _>>()
        }
        Block::BulletList(items) => items
            .iter()
            .map(|blocks| {
                blocks
                    .iter()
                    .map(stringify_pandoc_block)
                    .intersperse_with(|| Ok("\n\n".to_string()))
                    .collect::<Result<String, _>>()
                    .map(|s| "• ".to_string() + &s.replace("\n", "\n  "))
            })
            .intersperse_with(|| Ok("\n".to_string()))
            .collect::<Result<String, _>>(),
        Block::DefinitionList(pairs) => pairs
            .iter()
            .map(|(term, definitions)| {
                let term = term
                    .iter()
                    .map(stringify_pandoc_inline)
                    .collect::<Result<String, _>>()?;
                let definitions = definitions
                    .iter()
                    .map(|blocks| {
                        blocks
                            .iter()
                            .map(stringify_pandoc_block)
                            .intersperse_with(|| Ok("\n\n".to_string()))
                            .collect::<Result<String, _>>()
                    })
                    .intersperse_with(|| Ok("\n".to_string()))
                    .collect::<Result<String, _>>()?;
                Ok(term + "\n    " + &definitions.replace("\n", "\n    "))
            })
            .intersperse_with(|| Ok("\n".to_string()))
            .collect::<Result<String, _>>(),
        Block::Header(lvl, _, inline) => Ok(format!(
            "{} {}",
            "#".repeat(*lvl as usize).bold().to_string(),
            inline
                .iter()
                .map(stringify_pandoc_inline)
                .collect::<Result<String, _>>()?
        )),
        Block::HorizontalRule => Ok("---".to_string()),
        Block::Table(table) => {
            // TODO: handle more properties of table substructs
            // TODO: clean up this monstrosity and show a line
            // between the header and the rest
            let mut rows = table
                .head
                .rows
                .iter()
                .map(|row| {
                    row.cells
                        .iter()
                        .map(|cell| {
                            cell.content
                                .iter()
                                .map(stringify_pandoc_block)
                                .collect::<Result<String, _>>()
                        })
                        .collect::<Result<Vec<String>, _>>()
                })
                .collect::<Result<Vec<Vec<String>>, _>>()?;
            for row in table
                .bodies
                .iter()
                .map(|body| {
                    body.body
                        .iter()
                        .map(|row| {
                            row.cells
                                .iter()
                                .map(|cell| {
                                    cell.content
                                        .iter()
                                        .map(stringify_pandoc_block)
                                        .collect::<Result<String, _>>()
                                })
                                .collect::<Result<Vec<String>, _>>()
                        })
                        .flatten()
                        .collect::<Vec<Vec<String>>>()
                })
                .flatten()
            {
                rows.push(row);
            }
            let mut col_widths = Vec::from_iter(std::iter::repeat(0).take(rows[0].len()));
            for row in &rows {
                for (i, col) in row.iter().enumerate() {
                    if col_widths[i] < col.chars().count() {
                        col_widths[i] = col.chars().count();
                    }
                }
            }
            Ok("│ ".to_string()
                + &rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .enumerate()
                            .map(|(i, cell)| {
                                cell.to_string()
                                    + &(" ".repeat(col_widths[i] - cell.chars().count()))
                            })
                            .intersperse_with(|| " │ ".to_string())
                            .collect::<String>()
                    })
                    .intersperse_with(|| " │\n│ ".to_string())
                    .collect::<String>()
                + " │")
        }
        Block::Div(_, blocks) => blocks
            .iter()
            .map(stringify_pandoc_block)
            .intersperse_with(|| Ok("\n\n".to_string()))
            .collect::<Result<String, _>>(),
        Block::Null => Ok("".to_string()),
    }
}

#[cfg(feature = "pandoc")]
fn stringify_pandoc_inline<'a>(
    block: &'a pandoc_types::definition::Inline,
) -> Result<String, Box<dyn Error>> {
    use crossterm::style::Attribute;
    use pandoc_types::definition::{Inline, QuoteType};

    match block {
        Inline::Str(s) => Ok(s.to_string()),
        Inline::Emph(s) => s
            .iter()
            .map(stringify_pandoc_inline)
            .collect::<Result<String, _>>()
            .map(|s| s.italic().to_string()),
        Inline::Underline(s) => s
            .iter()
            .map(stringify_pandoc_inline)
            .collect::<Result<String, _>>()
            .map(|s| s.underlined().to_string()),
        Inline::Strong(s) => s
            .iter()
            .map(stringify_pandoc_inline)
            .collect::<Result<String, _>>()
            .map(|s| s.bold().to_string()),
        Inline::Strikeout(s) => s
            .iter()
            .map(stringify_pandoc_inline)
            .collect::<Result<String, _>>()
            .map(|s| s.attribute(Attribute::CrossedOut).to_string()),
        Inline::Superscript(inline) => Ok(format!(
            "^{}^",
            inline
                .iter()
                .map(stringify_pandoc_inline)
                .collect::<Result<String, _>>()?
        )),
        Inline::Subscript(inline) => Ok(format!(
            "~{}~",
            inline
                .iter()
                .map(stringify_pandoc_inline)
                .collect::<Result<String, _>>()?
        )),
        Inline::SmallCaps(s) => s
            .iter()
            .map(stringify_pandoc_inline)
            .collect::<Result<String, _>>()
            .map(|s| {
                s.chars()
                    .map(|c| {
                        if c >= 'a' && c <= 'z' {
                            converters::SMALL_CAPS[(c as u32 - 'a' as u32) as usize]
                        } else if c >= 'A' && c <= 'Z' {
                            converters::SMALL_CAPS[(c as u32 - 'A' as u32) as usize]
                        } else {
                            c
                        }
                    })
                    .collect::<String>()
            }),
        Inline::Quoted(quote_type, s) => s
            .iter()
            .map(stringify_pandoc_inline)
            .collect::<Result<String, _>>()
            .map(|s| match quote_type {
                QuoteType::SingleQuote => format!("‘{}’", s),
                QuoteType::DoubleQuote => format!("“{}”", s),
            }),
        // TODO: display citation details
        Inline::Cite(_, s) => s
            .iter()
            .map(stringify_pandoc_inline)
            .collect::<Result<String, _>>(),
        Inline::Code(_, s) => Ok(format!("`{}`", s)), // TODO: actually parse this
        Inline::Space => Ok(" ".to_string()),
        Inline::SoftBreak => Ok("\n".to_string()),
        Inline::LineBreak => Ok("\n\n".to_string()),
        Inline::Math(_, s) => Ok(format!("${}$", s)), // TODO  actually parse and render this, also figure out how to handle when this is a block vs an inline thing
        Inline::RawInline(_, s) => Ok(s.to_string()),
        // TODO: figure out if it's possible to keep track of where this is placed and handle mouse clicks on it
        Inline::Link(_, inline, _) => inline
            .iter()
            .map(stringify_pandoc_inline)
            .collect::<Result<String, _>>()
            .map(|s| s.blue().underlined().to_string()),
        // TODO: look into supporting various methods of displaying images in terminals for this
        Inline::Image(_, inline, target) => Ok(format!(
            "[{}]({})",
            inline
                .iter()
                .map(stringify_pandoc_inline)
                .collect::<Result<String, _>>()?,
            target.url
        )),
        // TODO: figure out a way to render this at the bottom of the screen
        Inline::Note(blocks) => blocks
            .iter()
            .map(stringify_pandoc_block)
            .collect::<Result<String, _>>(),
        Inline::Span(_, inline) => inline
            .iter()
            .map(stringify_pandoc_inline)
            .collect::<Result<String, _>>(),
    }
}

// TODO: add alternate implementation using configurable pandoc feature here with prettier printing
// (https://crates.io/search?q=pandoc)
#[cfg(not(feature = "pandoc"))]
fn print_card(
    stdout: &mut io::Stdout,
    component: usize,
    components: &Vec<String>,
) -> Result<(), io::Error> {
    use textwrap::{wrap, Options};

    execute!(stdout, cursor::MoveTo(0, 1))?;
    execute!(stdout, Clear(ClearType::FromCursorDown))?;
    lazy_static! {
        static ref RE: Regex = Regex::new("<!--([^-]|-[^-]|--[^>])*-->\n{0,2}").unwrap();
    }
    let joined = components[..component + 1].join("\n---\n");
    let raw = RE.replace_all(&joined, "");
    let lines: String = raw
        .split("\n")
        .map(|line| wrap(line, Options::with_termwidth()).join("\r\n"))
        .intersperse("\r\n".to_string())
        .collect();
    write!(stdout, "{}", lines)
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
