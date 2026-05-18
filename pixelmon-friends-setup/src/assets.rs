use crate::{launcher, paths::InstallPaths};
use anyhow::{Context, Result};
use std::{env, fs, path::PathBuf};

pub fn install_optional_assets(paths: &InstallPaths) -> Result<()> {
    for filename in ["servers.dat", "options.txt"] {
        if let Some(source) = find_asset(filename) {
            let destination = paths.game_dir.join(filename);
            if !destination.exists() {
                fs::copy(&source, &destination).with_context(|| {
                    format!(
                        "선택 자산 {}을(를) {} 위치로 복사할 수 없습니다",
                        source.display(),
                        destination.display()
                    )
                })?;
                launcher::print_installed_message(filename);
            }
        }
    }
    Ok(())
}

fn find_asset(filename: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("assets").join(filename));
        }
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(filename),
    );
    candidates.into_iter().find(|path| path.is_file())
}
