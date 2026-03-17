use std::path::{Path, PathBuf};

use directories::ProjectDirs;

pub struct DataDir {
    pub root: PathBuf,
    pub assets_dir: PathBuf,
    pub indexes_dir: PathBuf,
    pub objects_dir: PathBuf,
    pub instance_dir: PathBuf,
    pub versions_dir: PathBuf,
}

impl DataDir {
    pub fn resolve(game_dir_override: Option<&str>, assets_dir_override: Option<&str>) -> Self {
        let root = if let Some(dir) = game_dir_override {
            PathBuf::from(dir)
        } else {
            default_data_dir()
        };

        let assets_dir = if let Some(dir) = assets_dir_override {
            PathBuf::from(dir)
        } else {
            root.join("assets")
        };

        Self {
            indexes_dir: assets_dir.join("indexes"),
            objects_dir: assets_dir.join("objects"),
            instance_dir: root.join("instances").join("default"),
            versions_dir: root.join("versions"),
            assets_dir,
            root,
        }
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.indexes_dir)?;
        std::fs::create_dir_all(&self.objects_dir)?;
        std::fs::create_dir_all(&self.instance_dir)?;
        std::fs::create_dir_all(&self.versions_dir)?;
        Ok(())
    }
}

fn default_data_dir() -> PathBuf {
    ProjectDirs::from("", "", ".pomc")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| Path::new(".pomc").to_path_buf())
}
