//! Kothalipi: active application detection for App-Aware Tone & Vibe Coding mode.
//!
//! Detects the frontmost application name without requiring invasive screen
//! capture permissions, and classifies it into a writing context:
//! - `Coding`  — IDEs, code editors, terminals (Cursor, VS Code, Windsurf, Warp, Terminal, etc.)
//! - `Email`   — Email clients & formal document editors (Mail, Outlook, Word, Notion, etc.)
//! - `Chat`    — Messaging apps (WhatsApp, Messenger, Slack, Discord, Telegram, Messages, etc.)
//! - `General` — Everything else.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum AppCategory {
    Coding,
    Email,
    Chat,
    General,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ActiveApp {
    pub name: String,
    pub category: AppCategory,
}

/// Classify an application or window name into a writing category.
pub fn classify_app(name: &str) -> AppCategory {
    let lower = name.trim().to_lowercase();
    if lower.is_empty() {
        return AppCategory::General;
    }

    // IDEs, code editors & terminals -> Vibe Coding / Technical context
    const CODING_PATTERNS: &[&str] = &[
        "cursor",
        "visual studio code",
        "code",
        "windsurf",
        "antigravity",
        "zed",
        "xcode",
        "terminal",
        "iterm",
        "warp",
        "alacritty",
        "kitty",
        "ghostty",
        "wezterm",
        "hyper",
        "intellij",
        "webstorm",
        "pycharm",
        "goland",
        "clion",
        "rustrover",
        "android studio",
        "sublime text",
        "neovim",
        "windowsterminal",
        "powershell",
        "cmd.exe",
    ];
    if CODING_PATTERNS.iter().any(|p| {
        if *p == "code" {
            lower == "code" || lower == "code.exe" || lower.contains("vs code")
        } else {
            lower.contains(p)
        }
    }) {
        return AppCategory::Coding;
    }

    // Messaging & chat apps -> Casual, fast tone
    const CHAT_PATTERNS: &[&str] = &[
        "whatsapp",
        "messenger",
        "telegram",
        "slack",
        "discord",
        "messages",
        "signal",
        "viber",
        "teams",
        "imo",
        "wechat",
        "line",
    ];
    if CHAT_PATTERNS.iter().any(|p| lower.contains(p)) {
        return AppCategory::Chat;
    }

    // Email & formal document apps -> Professional, polite tone
    const EMAIL_PATTERNS: &[&str] = &[
        "mail",
        "outlook",
        "thunderbird",
        "spark",
        "superhuman",
        "gmail",
        "microsoft word",
        "winword",
        "pages",
        "notion",
        "obsidian",
        "google docs",
        "libreoffice writer",
    ];
    if EMAIL_PATTERNS.iter().any(|p| lower.contains(p)) {
        return AppCategory::Email;
    }

    AppCategory::General
}

#[cfg(target_os = "macos")]
fn frontmost_app_name() -> Option<String> {
    // `lsappinfo` is fast (~5ms) and requires no Automation/AppleEvents permission.
    let front = std::process::Command::new("lsappinfo")
        .arg("front")
        .output()
        .ok()?;
    if !front.status.success() {
        return None;
    }
    let asn = String::from_utf8_lossy(&front.stdout).trim().to_string();
    if asn.is_empty() {
        return None;
    }
    let info = std::process::Command::new("lsappinfo")
        .args(["info", "-only", "name", &asn])
        .output()
        .ok()?;
    if !info.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&info.stdout);
    // Format: `"LSDisplayName"="Cursor"`
    let name = line
        .split('=')
        .nth(1)?
        .trim()
        .trim_matches('"')
        .trim()
        .to_string();
    (!name.is_empty()).then_some(name)
}

#[cfg(target_os = "windows")]
fn frontmost_app_name() -> Option<String> {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut buf = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut buf);
        if len <= 0 {
            return None;
        }
        let title = String::from_utf16_lossy(&buf[..len as usize])
            .trim()
            .to_string();
        (!title.is_empty()).then_some(title)
    }
}

#[cfg(target_os = "linux")]
fn frontmost_app_name() -> Option<String> {
    let out = std::process::Command::new("xdotool")
        .args(["getactivewindow", "getwindowclassname"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!name.is_empty()).then_some(name)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn frontmost_app_name() -> Option<String> {
    None
}

/// Detect the currently focused application and its writing category.
pub fn detect_frontmost_app() -> Option<ActiveApp> {
    let name = frontmost_app_name()?;
    let category = classify_app(&name);
    Some(ActiveApp { name, category })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_coding_apps() {
        assert_eq!(classify_app("Cursor"), AppCategory::Coding);
        assert_eq!(classify_app("Code"), AppCategory::Coding);
        assert_eq!(classify_app("Visual Studio Code"), AppCategory::Coding);
        assert_eq!(classify_app("iTerm2"), AppCategory::Coding);
        assert_eq!(classify_app("Warp"), AppCategory::Coding);
        assert_eq!(classify_app("Antigravity"), AppCategory::Coding);
    }

    #[test]
    fn classifies_chat_and_email_apps() {
        assert_eq!(classify_app("WhatsApp"), AppCategory::Chat);
        assert_eq!(classify_app("Slack"), AppCategory::Chat);
        assert_eq!(classify_app("Messenger"), AppCategory::Chat);
        assert_eq!(classify_app("Mail"), AppCategory::Email);
        assert_eq!(classify_app("Microsoft Outlook"), AppCategory::Email);
        assert_eq!(classify_app("Notion"), AppCategory::Email);
    }

    #[test]
    fn unknown_apps_default_to_general() {
        assert_eq!(classify_app("Finder"), AppCategory::General);
        assert_eq!(classify_app(""), AppCategory::General);
    }
}
