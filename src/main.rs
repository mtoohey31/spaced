// TODO: Display errors to users instead of panicking
// TODO: Try and make clap lock down the types of arguments, such as paths, etc.
// TODO: Determine how error enums should be structured
// TODO: Add comments to a bunch of stuff, and look into how to properly document rust functions
// TODO: Support day turnover after midnight
// TODO: Display forgotten progress as red instead of green
// TODO: Randomize question order
// TODO: Display folder containing question
// TODO: Prevent skip after flipping a card
// TODO: Add automatic tests
// TODO: Package for AUR
// TODO: Display path when reviewing card

mod cli;
mod entities;
mod subcommands;

use subcommands::*;

fn main() {
    let matches = cli::build_cli().get_matches();
    match matches.subcommand_name() {
        Some("cards") => cards(matches.subcommand_matches("cards")),
        Some("import") => import(matches.subcommand_matches("import").unwrap()), // Can be unwrapped safely because clap will ensure the format argument is present
        Some("notes") => notes(matches.subcommand_matches("notes")),
        Some("review") | None => review(matches.subcommand_matches("review")),
        _ => panic!(), // Cannot occur since no other subcommands are specified in ../cli.yaml
    }
}
