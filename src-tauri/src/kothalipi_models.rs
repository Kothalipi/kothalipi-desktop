//! Kothalipi: Bangla speech models offered on top of Kothalipi's catalog.
//!
//! Kept out of `catalog/catalog.json` on purpose: that file must stay identical
//! to the upstream `@handy-computer/models` package, so upstream merges stay
//! clean. Every file is pinned to a Hugging Face commit and verified by
//! SHA-256 after download.
//!
//! To add a model: append a [`BanglaModel`] below with the repo, pinned
//! revision, file name, size and sha256 from the Hugging Face API
//! (`https://huggingface.co/api/models/<repo>?blobs=true`), and record its
//! accuracy in the main repo's `docs/MODELS.md`.

use std::collections::HashMap;

use crate::managers::model::{ModelInfo, ModelSource};

struct BanglaModel {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    repo: &'static str,
    revision: &'static str,
    filename: &'static str,
    size_bytes: u64,
    sha256: &'static str,
    accuracy: f32,
    speed: f32,
    recommended: bool,
    /// List position among Kothalipi models (lower = higher).
    rank: u32,
}

const BANGLA_MODELS: &[BanglaModel] = &[
    BanglaModel {
        id: "kothalipi-bn-qwen3-asr-0.6b",
        name: "Bangla · Qwen3-ASR 0.6B",
        description: "Best for Bangla dictation. Fine-tuned on 1,174 hours of Bangla speech; fast and offline.",
        repo: "sheikhmufrad/Qwen3-ASR-0.6B-Bengali-GGUF",
        revision: "889bc90cbec9573ccffaecdf25e61db8c19224f7",
        filename: "Qwen3-ASR-0.6B-Bengali-Q8_0.gguf",
        size_bytes: 850_423_584,
        sha256: "908175e58af8184eae84c7da45b878d28fff2a370491dfd53c02b81c79cb42b6",
        accuracy: 0.85,
        speed: 0.70,
        recommended: true,
        rank: 1,
    },
    BanglaModel {
        id: "kothalipi-bn-bengaliai-whisper-medium",
        name: "Bangla · Bengali.AI Whisper Medium",
        description: "Bengali.AI's Bangla Whisper, strong on read speech. Slower than Qwen3-ASR.",
        repo: "winshades/bengaliai-whisper-medium-ggml",
        revision: "11cf5cbd282e0e46a761093939952f74906ddce1",
        filename: "ggml-bengaliai-whisper-medium-q5_0.bin",
        size_bytes: 539_257_671,
        sha256: "2c64c7f1e2c8f2417ed1fe5b78c7238933c00a2cdc2ef33e7087a95475256605",
        accuracy: 0.80,
        speed: 0.45,
        recommended: false,
        rank: 2,
    },
];

/// Bangla model ids, best first.
pub fn bangla_model_ids() -> impl Iterator<Item = &'static str> {
    let mut models: Vec<&BanglaModel> = BANGLA_MODELS.iter().collect();
    models.sort_by_key(|m| m.rank);
    models.into_iter().map(|m| m.id)
}

/// Sort position for Kothalipi's Bangla models, ahead of every catalog model.
pub fn rank_of(model_id: &str) -> Option<u32> {
    BANGLA_MODELS
        .iter()
        .find(|m| m.id == model_id)
        .map(|m| m.rank)
}

/// Insert the Bangla models into the model registry.
pub fn seed(available_models: &mut HashMap<String, ModelInfo>) {
    for m in BANGLA_MODELS {
        available_models.insert(
            m.id.to_string(),
            ModelInfo {
                id: m.id.to_string(),
                name: m.name.to_string(),
                description: m.description.to_string(),
                filename: m.filename.to_string(),
                source: ModelSource::Url {
                    url: format!(
                        "https://huggingface.co/{}/resolve/{}/{}",
                        m.repo, m.revision, m.filename
                    ),
                    sha256: Some(m.sha256.to_string()),
                },
                size_mb: m.size_bytes / (1024 * 1024),
                is_downloaded: false,
                is_downloading: false,
                partial_size: 0,
                accuracy_score: m.accuracy,
                speed_score: m.speed,
                supports_translation: false,
                is_recommended: m.recommended,
                supported_languages: vec!["bn".to_string()],
                supports_language_selection: false,
                is_custom: false,
                supports_streaming: false,
                supports_language_detection: false,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_are_pinned_and_hashed() {
        for m in BANGLA_MODELS {
            assert_eq!(m.revision.len(), 40, "{}: pin a full commit sha", m.id);
            assert_eq!(m.sha256.len(), 64, "{}: sha256 required", m.id);
            assert!(m.size_bytes > 0, "{}: size required", m.id);
        }
    }

    #[test]
    fn seeds_bangla_only_models_first() {
        let mut map = HashMap::new();
        seed(&mut map);
        assert_eq!(map.len(), BANGLA_MODELS.len());
        assert!(map
            .values()
            .all(|m| m.supported_languages == ["bn"] && m.source_is_url()));
        assert_eq!(rank_of("kothalipi-bn-qwen3-asr-0.6b"), Some(1));
        assert_eq!(rank_of("whisper-small"), None);
    }

    trait SourceIsUrl {
        fn source_is_url(&self) -> bool;
    }
    impl SourceIsUrl for ModelInfo {
        fn source_is_url(&self) -> bool {
            matches!(self.source, ModelSource::Url { .. })
        }
    }
}
