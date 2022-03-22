use clap::{Arg, Command};

pub fn build_cli() -> Command<'static> {
    Command::new("spaced")
        .version("0.1.0")
        .author("Matthew Toohey <contact@mtoohey.com>")
        .about("Spaced repetition in YAML")
        .subcommand(
            Command::new("cards")
                .alias("c")
                .about("Handle cards")
                .subcommand(
                    Command::new("clear-history")
                        .alias("c")
                        .about("Clear review history")
                        .arg(
                            Arg::new("no-confirm")
                                .short('y')
                                .long("no-confirm")
                                .help("Do not ask for confirmation"),
                        )
                        .arg(Arg::new("PATH").index(1)),
                ),
        )
        .subcommand(
            Command::new("import")
                .alias("i")
                .about("Import from other formats")
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .help("The format of the file to import")
                        .takes_value(true)
                        .required(true)
                        .possible_values(&["mochi", "anki"]),
                )
                .arg(Arg::new("PATH").index(1).required(true))
                .arg(Arg::new("OUT_DIR").index(2).required(true)),
        )
        .subcommand(
            Command::new("notes")
                .alias("n")
                .about("Recursively list markdown files in the notes directory, omitting them if they contain they contain spaced: true in their frontmatter")
                .arg(
                    Arg::new("all")
                        .short('a')
                        .long("all")
                        .help("Show all notes"),
                )
                .arg(
                    Arg::new("edit")
                        .short('e')
                        .long("edit")
                        .help("Run one of $VISUAL, $EDITOR, or vim (with precedence in that rder) on all notes, if any are found"),
                ),
        )
        .subcommand(
            Command::new("review")
                .alias("r")
                .about("Review cards")
                .arg(
                    Arg::new("algorithm")
                        .short('a')
                        .long("algorithm")
                        .takes_value(true)
                        .possible_values(&[
                            "all",
                            "leitner",
                            // "half-life" // duolingo
                            // "super-memo"
                        ]),
                )
                .arg(Arg::new("PATH").index(1)),
        )
}
