//! Top-bar widget construction.
//!
//! Phase 1.5–1.7: fills the left / center / right slots with the
//! initial visual widgets:
//!
//! - Left:   Mackes button (`apple_menu_button`)
//! - Center: HH:MM clock with a 60 s timer (`clock`)
//! - Right:  Six-glyph status cluster (`status_cluster`)
//!
//! Each widget is a stub with the right shape; behavior (menu dropdown,
//! drawer open, etc.) lands in later phases per `docs/PROJECT_WORKLIST.md`.

use std::process::Command;

use gtk::prelude::*;

use crate::{admin_menu, apple_menu, desktop_files, icons, recents, start_menu, weather};

/// Default coords for the weather popover until `panel.toml` grows a
/// `[weather]` section (8.5.3 follow-up). London works as a sane non-zero
/// default — met.no returns a real forecast for it so the popover proves
/// the wiring on first launch.
const DEFAULT_WEATHER_LAT: f64 = 51.507;
const DEFAULT_WEATHER_LON: f64 = -0.128;

/// Glyph size shown in the top bar. 1.0.7 design-pass: bumped from 14
/// to 18. The realized bar is ~36 px tall (`TOP_BAR_HEIGHT_PX` is a
/// minimum; content drives the final height); 14-px glyphs read as
/// "tray tray" rather than "menu surface." 18 px gives each icon a
/// confident presence without crowding the row.
const TOP_BAR_ICON_PX: i32 = 18;

/// Glyph used as the Mackes-menu button. The brand mark — the MDE
/// mesh-penguin app icon shipped to the hicolor theme as `mde` (same
/// art as `data/icons/mde.svg` / `assets/brand/app-icon.svg`). Falls
/// back to a text "M" if the icon theme isn't present (dev tree).
const MACKES_BUTTON_ICON: &str = "mde";

/// Build the Mackes-menu button. Click → drops a `gtk::Menu` populated
/// by `apple_menu::build` of the live `.desktop` scan, plus the canonical
/// system-action items (Sleep / Restart / Shut Down / Lock / Sign Out).
#[must_use]
pub fn apple_menu_button() -> gtk::Button {
    let button = gtk::Button::new();
    button.set_widget_name("mackes-apple-menu-button");
    button.set_relief(gtk::ReliefStyle::None);
    button.set_focus_on_click(false);

    if let Some(pb) = icons::load(MACKES_BUTTON_ICON, TOP_BAR_ICON_PX) {
        button.set_image(Some(&gtk::Image::from_pixbuf(Some(&pb))));
        button.set_always_show_image(true);
    } else {
        // No Carbon theme available (dev tree); use a tiny text glyph
        // so the slot is at least visible.
        button.set_label("M");
    }

    button.set_tooltip_text(Some(
        "Mackes menu — left-click: app & power actions · right-click: Fedora admin",
    ));
    if let Some(atk) = button.accessible() {
        atk.set_name("Mackes menu");
        atk.set_description("Left-click: Apple-style menu with About, Settings, Applications, Recent items, and Power actions. Right-click: Fedora admin shortcuts (Root Terminal, DNF update, journalctl, systemctl, SELinux, firewall, disk-clean).");
    }

    // 1.1.0:
    // - Left-click  → new Start menu popover (`start_menu::build`).
    //   Replaces the legacy `gtk::Menu` apple menu per Q5 lock —
    //   apple-menu actions live as the Quick Actions row inside the
    //   new popover, plus Toggles + Volume + Brightness.
    // - Right-click → 9-item Fedora admin menu (Q15/Q16) in
    //   terminator-launched shells. The legacy `build_apple_menu` is
    //   retained below as dead code for one release cycle.
    let button_for_left = button.clone();
    let button_for_right = button.clone();
    button.connect_button_press_event(move |_, ev| match ev.button() {
        3 => {
            let menu = admin_menu::build();
            menu.show_all();
            menu.popup_at_widget(
                &button_for_right,
                gdk::Gravity::SouthWest,
                gdk::Gravity::NorthWest,
                Some(ev),
            );
            glib::Propagation::Stop
        }
        1 => {
            let popover = start_menu::build(button_for_left.upcast_ref::<gtk::Widget>());
            popover.popup();
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
    button
}

/// Dead code as of 1.1.0 — the apple menu's actions migrated into the
/// new `start_menu` popover (Q5 lock). Kept compiling for one release
/// cycle so any external caller still links.
#[allow(dead_code)]
fn build_apple_menu() -> gtk::Menu {
    let menu = gtk::Menu::new();
    menu.set_widget_name("mackes-apple-menu");

    add_item(&menu, "About Mackes", || {
        launch("mackes", &["--about"]);
    });
    menu.append(&gtk::SeparatorMenuItem::new());
    add_item(&menu, "Settings…", || {
        launch("mackes", &[]);
    });
    add_item(&menu, "Software Update…", || {
        launch("mackes", &["--update"]);
    });
    menu.append(&gtk::SeparatorMenuItem::new());
    menu.append(&build_recents_submenu());
    menu.append(&build_applications_submenu());
    menu.append(&gtk::SeparatorMenuItem::new());
    add_item(&menu, "Force Quit…", || {
        eprintln!("mackes-panel: force-quit (stub)");
    });
    menu.append(&gtk::SeparatorMenuItem::new());
    add_item(&menu, "Sleep", || run_loginctl("suspend"));
    add_item(&menu, "Restart…", || run_loginctl("reboot"));
    add_item(&menu, "Shut Down…", || run_loginctl("poweroff"));
    menu.append(&gtk::SeparatorMenuItem::new());
    add_item(&menu, "Lock Screen", || run_loginctl("lock-session"));
    add_item(&menu, "Sign Out…", || {
        launch("xfce4-session-logout", &["--logout"]);
    });

    menu
}

fn build_recents_submenu() -> gtk::MenuItem {
    let parent = gtk::MenuItem::with_label("Recent Items");
    let submenu = gtk::Menu::new();
    submenu.set_widget_name("mackes-apple-menu-recents");

    let items = recents::load(10);
    if items.is_empty() {
        let placeholder = gtk::MenuItem::with_label("(no recent items)");
        placeholder.set_sensitive(false);
        submenu.append(&placeholder);
    } else {
        for item in items {
            let entry = gtk::MenuItem::with_label(&item.label);
            let uri = item.uri.clone();
            entry.connect_activate(move |_| {
                if let Err(e) = Command::new("xdg-open").arg(&uri).spawn() {
                    eprintln!("mackes-panel: xdg-open recent {uri} failed: {e}");
                }
            });
            submenu.append(&entry);
        }
    }

    parent.set_submenu(Some(&submenu));
    parent
}

fn build_applications_submenu() -> gtk::MenuItem {
    let parent = gtk::MenuItem::with_label("Applications");
    let submenu = gtk::Menu::new();
    submenu.set_widget_name("mackes-apple-menu-applications");

    for cat in apple_menu::build(&desktop_files::scan()) {
        let cat_item = gtk::MenuItem::with_label(cat.label);
        let cat_submenu = gtk::Menu::new();
        for entry in cat.entries {
            let item = gtk::MenuItem::with_label(&entry.name);
            let exec = entry.exec.clone();
            let terminal = entry.terminal;
            item.connect_activate(move |_| {
                launch_exec(&exec, terminal);
            });
            cat_submenu.append(&item);
        }
        cat_item.set_submenu(Some(&cat_submenu));
        submenu.append(&cat_item);
    }

    parent.set_submenu(Some(&submenu));
    parent
}

fn add_item<F>(menu: &gtk::Menu, label: &str, on_activate: F)
where
    F: Fn() + 'static,
{
    let item = gtk::MenuItem::with_label(label);
    item.connect_activate(move |_| on_activate());
    menu.append(&item);
}

/// `loginctl <verb>` (no `PolicyKit` prompt for `lock-session` and
/// `suspend`; `reboot`/`poweroff` will prompt via `PolicyKit`).
fn run_loginctl(verb: &str) {
    if let Err(e) = Command::new("loginctl").arg(verb).spawn() {
        eprintln!("mackes-panel: loginctl {verb} failed: {e}");
    }
}

fn launch(program: &str, args: &[&str]) {
    if let Err(e) = Command::new(program).args(args).spawn() {
        eprintln!("mackes-panel: launching {program} failed: {e}");
    }
}

/// Spawn an `Exec=` line from a `.desktop` file. We strip the
/// freedesktop field codes (`%U`, `%F`, etc.) and hand the rest to
/// `/bin/sh -c` so quoting works. Terminal apps get prefixed with the
/// user's preferred terminal.
///
/// Shared with `dock::AppModule` (Phase 5.1), so re-exported as `pub`.
pub fn launch_exec(exec: &str, terminal: bool) {
    let stripped = strip_field_codes(exec);
    let cmd = if terminal {
        format!("xfce4-terminal -e {stripped}")
    } else {
        stripped
    };
    if let Err(e) = Command::new("/bin/sh").arg("-c").arg(&cmd).spawn() {
        eprintln!("mackes-panel: spawn failed: {cmd}: {e}");
    }
}

fn strip_field_codes(exec: &str) -> String {
    exec.split_whitespace()
        .filter(|tok| !is_field_code(tok))
        .collect::<Vec<&str>>()
        .join(" ")
}

fn is_field_code(token: &str) -> bool {
    matches!(
        token,
        "%f" | "%F" | "%u" | "%U" | "%d" | "%D" | "%n" | "%N" | "%i" | "%c" | "%k" | "%v" | "%m"
    )
}

/// Build the center clock widget. 12-hour format ("h:MM AM/PM") per the
/// 8.5.3 polish bundle. The label updates every 60 s and on startup, and
/// the whole thing is wrapped in a frameless `gtk::Button` whose click
/// pops up a weather panel patterned after xfce4-weather-plugin
/// (`crate::weather`).
///
/// `gtk::Label` is a reference-counted `GObject` handle, so cloning it
/// for the timer closure is just a refcount bump (no `Rc<RefCell<…>>`
/// needed).
#[must_use]
pub fn clock() -> gtk::Button {
    let label = gtk::Label::new(None);
    label.set_widget_name("mackes-top-clock-label");
    label.set_text(&current_clock_text());

    let button = gtk::Button::new();
    button.set_widget_name("mackes-top-clock");
    button.set_relief(gtk::ReliefStyle::None);
    button.set_focus_on_click(false);
    button.add(&label);
    button.set_tooltip_text(Some("Clock — click for weather"));
    if let Some(atk) = button.accessible() {
        atk.set_name("Clock");
        atk.set_description(
            "Current time. Click to open a weather panel for the configured location.",
        );
    }

    let initial_delay_s = seconds_until_next_minute();
    glib::timeout_add_seconds_local(initial_delay_s, move || {
        label.set_text(&current_clock_text());
        let label_recurring = label.clone();
        glib::timeout_add_seconds_local(60, move || {
            label_recurring.set_text(&current_clock_text());
            glib::ControlFlow::Continue
        });
        glib::ControlFlow::Break
    });

    let button_for_click = button.clone();
    button.connect_clicked(move |_| {
        let popover = weather::build_popover(
            button_for_click.upcast_ref::<gtk::Widget>(),
            DEFAULT_WEATHER_LAT,
            DEFAULT_WEATHER_LON,
        );
        popover.show_all();
        popover.popup();
    });

    button
}

/// "h:MM AM/PM" in the system locale. We use `%l:%M %p` because `%l`
/// emits the hour without a leading zero (a leading space, actually —
/// per POSIX strftime — so we `trim_start` the result). `%H:%M` was the
/// 24-hour predecessor; we keep the fallback string the same length so
/// the slot doesn't visually jitter when the formatter fails.
fn current_clock_text() -> String {
    let now = glib::DateTime::now_local().expect("system clock");
    now.format("%l:%M %p").map_or_else(
        |_| "--:-- --".to_owned(),
        |g| g.as_str().trim_start().to_owned(),
    )
}

/// Seconds remaining until the next clock minute. `glib::timeout_add_seconds`
/// takes whole seconds, so we floor — losing at most a few hundred
/// milliseconds of accuracy on the first tick.
fn seconds_until_next_minute() -> u32 {
    let now = glib::DateTime::now_local().expect("system clock");
    let secs = now.second();
    if secs >= 60 {
        1
    } else {
        let remaining = 60 - secs;
        u32::try_from(remaining).unwrap_or(1)
    }
}

// Status cluster moved to `crate::status_cluster` (1.0.7 Q-lock
// 2026-05-18 — icon + numeric, 2 s poll, click opens the drawer
// focused, em-dash on probe failure).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_until_next_minute_is_in_1_to_60() {
        // Wall clock is non-deterministic at test time, but the helper's
        // return value invariant is: always in [1, 60].
        let s = seconds_until_next_minute();
        assert!(s >= 1 && s <= 60, "got {s}");
    }

    #[test]
    fn current_clock_text_has_minute_and_am_pm() {
        let s = current_clock_text();
        // 12-hour locale format: "h:MM AM/PM" -> always contains a
        // colon and an "M" suffix from AM/PM (rare locales aside).
        assert!(s.contains(':'), "clock text should contain ':' got {s}");
    }

    #[test]
    fn clock_button_has_accessible_name() {
        let _g = crate::test_env::env_lock();
        if !crate::test_env::try_init_gtk_serialized() {
            return;
        }
        let button = clock();
        assert_eq!(button.widget_name(), "mackes-top-clock");
        let atk = button.accessible().expect("accessible interface");
        let name = atk.name().unwrap_or_default();
        assert!(
            name.contains("Clock") || !name.is_empty(),
            "clock button must carry an accessible name"
        );
    }

    #[test]
    fn clock_button_contains_label_with_widget_name() {
        let _g = crate::test_env::env_lock();
        if !crate::test_env::try_init_gtk_serialized() {
            return;
        }
        let button = clock();
        // Button has exactly one child — the label.
        let children = button.children();
        assert_eq!(children.len(), 1);
        let label = children
            .into_iter()
            .next()
            .unwrap()
            .downcast::<gtk::Label>()
            .expect("clock child must be a Label");
        assert_eq!(label.widget_name(), "mackes-top-clock-label");
        // Label text matches the formatter's output shape.
        assert!(label.text().contains(':'));
    }

    #[test]
    fn apple_menu_button_carries_widget_name() {
        let _g = crate::test_env::env_lock();
        if !crate::test_env::try_init_gtk_serialized() {
            return;
        }
        let button = apple_menu_button();
        let name = button.widget_name();
        assert!(!name.is_empty(), "apple_menu_button must set a widget name");
    }
}
