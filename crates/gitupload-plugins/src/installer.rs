use crate::sandbox::PluginError;
use crate::verify::verify_plugin_bundle;
use gitupload_core::plugin_manifest::PluginManifest;
use std::fs;
use std::path::{Path, PathBuf};

pub struct PluginInstaller {
    plugins_dir: PathBuf,
}

impl PluginInstaller {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self { plugins_dir }
    }

    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }

    pub fn install(
        &self,
        plugin_id: &str,
        bundle_bytes: &[u8],
        sha256: &str,
        signature: &str,
        pubkey: &str,
    ) -> Result<PathBuf, PluginError> {
        // Step 1: Verify cryptographic integrity & authenticity
        verify_plugin_bundle(bundle_bytes, sha256, signature, pubkey)?;

        let target_dir = self.plugins_dir.join(plugin_id);
        let staging_dir = self.plugins_dir.join(format!(".staging_{}", plugin_id));

        if staging_dir.exists() {
            let _ = fs::remove_dir_all(&staging_dir);
        }
        fs::create_dir_all(&staging_dir)?;

        // Step 2: Unpack tar.gz archive into staging directory
        let tar = flate2::read::GzDecoder::new(bundle_bytes);
        let mut archive = tar::Archive::new(tar);
        archive
            .unpack(&staging_dir)
            .map_err(|e| PluginError::Archive(format!("Failed to unpack tar.gz: {e}")))?;

        // Step 3: Verify manifest exists and is valid
        let manifest_path = staging_dir.join("plugin.toml");
        if !manifest_path.exists() {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(PluginError::ManifestError(
                "Extracted package is missing required 'plugin.toml'".into(),
            ));
        }

        let manifest_content = fs::read_to_string(&manifest_path)?;
        let manifest =
            PluginManifest::parse_toml(&manifest_content).map_err(PluginError::ManifestError)?;

        if manifest.plugin.id != plugin_id {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(PluginError::ManifestError(format!(
                "Manifest ID '{}' does not match requested package ID '{}'",
                manifest.plugin.id, plugin_id
            )));
        }

        let entrypoint_path = staging_dir.join(&manifest.plugin.entrypoint);
        if !entrypoint_path.exists() {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(PluginError::ManifestError(format!(
                "Entrypoint WASM file '{}' not found in package",
                manifest.plugin.entrypoint
            )));
        }

        // Step 4: Atomic Swap with Backup Rollback
        if target_dir.exists() {
            let backup_dir = self.plugins_dir.join(format!(".backup_{}", plugin_id));
            if backup_dir.exists() {
                let _ = fs::remove_dir_all(&backup_dir);
            }
            fs::rename(&target_dir, &backup_dir)?;
            if let Err(e) = fs::rename(&staging_dir, &target_dir) {
                // Rollback from backup
                let _ = fs::rename(&backup_dir, &target_dir);
                return Err(PluginError::Io(e));
            }
            let _ = fs::remove_dir_all(&backup_dir);
        } else {
            fs::rename(&staging_dir, &target_dir)?;
        }

        Ok(target_dir)
    }

    pub fn uninstall(&self, plugin_id: &str) -> Result<(), PluginError> {
        let target_dir = self.plugins_dir.join(plugin_id);
        if target_dir.exists() {
            fs::remove_dir_all(&target_dir)?;
        }
        Ok(())
    }
}
