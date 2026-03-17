// TODO: Move to launcher - asset downloading will be handled by mc-launcher when it exists.

use serde::Deserialize;
use thiserror::Error;

const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const RESOURCES_BASE_URL: &str = "https://resources.download.minecraft.net";

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("version {0} not found in manifest")]
    VersionNotFound(String),
}

#[derive(Deserialize)]
struct VersionManifest {
    versions: Vec<VersionEntry>,
}

#[derive(Deserialize)]
struct VersionEntry {
    id: String,
    url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionJson {
    asset_index: AssetIndexRef,
    downloads: Downloads,
}

#[derive(Deserialize)]
struct AssetIndexRef {
    #[allow(dead_code)]
    id: String,
    url: String,
}

#[derive(Deserialize)]
struct Downloads {
    client: DownloadEntry,
}

#[derive(Deserialize)]
struct DownloadEntry {
    url: String,
    sha1: String,
    size: u64,
}

#[derive(Deserialize)]
struct AssetIndexJson {
    objects: std::collections::HashMap<String, AssetObject>,
}

#[derive(Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

pub struct DownloadProgress {
    pub downloaded: u32,
    pub total: u32,
    pub status: String,
}

pub async fn download_assets_with_progress(
    data: &crate::data::DataDir,
    version: &str,
    progress: &dyn Fn(DownloadProgress),
) -> Result<(), DownloadError> {
    let client = reqwest::Client::new();

    let index_path = data.indexes_dir.join(format!("{version}.json"));

    let (asset_index, version_json) = if index_path.exists() {
        log::info!("Asset index for {version} already cached");
        let content = std::fs::read_to_string(&index_path)?;
        let idx = serde_json::from_str::<AssetIndexJson>(&content)?;
        (idx, None)
    } else {
        let (idx, vj) = fetch_version_metadata(&client, version, &index_path).await?;
        (idx, Some(vj))
    };

    download_asset_objects(data, &client, &asset_index, progress).await?;
    download_client_jar(data, &client, version, version_json.as_ref(), progress).await?;

    Ok(())
}

async fn fetch_version_metadata(
    client: &reqwest::Client,
    version: &str,
    index_path: &std::path::Path,
) -> Result<(AssetIndexJson, VersionJson), DownloadError> {
    log::info!("Fetching version manifest...");
    let manifest: VersionManifest = client
        .get(VERSION_MANIFEST_URL)
        .send()
        .await?
        .json()
        .await?;

    let version_entry = manifest
        .versions
        .iter()
        .find(|v| v.id == version)
        .ok_or_else(|| DownloadError::VersionNotFound(version.to_string()))?;

    log::info!("Fetching version JSON for {version}...");
    let version_json: VersionJson = client.get(&version_entry.url).send().await?.json().await?;

    log::info!("Fetching asset index...");
    let index_content = client
        .get(&version_json.asset_index.url)
        .send()
        .await?
        .text()
        .await?;

    std::fs::write(index_path, &index_content)?;
    let idx = serde_json::from_str::<AssetIndexJson>(&index_content)?;

    Ok((idx, version_json))
}

async fn download_asset_objects(
    data: &crate::data::DataDir,
    client: &reqwest::Client,
    asset_index: &AssetIndexJson,
    progress: &dyn Fn(DownloadProgress),
) -> Result<(), DownloadError> {
    let total = asset_index.objects.len() as u32;
    let mut downloaded = 0u32;
    let mut skipped = 0u32;

    for (name, obj) in &asset_index.objects {
        let prefix = &obj.hash[..2];
        let dir = data.objects_dir.join(prefix);
        let path = dir.join(&obj.hash);

        if path.exists() && std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) == obj.size {
            skipped += 1;
            continue;
        }

        std::fs::create_dir_all(&dir)?;

        let url = format!("{RESOURCES_BASE_URL}/{prefix}/{}", obj.hash);
        let bytes = download_with_retry(client, &url, 3).await?;

        let actual_hash = format!("{}", sha1_smol::Sha1::from(&bytes).digest());
        if actual_hash != obj.hash {
            log::warn!(
                "Hash mismatch for {name}: expected {}, got {actual_hash}",
                obj.hash
            );
            continue;
        }

        std::fs::write(&path, &bytes)?;
        downloaded += 1;

        if downloaded % 50 == 0 {
            let need = total - skipped;
            progress(DownloadProgress {
                downloaded,
                total: need,
                status: format!("Downloading assets ({downloaded}/{need})"),
            });
        }
    }

    log::info!(
        "Asset download complete: {downloaded} downloaded, {skipped} already present, {total} total"
    );

    Ok(())
}

async fn download_client_jar(
    data: &crate::data::DataDir,
    client: &reqwest::Client,
    version: &str,
    cached_version_json: Option<&VersionJson>,
    progress: &dyn Fn(DownloadProgress),
) -> Result<(), DownloadError> {
    let jar_assets_dir = data.assets_dir.join("jar");
    let marker = jar_assets_dir.join(".extracted");
    if marker.exists() {
        log::info!("Client JAR assets already extracted");
        return Ok(());
    }

    let fetched;
    let dl = if let Some(vj) = cached_version_json {
        &vj.downloads.client
    } else {
        log::info!("Fetching version manifest for client JAR...");
        let manifest: VersionManifest = client
            .get(VERSION_MANIFEST_URL)
            .send()
            .await?
            .json()
            .await?;
        let entry = manifest
            .versions
            .iter()
            .find(|v| v.id == version)
            .ok_or_else(|| DownloadError::VersionNotFound(version.to_string()))?;
        let vj: VersionJson = client.get(&entry.url).send().await?.json().await?;
        fetched = vj.downloads.client;
        &fetched
    };

    let jar_path = data.versions_dir.join(format!("{version}.jar"));
    if !jar_path.exists() || std::fs::metadata(&jar_path).map(|m| m.len()).unwrap_or(0) != dl.size {
        progress(DownloadProgress {
            downloaded: 0,
            total: 1,
            status: "Downloading client JAR...".into(),
        });
        let bytes = download_with_retry(client, &dl.url, 3).await?;

        let actual_hash = format!("{}", sha1_smol::Sha1::from(&bytes).digest());
        if actual_hash != dl.sha1 {
            log::warn!(
                "Client JAR hash mismatch: expected {}, got {actual_hash}",
                dl.sha1
            );
        }

        std::fs::create_dir_all(&data.versions_dir)?;
        std::fs::write(&jar_path, &bytes)?;
    }

    progress(DownloadProgress {
        downloaded: 0,
        total: 1,
        status: "Extracting client JAR...".into(),
    });
    extract_jar_assets(&jar_path, &jar_assets_dir)?;

    std::fs::write(&marker, version)?;
    Ok(())
}

fn extract_jar_assets(
    jar_path: &std::path::Path,
    output_dir: &std::path::Path,
) -> Result<(), DownloadError> {
    log::info!("Extracting assets from client JAR...");
    let file = std::fs::File::open(jar_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut extracted = 0u32;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();

        if !name.starts_with("assets/") || entry.is_dir() {
            continue;
        }

        let out_path = output_dir.join(&name);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut out_file = std::fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out_file)?;
        extracted += 1;
    }

    log::info!("Extracted {extracted} assets from client JAR");
    Ok(())
}

async fn download_with_retry(
    client: &reqwest::Client,
    url: &str,
    max_retries: u32,
) -> Result<Vec<u8>, reqwest::Error> {
    let mut last_err = None;
    for attempt in 0..max_retries {
        match client.get(url).send().await?.bytes().await {
            Ok(bytes) => return Ok(bytes.to_vec()),
            Err(e) => {
                log::warn!("Download attempt {} failed for {url}: {e}", attempt + 1);
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap())
}

pub fn needs_download(data: &crate::data::DataDir) -> bool {
    let no_index = !data.indexes_dir.exists()
        || std::fs::read_dir(&data.indexes_dir)
            .map(|mut d| d.next().is_none())
            .unwrap_or(true);
    let no_jar_assets = !data.assets_dir.join("jar").join(".extracted").exists();
    no_index || no_jar_assets
}
