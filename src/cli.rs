use clap::{App, Arg, SubCommand};

pub fn build_cli() -> App<'static, 'static> {
    App::new("spaced")
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
}