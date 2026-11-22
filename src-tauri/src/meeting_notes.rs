//! Kothalipi: Bot-free Bangla-English Meeting Notes.
//!
//! Records meeting audio locally (no bot joins the call), transcribes the
//! Bangla + English conversation, and uses Gemini to produce structured
//! meeting notes (Summary, Key Discussion Points, and Action Items).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use log::{info, warn};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::audio_toolkit::VadPolicy;
use crate::cloud_stt;
use crate::managers::audio::AudioRecordingManager;
use crate::managers::transcription::TranscriptionManager;
use crate::settings::{get_settings, CloudProvider};

const MEETING_BINDING_ID: &str = "meeting_notes";
const MAX_STORED_NOTES: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MeetingNote {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub duration_secs: u64,
    pub transcript: String,
    pub summary: String,
    pub key_points: Vec<String>,
    pub action_items: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiSummaryPayload {
    title: Option<String>,
    summary: Option<String>,
    #[serde(default)]
    key_points: Vec<String>,
    #[serde(default)]
    action_items: Vec<String>,
}

fn notes_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app data dir: {e}"))?;
    Ok(dir.join("meeting_notes.json"))
}

fn load_notes(app: &AppHandle) -> Vec<MeetingNote> {
    let Ok(path) = notes_path(app) else {
        return Vec::new();
    };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_notes(app: &AppHandle, notes: &[MeetingNote]) -> Result<(), String> {
    let path = notes_path(app)?;
    let json = serde_json::to_string_pretty(notes)
        .map_err(|e| format!("Failed to serialize notes: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("Failed to write notes: {e}"))
}

/// Generate a structured Bangla-English meeting summary using Gemini.
async fn summarize_with_gemini(
    api_key: &str,
    transcript: &str,
    fallback_title: &str,
) -> (String, String, Vec<String>, Vec<String>) {
    let trimmed = transcript.trim();
    if api_key.trim().is_empty() || trimmed.is_empty() {
        return (
            fallback_title.to_string(),
            "Add a free Google Gemini API key in General Settings to auto-generate AI summaries and action items.".to_string(),
            Vec::new(),
            Vec::new(),
        );
    }

    let system = "You are an expert executive assistant summarizing a Bangla-English (Banglish) meeting transcript.\n\
        Return ONLY valid JSON with this exact shape (no markdown fences, no extra text):\n\
        {\n\
          \"title\": \"Short descriptive meeting title (in the meeting's main language)\",\n\
          \"summary\": \"2-4 sentence concise overview of the meeting\",\n\
          \"key_points\": [\"Key point 1\", \"Key point 2\"],\n\
          \"action_items\": [\"Action item 1 (with owner/deadline if mentioned)\"]\n\
        }\n\
        Keep technical terms in English and Bangla context natural and clear.";

    let body = serde_json::json!({
        "systemInstruction": { "parts": [{ "text": system }] },
        "contents": [{ "role": "user", "parts": [{ "text": trimmed }] }],
        "generationConfig": {
            "temperature": 0.2,
            "responseMimeType": "application/json"
        }
    });

    let models = [
        "gemini-2.5-flash",
        "gemini-flash-latest",
        "gemini-flash-lite-latest",
    ];
    let Ok(client) = cloud_stt::client() else {
        return (
            fallback_title.to_string(),
            String::new(),
            Vec::new(),
            Vec::new(),
        );
    };

    for model in models {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
        );
        let resp = client
            .post(&url)
            .header("x-goog-api-key", api_key.trim())
            .timeout(Duration::from_secs(30))
            .json(&body)
            .send()
            .await;

        if let Ok(response) = resp {
            if response.status().is_success() {
                if let Ok(text) = response.text().await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(raw_content) = cloud_stt::gemini_text(&json) {
                            let clean = raw_content
                                .trim()
                                .trim_start_matches("```json")
                                .trim_start_matches("```")
                                .trim_end_matches("```")
                                .trim();
                            if let Ok(parsed) = serde_json::from_str::<GeminiSummaryPayload>(clean)
                            {
                                let title = parsed
                                    .title
                                    .filter(|t| !t.trim().is_empty())
                                    .unwrap_or_else(|| fallback_title.to_string());
                                let summary = parsed.summary.unwrap_or_default();
                                info!("Generated meeting summary via {model}");
                                return (title, summary, parsed.key_points, parsed.action_items);
                            }
                        }
                    }
                }
            }
        }
    }

    warn!("Meeting summary generation fell back to raw transcript");
    (
        fallback_title.to_string(),
        "Transcript saved. Could not reach Gemini to generate summary.".to_string(),
        Vec::new(),
        Vec::new(),
    )
}

#[tauri::command]
#[specta::specta]
pub fn start_meeting_recording(app: AppHandle) -> Result<(), String> {
    let rm = app.state::<Arc<AudioRecordingManager>>();
    rm.try_start_recording(MEETING_BINDING_ID, VadPolicy::Disabled)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn cancel_meeting_recording(app: AppHandle) -> Result<(), String> {
    let rm = app.state::<Arc<AudioRecordingManager>>();
    rm.cancel_recording();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn stop_and_summarize_meeting(
    app: AppHandle,
    title: Option<String>,
    duration_secs: u64,
) -> Result<MeetingNote, String> {
    let rm = app.state::<Arc<AudioRecordingManager>>();
    let tm = app.state::<Arc<TranscriptionManager>>();
    let cancel_gen = rm.cancel_generation();

    let samples = rm
        .stop_recording(MEETING_BINDING_ID, cancel_gen)
        .ok_or_else(|| "No active meeting recording found".to_string())?;

    if samples.is_empty() {
        return Err("No audio was captured during the meeting.".to_string());
    }

    let mut settings = get_settings(&app);
    // Meeting notes always preserve Bangla + English code-switching unless disabled
    settings.selected_language = "bn".to_string();
    settings.translate_to_english = false;

    let outcome = cloud_stt::transcribe_recording(&app, &tm, samples, settings.clone())
        .await
        .map_err(|e| format!("Failed to transcribe meeting audio: {e}"))?;

    let transcript = outcome.text.trim().to_string();
    if transcript.is_empty() {
        return Err("Could not detect any speech in the recording.".to_string());
    }

    let fallback_title = title.filter(|t| !t.trim().is_empty()).unwrap_or_else(|| {
        format!(
            "Meeting · {}",
            chrono::Local::now().format("%b %d, %I:%M %p")
        )
    });

    let gemini_key = settings
        .cloud_api_keys
        .get(CloudProvider::Gemini.key_id())
        .cloned()
        .unwrap_or_default();

    let (final_title, summary, key_points, action_items) =
        summarize_with_gemini(&gemini_key, &transcript, &fallback_title).await;

    let note = MeetingNote {
        id: format!("mtg-{}", chrono::Utc::now().timestamp_millis()),
        title: final_title,
        created_at: chrono::Utc::now().timestamp(),
        duration_secs,
        transcript,
        summary,
        key_points,
        action_items,
    };

    let mut notes = load_notes(&app);
    notes.insert(0, note.clone());
    notes.truncate(MAX_STORED_NOTES);
    save_notes(&app, &notes)?;

    Ok(note)
}

#[tauri::command]
#[specta::specta]
pub fn get_meeting_notes(app: AppHandle) -> Result<Vec<MeetingNote>, String> {
    Ok(load_notes(&app))
}

#[tauri::command]
#[specta::specta]
pub fn delete_meeting_note(app: AppHandle, id: String) -> Result<(), String> {
    let mut notes = load_notes(&app);
    notes.retain(|n| n.id != id);
    save_notes(&app, &notes)
}
