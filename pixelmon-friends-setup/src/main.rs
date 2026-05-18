#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]

mod assets;
mod download;
mod gui;
mod hash;
mod java;
mod launcher;
mod launcher_profile;
mod manifest;
mod neoforge;
mod optimization;
mod paths;
mod pixelmon;
mod process;
mod server_list;
mod ui_events;

use anyhow::{Context, Result};
use manifest::load_manifest;
use paths::InstallPaths;

fn main() {
    #[cfg(windows)]
    {
        let exit_code = gui::run();
        std::process::exit(exit_code);
    }

    #[cfg(not(windows))]
    {
        std::process::exit(console_main());
    }
}

#[cfg(not(windows))]
fn console_main() -> i32 {
    match run_installation() {
        Ok(()) => {
            println!();
            launcher::print_completion_message();
            launcher::wait_for_enter_silent();
            0
        }
        Err(err) => {
            eprintln!();
            eprintln!("설치를 계속할 수 없습니다:");
            eprintln!("{err:#}");
            launcher::wait_for_enter();
            1
        }
    }
}

fn run_installation() -> Result<()> {
    ui_events::log("픽셀몬 프렌즈 설치 도우미");
    ui_events::log("========================");

    let manifest = load_manifest();
    let paths = InstallPaths::detect().context("윈도우 마인크래프트 경로를 찾을 수 없습니다")?;

    ui_events::log(format!("APPDATA: {}", paths.appdata.display()));
    ui_events::log(format!(
        "마인크래프트 데이터 폴더: {}",
        paths.minecraft_dir.display()
    ));
    ui_events::log(format!("전용 게임 폴더: {}", paths.game_dir.display()));
    ui_events::log(format!(
        "설정된 서버: {} ({})",
        manifest.server_name, manifest.server_address
    ));

    ui_events::progress(0.08, "마인크래프트 폴더를 확인하는 중...");
    paths.validate_minecraft_dir()?;

    ui_events::progress(0.14, "마인크래프트 런처 실행 여부를 확인하는 중...");
    process::ensure_launcher_not_running()?;

    ui_events::progress(0.20, "설치 폴더를 만드는 중...");
    paths.ensure_dirs()?;

    ui_events::progress(0.28, "Java 21 이상을 확인하는 중...");
    java::ensure_java(manifest.java_major)?;

    ui_events::progress(0.38, "NeoForge를 설치하는 중...");
    neoforge::ensure_neoforge(&manifest, &paths)?;

    ui_events::progress(0.52, "Pixelmon을 설치하는 중...");
    pixelmon::install_pixelmon(&manifest, &paths)?;

    ui_events::progress(0.72, "최적화 모드를 설치하는 중...");
    optimization::install_optimization_mods(&manifest, &paths)?;

    ui_events::progress(0.86, "공식 런처 프로필을 추가하는 중...");
    launcher_profile::upsert_launcher_profile(&manifest, &paths)?;

    ui_events::progress(0.92, "선택 자산을 복사하는 중...");
    assets::install_optional_assets(&paths)?;

    ui_events::progress(0.96, "서버 목록을 추가하는 중...");
    server_list::upsert_server(&manifest, &paths)?;

    Ok(())
}
