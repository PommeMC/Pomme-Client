use crate::storage::data_dir;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU64;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_NAME_LENGTH: usize = 25;
const MAX_DIRNAME_LENGTH: usize = 32;
#[cfg(target_os = "windows")]
const RESERVED_DIRNAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];
#[cfg(target_os = "windows")]
const FORBIDDEN_CHAR: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum InstallationError {
    #[error("Invalid name")]
    InvalidName,
    #[error("Name too long, max {0} characters")]
    NameTooLong(usize),
    #[error("Invalid directory")]
    InvalidDirectory,
    #[error("Directory too long, max {0} characters")]
    DirectoryTooLong(usize),
    #[error("Invalid character in directory: {0}")]
    InvalidCharacter(char),
    #[cfg(target_os = "windows")]
    #[error("Reserved name: {0}")]
    ReservedName(String),
    #[error("Trailing space or dot")]
    TrailingDot,
    #[error("Directory already exists")]
    DirectoryAlreadyExists,
    #[error("IO error: {0}")]
    Io(String),
    #[error("JSON error: {0}")]
    Json(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    pub id: String,
    pub icon: Option<String>,
    pub name: String,
    pub version: String,
    pub last_played: Option<NonZeroU64>,
    pub created_at: u64,
    pub directory: String,
    pub width: u32,
    pub height: u32,
    pub can_delete: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NewInstallPayload {
    pub icon: Option<String>,
    pub name: String,
    pub version: String,
    pub directory: String,
    pub width: u32,
    pub height: u32,
}

impl Installation {
    pub fn try_new(data: NewInstallPayload) -> Result<Self, InstallationError> {
        let created_at = u64::from(Self::now_millis());

        Ok(Self {
            id: Self::generate_id(created_at),
            last_played: None,
            created_at,
            can_delete: true,

            icon: data.icon,
            name: Self::try_name(data.name)?,
            version: data.version,
            directory: Self::try_directory(data.directory)?,
            width: data.width,
            height: data.height,
        })
    }

    fn generate_id(created_at: u64) -> String {
        let mut state = created_at;
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
        let suffix: String = (0..4)
            .map(|_| {
                state = state
                    // Knuth MMIX LCG constants - guarantee full 2^64 period
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                chars[((state >> 33) as usize) % chars.len()]
            })
            .collect();

        format!("{created_at}-{suffix}")
    }

    fn now_millis() -> NonZeroU64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| NonZeroU64::new(d.as_millis() as u64))
            .unwrap_or(NonZeroU64::MIN)
    }

    fn try_name(name: String) -> Result<String, InstallationError> {
        if name.trim().is_empty() {
            return Err(InstallationError::InvalidName);
        }
        if name.len() > MAX_NAME_LENGTH {
            return Err(InstallationError::NameTooLong(MAX_NAME_LENGTH));
        }
        Ok(name)
    }

    fn try_directory(dir: String) -> Result<String, InstallationError> {
        if dir.trim().is_empty() || dir == "." || dir == ".." {
            return Err(InstallationError::InvalidDirectory);
        }
        if dir.len() > MAX_DIRNAME_LENGTH {
            return Err(InstallationError::DirectoryTooLong(MAX_DIRNAME_LENGTH));
        }
        if let Some(c) = dir.chars().find(|c| *c == '\0' || *c == '/' || *c == '\\') {
            return Err(InstallationError::InvalidCharacter(c));
        }
        if dir.ends_with('.') {
            return Err(InstallationError::TrailingDot);
        }
        #[cfg(target_os = "windows")]
        Self::validate_directory_os(&dir)?;
        Ok(dir)
    }

    #[cfg(target_os = "windows")]
    fn validate_directory_os(dir: &str) -> Result<(), InstallationError> {
        let stem = dir.split('.').next().unwrap_or("").to_uppercase();
        if RESERVED_DIRNAMES.contains(&stem.as_str()) {
            return Err(InstallationError::ReservedName(dir.to_string()));
        }

        if let Some(c) = dir.chars().find(|c| FORBIDDEN_CHAR.contains(c)) {
            return Err(InstallationError::InvalidCharacter(c));
        }

        Ok(())
    }
}

impl From<std::io::Error> for InstallationError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<serde_json::Error> for InstallationError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e.to_string())
    }
}

pub struct InstallationRegistry;

impl InstallationRegistry {
    pub fn load() -> Result<Vec<Installation>, InstallationError> {
        let path = data_dir().join("installations.json");
        if !path.exists() {
            return Ok(vec![]);
        }
        let raw = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn save(list: &[Installation]) -> Result<(), InstallationError> {
        let json = serde_json::to_string_pretty(list)?;
        std::fs::write(data_dir().join("installations.json"), json)?;
        Ok(())
    }

    pub fn register(installation: Installation) -> Result<(), InstallationError> {
        let mut list = Self::load()?;

        if list.iter().any(|i| i.directory == installation.directory) {
            return Err(InstallationError::DirectoryAlreadyExists);
        }

        list.push(installation);
        Self::save(&list)
    }
}
