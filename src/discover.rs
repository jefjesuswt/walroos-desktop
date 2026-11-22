//! Finds the hubs on your tailnet: asks tailscaled which machines are online, then asks each one
//! who you are, the way the app reaches a hub. A machine that answers as a hub is one.

use std::process::Command;

use serde::Deserialize;

use crate::api::User;
use crate::hub::{self, Error};

/// A hub on the tailnet, and who it says you are (or why it won't).
pub struct Found {
    pub name: String,
    pub me: Result<User, String>,
}

/// The hubs among the online machines of your tailnet, by name.
pub fn hubs(proxy: Option<&str>) -> Result<Vec<Found>, String> {
    let names = online_peers(&tailscale_status()?)?;
    let found = std::thread::scope(|s| {
        let asks: Vec<_> = names
            .iter()
            .map(|n| s.spawn(move || (n.clone(), hub::connect(n, proxy))))
            .collect();
        asks.into_iter().filter_map(|a| a.join().ok()).collect::<Vec<_>>()
    });
    Ok(found
        .into_iter()
        .filter_map(|(name, answer)| match answer {
            Ok((_, me)) => Some(Found { name, me: Ok(me) }),
            Err(Error::Stranger(m)) => Some(Found { name, me: Err(m) }),
            Err(_) => None, // not a hub, or not answering
        })
        .collect())
}

fn tailscale_status() -> Result<String, String> {
    let out = Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .map_err(|e| format!("run tailscale: {e}. Type the hub's name instead."))?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr);
        return Err(format!("tailscale status: {}", said.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[derive(Deserialize)]
struct Status {
    #[serde(rename = "Peer", default)]
    peer: Option<std::collections::HashMap<String, Peer>>, // null with no peers
}

#[derive(Deserialize)]
struct Peer {
    #[serde(rename = "DNSName", default)]
    dns_name: String,
    #[serde(rename = "Online", default)]
    online: bool,
}

/// The MagicDNS names (first label) of the machines that are online, sorted.
fn online_peers(json: &str) -> Result<Vec<String>, String> {
    let status: Status = serde_json::from_str(json).map_err(|e| format!("read tailscale status: {e}"))?;
    let mut names: Vec<String> = status
        .peer
        .unwrap_or_default()
        .into_values()
        .filter(|p| p.online)
        .filter_map(|p| p.dns_name.split('.').next().map(str::to_string))
        .filter(|n| !n.is_empty())
        .collect();
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_the_online_peers_by_short_name() {
        let json = r#"{"Self":{"DNSName":"me.tailnet.ts.net."},"Peer":{
            "a":{"DNSName":"prod-1.tailnet.ts.net.","Online":true},
            "b":{"DNSName":"phone.tailnet.ts.net.","Online":false},
            "c":{"DNSName":"hub-1.tailnet.ts.net.","Online":true},
            "d":{"DNSName":"","Online":true}}}"#;
        assert_eq!(online_peers(json).unwrap(), ["hub-1", "prod-1"]);
    }

    #[test]
    fn no_peers_is_none_found() {
        assert!(online_peers(r#"{"Peer":null}"#).unwrap().is_empty());
        assert!(online_peers("{}").unwrap().is_empty());
    }
}
