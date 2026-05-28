//! Binary entry point for the Win10-style Start popover applet.
//! Walks the freedesktop `.desktop` index, applies the pinned
//! overlay, and writes the popover state to the host over stdio.
//! `--manifest` prints the applet's manifest JSON and exits
//! (called by the panel host during registration discovery).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use mde_applet_api::HostMessage;
use mde_applet_start_menu::{
    all_apps, format_now, handle_host, manifest, parse_desktop_file, parse_pinned, pinned_pane,
    pinned_path, search, AppEntry,
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
                eprintln!("mde-applet-start-menu: serialize manifest: {e}");
                ExitCode::FAILURE
            }
        }
    } else if argv.iter().any(|a| a == "--now") {
        println!("{}", current_snapshot());
        ExitCode::SUCCESS
    } else {
        run_loop()
    }
}

fn current_snapshot() -> String {
    let entries = load_all_entries();
    let pinned_raw = std::fs::read_to_string(pinned_path()).unwrap_or_default();
    let pinned_rows = parse_pinned(&pinned_raw);
    let pinned = pinned_pane(&entries, &pinned_rows);
    let visible = all_apps(entries.clone());
    // Match search behavior for the snapshot — no search
    // query in --now mode (the panel host re-issues with
    // its current text-field state).
    let hits = search(&entries, "").len();
    format_now(&pinned, &visible, hits)
}

fn load_all_entries() -> Vec<AppEntry> {
    let mut out = Vec::new();
    for dir in application_dirs() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
                continue;
            }
            let Some(base) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            let Ok(raw) = std::fs::read_to_string(&path) else {
                continue;
            };
            out.push(parse_desktop_file(&base, &raw));
        }
    }
    out
}

fn application_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        out.push(Path::new(&home).join(".local/share/applications"));
    }
    let xdg =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for component in xdg.split(':') {
        if component.is_empty() {
            continue;
        }
        out.push(Path::new(component).join("applications"));
    }
    out
}

fn run_loop() -> ExitCode {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let reader = BufReader::new(stdin.lock());
    let _ = writeln!(stdout, "{}", current_snapshot());
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
            let _ = writeln!(stdout, "{}", current_snapshot());
            let _ = stdout.flush();
        }
    }
    ExitCode::SUCCESS
}
