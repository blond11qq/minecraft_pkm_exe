use crate::hash::verify_sha512;
use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::{
    fs,
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

pub const USER_AGENT: &str = "pixelmon-friends-setup/0.1.0";

pub fn http_client() -> Result<Client> {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(120))
        .build()
        .context("HTTP 클라이언트를 만들 수 없습니다")
}

pub fn download_to_temp(url: &str, path: &Path) -> Result<()> {
    let client = http_client()?;
    let mut response = client
        .get(url)
        .send()
        .with_context(|| format!("다운로드할 수 없습니다: {url}"))?
        .error_for_status()
        .with_context(|| format!("다운로드에 실패했습니다: {url}"))?;

    let mut file = File::create(path)
        .with_context(|| format!("임시 파일을 만들 수 없습니다: {}", path.display()))?;
    io::copy(&mut response, &mut file)
        .with_context(|| format!("임시 파일에 쓸 수 없습니다: {}", path.display()))?;
    file.flush()
        .with_context(|| format!("임시 파일 저장을 완료할 수 없습니다: {}", path.display()))?;
    Ok(())
}

pub fn atomic_download(
    url: &str,
    temp_dir: &Path,
    final_path: &Path,
    expected_sha512: Option<&str>,
) -> Result<()> {
    fs::create_dir_all(temp_dir)
        .with_context(|| format!("임시 폴더를 만들 수 없습니다: {}", temp_dir.display()))?;
    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("폴더를 만들 수 없습니다: {}", parent.display()))?;
    }

    let temp_path = temp_path_for(temp_dir, final_path);
    let _ = fs::remove_file(&temp_path);

    let result = (|| {
        download_to_temp(url, &temp_path)?;
        if let Some(expected) = expected_sha512 {
            verify_sha512(&temp_path, expected)?;
        }
        let _ = fs::remove_file(final_path);
        fs::rename(&temp_path, final_path).with_context(|| {
            format!(
                "{} 파일을 {} 위치로 옮길 수 없습니다",
                temp_path.display(),
                final_path.display()
            )
        })?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn temp_path_for(temp_dir: &Path, final_path: &Path) -> PathBuf {
    let name = final_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download.bin");
    temp_dir.join(format!("{name}.download"))
}
