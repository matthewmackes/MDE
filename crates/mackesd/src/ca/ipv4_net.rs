//! Minimal IPv4 CIDR network type (NF-2.3).
//!
//! The `ipnetwork` crate isn't on the workspace and bringing it in
//! for a single struct is heavier than the requirement. This module
//! defines the bare-minimum `Ipv4Network` shape `sign_peer_cert`
//! needs: parse a CIDR string, iterate the usable host addresses,
//! and round-trip back through `Display`.

use std::fmt;
use std::net::Ipv4Addr;
use std::str::FromStr;

/// An IPv4 CIDR network — `address` is the network's base IP and
/// `prefix_len` is the CIDR mask length (0..=32).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv4Network {
    address: Ipv4Addr,
    prefix_len: u8,
}

impl Ipv4Network {
    /// Construct a new network from a base IP + prefix.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `prefix_len` exceeds 32.
    pub fn new(address: Ipv4Addr, prefix_len: u8) -> Result<Self, String> {
        if prefix_len > 32 {
            return Err(format!("prefix_len {prefix_len} > 32"));
        }
        Ok(Self {
            // Canonicalize the network base: mask out the host bits.
            address: mask_network(address, prefix_len),
            prefix_len,
        })
    }

    /// The network's base address (host bits zeroed).
    #[must_use]
    pub const fn network(&self) -> Ipv4Addr {
        self.address
    }

    /// The network's prefix length.
    #[must_use]
    pub const fn prefix_len(&self) -> u8 {
        self.prefix_len
    }

    /// Iterate every host address inside the CIDR.
    ///
    /// For Nebula overlay allocation we want every host that's
    /// neither the all-zeros base address nor the all-ones
    /// broadcast — Nebula's documentation reserves `.1` for the
    /// lighthouse anchor by convention, but the protocol itself
    /// is happy with any non-broadcast address.
    pub fn hosts(&self) -> Ipv4HostIter {
        let total = self.total_addresses();
        Ipv4HostIter {
            base: u32::from(self.address),
            // Skip the base address (.0). The broadcast (.255 in a
            // /24 etc) is excluded by the upper bound below.
            next_offset: 1,
            end_offset: if total >= 2 { total - 1 } else { total },
        }
    }

    /// Total addresses in the network including network + broadcast.
    /// `/32` returns `1`; `/0` returns `u32::MAX as u64 + 1`.
    #[must_use]
    pub fn total_addresses(&self) -> u64 {
        if self.prefix_len == 0 {
            u64::from(u32::MAX) + 1
        } else {
            1u64 << (32 - self.prefix_len)
        }
    }

    /// True iff `ip` falls inside the network.
    #[must_use]
    pub fn contains(&self, ip: Ipv4Addr) -> bool {
        mask_network(ip, self.prefix_len) == self.address
    }
}

impl FromStr for Ipv4Network {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (ip_part, prefix_part) = s
            .split_once('/')
            .ok_or_else(|| format!("missing '/' in CIDR {s}"))?;
        let ip: Ipv4Addr = ip_part
            .parse()
            .map_err(|e| format!("bad IPv4 address {ip_part}: {e}"))?;
        let prefix: u8 = prefix_part
            .parse()
            .map_err(|e| format!("bad prefix length {prefix_part}: {e}"))?;
        Self::new(ip, prefix)
    }
}

impl fmt::Display for Ipv4Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.address, self.prefix_len)
    }
}

fn mask_network(ip: Ipv4Addr, prefix_len: u8) -> Ipv4Addr {
    if prefix_len == 0 {
        return Ipv4Addr::new(0, 0, 0, 0);
    }
    let raw = u32::from(ip);
    let mask: u32 = u32::MAX
        .checked_shl(u32::from(32 - prefix_len))
        .unwrap_or(0);
    Ipv4Addr::from(raw & mask)
}

/// Iterator over the usable host addresses in an [`Ipv4Network`].
pub struct Ipv4HostIter {
    base: u32,
    next_offset: u64,
    end_offset: u64,
}

impl Iterator for Ipv4HostIter {
    type Item = Ipv4Addr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_offset >= self.end_offset {
            return None;
        }
        // base + next_offset is guaranteed <= u32::MAX because the
        // network's prefix bounds the offset to (1<<(32-prefix)).
        let raw = self.base.saturating_add(
            u32::try_from(self.next_offset).expect("offset bounded by prefix length"),
        );
        self.next_offset += 1;
        Some(Ipv4Addr::from(raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_canonical_form() {
        let net: Ipv4Network = "10.42.0.0/16".parse().unwrap();
        assert_eq!(net.to_string(), "10.42.0.0/16");
        assert_eq!(net.network(), Ipv4Addr::new(10, 42, 0, 0));
        assert_eq!(net.prefix_len(), 16);
    }

    #[test]
    fn parse_canonicalizes_non_aligned_base() {
        // `10.42.7.5/16` collapses to the /16 base `10.42.0.0`.
        let net: Ipv4Network = "10.42.7.5/16".parse().unwrap();
        assert_eq!(net.network(), Ipv4Addr::new(10, 42, 0, 0));
    }

    #[test]
    fn parse_rejects_bad_input() {
        assert!("10.42.0.0".parse::<Ipv4Network>().is_err());
        assert!("10.42.0.0/33".parse::<Ipv4Network>().is_err());
        assert!("foo/16".parse::<Ipv4Network>().is_err());
    }

    #[test]
    fn contains_recognizes_in_and_out() {
        let net: Ipv4Network = "10.42.0.0/16".parse().unwrap();
        assert!(net.contains(Ipv4Addr::new(10, 42, 1, 1)));
        assert!(!net.contains(Ipv4Addr::new(10, 43, 0, 0)));
    }

    #[test]
    fn hosts_skips_base_and_broadcast_for_slash24() {
        let net: Ipv4Network = "10.42.0.0/24".parse().unwrap();
        let hosts: Vec<_> = net.hosts().take(5).collect();
        assert_eq!(hosts[0], Ipv4Addr::new(10, 42, 0, 1));
        assert_eq!(hosts[4], Ipv4Addr::new(10, 42, 0, 5));
        let all: Vec<_> = net.hosts().collect();
        assert_eq!(all.len(), 254);
        assert_eq!(*all.last().unwrap(), Ipv4Addr::new(10, 42, 0, 254));
    }

    #[test]
    fn hosts_handles_slash16_starts_at_dot_one() {
        let net: Ipv4Network = "10.42.0.0/16".parse().unwrap();
        let first = net.hosts().next().unwrap();
        assert_eq!(first, Ipv4Addr::new(10, 42, 0, 1));
    }
}
