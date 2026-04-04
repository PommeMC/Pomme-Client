use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static DATA_DIR: LazyLock<PathBuf> = {
    LazyLock::new(|| {
        directories::ProjectDirs::from("", "", ".pomme")
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".pomme"))
    })
};

/// `.pomme/`
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

/// `.pomme/assets/`
pub fn assets_dir() -> PathBuf {
    data_dir().join("assets")
}
/// `.pomme/assets/indexes/`
pub fn indexes_dir() -> PathBuf {
    assets_dir().join("indexes")
}
/// `.pomme/assets/objects/`
pub fn objects_dir() -> PathBuf {
    assets_dir().join("objects")
}

/// `.pomme/pomme_assets/`
pub fn pomme_assets_dir() -> PathBuf {
    data_dir().join("pomme_assets")
}

/// `.pomme/versions/`
pub fn versions_dir() -> PathBuf {
    data_dir().join("versions")
}
/// `.pomme/versions/{version}/`
pub fn version_dir(version: &str) -> PathBuf {
    versions_dir().join(version)
}
/// `.pomme/versions/{version}/{version}.jar`
pub fn version_jar(version: &str) -> PathBuf {
    version_dir(version).join(format!("{version}.jar"))
}
/// `.pomme/versions/{version}/extracted/`
pub fn version_extracted_dir(version: &str) -> PathBuf {
    version_dir(version).join("extracted")
}
/// `.pomme/versions/{version}/extracted/.extracted`
pub fn version_extracted_marker(version: &str) -> PathBuf {
    version_extracted_dir(version).join(".extracted")
}

/// `.pomme/installations/`
pub fn installations_dir() -> PathBuf {
    data_dir().join("installations")
}

/// `.pomme/settings.json`
pub fn settings_file() -> PathBuf {
    data_dir().join("settings.json")
}
/// `.pomme/accounts.json`
pub fn accounts_file() -> PathBuf {
    data_dir().join("accounts.json")
}
