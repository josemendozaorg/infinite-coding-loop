use anyhow::{Context, Result};
use jsonschema::JSONSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IclConfig {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub version: String,
    pub app_id: String,
    pub app_name: String,
    #[serde(default = "default_docs_folder")]
    pub docs_folder: String,
}

pub fn default_docs_folder() -> String {
    "spec".to_string()
}

impl IclConfig {
    pub fn validate(&self) -> Result<()> {
        let schema_json = include_str!("../../../ontology-schema/meta/icl.schema.json");
        let schema_val: Value = serde_json::from_str(schema_json)?;
        let compiled = JSONSchema::compile(&schema_val)
            .map_err(|e| anyhow::anyhow!("Failed to compile schema: {}", e))?;

        let instance = serde_json::to_value(self)?;
        if let Err(errors) = compiled.validate(&instance) {
            let error_msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
            anyhow::bail!("ICL Config validation failed: {}", error_msgs.join(", "));
        }
        Ok(())
    }
}

pub async fn load_icl_config(work_dir: &Path) -> Result<Option<IclConfig>> {
    let icl_json_path = work_dir.join(".infinitecodingloop").join("icl.json");
    if icl_json_path.exists() {
        let content = fs::read_to_string(icl_json_path).await?;
        let config: IclConfig = serde_json::from_str(&content)?;
        config
            .validate()
            .context("Failed to validate loaded icl.json")?;
        Ok(Some(config))
    } else {
        // Fallback for discovery if the migration hasn't happened yet
        let app_json_path = work_dir.join(".infinitecodingloop").join("app.json");
        if app_json_path.exists() {
            let content = fs::read_to_string(app_json_path).await?;
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                let app_id = val
                    .get("app_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let app_name = val
                    .get("app_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                return Ok(Some(IclConfig {
                    schema: None,
                    version: "0.0.0".to_string(),
                    app_id,
                    app_name,
                    docs_folder: "spec".to_string(),
                }));
            }
        }
        Ok(None)
    }
}

pub async fn ensure_infinite_coding_loop(
    work_dir: &Path,
    app_name: &str,
    app_id: &str,
    docs_folder: &str,
) -> Result<()> {
    let icl_dir = work_dir.join(".infinitecodingloop");
    if !icl_dir.exists() {
        fs::create_dir_all(&icl_dir).await?;
    }

    let icl_json_path = icl_dir.join("icl.json");
    if !icl_json_path.exists() {
        // Migration logic
        let mut final_app_id = app_id.to_string();
        let mut final_app_name = app_name.to_string();
        let mut final_docs_folder = docs_folder.to_string();

        let app_json_path = icl_dir.join("app.json");
        let config_json_path = icl_dir.join("config.json");

        if app_json_path.exists() {
            if let Ok(content) = fs::read_to_string(&app_json_path).await {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(id) = val.get("app_id").and_then(|v| v.as_str()) {
                        final_app_id = id.to_string();
                    }
                    if let Some(name) = val.get("app_name").and_then(|v| v.as_str()) {
                        final_app_name = name.to_string();
                    }
                }
            }
        }

        if config_json_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_json_path).await {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(docs) = val.get("docs_folder").and_then(|v| v.as_str()) {
                        final_docs_folder = docs.to_string();
                    }
                }
            }
        }

        let config = IclConfig {
            schema: None,
            version: "1.0.0".to_string(),
            app_id: final_app_id,
            app_name: final_app_name,
            docs_folder: final_docs_folder,
        };

        config
            .validate()
            .context("Failed to validate new icl.json configuration")?;

        let content = serde_json::to_string_pretty(&config)?;
        fs::write(&icl_json_path, content).await?;

        // Cleanup old files
        if app_json_path.exists() {
            let _ = fs::remove_file(app_json_path).await;
        }
        if config_json_path.exists() {
            let _ = fs::remove_file(config_json_path).await;
        }
    }

    // Ensure iterations directory
    fs::create_dir_all(icl_dir.join("iterations")).await?;

    // Ensure docs folder exists
    let docs_dir = work_dir.join(docs_folder);
    if !docs_dir.exists() {
        fs::create_dir_all(&docs_dir).await?;
    }

    Ok(())
}

pub async fn discover_projects(base_dir: &Path) -> Result<Vec<(PathBuf, IclConfig)>> {
    let mut projects = Vec::new();

    // Check the base dir itself
    if let Some(config) = load_icl_config(base_dir).await? {
        projects.push((base_dir.to_path_buf(), config));
    }

    // Check subdirectories
    if let Ok(mut entries) = fs::read_dir(base_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                if let Some(config) = load_icl_config(&path).await? {
                    projects.push((path, config));
                }
            }
        }
    }

    // Sort by app name for consistent display
    projects.sort_by(|a, b| a.1.app_name.cmp(&b.1.app_name));
    Ok(projects)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_validation() {
        let config = IclConfig {
            schema: None,
            version: "1.0.0".to_string(),
            app_id: "test-id".to_string(),
            app_name: "TestApp".to_string(),
            docs_folder: "spec".to_string(),
        };
        assert!(config.validate().is_ok());

        let invalid_config = IclConfig {
            schema: None,
            version: "invalid".to_string(),
            app_id: "test-id".to_string(),
            app_name: "TestApp".to_string(),
            docs_folder: "spec".to_string(),
        };
        assert!(invalid_config.validate().is_err());
    }

    #[tokio::test]
    async fn test_migration() {
        let tmp = tempdir().unwrap();
        let icl_dir = tmp.path().join(".infinitecodingloop");
        fs::create_dir_all(&icl_dir).await.unwrap();

        let app_json = r#"{ "app_id": "old-id", "app_name": "OldApp" }"#;
        fs::write(icl_dir.join("app.json"), app_json).await.unwrap();

        ensure_infinite_coding_loop(tmp.path(), "Fallback", "fallback-id", "spec")
            .await
            .unwrap();

        let config = load_icl_config(tmp.path()).await.unwrap().unwrap();
        assert_eq!(config.app_id, "old-id");
        assert_eq!(config.app_name, "OldApp");
        assert!(!icl_dir.join("app.json").exists());
    }
}
