use anyhow::{anyhow, Context, Result};
use std::{env, fs, path::PathBuf};

#[derive(Clone, Debug)]
pub struct InstallPaths {
    pub appdata: PathBuf,
    pub minecraft_dir: PathBuf,
    pub game_dir: PathBuf,
    pub mods_dir: PathBuf,
    pub config_dir: PathBuf,
    pub backups_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub launcher_profile_candidates: Vec<PathBuf>,
}

impl InstallPaths {
    pub fn detect() -> Result<Self> {
        let appdata = env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow!("APPDATA를 찾을 수 없습니다. 이 설치 도우미는 윈도우 일반 사용자 계정에서 실행해야 합니다."))?;
        Ok(Self::from_appdata(appdata))
    }

    pub fn from_appdata(appdata: PathBuf) -> Self {
        let minecraft_dir = appdata.join(".minecraft");
        let game_dir = appdata.join(".minecraft-pixelmon-friends");
        let mods_dir = game_dir.join("mods");
        let config_dir = game_dir.join("config");
        let backups_dir = game_dir.join("backups");
        let logs_dir = game_dir.join("logs");
        let temp_dir = game_dir.join("tmp");
        let launcher_profile_candidates = vec![
            minecraft_dir.join("launcher_profiles.json"),
            minecraft_dir.join("launcher_profiles_microsoft_store.json"),
        ];

        Self {
            appdata,
            minecraft_dir,
            game_dir,
            mods_dir,
            config_dir,
            backups_dir,
            logs_dir,
            temp_dir,
            launcher_profile_candidates,
        }
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        for dir in [
            &self.game_dir,
            &self.mods_dir,
            &self.config_dir,
            &self.backups_dir,
            &self.logs_dir,
            &self.temp_dir,
        ] {
            fs::create_dir_all(dir)
                .with_context(|| format!("폴더를 만들 수 없습니다: {}", dir.display()))?;
        }
        Ok(())
    }

    pub fn validate_minecraft_dir(&self) -> Result<()> {
        if self.minecraft_dir.is_dir() {
            Ok(())
        } else {
            Err(anyhow!(
                "{} 폴더를 찾을 수 없습니다. 공식 마인크래프트 런처를 설치하고 한 번 실행한 뒤 완전히 닫고, 이 설치 도우미를 다시 실행해주세요.",
                self.minecraft_dir.display()
            ))
        }
    }
}
