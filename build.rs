use clap_complete::{generate_to, Shell};
use std::env;

include!("src/cli.rs");

fn main() {
    let outdir = match env::var_os("OUT_DIR") {
        None => return,
        Some(outdir) => outdir,
    };
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
