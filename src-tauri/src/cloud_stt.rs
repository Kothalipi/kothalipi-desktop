//! Kothalipi: cloud speech-to-text with automatic fallback.
//!
//! In Cloud mode a recording is sent to the user's engines in order (Gemini
//! first, then OpenAI by default). Any failure — rate limit, quota, server
//! error, bad key, no internet — moves on to the next engine. When every cloud
//! engine fails, the caller falls back to the offline model, so dictation
//! keeps working on a plane.
//!
//! Keys belong to the user and are sent only to the provider they're for.
//! Claude has no audio input, so it is offered for text polish (AI
//! post-processing), not here.

use std::time::Duration;

use base64::Engine as _;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::settings::{AppSettings, CloudEngine, CloudProvider};

/// Recordings are 16 kHz mono f32.
const SAMPLE_RATE: u32 = 16_000;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(45);

pub const GEMINI_DEFAULT_MODEL: &str = "gemini-3.5-transcribe";
pub const OPENAI_DEFAULT_MODEL: &str = "gpt-transcribe";

/// Why one engine failed; every kind moves on to the next engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    BadKey,
    RateLimited,
    ServerError,
    Network,
    BadResponse,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct EngineFailure {
    pub provider: CloudProvider,
    pub kind: FailureKind,
    pub detail: String,
}

impl CloudProvider {
    pub fn key_id(&self) -> &'static str {
        match self {
            CloudProvider::Gemini => "gemini",
            CloudProvider::OpenAi => "openai",
        }
    }
}

/// The engines that will be tried, in order: enabled and with a key.
pub fn usable_engines(settings: &AppSettings) -> Vec<CloudEngine> {
    settings
        .cloud_engines
        .iter()
        .filter(|e| e.enabled && !api_key(settings, &e.provider).is_empty())
        .cloned()
        .collect()
}

fn api_key<'a>(settings: &'a AppSettings, provider: &CloudProvider) -> &'a str {
    settings
        .cloud_api_keys
        .get(provider.key_id())
        .map(|k| k.trim())
        .unwrap_or("")
}

/// Try each usable engine in order. `Ok(text)` from the first that works;
/// `Err(failures)` (one per engine tried) when none did.
pub async fn transcribe(
    settings: &AppSettings,
    samples: &[f32],
    language: Option<&str>,
) -> Result<String, Vec<EngineFailure>> {
    let wav = encode_wav(samples);
    let mut failures = Vec::new();
    for engine in usable_engines(settings) {
        let key = api_key(settings, &engine.provider);
        let result = match engine.provider {
            CloudProvider::Gemini => gemini(key, &engine.model, &wav, language).await,
            CloudProvider::OpenAi => openai(key, &engine.model, &wav, language).await,
        };
        match result {
            Ok(text) => {
                info!(
                    "Cloud transcription by {:?} ({})",
                    engine.provider, engine.model
                );
                return Ok(text);
            }
            Err((kind, detail)) => {
                warn!(
                    "Cloud engine {:?} failed ({:?}); trying the next",
                    engine.provider, kind
                );
                failures.push(EngineFailure {
                    provider: engine.provider.clone(),
                    kind,
                    detail,
                });
            }
        }
    }
    Err(failures)
}

type EngineResult = Result<String, (FailureKind, String)>;

fn client() -> Result<reqwest::Client, (FailureKind, String)> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| (FailureKind::Network, e.to_string()))
}

fn classify_status(status: reqwest::StatusCode, body: &str) -> (FailureKind, String) {
    let kind = match status.as_u16() {
        401 | 403 => FailureKind::BadKey,
        429 => FailureKind::RateLimited,
        s if s >= 500 => FailureKind::ServerError,
        _ => FailureKind::BadResponse,
    };
    // Bodies can echo request details; keep the log short.
    let detail: String = body.chars().take(300).collect();
    (kind, format!("HTTP {status}: {detail}"))
}

fn network(e: reqwest::Error) -> (FailureKind, String) {
    (FailureKind::Network, e.to_string())
}

/// BCP-47 tag for Gemini's language hint.
fn gemini_language(language: Option<&str>) -> Option<&'static str> {
    match language? {
        "bn" => Some("bn-BD"),
        "en" => Some("en-US"),
        _ => None,
    }
}

fn gemini_body(model: &str, wav: &[u8], language: Option<&str>) -> serde_json::Value {
    let audio = serde_json::json!({
        "inlineData": {
            "mimeType": "audio/wav",
            "data": base64::engine::general_purpose::STANDARD.encode(wav),
        }
    });
    if model.contains("transcribe") {
        // Dedicated transcription model: the hint goes in its config.
        let codes: Vec<&str> = gemini_language(language).into_iter().collect();
        serde_json::json!({
            "contents": [{ "parts": [audio] }],
            "generationConfig": { "audioTranscriptionConfig": { "languageCodes": codes } },
        })
    } else {
        // A general Gemini model: ask for a verbatim transcript.
        let lang = match language {
            Some("bn") => "Bangla. Write Bangla in Bangla script and keep English words in English",
            Some("en") => "English",
            _ => "the spoken language",
        };
        let prompt = format!(
            "Transcribe this audio verbatim in {lang}. Add normal punctuation. \
             Output only the transcript, nothing else."
        );
        serde_json::json!({
            "contents": [{ "parts": [{ "text": prompt }, audio] }],
            "generationConfig": { "temperature": 0 },
        })
    }
}

fn gemini_text(json: &serde_json::Value) -> Option<String> {
    let parts = json.pointer("/candidates/0/content/parts")?.as_array()?;
    let text: String = parts
        .iter()
        .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
        .collect();
    Some(text)
}

async fn gemini(key: &str, model: &str, wav: &[u8], language: Option<&str>) -> EngineResult {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model.trim()
    );
    let response = client()?
        .post(url)
        .header("x-goog-api-key", key)
        .json(&gemini_body(model, wav, language))
        .send()
        .await
        .map_err(network)?;
    let status = response.status();
    let body = response.text().await.map_err(network)?;
    if !status.is_success() {
        return Err(classify_status(status, &body));
    }
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| (FailureKind::BadResponse, e.to_string()))?;
    gemini_text(&json)
        .map(|t| t.trim().to_string())
        .ok_or((FailureKind::BadResponse, "no transcript in response".into()))
}

#[derive(Deserialize)]
struct OpenAiTranscript {
    text: String,
}

async fn openai(key: &str, model: &str, wav: &[u8], language: Option<&str>) -> EngineResult {
    let file = reqwest::multipart::Part::bytes(wav.to_vec())
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| (FailureKind::BadResponse, e.to_string()))?;
    let mut form = reqwest::multipart::Form::new()
        .part("file", file)
        .text("model", model.trim().to_string());
    if let Some(lang) = language.filter(|l| *l != "auto") {
        // gpt-transcribe takes `languages[]`; older models take `language`.
        let field = if model.starts_with("gpt-transcribe") {
            "languages[]"
        } else {
            "language"
        };
        form = form.text(field, lang.to_string());
    }
    let response = client()?
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(key)
        .multipart(form)
        .send()
        .await
        .map_err(network)?;
    let status = response.status();
    let body = response.text().await.map_err(network)?;
    if !status.is_success() {
        return Err(classify_status(status, &body));
    }
    serde_json::from_str::<OpenAiTranscript>(&body)
        .map(|t| t.text.trim().to_string())
        .map_err(|e| (FailureKind::BadResponse, e.to_string()))
}

/// Kothalipi's transcription entry point for a finished recording: cloud
/// engines first when Cloud mode has any, then the offline model. When the
/// offline model is missing too, raise the "pick a model" notice so the
/// user is pointed at a download.
pub async fn transcribe_recording(
    app: &tauri::AppHandle,
    tm: &crate::managers::transcription::TranscriptionManager,
    samples: Vec<f32>,
) -> anyhow::Result<crate::managers::transcription::Transcription> {
    let settings = crate::language_hotkeys::session_settings(app);
    if cloud_ready(&settings) {
        let language = Some(settings.selected_language.as_str()).filter(|l| *l != "auto");
        match transcribe(&settings, &samples, language).await {
            Ok(text) => {
                // The live decoder (if any) isn't needed any more.
                tm.cancel_stream();
                return Ok(crate::managers::transcription::Transcription {
                    text: text.split_whitespace().collect::<Vec<_>>().join(" "),
                    language: None,
                });
            }
            Err(failures) => {
                warn!(
                    "All {} cloud engine(s) failed; falling back to the offline model",
                    failures.len()
                );
                if !tm.is_model_loaded() && settings.selected_model.is_empty() {
                    tm.raise_no_model_selected();
                    return Err(anyhow::anyhow!(
                        "Cloud engines failed and no offline model is installed"
                    ));
                }
            }
        }
    }
    tm.transcribe_recording(samples)
}

/// Cloud mode with at least one engine that has a key.
pub fn cloud_ready(settings: &AppSettings) -> bool {
    settings.listen_mode == crate::settings::ListenMode::Cloud
        && !usable_engines(settings).is_empty()
}

/// Check a key without sending audio: list the provider's models.
pub async fn check_key(provider: &CloudProvider, key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Paste a key first".into());
    }
    let request = match provider {
        CloudProvider::Gemini => client()
            .map_err(|e| e.1)?
            .get("https://generativelanguage.googleapis.com/v1beta/models?pageSize=1")
            .header("x-goog-api-key", key),
        CloudProvider::OpenAi => client()
            .map_err(|e| e.1)?
            .get("https://api.openai.com/v1/models")
            .bearer_auth(key),
    };
    let response = request
        .send()
        .await
        .map_err(|_| "Couldn't reach the provider. Check your internet.".to_string())?;
    match response.status().as_u16() {
        200..=299 => Ok(()),
        401 | 403 => Err("This key was rejected".into()),
        429 => Ok(()), // A valid key that is over its limit right now.
        s => Err(format!("The provider answered HTTP {s}")),
    }
}

/// 16-bit PCM WAV, the most widely accepted upload format.
pub fn encode_wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with(engines: Vec<CloudEngine>, keys: &[(&str, &str)]) -> AppSettings {
        let mut s = AppSettings {
            cloud_engines: engines,
            ..Default::default()
        };
        for (k, v) in keys {
            s.cloud_api_keys.insert(k.to_string(), v.to_string());
        }
        s
    }

    #[test]
    fn wav_header_is_valid() {
        let wav = encode_wav(&[0.0, 0.5, -0.5]);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(wav.len(), 44 + 6);
        let reader = hound::WavReader::new(std::io::Cursor::new(wav)).unwrap();
        assert_eq!(reader.spec().sample_rate, 16_000);
        assert_eq!(reader.len(), 3);
    }

    #[test]
    fn only_enabled_engines_with_keys_are_tried_in_order() {
        let engines = vec![
            CloudEngine {
                provider: CloudProvider::Gemini,
                model: GEMINI_DEFAULT_MODEL.into(),
                enabled: true,
            },
            CloudEngine {
                provider: CloudProvider::OpenAi,
                model: OPENAI_DEFAULT_MODEL.into(),
                enabled: true,
            },
        ];
        let s = settings_with(engines.clone(), &[("openai", "sk-x"), ("gemini", "  ")]);
        let usable = usable_engines(&s);
        assert_eq!(usable.len(), 1);
        assert_eq!(usable[0].provider, CloudProvider::OpenAi);

        let mut disabled = engines;
        disabled[1].enabled = false;
        let s = settings_with(disabled, &[("openai", "sk-x")]);
        assert!(usable_engines(&s).is_empty());
    }

    #[test]
    fn classifies_failures_so_every_one_falls_through() {
        use reqwest::StatusCode;
        assert_eq!(
            classify_status(StatusCode::TOO_MANY_REQUESTS, "").0,
            FailureKind::RateLimited
        );
        assert_eq!(
            classify_status(StatusCode::UNAUTHORIZED, "").0,
            FailureKind::BadKey
        );
        assert_eq!(
            classify_status(StatusCode::BAD_GATEWAY, "").0,
            FailureKind::ServerError
        );
        assert_eq!(
            classify_status(StatusCode::NOT_FOUND, "").0,
            FailureKind::BadResponse
        );
    }

    #[test]
    fn gemini_request_uses_transcription_config_or_prompt() {
        let b = gemini_body(GEMINI_DEFAULT_MODEL, &[1, 2], Some("bn"));
        assert_eq!(
            b.pointer("/generationConfig/audioTranscriptionConfig/languageCodes/0"),
            Some(&serde_json::json!("bn-BD"))
        );
        assert!(b.pointer("/contents/0/parts/0/inlineData/data").is_some());

        let g = gemini_body("gemini-3-flash", &[1], Some("en"));
        assert!(g.pointer("/contents/0/parts/0/text").is_some());
        assert!(g
            .pointer("/generationConfig/audioTranscriptionConfig")
            .is_none());
    }

    #[test]
    fn reads_gemini_text_parts() {
        let json = serde_json::json!({
            "candidates": [{ "content": { "parts": [{ "text": "আমি " }, { "text": "ভাত খাই।" }] } }]
        });
        assert_eq!(gemini_text(&json).as_deref(), Some("আমি ভাত খাই।"));
    }
}
