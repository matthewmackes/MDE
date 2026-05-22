//! Network page — first-run NM bring-up.
//!
//! Reads `nmcli -t -f NAME,DEVICE,TYPE,STATE connection show` to
//! enumerate the available NetworkManager connections. The user
//! picks one (or skips); the wizard records the choice in the
//! state and moves on. Activation happens at Apply time.

/// One NM connection record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NmConnection {
    pub name: String,
    pub device: String,
    pub kind: String,
    pub state: String,
}

impl NmConnection {
    /// True when the connection has a current device + active state.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.state == "activated"
    }
}

/// Pure parser — extract NmConnection records from nmcli's
/// `-t -f NAME,DEVICE,TYPE,STATE connection show` output.
/// Columns are colon-separated; we trust nmcli's escaping.
#[must_use]
pub fn parse_nmcli_connections(output: &str) -> Vec<NmConnection> {
    let mut out = Vec::new();
    for line in output.lines() {
        let mut parts = line.split(':');
        let name = parts.next().unwrap_or("").trim();
        let device = parts.next().unwrap_or("").trim();
        let kind = parts.next().unwrap_or("").trim();
        let state = parts.next().unwrap_or("").trim();
        if name.is_empty() {
            continue;
        }
        out.push(NmConnection {
            name: name.into(),
            device: device.into(),
            kind: kind.into(),
            state: state.into(),
        });
    }
    out
}

/// Build the argv for activating the connection.
#[must_use]
pub fn build_activate_argv(connection_name: &str) -> Vec<String> {
    vec![
        "nmcli".into(),
        "connection".into(),
        "up".into(),
        connection_name.into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_extracts_records() {
        let out = "\
Wired connection 1:enp3s0:ethernet:activated
Home Wifi:wlan0:wifi:
Backup Net::vpn:";
        let conns = parse_nmcli_connections(out);
        assert_eq!(conns.len(), 3);
        assert_eq!(conns[0].name, "Wired connection 1");
        assert_eq!(conns[0].device, "enp3s0");
        assert_eq!(conns[0].kind, "ethernet");
        assert_eq!(conns[0].state, "activated");
        assert!(conns[0].is_active());
    }

    #[test]
    fn parse_skips_empty_lines() {
        let out = "\

a:b:c:d

x:y:z:activated
";
        let conns = parse_nmcli_connections(out);
        assert_eq!(conns.len(), 2);
    }

    #[test]
    fn parse_skips_blank_name() {
        let out = ":enp3s0:ethernet:activated\nReal:wlan0:wifi:";
        let conns = parse_nmcli_connections(out);
        assert_eq!(conns.len(), 1);
        assert_eq!(conns[0].name, "Real");
    }

    #[test]
    fn is_active_only_for_activated_state() {
        let c = NmConnection {
            name: "x".into(),
            device: "y".into(),
            kind: "z".into(),
            state: "activated".into(),
        };
        assert!(c.is_active());
        let c2 = NmConnection {
            state: "deactivated".into(),
            ..c.clone()
        };
        assert!(!c2.is_active());
    }

    #[test]
    fn activate_argv_uses_connection_up() {
        let argv = build_activate_argv("Home Wifi");
        assert_eq!(argv, vec!["nmcli", "connection", "up", "Home Wifi"]);
    }
}
