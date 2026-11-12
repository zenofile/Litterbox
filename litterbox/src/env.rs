use anyhow::Result;
use clap::ValueEnum;
use shared::env::get_env;
use std::path::PathBuf;

pub use shared::env::xdg_runtime_dir;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum EnvPreset {
    /// Forward terminal settings ($TERM and $COLORTERM)
    Term,
    /// Forward locale settings ($LANG and $LC_*)
    Locale,
    /// Forward host timezone ($TZ)
    #[value(alias = "timezone")]
    Time,
}

impl EnvPreset {
    /// Resolve this preset into a list of "KEY=VALUE" strings from the current host environment.
    pub fn resolve_env_vars(&self) -> Vec<String> {
        match self {
            EnvPreset::Term => {
                let mut vars = Vec::new();
                if let Ok(term) = std::env::var("TERM") {
                    vars.push(format!("TERM={term}"));
                }
                if let Ok(colorterm) = std::env::var("COLORTERM") {
                    vars.push(format!("COLORTERM={colorterm}"));
                }
                vars
            }
            EnvPreset::Locale => {
                let mut vars = Vec::new();
                if let Ok(lang) = std::env::var("LANG") {
                    vars.push(format!("LANG={lang}"));
                }
                for (key, val) in std::env::vars() {
                    if key.starts_with("LC_") {
                        vars.push(format!("{key}={val}"));
                    }
                }
                vars.sort();
                vars
            }
            EnvPreset::Time => {
                let mut vars = Vec::new();
                if let Some(tz) = resolve_host_timezone() {
                    vars.push(format!("TZ={tz}"));
                }
                vars
            }
        }
    }
}

/// Determine the host timezone, preferring $TZ if set and non-empty,
/// otherwise querying system timezone detection via `iana-time-zone`.
pub fn resolve_host_timezone() -> Option<String> {
    if let Ok(tz) = std::env::var("TZ") {
        if !tz.is_empty() {
            return Some(tz);
        }
    }
    match iana_time_zone::get_timezone() {
        Ok(tz) if !tz.is_empty() => Some(tz),
        Ok(_) => None,
        Err(err) => {
            log::debug!("Could not determine host timezone: {err}");
            None
        }
    }
}

/// Resolve an environment variable argument: either an explicit "KEY=VALUE" string,
/// or a "KEY" whose value is looked up in the current host environment.
pub fn resolve_env_arg(entry: &str) -> Option<String> {
    if entry.contains('=') {
        Some(entry.to_string())
    } else {
        std::env::var(entry)
            .ok()
            .map(|val| format!("{entry}={val}"))
    }
}

pub fn home_dir() -> Result<PathBuf> {
    get_env("HOME").map(PathBuf::from)
}

pub fn wayland_display() -> Result<String> {
    get_env("WAYLAND_DISPLAY")
}

pub fn litterbox_binary_path() -> PathBuf {
    std::env::current_exe().expect("Binary path should be defined.")
}

pub fn lbx_init_binary_path() -> PathBuf {
    litterbox_binary_path()
        .parent()
        .expect("Litterbox binary path should have a parent")
        .join("lbx-init")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_env_arg_with_explicit_value() {
        assert_eq!(resolve_env_arg("FOO=BAR"), Some("FOO=BAR".to_string()));
    }

    #[test]
    fn test_resolve_env_arg_from_host() {
        // SAFETY: No other threads are reading or writing to env variables.
        unsafe {
            std::env::set_var("TEST_LITTERBOX_VAR", "123");
        }
        assert_eq!(
            resolve_env_arg("TEST_LITTERBOX_VAR"),
            Some("TEST_LITTERBOX_VAR=123".to_string())
        );
    }

    #[test]
    fn test_resolve_env_arg_unset_returns_none() {
        assert_eq!(resolve_env_arg("DEFINITELY_UNSET_VAR_XYZ_123"), None);
    }

    #[test]
    fn test_term_preset() {
        // SAFETY: No other threads are reading or writing to env variables.
        unsafe {
            std::env::set_var("TERM", "xterm-ghostty");
            std::env::set_var("COLORTERM", "truecolor");
        }
        let vars = EnvPreset::Term.resolve_env_vars();
        assert!(vars.contains(&"TERM=xterm-ghostty".to_string()));
        assert!(vars.contains(&"COLORTERM=truecolor".to_string()));
    }

    #[test]
    fn test_locale_preset() {
        // SAFETY: No other threads are reading or writing to env variables.
        unsafe {
            std::env::set_var("LANG", "en_US.UTF-8");
            std::env::set_var("LC_COLLATE", "C");
        }
        let vars = EnvPreset::Locale.resolve_env_vars();
        assert!(vars.contains(&"LANG=en_US.UTF-8".to_string()));
        assert!(vars.contains(&"LC_COLLATE=C".to_string()));
    }

    #[test]
    fn test_time_preset_from_env() {
        // SAFETY: No other threads are reading or writing to env variables.
        unsafe {
            std::env::set_var("TZ", "Antarctica/Troll");
        }
        let vars = EnvPreset::Time.resolve_env_vars();
        assert_eq!(vars, vec!["TZ=Antarctica/Troll".to_string()]);
    }

    #[test]
    fn test_time_preset_from_system() {
        // SAFETY: No other threads are reading or writing to env variables.
        unsafe {
            std::env::remove_var("TZ");
        }
        if let Some(expected_tz) = resolve_host_timezone() {
            let vars = EnvPreset::Time.resolve_env_vars();
            assert_eq!(vars, vec![format!("TZ={expected_tz}")]);
        }
    }
}
