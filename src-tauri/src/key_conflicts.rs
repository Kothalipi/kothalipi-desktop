//! Kothalipi: find macOS settings that steal the language keys.
//!
//! Two system settings fight Kothalipi's default keys:
//! - "Press 🌐 key to" (anything but *Do Nothing*) takes the Fn key.
//! - A Dictation shortcut of "Press Right Command Key Twice" (or the Fn key)
//!   fires macOS dictation on top of ours.
//!
//! Only conflicts with keys the user actually bound are reported. Nothing is
//! changed: onboarding explains the fix and opens Keyboard settings.

use serde::Serialize;
use specta::Type;
use tauri::AppHandle;

#[derive(Serialize, Type, Debug, Default, Clone, PartialEq, Eq)]
pub struct KeyConflicts {
    /// A bound key uses Fn, and the 🌐 key isn't set to "Do Nothing".
    pub globe_key_busy: bool,
    /// A bound key uses Right Command, and macOS Dictation starts on it.
    pub dictation_on_right_command: bool,
    /// A bound key uses Fn, and macOS Dictation starts on it.
    pub dictation_on_fn: bool,
}

#[cfg(test)]
impl KeyConflicts {
    pub fn any(&self) -> bool {
        self.globe_key_busy || self.dictation_on_right_command || self.dictation_on_fn
    }
}

/// Modifier flags in a macOS symbolic hotkey (NSEvent modifier masks).
const COMMAND_MASK: u64 = 0x10_0000;
const RIGHT_COMMAND_DEVICE_BIT: u64 = 0x10;
const FN_MASK: u64 = 0x80_0000;
/// The Dictation entry in `com.apple.symbolichotkeys`.
const DICTATION_HOTKEY_ID: &str = "164";

fn binding_tokens(binding: &str) -> impl Iterator<Item = String> + '_ {
    binding.split('+').map(|p| p.trim().to_ascii_lowercase())
}

fn uses_fn(binding: &str) -> bool {
    binding_tokens(binding).any(|t| t == "fn")
}

fn uses_right_command(binding: &str) -> bool {
    binding_tokens(binding).any(|t| {
        matches!(
            t.as_str(),
            "command_right" | "cmd_right" | "cmdright" | "rcmd" | "commandright" | "rcommand"
        )
    })
}

/// `(enabled, modifier flags)` of the Dictation shortcut, from the text of
/// `defaults read com.apple.symbolichotkeys AppleSymbolicHotKeys`.
fn parse_dictation_hotkey(text: &str) -> Option<(bool, u64)> {
    let start = text.find(&format!("{DICTATION_HOTKEY_ID} ="))?;
    let block = &text[start..];
    // The entry ends at the next top-level id; 400 chars covers it.
    let block = &block[..block.len().min(400)];
    let enabled = block.contains("enabled = 1");
    let params = block.split("parameters").nth(1)?;
    let first = params
        .split(|c: char| !c.is_ascii_digit())
        .find(|s| !s.is_empty())?
        .parse()
        .ok()?;
    Some((enabled, first))
}

fn conflicts_from(
    bindings: &[String],
    globe_usage: Option<u32>,
    dictation: Option<(bool, u64)>,
) -> KeyConflicts {
    let want_fn = bindings.iter().any(|b| uses_fn(b));
    let want_right_cmd = bindings.iter().any(|b| uses_right_command(b));
    let (dictation_on, flags) = dictation.unwrap_or((false, 0));
    KeyConflicts {
        // Unset means the macOS default, which is never "Do Nothing" (0).
        globe_key_busy: want_fn && globe_usage != Some(0),
        dictation_on_right_command: want_right_cmd
            && dictation_on
            && flags & COMMAND_MASK != 0
            && flags & RIGHT_COMMAND_DEVICE_BIT != 0,
        dictation_on_fn: want_fn && dictation_on && flags & FN_MASK != 0,
    }
}

#[cfg(target_os = "macos")]
fn defaults_read(domain: &str, key: &str) -> Option<String> {
    let out = std::process::Command::new("defaults")
        .args(["read", domain, key])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Check the bound language and transcribe keys against macOS settings.
#[tauri::command]
#[specta::specta]
pub fn check_key_conflicts(app: AppHandle) -> KeyConflicts {
    #[cfg(target_os = "macos")]
    {
        let bindings: Vec<String> = crate::settings::get_settings(&app)
            .bindings
            .values()
            .filter(|b| b.id != "cancel")
            .map(|b| b.current_binding.clone())
            .collect();
        let globe = defaults_read("com.apple.HIToolbox", "AppleFnUsageType")
            .and_then(|s| s.trim().parse().ok());
        let dictation = defaults_read("com.apple.symbolichotkeys", "AppleSymbolicHotKeys")
            .and_then(|s| parse_dictation_hotkey(&s));
        conflicts_from(&bindings, globe, dictation)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        KeyConflicts::default()
    }
}

/// Open System Settings → Keyboard, where both conflicts are fixed.
#[tauri::command]
#[specta::specta]
pub fn open_keyboard_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.Keyboard-Settings.extension")
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Keyboard settings shortcut is only available on macOS".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOTKEYS_RIGHT_CMD: &str = r#"{
    160 =     {
        enabled = 0;
    };
    164 =     {
        enabled = 1;
        value =         {
            parameters =             (
                1048592,
                18446744073708503023
            );
            type = modifier;
        };
    };
    175 =     {
        enabled = 1;
    };
}"#;

    #[test]
    fn parses_the_dictation_hotkey() {
        assert_eq!(
            parse_dictation_hotkey(HOTKEYS_RIGHT_CMD),
            Some((true, 1_048_592))
        );
        assert_eq!(parse_dictation_hotkey("{ 160 = { enabled = 0; }; }"), None);
    }

    #[test]
    fn flags_right_command_dictation_and_busy_globe_key() {
        let keys = vec!["command_right".to_string(), "fn".to_string()];
        let c = conflicts_from(&keys, None, parse_dictation_hotkey(HOTKEYS_RIGHT_CMD));
        assert!(c.globe_key_busy);
        assert!(c.dictation_on_right_command);
        assert!(!c.dictation_on_fn);
    }

    #[test]
    fn no_conflict_when_fixed_or_not_bound() {
        let keys = vec!["command_right".to_string(), "fn".to_string()];
        let fixed = conflicts_from(&keys, Some(0), Some((false, 1_048_592)));
        assert!(!fixed.any());

        let other_keys = vec!["option+space".to_string()];
        let c = conflicts_from(&other_keys, None, Some((true, 1_048_592)));
        assert!(!c.any());
    }

    #[test]
    fn left_command_dictation_is_not_ours() {
        let keys = vec!["command_right".to_string()];
        // 0x100008: Command with the left-side device bit.
        let c = conflicts_from(&keys, Some(0), Some((true, 0x10_0008)));
        assert!(!c.dictation_on_right_command);
    }
}
