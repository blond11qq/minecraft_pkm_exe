use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha512};
use std::{fs::File, io::Read, path::Path};

pub fn sha512_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)
        .with_context(|| format!("해시 확인을 위해 파일을 열 수 없습니다: {}", path.display()))?;
    let mut hasher = Sha512::new();
    let mut buf = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buf).with_context(|| {
            format!(
                "해시 확인을 위해 파일을 읽을 수 없습니다: {}",
                path.display()
            )
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn verify_sha512(path: &Path, expected: &str) -> Result<()> {
    let actual = sha512_file(path)?;
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(anyhow!(
            "{} 파일의 해시 검증에 실패했습니다. 예상 sha512: {expected}, 실제 sha512: {actual}.",
            path.display()
        ))
    }
}
