//! Persistence for app settings (AI model config + default output dir) in a JSON
//! file under the app data dir.

use std::{io::Write, path::Path};

use misclab_core::node::NodeEnv;

const FILE: &str = "settings.json";

pub fn load(dir: &Path) -> NodeEnv {
    std::fs::read_to_string(dir.join(FILE))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(dir: &Path, settings: &NodeEnv) -> std::io::Result<()> {
    let json = serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?;
    let pending = dir.join(format!(".settings-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)?;
        file.write_all(&json)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&pending, dir.join(FILE))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(pending);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_replace_existing_file_and_round_trip_without_temporary_files() {
        let dir = std::env::temp_dir().join(format!("misclab-settings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let mut settings = NodeEnv::default();
        save(&dir, &settings).unwrap();
        settings.ai.llm.model = "fixture-model".into();
        settings.ai.llm.api_key = "fixture-key".into();
        settings.output_dir = "中文目录🙂".into();
        save(&dir, &settings).unwrap();
        assert_eq!(
            serde_json::to_value(load(&dir)).unwrap(),
            serde_json::to_value(settings).unwrap()
        );
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_file(dir.join(FILE)).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn failed_replace_keeps_existing_target_and_cleans_pending_file() {
        let dir = std::env::temp_dir().join(format!("misclab-settings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        std::fs::create_dir(dir.join(FILE)).unwrap();
        assert!(save(&dir, &NodeEnv::default()).is_err());
        assert!(dir.join(FILE).is_dir());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir(dir.join(FILE)).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
