//! Kothalipi: what the cloud engines were used for today, and the last limit
//! a provider said it hit.
//!
//! Neither Google nor OpenAI lets an app read a key's remaining quota, so this
//! is Kothalipi's own count (requests and minutes of audio sent today) plus the
//! exact limit message from the provider's reply when one was reached. Stored in
//! the app data folder; holds no keys and no audio.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter, Manager};

use crate::cloud_stt::{EngineFailure, FailureKind, LimitInfo};
use crate::settings::CloudProvider;

static LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct ProviderUsage {
    pub requests: u32,
    pub ok: u32,
    pub failed: u32,
    pub audio_secs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LimitHit {
    pub scope: String,
    pub limit: Option<u32>,
    pub message: String,
    /// Unix seconds when it happened.
    pub hit_at: f64,
    pub retry_after_secs: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct CloudUsage {
    /// Local calendar day these counts belong to (YYYY-MM-DD).
    pub day: String,
    pub providers: HashMap<String, ProviderUsage>,
    pub limits: HashMap<String, LimitHit>,
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn now_secs() -> f64 {
    chrono::Utc::now().timestamp() as f64
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("cloud_usage.json"))
}

fn load(app: &AppHandle) -> CloudUsage {
    let mut u: CloudUsage = path(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    // New day: counts start over. A daily limit message stays until its day ends.
    if u.day != today() {
        u.providers.clear();
        u.limits.retain(|_, l| l.scope == "credit");
        u.day = today();
    }
    u
}

fn save(app: &AppHandle, usage: &CloudUsage) {
    if let Some(p) = path(app) {
        if let Some(dir) = p.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(s) = serde_json::to_string(usage) {
            let _ = fs::write(p, s);
        }
    }
    let _ = app.emit("cloud-usage-changed", ());
}

pub fn record_success(app: &AppHandle, provider: &CloudProvider, audio_secs: f64) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut u = load(app);
    let p = u
        .providers
        .entry(provider.key_id().to_string())
        .or_default();
    p.requests += 1;
    p.ok += 1;
    p.audio_secs += audio_secs;
    // It answered, so whatever limit was recorded is over.
    u.limits.remove(provider.key_id());
    save(app, &u);
}

pub fn record_failures(app: &AppHandle, failures: &[EngineFailure]) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut u = load(app);
    for f in failures {
        let id = f.provider.key_id().to_string();
        let p = u.providers.entry(id.clone()).or_default();
        p.requests += 1;
        p.failed += 1;
        if f.kind == FailureKind::RateLimited {
            if let Some(l) = &f.limit {
                u.limits.insert(id, hit_from(l));
            }
        }
    }
    save(app, &u);
}

fn hit_from(l: &LimitInfo) -> LimitHit {
    LimitHit {
        scope: l.scope.clone(),
        limit: l.limit,
        message: l.message.clone(),
        hit_at: now_secs(),
        retry_after_secs: l.retry_after_secs,
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_cloud_usage(app: AppHandle) -> CloudUsage {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    load(&app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_round_trips_through_json() {
        let mut u = CloudUsage {
            day: "2026-11-16".into(),
            ..Default::default()
        };
        u.providers.insert(
            "gemini".into(),
            ProviderUsage {
                requests: 3,
                ok: 2,
                failed: 1,
                audio_secs: 12.5,
            },
        );
        let s = serde_json::to_string(&u).unwrap();
        let back: CloudUsage = serde_json::from_str(&s).unwrap();
        assert_eq!(back.providers["gemini"].requests, 3);
    }
}
