use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, ExitCode};

use mde_applet_api::HostMessage;
use mde_applet_status_cluster::{
    find_battery_dir, format_cluster, handle_host, manifest, parse_battery,
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
                eprintln!("mde-applet-status-cluster: serialize manifest: {e}");
                ExitCode::FAILURE
            }
        }
    } else if argv.iter().any(|a| a == "--now") {
        println!("{}", current_cluster());
        ExitCode::SUCCESS
    } else {
        run_loop()
    }
}

fn current_cluster() -> String {
    let battery = find_battery_dir(Path::new("/sys/class/power_supply")).map(|d| parse_battery(&d));
    let profile = current_profile();
    format_cluster(battery.as_ref(), &profile)
}

fn current_profile() -> String {
    let Ok(output) = Command::new("powerprofilesctl").arg("get").output() else {
        return String::new();
    };
    if !output.status.success() {
        return String::new();
    }
    String::from_utf8(output.stdout)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn run_loop() -> ExitCode {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let reader = BufReader::new(stdin.lock());
    let _ = writeln!(stdout, "{}", current_cluster());
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
            let _ = writeln!(stdout, "{}", current_cluster());
            let _ = stdout.flush();
        }
    }
    ExitCode::SUCCESS
}
