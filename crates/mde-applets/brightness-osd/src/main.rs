use std::io::{BufRead, BufReader, Write};
use std::process::ExitCode;

use mde_applet_api::HostMessage;
use mde_applet_brightness_osd::{format_osd, handle_host, manifest};

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    if argv.iter().any(|a| a == "--manifest") {
        match serde_json::to_string_pretty(&manifest()) {
            Ok(j) => {
                println!("{j}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("mde-applet-brightness-osd: serialize manifest: {e}");
                ExitCode::FAILURE
            }
        }
    } else if let Some(i) = argv.iter().position(|a| a == "--render") {
        let pct = argv
            .get(i + 1)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        println!("{}", format_osd(pct));
        ExitCode::SUCCESS
    } else {
        run_loop()
    }
}

fn run_loop() -> ExitCode {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let reader = BufReader::new(stdin.lock());
    for line in reader.lines() {
        let Ok(line) = line else {
            return ExitCode::from(2);
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(msg) = serde_json::from_str::<HostMessage>(trimmed) {
            if matches!(msg, HostMessage::Shutdown) {
                return ExitCode::SUCCESS;
            }
            if !handle_host(&msg) {
                continue;
            }
            continue;
        }
        let pct = trimmed.parse::<u32>().unwrap_or(0);
        let _ = writeln!(stdout, "{}", format_osd(pct));
        let _ = stdout.flush();
    }
    ExitCode::SUCCESS
}
