use clap::{App, Arg, SubCommand};

pub fn build_cli() -> App<'static, 'static> {
    App::new("spaced")
        .version("0.1.0")
        .author("Matthew Toohey <contact@mtoohey.com>")
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
            SubCommand::with_name("import")
                .alias("i")
                .about("Import from other formats")
                .arg(
                    Arg::with_name("format")
                        .short("f")
                        .long("format")
                        .help("The format of the file to import")
                        .takes_value(true)
                        .required(true)
                        .possible_values(&["mochi", "anki"]),
                )
                .arg(Arg::with_name("PATH").index(1).required(true))
                .arg(Arg::with_name("OUT_DIR").index(2).required(true)),
        )
        .subcommand(
            SubCommand::with_name("notes")
                .alias("n")
                .about("Recursively list markdown files in the notes directory, omitting them if they contain they contain spaced: true in their frontmatter")
                .arg(
                    Arg::with_name("all")
                        .short("a")
                        .long("all")
                        .help("Show all notes"),
                )
                .arg(
                    Arg::with_name("edit")
                        .short("e")
                        .long("edit")
                        .help("Run one of $VISUAL, $EDITOR, or vim (with precedence in that rder) on all notes"),
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
                .arg(Arg::with_name("PATH").index(1)),
        )
}
