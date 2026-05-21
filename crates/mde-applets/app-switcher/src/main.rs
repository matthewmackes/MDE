use std::io::{BufRead, BufReader, Write};
use std::process::{Command, ExitCode};

use mde_applet_api::HostMessage;
use mde_applet_app_switcher::{format_strip, handle_host, manifest, parse_windows};

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    if argv.iter().any(|a| a == "--manifest") {
        match serde_json::to_string_pretty(&manifest()) {
            Ok(j) => {
                println!("{j}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("mde-applet-app-switcher: serialize manifest: {e}");
                ExitCode::FAILURE
            }
        }
    } else if argv.iter().any(|a| a == "--now") {
        println!("{}", current_strip());
        ExitCode::SUCCESS
    } else {
        run_loop()
    }
}

fn current_strip() -> String {
    let raw = run_swaymsg_tree();
    format_strip(&parse_windows(&raw))
}

fn run_swaymsg_tree() -> String {
    let Ok(output) = Command::new("swaymsg").args(["-t", "get_tree"]).output() else {
        return String::new();
    };
    if !output.status.success() {
        return String::new();
    }
    String::from_utf8(output.stdout).unwrap_or_default()
}

fn run_loop() -> ExitCode {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let reader = BufReader::new(stdin.lock());
    let _ = writeln!(stdout, "{}", current_strip());
    let _ = stdout.flush();
    for line in reader.lines() {
        let Ok(line) = line else {
            return ExitCode::from(2);
        };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<HostMessage>(&line) else {
            return ExitCode::from(2);
        };
        if matches!(msg, HostMessage::Shutdown) {
            return ExitCode::SUCCESS;
        }
        if handle_host(&msg) {
            let _ = writeln!(stdout, "{}", current_strip());
            let _ = stdout.flush();
        }
    }
    ExitCode::SUCCESS
}
