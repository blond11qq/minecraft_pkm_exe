use crate::{
    download::atomic_download,
    launcher,
    manifest::{AdditionalMod, Manifest},
    paths::InstallPaths,
    pixelmon::{fetch_versions, select_modrinth_file},
    ui_events,
};
use anyhow::{anyhow, Context, Result};
use std::{fs, path::Path};

const RETIRED_MOD_PREFIXES: &[&str] = &[
    "embeddium-",
    "jei-",
    "jade-",
    "xaeroworldmap-",
    "xaerominimap-",
    "mousetweaks-",
    "balm-",
    "waystones-",
    "lootr-",
];

pub fn install_optimization_mods(manifest: &Manifest, paths: &InstallPaths) -> Result<()> {
    remove_retired_bundled_mods(&paths.mods_dir)?;

    if manifest.additional_mods.is_empty() {
        return Ok(());
    }

    ui_events::log("최적화 모드를 설치하는 중...");
    for additional_mod in &manifest.additional_mods {
        install_additional_mod(additional_mod, manifest, paths)?;
    }

    Ok(())
}

fn install_additional_mod(
    additional_mod: &AdditionalMod,
    manifest: &Manifest,
    paths: &InstallPaths,
) -> Result<()> {
    ui_events::log(format!("Modrinth에서 {}을(를) 찾는 중...", additional_mod.name));
    let versions = fetch_versions(&additional_mod.modrinth_project)?;
    let file = select_modrinth_file(&versions, &manifest.minecraft_version).ok_or_else(|| {
        anyhow!(
            "Modrinth에서 마인크래프트 {} 및 NeoForge에 맞는 {} 파일을 찾을 수 없습니다.",
            manifest.minecraft_version,
            additional_mod.name
        )
    })?;

    remove_old_mod_jars(&paths.mods_dir, &additional_mod.filename_prefixes)?;
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
            "{}을(를) {}에 설치할 수 없습니다",
            additional_mod.name,
            paths.mods_dir.display()
        )
    })?;
    launcher::print_installed_message(&additional_mod.name);

    Ok(())
}

fn remove_retired_bundled_mods(mods_dir: &Path) -> Result<()> {
    let retired_prefixes = RETIRED_MOD_PREFIXES
        .iter()
        .map(|prefix| prefix.to_string())
        .collect::<Vec<_>>();
    remove_old_mod_jars(mods_dir, &retired_prefixes)
}

fn remove_old_mod_jars(mods_dir: &Path, filename_prefixes: &[String]) -> Result<()> {
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

        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let is_target = name.ends_with(".jar")
            && filename_prefixes
                .iter()
                .any(|prefix| name.starts_with(&prefix.to_ascii_lowercase()));
        if is_target {
            fs::remove_file(entry.path())
                .with_context(|| format!("기존 추가 모드 jar를 삭제할 수 없습니다: {name}"))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::pixelmon::{select_modrinth_file, ModrinthFile, ModrinthHashes, ModrinthVersion};

    #[test]
    fn selects_latest_neoforge_optimization_file_for_minecraft_version() {
        let versions = vec![
            ModrinthVersion {
                version_number: "1.0.0".to_string(),
                game_versions: vec!["1.21".to_string()],
                loaders: vec!["neoforge".to_string()],
                files: vec![file("wrong-mc.jar", true)],
            },
            ModrinthVersion {
                version_number: "1.0.1".to_string(),
                game_versions: vec!["1.21.1".to_string()],
                loaders: vec!["fabric".to_string()],
                files: vec![file("wrong-loader.jar", true)],
            },
            ModrinthVersion {
                version_number: "1.0.2".to_string(),
                game_versions: vec!["1.21.1".to_string()],
                loaders: vec!["neoforge".to_string()],
                files: vec![file("secondary.jar", false), file("primary.jar", true)],
            },
        ];

        let selected = select_modrinth_file(&versions, "1.21.1").unwrap();
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
