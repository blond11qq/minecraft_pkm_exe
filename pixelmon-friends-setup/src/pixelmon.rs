use crate::{
    download::{atomic_download, http_client},
    launcher,
    manifest::Manifest,
    paths::InstallPaths,
    ui_events,
};
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Clone, Debug, Deserialize)]
pub struct ModrinthVersion {
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<ModrinthFile>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ModrinthFile {
    pub url: String,
    pub filename: String,
    pub hashes: ModrinthHashes,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ModrinthHashes {
    pub sha512: Option<String>,
}

pub fn install_pixelmon(manifest: &Manifest, paths: &InstallPaths) -> Result<()> {
    ui_events::log(format!(
        "Modrinth에서 Pixelmon {}을(를) 찾는 중...",
        manifest.pixelmon_version
    ));
    let versions = fetch_versions(&manifest.pixelmon_modrinth_project)?;
    let file = select_pixelmon_file(
        &versions,
        &manifest.pixelmon_version,
        &manifest.minecraft_version,
    )
    .ok_or_else(|| {
        anyhow!(
            "Modrinth에서 마인크래프트 {} 및 NeoForge 호환 로더에 맞는 Pixelmon {}을(를) 찾을 수 없습니다.",
            manifest.minecraft_version,
            manifest.pixelmon_version,
        )
    })?;

    remove_old_pixelmon_jars(&paths.mods_dir)?;
    let final_path = paths.mods_dir.join(&file.filename);
    ui_events::log(format!("{}을(를) 다운로드하는 중...", file.filename));
    atomic_download(
        &file.url,
        &paths.temp_dir,
        &final_path,
        file.hashes.sha512.as_deref(),
    )
    .with_context(|| {
        format!(
            "Pixelmon을 {}에 설치할 수 없습니다",
            paths.mods_dir.display()
        )
    })?;
    launcher::print_installed_message("Pixelmon");
    Ok(())
}

pub fn fetch_versions(project: &str) -> Result<Vec<ModrinthVersion>> {
    let url = format!("https://api.modrinth.com/v2/project/{project}/version");
    let client = http_client()?;
    client
        .get(&url)
        .send()
        .with_context(|| format!("Modrinth에 접속할 수 없습니다: {url}"))?
        .error_for_status()
        .with_context(|| format!("Modrinth에서 오류를 반환했습니다: {url}"))?
        .json()
        .context("Modrinth 버전 응답을 해석할 수 없습니다")
}

pub fn select_pixelmon_file<'a>(
    versions: &'a [ModrinthVersion],
    pixelmon_version: &str,
    minecraft_version: &str,
) -> Option<&'a ModrinthFile> {
    let version = versions.iter().find(|version| {
        version.version_number == pixelmon_version
            && is_minecraft_neoforge_compatible(version, minecraft_version)
    })?;

    select_primary_file(version)
}

pub fn select_modrinth_file<'a>(
    versions: &'a [ModrinthVersion],
    minecraft_version: &str,
) -> Option<&'a ModrinthFile> {
    let version = versions
        .iter()
        .find(|version| is_minecraft_neoforge_compatible(version, minecraft_version))?;

    select_primary_file(version)
}

fn is_minecraft_neoforge_compatible(version: &ModrinthVersion, minecraft_version: &str) -> bool {
    version
        .game_versions
        .iter()
        .any(|game| game == minecraft_version)
        && version.loaders.iter().any(|loader| {
            loader.eq_ignore_ascii_case("neoforge") || loader.eq_ignore_ascii_case("forge")
        })
}

fn select_primary_file(version: &ModrinthVersion) -> Option<&ModrinthFile> {
    version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
}

fn remove_old_pixelmon_jars(mods_dir: &Path) -> Result<()> {
    fs::create_dir_all(mods_dir)
        .with_context(|| format!("mods 폴더를 만들 수 없습니다: {}", mods_dir.display()))?;
    for entry in fs::read_dir(mods_dir)
        .with_context(|| format!("mods 폴더를 읽을 수 없습니다: {}", mods_dir.display()))?
    {
        let entry = entry
            .with_context(|| format!("{} 안의 항목을 읽을 수 없습니다", mods_dir.display()))?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_ascii_lowercase();
        if lower.starts_with("pixelmon-") && lower.ends_with(".jar") {
            fs::remove_file(entry.path())
                .with_context(|| format!("기존 Pixelmon jar를 삭제할 수 없습니다: {name}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{select_pixelmon_file, ModrinthFile, ModrinthHashes, ModrinthVersion};

    #[test]
    fn selects_matching_pixelmon_neoforge_file() {
        let versions = vec![
            ModrinthVersion {
                version_number: "9.3.15".to_string(),
                game_versions: vec!["1.21.1".to_string()],
                loaders: vec!["neoforge".to_string()],
                files: vec![file("wrong.jar", false)],
            },
            ModrinthVersion {
                version_number: "9.3.16".to_string(),
                game_versions: vec!["1.20.1".to_string()],
                loaders: vec!["neoforge".to_string()],
                files: vec![file("wrong-mc.jar", false)],
            },
            ModrinthVersion {
                version_number: "9.3.16".to_string(),
                game_versions: vec!["1.21.1".to_string()],
                loaders: vec!["neoforge".to_string()],
                files: vec![file("secondary.jar", false), file("primary.jar", true)],
            },
        ];

        let selected = select_pixelmon_file(&versions, "9.3.16", "1.21.1").unwrap();
        assert_eq!(selected.filename, "primary.jar");
    }

    fn file(filename: &str, primary: bool) -> ModrinthFile {
        ModrinthFile {
            url: format!("https://example.com/{filename}"),
            filename: filename.to_string(),
            hashes: ModrinthHashes {
                sha512: Some("abc".to_string()),
            },
            primary,
        }
    }
}
