//! Kothalipi: one hotkey per language.
//!
//! Hold the Bangla key to dictate Bangla, the English key to dictate English.
//! The key decides the language for that one recording; nothing is written to
//! the saved settings. Each language also picks its model:
//!
//! - Bangla: the first downloaded Kothalipi Bangla model, else the selected model.
//! - English: the selected model if it hears English, else the first
//!   downloaded model that does.

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager};

use crate::managers::model::ModelManager;
use crate::settings::AppSettings;

pub const BANGLA_BINDING: &str = "transcribe_bangla";
pub const ENGLISH_BINDING: &str = "transcribe_english";
pub const TRANSLATE_BINDING: &str = "transcribe_translate";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageKey {
    Bangla,
    English,
    Translate,
}

impl LanguageKey {
    pub fn for_binding(binding_id: &str) -> Option<Self> {
        match binding_id {
            BANGLA_BINDING => Some(Self::Bangla),
            ENGLISH_BINDING => Some(Self::English),
            TRANSLATE_BINDING => Some(Self::Translate),
            _ => None,
        }
    }

    fn language(self) -> &'static str {
        match self {
            Self::Bangla => "bn",
            Self::English => "en",
            // Translate listens to Bangla speech and translates output to English.
            Self::Translate => "bn",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Session {
    key: LanguageKey,
    model: Option<String>,
}

/// The language session of the current recording, if a language key started it.
static SESSION: Mutex<Option<Session>> = Mutex::new(None);

/// Start a recording's session: set by a language key, cleared by any other.
pub fn begin(app: &AppHandle, binding_id: &str) {
    let session = LanguageKey::for_binding(binding_id).map(|key| Session {
        key,
        model: pick_model(app, key),
    });
    if let Some(s) = &session {
        log::info!(
            "Language key {:?}: model {:?}",
            s.key,
            s.model.as_deref().unwrap_or("(selected)")
        );
    }
    // The recording pill shows which language this key is for ("" = none).
    let _ = app.emit(
        "overlay-language",
        session.as_ref().map(|s| s.key.language()).unwrap_or(""),
    );
    *SESSION.lock().unwrap_or_else(|e| e.into_inner()) = session;
}

/// The settings a transcription runs with: the saved settings, with the
/// current language key's language and model laid over them.
pub fn apply(mut settings: AppSettings) -> AppSettings {
    let session = SESSION.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if let Some(session) = session {
        overlay(&mut settings, &session);
    }
    settings
}

fn overlay(settings: &mut AppSettings, session: &Session) {
    settings.selected_language = session.key.language().to_string();
    match session.key {
        LanguageKey::Bangla => {
            settings.translate_to_english = false;
        }
        LanguageKey::English | LanguageKey::Translate => {
            // Dictating with English or Translate key forces output to English
            settings.translate_to_english = true;
        }
    }
    if let Some(model) = &session.model {
        settings.selected_model = model.clone();
    }
}

/// `get_settings` for the transcription pipeline.
pub fn session_settings(app: &AppHandle) -> AppSettings {
    apply(crate::settings::get_settings(app))
}

fn pick_model(app: &AppHandle, key: LanguageKey) -> Option<String> {
    let models = app.try_state::<Arc<ModelManager>>()?;
    let selected = crate::settings::get_settings(app).selected_model;
    let hears = |id: &str, lang: &str| {
        models
            .get_model_info(id)
            .is_some_and(|m| m.is_downloaded && m.supported_languages.iter().any(|l| l == lang))
    };
    match key {
        LanguageKey::Bangla => crate::kothalipi_models::bangla_model_ids()
            .find(|id| hears(id, "bn"))
            .map(str::to_string),
        LanguageKey::English if hears(&selected, "en") => None,
        LanguageKey::English => models
            .get_available_models()
            .into_iter()
            .find(|m| m.is_downloaded && m.supported_languages.iter().any(|l| l == "en"))
            .map(|m| m.id),
        LanguageKey::Translate => {
            // Prefer a downloaded model that natively supports translation (e.g. Whisper),
            // else fall back to a Bangla model (Gemini translates the transcript).
            models
                .get_available_models()
                .into_iter()
                .find(|m| m.is_downloaded && m.supports_translation)
                .map(|m| m.id)
                .or_else(|| {
                    crate::kothalipi_models::bangla_model_ids()
                        .find(|id| hears(id, "bn"))
                        .map(str::to_string)
                })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_bindings_to_languages() {
        assert_eq!(
            LanguageKey::for_binding(BANGLA_BINDING),
            Some(LanguageKey::Bangla)
        );
        assert_eq!(
            LanguageKey::for_binding(ENGLISH_BINDING),
            Some(LanguageKey::English)
        );
        assert_eq!(
            LanguageKey::for_binding(TRANSLATE_BINDING),
            Some(LanguageKey::Translate)
        );
        assert_eq!(LanguageKey::for_binding("transcribe"), None);
    }

    #[test]
    fn overlay_sets_language_and_model_only_for_the_session() {
        let mut settings = AppSettings {
            selected_language: "auto".into(),
            selected_model: "whisper".into(),
            translate_to_english: true,
            ..Default::default()
        };
        overlay(
            &mut settings,
            &Session {
                key: LanguageKey::Bangla,
                model: Some("kothalipi-bn-qwen3-asr-0.6b".into()),
            },
        );
        assert_eq!(settings.selected_language, "bn");
        assert_eq!(settings.selected_model, "kothalipi-bn-qwen3-asr-0.6b");
        assert!(!settings.translate_to_english);

        let mut english = AppSettings {
            selected_model: "parakeet".into(),
            ..Default::default()
        };
        overlay(
            &mut english,
            &Session {
                key: LanguageKey::English,
                model: None,
            },
        );
        assert_eq!(english.selected_language, "en");
        assert_eq!(english.selected_model, "parakeet");
    }
}
