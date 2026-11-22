//! Where the app finds the hub between runs: the Settings tab saves it to
//! `$XDG_CONFIG_HOME/walroos-desktop/config.json` (`~/.config/...` without it). WALROOS_HUB and
//! WALROOS_PROXY, when set, win over the file for that run.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::link::Target;

/// The hub's default name on the tailnet (`walroos hub --hostname`).
pub const DEFAULT_HUB: &str = "walroos-hub";

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
struct Saved {
    hub: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    proxy: Option<String>,
}

/// Where the config file lives, or None when there's no home to put it in.
pub fn path() -> Option<PathBuf> {
    let var = |k| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|h| h.join(".config")))?;
    Some(base.join("walroos-desktop").join("config.json"))
}

/// The target for this run: the environment over the saved file over the default.
pub fn load() -> Target {
    let var = |k| std::env::var(k).ok().filter(|v: &String| !v.is_empty());
    let saved = path().and_then(|p| read(&p));
    pick(saved, var("WALROOS_HUB"), var("WALROOS_PROXY"))
}

/// Which variables override the saved file in this run, to say so in Settings.
pub fn overridden() -> Vec<&'static str> {
    ["WALROOS_HUB", "WALROOS_PROXY"]
        .into_iter()
        .filter(|k| std::env::var(k).is_ok_and(|v| !v.is_empty()))
        .collect()
}

pub fn save(target: &Target) -> Result<(), String> {
    let p = path().ok_or("no HOME to save the settings in")?;
    write(&p, target).map_err(|e| format!("save {}: {e}", p.display()))
}

fn pick(saved: Option<Saved>, hub: Option<String>, proxy: Option<String>) -> Target {
    let saved = saved.unwrap_or_default();
    Target {
        hub: hub
            .or(Some(saved.hub).filter(|h| !h.is_empty()))
            .unwrap_or_else(|| DEFAULT_HUB.into()),
        proxy: proxy.or(saved.proxy),
    }
}

fn read(p: &Path) -> Option<Saved> {
    serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()
}

fn write(p: &Path, target: &Target) -> std::io::Result<()> {
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let saved = Saved {
        hub: target.hub.clone(),
        proxy: target.proxy.clone(),
    };
    std::fs::write(p, serde_json::to_string_pretty(&saved)? + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(hub: &str, proxy: Option<&str>) -> Option<Saved> {
        Some(Saved {
            hub: hub.into(),
            proxy: proxy.map(Into::into),
        })
    }

    #[test]
    fn the_environment_wins_over_the_file() {
        let t = pick(saved("from-file", Some("p:1")), Some("from-env".into()), None);
        assert_eq!((t.hub.as_str(), t.proxy.as_deref()), ("from-env", Some("p:1")));
    }

    #[test]
    fn the_file_wins_over_the_default() {
        let t = pick(saved("from-file", None), None, None);
        assert_eq!((t.hub.as_str(), t.proxy), ("from-file", None));
        assert_eq!(pick(None, None, None).hub, DEFAULT_HUB);
        assert_eq!(pick(saved("", None), None, None).hub, DEFAULT_HUB);
    }

    #[test]
    fn saves_and_reads_back() {
        let dir = std::env::temp_dir().join(format!("walroos-desktop-test-{}", std::process::id()));
        let p = dir.join("sub").join("config.json");
        let t = Target {
            hub: "my-hub".into(),
            proxy: Some("localhost:1055".into()),
        };
        write(&p, &t).unwrap();
        assert_eq!(read(&p), saved("my-hub", Some("localhost:1055")));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
