//! phishkit_ctl — alias binary for the headless control plane.

use std::env;
use std::process;

use phishkit_core::cli;
use serde_json::json;

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let wants_help = args.is_empty()
        || args.iter().any(|a| a == "-h" || a == "--help")
        || args.first().map(|a| a == "help").unwrap_or(false);
    if wants_help {
        eprint!("{}", cli::render_help(cli::want_color()));
        process::exit(0);
    }
    let cmd = args.remove(0);
    match cli::run(&cmd, &args) {
        Ok(v) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
            );
        }
        Err(e) => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&json!({ "error": e.to_string() }))
                    .unwrap_or_else(|_| format!(r#"{{"error":"{e}"}}"#))
            );
            process::exit(1);
        }
    }
}
