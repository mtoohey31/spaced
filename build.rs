use clap_complete::{generate_to, Shell};
use std::{env, process::exit};

include!("src/cli.rs");

fn main() {
    let outdir = match env::var_os("OUT_DIR") {
        None => exit(1),
        Some(outdir) => outdir,
    };

    #[cfg(feature = "pandoc")]
    {
        use syntect::{dumps::dump_to_file, highlighting::ThemeSet};

        let theme = ThemeSet::get_theme("./assets/base16.tmTheme").unwrap();
        dump_to_file(&theme, "./assets/base16.themedump").unwrap();
    }

    let cmd = build_cli();
    let target_shells = [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell];
    for shell in target_shells {
        generate_to(
            shell,
            &mut cmd.clone(),
            cmd.get_name().to_string(),
            outdir.clone(),
        )
        .expect(&format!("Failed to generate completions for {}", shell));
    }
}
