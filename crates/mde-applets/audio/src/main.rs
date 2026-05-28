//! Binary entry point for the audio top-bar-right applet. Polls
//! `pactl` on a 2 s tick + writes the formatted chip + the
//! current state to the host over stdio. `--manifest` prints the
//! applet's manifest JSON and exits (called by the panel host
//! during registration discovery).

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, ExitCode};

use mde_applet_api::HostMessage;
use mde_applet_audio::{format_chip, handle_host, manifest, parse_mute, parse_volume, AudioState};

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    if argv.iter().any(|a| a == "--manifest") {
        match serde_json::to_string_pretty(&manifest()) {
            Ok(j) => {
                println!("{j}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("mde-applet-audio: serialize manifest: {e}");
                ExitCode::FAILURE
            }
        }
    } else if argv.iter().any(|a| a == "--now") {
        println!("{}", current_chip());
        ExitCode::SUCCESS
    } else {
        run_loop()
    }
}

fn current_chip() -> String {
    let state = sample_state();
    format_chip(state)
}

fn sample_state() -> AudioState {
    let vol_raw = run_pactl(&["get-sink-volume", "@DEFAULT_SINK@"]);
    let mute_raw = run_pactl(&["get-sink-mute", "@DEFAULT_SINK@"]);
    AudioState {
        volume_pct: parse_volume(&vol_raw),
        muted: parse_mute(&mute_raw),
    }
}

fn run_pactl(args: &[&str]) -> String {
    let Ok(out) = Command::new("pactl").args(args).output() else {
        return String::new();
    };
    if !out.status.success() {
        return String::new();
    }
    String::from_utf8(out.stdout).unwrap_or_default()
}

fn run_loop() -> ExitCode {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let reader = BufReader::new(stdin.lock());
    let _ = writeln!(stdout, "{}", current_chip());
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
            let _ = writeln!(stdout, "{}", current_chip());
            let _ = stdout.flush();
        }
    }
    ExitCode::SUCCESS
}
