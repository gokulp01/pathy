use std::path::{Path, PathBuf};

use zed_extension_api as zed;
use zed::settings::LspSettings;
use zed::{download_file, make_file_executable, serde_json, Command, DownloadedFileType};

const LANGUAGE_SERVER_ID: &str = "pathy";
const DEFAULT_REPO: &str = "placeholder/zed-pathy";
const CACHE_ROOT_DIR: &str = "cache";

#[derive(Debug, Clone)]
struct ExtensionConfig {
    auto_download: bool,
    server_path: Option<String>,
    release_channel: String,
    base_url: Option<String>,
    verify_checksum: bool,
    cache_dir: Option<String>,
}

impl Default for ExtensionConfig {
    fn default() -> Self {
        Self {
            auto_download: true,
            server_path: None,
            release_channel: "stable".to_string(),
            base_url: None,
            verify_checksum: true,
            cache_dir: None,
        }
    }
}

struct PathyExtension;

impl zed::Extension for PathyExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        if language_server_id.as_ref() != LANGUAGE_SERVER_ID {
            return Err("unknown language server".to_string());
        }

        let settings = LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree)
            .map_err(|err| format!("settings error: {err}"))?;
        let config = load_extension_config(settings.settings.as_ref());

        if let Some(path) = config.server_path.as_ref() {
            let resolved = PathBuf::from(path);
            if resolved.exists() {
                return Ok(Command::new(resolved.to_string_lossy())
                    .envs(worktree.shell_env()));
            }
            return Err(format!("server_path not found: {path}"));
        }

        if !config.auto_download {
            return Err("auto_download disabled and no server_path provided".to_string());
        }

        if config.release_channel != "stable" {
            return Err("only stable release_channel is supported".to_string());
        }

        let version = extension_version();
        let platform = current_platform()?;
        let cache_root = cache_root(&config)?;
        let cache_path = cached_binary_path(&cache_root, &version, &platform);

        if cache_path.exists() {
            return Ok(Command::new(cache_path.to_string_lossy()).envs(worktree.shell_env()));
        }

        let base_url = config.base_url.clone().unwrap_or_else(|| {
            format!(
                "https://github.com/{DEFAULT_REPO}/releases/download/v{version}"
            )
        });

        let (asset_name, archive_type) = asset_name_for(&version, &platform)?;
        let archive_path = cache_root.join(format!("{asset_name}"));
        let checksum_url = format!("{base_url}/checksums-{version}.txt");
        let checksum_path = cache_root.join(format!("checksums-{version}.txt"));

        ensure_dir(cache_root.as_path())?;

        let checksum_path_str = checksum_path.to_string_lossy().to_string();
        download_file(
            &checksum_url,
            &checksum_path_str,
            DownloadedFileType::Uncompressed,
        )
        .map_err(|err| format!("checksum download failed: {err}"))?;

        let checksums = read_to_string(&checksum_path)?;
        let expected = parse_checksum(&checksums, &asset_name)
            .ok_or_else(|| "checksum missing for asset".to_string())?;

        let archive_path_str = archive_path.to_string_lossy().to_string();
        download_file(
            &format!("{base_url}/{asset_name}"),
            &archive_path_str,
            DownloadedFileType::Uncompressed,
        )
        .map_err(|err| format!("asset download failed: {err}"))?;

        if config.verify_checksum {
            let digest = sha256_hex(&archive_path)?;
            if digest != expected {
                std::fs::remove_file(&archive_path).ok();
                return Err("checksum verification failed".to_string());
            }
        }

        extract_archive(&archive_path, &cache_root, &platform, archive_type)?;

        let extracted = extracted_binary_path(&cache_root, &platform);
        if !extracted.exists() {
            return Err("expected extracted binary missing".to_string());
        }

        if !is_windows() {
            let extracted_str = extracted.to_string_lossy().to_string();
            make_file_executable(&extracted_str)
                .map_err(|err| format!("chmod failed: {err}"))?;
        }

        let final_path = cache_path;
        ensure_dir(final_path.parent().unwrap())?;
        std::fs::rename(&extracted, &final_path).map_err(|err| err.to_string())?;

        Ok(Command::new(final_path.to_string_lossy()).envs(worktree.shell_env()))
    }
}

fn load_extension_config(settings: Option<&serde_json::Value>) -> ExtensionConfig {
    let mut config = ExtensionConfig::default();
    let Some(settings) = settings else {
        return config;
    };
    let Some(map) = settings.as_object() else {
        return config;
    };

    for (key, value) in map {
        match key.as_str() {
            "auto_download" => set_bool(&mut config.auto_download, value),
            "server_path" => {
                if let Some(s) = value.as_str() {
                    config.server_path = Some(s.to_string());
                }
            }
            "release_channel" => {
                if let Some(s) = value.as_str() {
                    config.release_channel = s.to_string();
                }
            }
            "base_url" => {
                if let Some(s) = value.as_str() {
                    config.base_url = Some(s.to_string());
                }
            }
            "verify_checksum" => set_bool(&mut config.verify_checksum, value),
            "cache_dir" => {
                if let Some(s) = value.as_str() {
                    config.cache_dir = Some(s.to_string());
                }
            }
            _ => {}
        }
    }

    config
}

fn set_bool(target: &mut bool, value: &serde_json::Value) {
    if let Some(v) = value.as_bool() {
        *target = v;
    }
}

fn extension_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[derive(Debug, Clone)]
struct PlatformInfo {
    os: String,
    arch: String,
}

fn current_platform() -> zed::Result<PlatformInfo> {
    let (os, arch) = zed::current_platform();
    let os = match os {
        zed::Os::Mac => "macos",
        zed::Os::Linux => "linux",
        zed::Os::Windows => "windows",
    };
    let arch = match arch {
        zed::Architecture::Aarch64 => "aarch64",
        zed::Architecture::X8664 => "x86_64",
        zed::Architecture::X86 => "x86",
    };
    Ok(PlatformInfo {
        os: os.to_string(),
        arch: arch.to_string(),
    })
}

fn asset_name_for(
    version: &str,
    platform: &PlatformInfo,
) -> zed::Result<(String, DownloadedFileType)> {
    if platform.arch != "x86_64" && platform.arch != "aarch64" {
        return Err("unsupported architecture".to_string());
    }
    let archive = match platform.os.as_str() {
        "windows" => "zip",
        "macos" | "linux" => "tar.gz",
        _ => return Err("unsupported platform".to_string()),
    };
    let filename = format!(
        "pathy-server_{version}_{}_{}.{archive}",
        platform.os, platform.arch
    );
    let file_type = match archive {
        "zip" => DownloadedFileType::Zip,
        "tar.gz" => DownloadedFileType::GzipTar,
        _ => DownloadedFileType::Uncompressed,
    };
    Ok((filename, file_type))
}

fn cached_binary_path(cache_root: &Path, version: &str, platform: &PlatformInfo) -> PathBuf {
    let mut path = cache_root
        .join("pathy")
        .join(version)
        .join(&platform.os)
        .join(&platform.arch)
        .join("pathy-server");
    if platform.os == "windows" {
        path.set_extension("exe");
    }
    path
}

fn extracted_binary_path(cache_root: &Path, platform: &PlatformInfo) -> PathBuf {
    let mut path = cache_root.join("pathy-server");
    if platform.os == "windows" {
        path.set_extension("exe");
    }
    path
}

fn cache_root(config: &ExtensionConfig) -> zed::Result<PathBuf> {
    if let Some(dir) = config.cache_dir.as_ref() {
        let path = PathBuf::from(dir);
        if path.is_absolute() {
            return Err("cache_dir must be relative to extension working directory".to_string());
        }
        return Ok(path);
    }
    Ok(PathBuf::from(CACHE_ROOT_DIR))
}

fn ensure_dir(dir: &Path) -> zed::Result<()> {
    std::fs::create_dir_all(dir).map_err(|err| err.to_string())
}

fn parse_checksum(checksums: &str, filename: &str) -> Option<String> {
    for line in checksums.lines() {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?;
        if name == filename {
            return Some(hash.to_string());
        }
    }
    None
}

fn sha256_hex(path: &Path) -> zed::Result<String> {
    use sha2::{Digest, Sha256};
    let data = std::fs::read(path).map_err(|err| err.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let digest = hasher.finalize();
    Ok(format!("{:x}", digest))
}

fn read_to_string(path: &Path) -> zed::Result<String> {
    std::fs::read_to_string(path).map_err(|err| err.to_string())
}

fn extract_archive(
    archive_path: &Path,
    cache_root: &Path,
    platform: &PlatformInfo,
    archive_type: DownloadedFileType,
) -> zed::Result<()> {
    match archive_type {
        DownloadedFileType::GzipTar => extract_tar_gz(archive_path, cache_root, platform),
        DownloadedFileType::Zip => extract_zip(archive_path, cache_root, platform),
        _ => Err("unsupported archive type".to_string()),
    }
}

fn extract_tar_gz(
    archive_path: &Path,
    cache_root: &Path,
    platform: &PlatformInfo,
) -> zed::Result<()> {
    use flate2::read::GzDecoder;
    use std::fs::File;
    use tar::Archive;

    let file = File::open(archive_path).map_err(|err| err.to_string())?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    let target_name = if platform.os == "windows" {
        "pathy-server.exe"
    } else {
        "pathy-server"
    };

    for entry in archive.entries().map_err(|err| err.to_string())? {
        let mut entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path().map_err(|err| err.to_string())?;
        if path.file_name().and_then(|n| n.to_str()) == Some(target_name) {
            let dest = extracted_binary_path(cache_root, platform);
            ensure_dir(dest.parent().unwrap())?;
            entry.unpack(&dest).map_err(|err| err.to_string())?;
            return Ok(());
        }
    }

    Err("binary not found in archive".to_string())
}

fn extract_zip(
    archive_path: &Path,
    cache_root: &Path,
    platform: &PlatformInfo,
) -> zed::Result<()> {
    use std::fs::File;
    use zip::ZipArchive;

    let file = File::open(archive_path).map_err(|err| err.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|err| err.to_string())?;
    let target_name = if platform.os == "windows" {
        "pathy-server.exe"
    } else {
        "pathy-server"
    };

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|err| err.to_string())?;
        if file.name().ends_with(target_name) {
            let dest = extracted_binary_path(cache_root, platform);
            ensure_dir(dest.parent().unwrap())?;
            let mut out = std::fs::File::create(&dest).map_err(|err| err.to_string())?;
            std::io::copy(&mut file, &mut out).map_err(|err| err.to_string())?;
            return Ok(());
        }
    }

    Err("binary not found in archive".to_string())
}

fn is_windows() -> bool {
    cfg!(windows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_name_mapping() {
        let platform = PlatformInfo {
            os: "linux".into(),
            arch: "x86_64".into(),
        };
        let (name, _ty) = asset_name_for("0.4.0", &platform).unwrap();
        assert_eq!(name, "pathy-server_0.4.0_linux_x86_64.tar.gz");
    }

    #[test]
    fn checksum_parsing() {
        let data = "abcd1234  pathy-server_0.4.0_linux_x86_64.tar.gz\n";
        let hash = parse_checksum(data, "pathy-server_0.4.0_linux_x86_64.tar.gz").unwrap();
        assert_eq!(hash, "abcd1234");
    }

    #[test]
    fn cache_dir_relative() {
        let mut config = ExtensionConfig::default();
        config.cache_dir = Some("my-cache".to_string());
        let path = cache_root(&config).unwrap();
        assert_eq!(path, PathBuf::from("my-cache"));
    }
}

zed::register_extension!(PathyExtension);
