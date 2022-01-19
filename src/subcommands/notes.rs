use crate::entities::notes;
use std::process::Command;
use std::{env, fs::metadata};

pub fn notes(matches: Option<&clap::ArgMatches>) {
    let (path, all, edit) = match matches {
        Some(m) => (
            m.value_of("PATH").unwrap_or("."),
            m.is_present("all"),
            m.is_present("edit"),
        ),
        _ => (".", false, false),
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

    if edit {
        Command::new(
            env::var("VISUAL").unwrap_or(env::var("EDITOR").unwrap_or(String::from("vim"))),
        )
        .args(entries.iter().map(|e| e.path().as_os_str()))
        .status()
        .expect("failed to execute $VISUAL, $EDITOR, or vim command");
    } else {
        for entry in entries {
            println!("{}", entry.path().display());
        }
    }
}
