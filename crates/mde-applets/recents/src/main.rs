use std::io::{BufRead, BufReader, Write};
use std::process::ExitCode;

use mde_applet_api::HostMessage;
use mde_applet_recents::{
    format_widget, handle_host, manifest, parse_xbel, recents_xbel_path, top_n, RECENTS_CAP,
};

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    if argv.iter().any(|a| a == "--manifest") {
        match serde_json::to_string_pretty(&manifest()) {
            Ok(j) => {
                println!("{j}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("mde-applet-recents: serialize manifest: {e}");
                ExitCode::FAILURE
            }
        }
    } else if argv.iter().any(|a| a == "--now") {
        println!("{}", current_widget());
        ExitCode::SUCCESS
    } else {
        run_loop()
    }
}

fn current_widget() -> String {
    let path = recents_xbel_path();
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let rows = top_n(parse_xbel(&raw), RECENTS_CAP);
    format_widget(&rows)
}

fn run_loop() -> ExitCode {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let reader = BufReader::new(stdin.lock());
    let _ = writeln!(stdout, "{}", current_widget());
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
            let _ = writeln!(stdout, "{}", current_widget());
            let _ = stdout.flush();
        }
    }
    ExitCode::SUCCESS
}
