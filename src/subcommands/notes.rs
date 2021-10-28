use crate::entities::notes;
use std::fs::metadata;

pub fn notes(matches: Option<&clap::ArgMatches>) {
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
