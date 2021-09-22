extern crate clap;

use clap::Shell;
use std::env;

include!("src/cli.rs");

fn main() {
    let outdir = match env::var_os("OUT_DIR") {
        None => return,
        Some(outdir) => outdir,
    };
    let mut app = build_cli();
    let target_shells = [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell];
    for shell in target_shells {
        app.gen_completions("spaced", shell, outdir.clone());
    }
}
