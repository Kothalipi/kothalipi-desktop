//! Kothalipi: fixtures for the browser UI preview (`bun run dev`, open `/?mock`).
//!
//! Regenerate after changing settings or models:
//!   cargo test --lib dev_fixtures -- --ignored

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::managers::model::{DiskStatus, ModelInfo};

    #[test]
    #[ignore = "writes src/dev/fixtures.json for the UI preview"]
    fn dump_dev_fixtures() {
        let settings = crate::settings::get_default_settings();
        let mut models: HashMap<String, ModelInfo> = HashMap::new();
        crate::kothalipi_models::seed(&mut models);
        for d in crate::catalog::CATALOG.iter() {
            models
                .entry(d.id.clone())
                .or_insert_with(|| d.to_model_info(&DiskStatus::default()));
        }
        let mut list: Vec<ModelInfo> = models.into_values().collect();
        list.sort_by(|a, b| {
            crate::kothalipi_models::rank_of(&a.id)
                .unwrap_or(u32::MAX)
                .cmp(&crate::kothalipi_models::rank_of(&b.id).unwrap_or(u32::MAX))
                .then(crate::catalog::rank_of(&a.id).cmp(&crate::catalog::rank_of(&b.id)))
                .then_with(|| a.name.cmp(&b.name))
        });
        let json = serde_json::json!({ "settings": settings, "models": list });
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/dev/fixtures.json");
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
        std::fs::write(path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
    }
}
