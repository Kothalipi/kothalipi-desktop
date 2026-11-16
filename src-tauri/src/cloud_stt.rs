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
    /// Set when the provider said a limit was reached (HTTP 429).
    pub limit: Option<LimitInfo>,
}

/// Which limit a provider said was reached, in plain words.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct LimitInfo {
    /// "minute", "day", "credit" or "other".
    pub scope: String,
    /// The provider's number for that limit, when it gave one.
    pub limit: Option<u32>,
    pub retry_after_secs: Option<u32>,
    /// One sentence for the user.
    pub message: String,
}

/// An engine's failure before it is attached to a provider.
#[derive(Debug, Clone)]
pub struct EngineError {
    kind: FailureKind,
    detail: String,
    limit: Option<LimitInfo>,
}

impl From<(FailureKind, String)> for EngineError {
    fn from((kind, detail): (FailureKind, String)) -> Self {
        Self {
            kind,
            detail,
            limit: None,
        }
    }
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
) -> Result<(String, CloudProvider), Vec<EngineFailure>> {
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
                return Ok((text, engine.provider.clone()));
            }
            Err(e) => {
                warn!(
                    "Cloud engine {:?} failed ({:?}): {}; trying the next",
                    engine.provider, e.kind, e.detail
                );
                failures.push(EngineFailure {
                    provider: engine.provider.clone(),
                    kind: e.kind,
                    detail: e.detail,
                    limit: e.limit,
                });
            }
        }
    }
    Err(failures)
}

type EngineResult = Result<String, EngineError>;

fn client() -> Result<reqwest::Client, (FailureKind, String)> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| (FailureKind::Network, e.to_string()))
}

/// Read the limit out of a 429 reply. Google sends a `QuotaFailure` (which
/// quota, and its value) and a `RetryInfo` (how long to wait); OpenAI sends a
/// code such as `insufficient_quota` and "try again in 20s" in the message.
pub fn parse_limit(provider: &CloudProvider, body: &str) -> LimitInfo {
    let json: serde_json::Value = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
    let name = provider_label(provider);
    let mut scope = "other".to_string();
    let mut limit: Option<u32> = None;
    let mut retry: Option<u32> = None;

    match provider {
        CloudProvider::Gemini => {
            if let Some(details) = json.pointer("/error/details").and_then(|d| d.as_array()) {
                for d in details {
                    if let Some(v) = d.pointer("/violations/0") {
                        let id = v.get("quotaId").and_then(|x| x.as_str()).unwrap_or("");
                        scope = if id.contains("PerDay") {
                            "day"
                        } else if id.contains("PerMinute") {
                            "minute"
                        } else {
                            "other"
                        }
                        .to_string();
                        limit = v.get("quotaValue").and_then(|x| {
                            x.as_str()
                                .and_then(|s| s.parse().ok())
                                .or(x.as_u64().map(|n| n as u32))
                        });
                    }
                    if let Some(r) = d.get("retryDelay").and_then(|x| x.as_str()) {
                        retry = r
                            .trim_end_matches('s')
                            .parse::<f64>()
                            .ok()
                            .map(|f| f.ceil() as u32);
                    }
                }
            }
        }
        CloudProvider::OpenAi => {
            let code = json
                .pointer("/error/code")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let msg = json
                .pointer("/error/message")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            if code == "insufficient_quota" {
                scope = "credit".into();
            } else if code.contains("rate_limit") {
                scope = "minute".into();
            }
            // "Please try again in 1.2s" / "in 20s" / "in 6m0s"
            if let Some(i) = msg.find("try again in ") {
                let rest = &msg[i + 13..];
                let num: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                if let Ok(n) = num.parse::<f64>() {
                    let unit = rest[num.len()..].chars().next().unwrap_or('s');
                    retry = Some((if unit == 'm' { n * 60.0 } else { n }).ceil() as u32);
                }
            }
        }
    }

    let wait = retry
        .map(|s| format!(" Try again in {s} seconds."))
        .unwrap_or_default();
    let message = match (provider, scope.as_str()) {
        (_, "credit") => format!(
            "{name} has no credit left on this key. Add credit in your {name} billing page, or use another engine."
        ),
        (_, "day") => format!(
            "{name}'s daily limit is used up{}. It resets daily (Google resets at midnight Pacific Time).{}",
            limit.map(|n| format!(" ({n} requests a day)")).unwrap_or_default(),
            if retry.is_some() { wait.clone() } else { String::new() }
        ),
        (_, "minute") => format!(
            "{name}'s per-minute limit was hit{}.{wait}",
            limit.map(|n| format!(" ({n} a minute)")).unwrap_or_default()
        ),
        _ => format!("{name}'s usage limit was reached.{wait}"),
    };
    LimitInfo {
        scope,
        limit,
        retry_after_secs: retry,
        message,
    }
}

fn classify_response(
    provider: &CloudProvider,
    status: reqwest::StatusCode,
    body: &str,
) -> EngineError {
    let (kind, detail) = classify_status(status, body);
    let limit = (kind == FailureKind::RateLimited).then(|| parse_limit(provider, body));
    EngineError {
        kind,
        detail,
        limit,
    }
}

fn classify_status(status: reqwest::StatusCode, body: &str) -> (FailureKind, String) {
    // Google answers a bad key with HTTP 400 and API_KEY_INVALID, not 401.
    let bad_key_body = body.contains("API_KEY_INVALID") || body.contains("API key not valid");
    let kind = match status.as_u16() {
        _ if bad_key_body => FailureKind::BadKey,
        _ if body.contains("RESOURCE_EXHAUSTED") || body.contains("insufficient_quota") => {
            FailureKind::RateLimited
        }
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
    // The dedicated transcribe model answers with
    // `parts[].audioTranscription.text`; general Gemini models use `parts[].text`.
    let text: String = parts
        .iter()
        .filter_map(|p| {
            p.pointer("/audioTranscription/text")
                .or_else(|| p.get("text"))
                .and_then(|t| t.as_str())
        })
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
        return Err(classify_response(&CloudProvider::Gemini, status, &body));
    }
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| (FailureKind::BadResponse, e.to_string()))?;
    gemini_text(&json)
        .map(|t| t.trim().to_string())
        .ok_or_else(|| {
            (
                FailureKind::BadResponse,
                "no transcript in response".to_string(),
            )
                .into()
        })
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
        return Err(classify_response(&CloudProvider::OpenAi, status, &body));
    }
    serde_json::from_str::<OpenAiTranscript>(&body)
        .map(|t| t.text.trim().to_string())
        .map_err(|e| (FailureKind::BadResponse, e.to_string()).into())
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
            Ok((text, provider)) => {
                crate::cloud_usage::record_success(app, &provider, samples.len() as f64 / 16_000.0);
                // The live decoder (if any) isn't needed any more.
                tm.cancel_stream();
                return Ok(crate::managers::transcription::Transcription {
                    text: text.split_whitespace().collect::<Vec<_>>().join(" "),
                    language: None,
                });
            }
            Err(failures) => {
                crate::cloud_usage::record_failures(app, &failures);
                let summary = describe_failures(&failures);
                warn!("All cloud engine(s) failed ({summary}); trying the offline model");
                // The offline model is the last resort. When there is none, say
                // why the cloud failed instead of a bare "no model loaded".
                use tauri::Manager as _;
                let offline_ready = tm.is_model_loaded()
                    || (!settings.selected_model.is_empty()
                        && app
                            .state::<std::sync::Arc<crate::managers::model::ModelManager>>()
                            .get_model_path(&settings.selected_model)
                            .is_ok());
                if !offline_ready {
                    return Err(anyhow::anyhow!(
                        "{summary}. No offline model is installed as a backup, so nothing could be typed."
                    ));
                }
                // Cloud mode no longer preloads the offline model, so start it now.
                tm.initiate_model_load(true);
            }
        }
    }
    tm.transcribe_recording(samples)
}

fn provider_label(p: &CloudProvider) -> &'static str {
    match p {
        CloudProvider::Gemini => "Gemini",
        CloudProvider::OpenAi => "OpenAI",
    }
}

/// One plain sentence per failed engine, for the notice the user sees.
pub fn describe_failures(failures: &[EngineFailure]) -> String {
    failures
        .iter()
        .map(|f| {
            let why = match f.kind {
                FailureKind::BadKey => "the API key was rejected".to_string(),
                FailureKind::RateLimited => match &f.limit {
                    // The limit message already names the provider.
                    Some(l) => return l.message.clone(),
                    None => "the rate limit or quota is used up".to_string(),
                },
                FailureKind::ServerError => "the service had an error".to_string(),
                FailureKind::Network => "it could not connect (check the internet)".to_string(),
                FailureKind::BadResponse => {
                    let d: String = f.detail.chars().take(140).collect();
                    format!("it answered with an error ({d})")
                }
            };
            format!("{}: {}", provider_label(&f.provider), why)
        })
        .collect::<Vec<_>>()
        .join("; ")
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
    #[test]
    fn gemini_transcribe_model_reply_is_read() {
        // Shape taken from a real gemini-3.5-transcribe response.
        let json = serde_json::json!({
            "candidates": [{ "content": { "parts": [{ "audioTranscription": { "text": "Hello hello." } }], "role": "model" }, "finishReason": "STOP" }]
        });
        assert_eq!(super::gemini_text(&json).as_deref(), Some("Hello hello."));
    }

    #[test]
    fn gemini_daily_limit_is_decoded() {
        let body = r#"{"error":{"code":429,"status":"RESOURCE_EXHAUSTED","details":[
            {"@type":"type.googleapis.com/google.rpc.QuotaFailure","violations":[{"quotaId":"GenerateRequestsPerDayPerProjectPerModel-FreeTier","quotaValue":"20"}]},
            {"@type":"type.googleapis.com/google.rpc.RetryInfo","retryDelay":"46s"}]}}"#;
        let l = super::parse_limit(&crate::settings::CloudProvider::Gemini, body);
        assert_eq!(l.scope, "day");
        assert_eq!(l.limit, Some(20));
        assert_eq!(l.retry_after_secs, Some(46));
        assert!(
            l.message.contains("daily limit") && l.message.contains("20"),
            "{}",
            l.message
        );
    }

    #[test]
    fn openai_no_credit_is_decoded() {
        let body = r#"{"error":{"code":"insufficient_quota","message":"You exceeded your current quota"}}"#;
        let l = super::parse_limit(&crate::settings::CloudProvider::OpenAi, body);
        assert_eq!(l.scope, "credit");
        assert!(l.message.contains("no credit"));
    }

    #[test]
    fn openai_per_minute_limit_reads_the_wait() {
        let body = r#"{"error":{"code":"rate_limit_exceeded","message":"Rate limit reached. Please try again in 1.2s."}}"#;
        let l = super::parse_limit(&crate::settings::CloudProvider::OpenAi, body);
        assert_eq!(l.scope, "minute");
        assert_eq!(l.retry_after_secs, Some(2));
    }

    #[test]
    fn google_bad_key_in_a_400_is_a_bad_key() {
        let body = r#"{"error":{"code":400,"status":"INVALID_ARGUMENT","details":[{"reason":"API_KEY_INVALID"}]}}"#;
        let (kind, _) = super::classify_status(reqwest::StatusCode::BAD_REQUEST, body);
        assert_eq!(kind, super::FailureKind::BadKey);
    }

    #[test]
    fn failures_are_described_in_plain_words() {
        let f = vec![super::EngineFailure {
            provider: crate::settings::CloudProvider::Gemini,
            kind: super::FailureKind::RateLimited,
            detail: String::new(),
            limit: None,
        }];
        let s = super::describe_failures(&f);
        assert!(s.starts_with("Gemini:") && s.contains("quota"), "{s}");
    }

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
