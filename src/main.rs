// TODO: display errors to users instead of panicking
// TODO: try and make clap lock down the types of arguments, such as paths, etc.
// TODO: add comments to a bunch of stuff, and look into how to properly document rust functions
// TODO: support day turnover after midnight
// TODO: display forgotten progress as red instead of green
// TODO: randomize question order within `cards` folders, but keep each separate folder in a chunk together, also, review questions that have already been seen that day after ones that haven't
// TODO: display folder containing question
// TODO: prevent skip after flipping a card
// TODO: add automatic tests
// TODO: package for AUR
// TODO: display path when reviewing card
// TODO: support `.spacedignore` files (this could be a separate crate if some else hasn't already done it)
// TODO: support `.spacedhistory` for stats purposes
// TODO: display time taken so far
// TODO: unify error handling
// TODO: add config file support and allow for location arguments in notes and review subcommands
// with intelligent detection of whether there are globs so that `cards` and `notes` aren't
// hardcoded subdirectory names
// TODO: re-add printed stats after review
// TODO: fix assumption of \n line endings
#![feature(iter_intersperse)]

mod cli;
mod entities;
mod error;
mod subcommands;

use subcommands::*;

fn main() {
    let matches = cli::build_cli().get_matches();
    match matches.subcommand_name() {
        Some("cards") => cards(matches.subcommand_matches("cards")),
        #[cfg(feature = "import")]
        Some("import") => import(matches.subcommand_matches("import").unwrap()), // Can be unwrapped safely because clap will ensure the format argument is present
        Some("notes") => notes(matches.subcommand_matches("notes")),
        Some("review") | None => review(matches.subcommand_matches("review")).unwrap(), // TODO: make all subcommands return results and handle them here
        _ => panic!(), // Cannot occur since no other subcommands are specified
    }
}
