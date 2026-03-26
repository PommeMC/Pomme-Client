use crate::installations::{Installation, InstallationError};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static DATA_DIR: LazyLock<PathBuf> = {
    LazyLock::new(|| {
        directories::ProjectDirs::from("", "", ".pomme")
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".pomme"))
    })
};

pub fn data_dir() -> &'static Path {
    &DATA_DIR
}

fn ensure_file(path: &Path, default: &str) {
    if !path.exists() {
        let _ = std::fs::write(path, default);
    }
}

pub fn ensure_dirs() {
    let _ = std::fs::create_dir_all(assets_dir());
    let _ = std::fs::create_dir_all(pomme_assets_dir());
    let _ = std::fs::create_dir_all(versions_dir());
    let _ = std::fs::create_dir_all(installations_dir());

    let _ = std::fs::create_dir_all(indexes_dir());
    let _ = std::fs::create_dir_all(objects_dir());

    ensure_file(&settings_file(), "{}");
    ensure_file(&accounts_file(), "[]");
}

pub fn assets_dir() -> PathBuf {
    data_dir().join("assets")
}
pub fn indexes_dir() -> PathBuf {
    assets_dir().join("indexes")
}
pub fn objects_dir() -> PathBuf {
    assets_dir().join("objects")
}

pub fn pomme_assets_dir() -> PathBuf {
    data_dir().join("pomme_assets")
}

pub fn versions_dir() -> PathBuf {
    data_dir().join("versions")
}
pub fn version_dir(version: &str) -> PathBuf {
    versions_dir().join(version)
}
pub fn version_jar(version: &str) -> PathBuf {
    version_dir(version).join(format!("{version}.jar"))
}
pub fn version_extracted_dir(version: &str) -> PathBuf {
    version_dir(version).join("extracted")
}
pub fn version_extracted_marker(version: &str) -> PathBuf {
    version_extracted_dir(version).join(".extracted")
}

pub fn installations_dir() -> PathBuf {
    data_dir().join("installations")
}

pub fn settings_file() -> PathBuf {
    data_dir().join("settings.json")
}
pub fn accounts_file() -> PathBuf {
    data_dir().join("accounts.json")
}

pub fn create_installation_fs(installation: &Installation) -> Result<(), InstallationError> {
    let instance_dir = installations_dir().join(&installation.directory);
    if instance_dir.exists() {
        return Err(InstallationError::DirectoryAlreadyExists);
    }

    for sub in &["mods", "resourcepacks", "shaderpacks"] {
        std::fs::create_dir_all(instance_dir.join(sub))?;
    }

    let install_json = serde_json::to_string_pretty(installation)?;
    std::fs::write(instance_dir.join("installation.json"), install_json)?;

    std::fs::write(
        instance_dir.join("servers.json"),
        serde_json::to_string_pretty(&serde_json::json!([{
          "name": "Test server",
          "address": "mc.kasane.love:29666",
          "resourcePack": "prompt"
        }]))?,
    )?;

    std::fs::write(
        instance_dir.join("options.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "video_settings": {
                "render_distance": 16
            }
        }))?,
    )?;

    write_icon(&instance_dir, installation.icon.as_deref())?;

    Ok(())
}

pub fn write_icon(instance_dir: &Path, icon: Option<&str>) -> Result<(), InstallationError> {
    let dest = instance_dir.join("icon.png");

    match icon {
        Some(data) if data.starts_with("data:image/png;base64,") => {
            use base64::Engine;
            let b64 = &data["data:image/png;base64,".len()..];
            let bytes = &base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| InstallationError::Io(e.to_string()))?;
            std::fs::write(dest, bytes)?;
        }
        Some(path) => {
            std::fs::copy(path, dest)?;
        }
        None => {}
    }

    Ok(())
}
