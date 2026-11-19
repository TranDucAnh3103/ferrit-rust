use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginCapabilities {
    #[serde(default)]
    pub git_read: bool,
    #[serde(default)]
    pub git_write: bool,
    #[serde(default)]
    pub commit_hook: bool,
    #[serde(default)]
    pub ai_prompt_filter: bool,
    #[serde(default)]
    pub network_access: bool,
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: u32,
}

fn default_max_memory_mb() -> u32 {
    16
}

impl Default for PluginCapabilities {
    fn default() -> Self {
        Self {
            git_read: false,
            git_write: false,
            commit_hook: false,
            ai_prompt_filter: false,
            network_access: false,
            max_memory_mb: 16,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub entrypoint: String,
    #[serde(default)]
    pub homepage: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginManifest {
    pub plugin: PluginMetadata,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserPluginRecord {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub entrypoint: String,
    pub capabilities: PluginCapabilities,
    pub is_enabled: bool,
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

impl PluginManifest {
    pub fn parse_toml(content: &str) -> Result<Self, String> {
        let manifest: PluginManifest =
            toml::from_str(content).map_err(|e| format!("Invalid plugin.toml: {e}"))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), String> {
        // Reverse-DNS format: ^[a-zA-Z0-9_-]+(\.[a-zA-Z0-9_-]+)+$
        let id_regex = regex::Regex::new(r"^[a-zA-Z0-9_-]+(\.[a-zA-Z0-9_-]+)+$")
            .map_err(|e| format!("Regex error: {e}"))?;
        if !id_regex.is_match(&self.plugin.id) {
            return Err(format!(
                "Invalid plugin id '{}'. Must follow reverse-DNS format (e.g. org.gitupload.my-plugin)",
                self.plugin.id
            ));
        }

        if self.plugin.name.trim().is_empty() {
            return Err("Plugin name cannot be empty".to_string());
        }

        if self.plugin.entrypoint.trim().is_empty() {
            return Err("Plugin entrypoint cannot be empty".to_string());
        }

        if self.capabilities.max_memory_mb == 0 || self.capabilities.max_memory_mb > 32 {
            return Err(format!(
                "Invalid max_memory_mb: {}. Must be between 1 and 32 MB",
                self.capabilities.max_memory_mb
            ));
        }

        Ok(())
    }
}
