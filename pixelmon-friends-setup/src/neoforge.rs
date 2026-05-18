use crate::{
    download::atomic_download, launcher, manifest::Manifest, paths::InstallPaths, ui_events,
};
use anyhow::{anyhow, Context, Result};
use std::{path::PathBuf, process::Command};

pub fn ensure_neoforge(manifest: &Manifest, paths: &InstallPaths) -> Result<()> {
    if neoforge_installed(manifest, paths) {
        ui_events::log(format!(
            "NeoForge {}은(는) 이미 설치되어 있습니다.",
            manifest.neoforge_version
        ));
        launcher::print_installed_message("NeoForge");
        return Ok(());
    }

    let installer_name = format!("neoforge-{}-installer.jar", manifest.neoforge_version);
    let url = format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{0}/neoforge-{0}-installer.jar",
        manifest.neoforge_version
    );
    let installer_path = paths.temp_dir.join(installer_name);

    ui_events::log(format!(
        "NeoForge 설치 프로그램 {}을(를) 다운로드하는 중...",
        manifest.neoforge_version
    ));
    atomic_download(&url, &paths.temp_dir, &installer_path, None)
        .context("NeoForge 설치 프로그램을 다운로드할 수 없습니다")?;

    ui_events::log("NeoForge 설치 프로그램을 엽니다. Install client를 선택하고 완료하면 설치 도우미가 계속 진행됩니다.");
    let status = Command::new("java")
        .arg("-jar")
        .arg(&installer_path)
        .status()
        .context("Java로 NeoForge 설치 프로그램을 시작할 수 없습니다")?;

    if !status.success() {
        return Err(anyhow!(
            "NeoForge 설치 프로그램이 정상적으로 완료되지 않았습니다. 설치 도우미를 다시 실행하고 NeoForge 설치 프로그램에서 Install client를 선택해주세요."
        ));
    }

    if neoforge_installed(manifest, paths) {
        launcher::print_installed_message("NeoForge");
        Ok(())
    } else {
        Err(anyhow!(
            "NeoForge 설치 프로그램이 종료된 뒤에도 NeoForge가 감지되지 않았습니다. NeoForge 설치 프로그램에서 Install client를 선택한 뒤 이 설치 도우미를 다시 실행해주세요."
        ))
    }
}

fn neoforge_installed(manifest: &Manifest, paths: &InstallPaths) -> bool {
    neoforge_version_json(manifest, paths).is_file()
}

fn neoforge_version_json(manifest: &Manifest, paths: &InstallPaths) -> PathBuf {
    paths
        .minecraft_dir
        .join("versions")
        .join(&manifest.neoforge_version_id)
        .join(format!("{}.json", manifest.neoforge_version_id))
}
