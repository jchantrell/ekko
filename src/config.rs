use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub graphiti: GraphitiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphitiConfig {
    pub llm: LlmConfig,
    pub embedder: EmbedderConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub api_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedderConfig {
    pub provider: String,
    pub model: String,
    pub dimensions: u32,
    pub api_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub provider: String,
    pub uri: String,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub database: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            graphiti: GraphitiConfig {
                llm: LlmConfig {
                    provider: "openai".into(),
                    model: "llama3.2:3b".into(),
                    api_url: "http://localhost:11434/v1".into(),
                    api_key: "ollama".into(),
                },
                embedder: EmbedderConfig {
                    provider: "openai".into(),
                    model: "nomic-embed-text".into(),
                    dimensions: 768,
                    api_url: "http://localhost:11434/v1".into(),
                    api_key: "ollama".into(),
                },
                database: DatabaseConfig {
                    provider: "falkordb".into(),
                    uri: "bolt://localhost:6379".into(),
                    user: None,
                    password: None,
                    database: Some("default_db".into()),
                },
            },
        }
    }
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("could not determine config directory")?
            .join("ekko");
        Ok(dir)
    }

    pub fn data_dir() -> Result<PathBuf> {
        let dir = dirs::data_dir()
            .context("could not determine data directory")?
            .join("ekko");
        Ok(dir)
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn venv_dir() -> Result<PathBuf> {
        Ok(Self::data_dir()?.join("venv"))
    }

    pub fn shim_dir() -> Result<PathBuf> {
        Ok(Self::data_dir()?.join("shim"))
    }

    pub fn shim_path() -> Result<PathBuf> {
        Ok(Self::shim_dir()?.join("__main__.py"))
    }

    pub fn python_path() -> Result<PathBuf> {
        Ok(Self::venv_dir()?.join("bin").join("python"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config: Self = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let content = toml::to_string_pretty(self).context("failed to serialize config")?;
        std::fs::write(&path, content)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn exists() -> bool {
        Self::config_path().map(|p| p.exists()).unwrap_or(false)
    }
}
