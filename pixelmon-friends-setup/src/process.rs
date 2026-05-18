use anyhow::{anyhow, Result};
use sysinfo::System;

pub fn is_minecraft_launcher_running() -> bool {
    let mut system = System::new_all();
    system.refresh_processes();

    system.processes().values().any(|process| {
        let name = process.name().to_ascii_lowercase();
        name.contains("minecraft") && name.contains("launcher")
    })
}

pub fn ensure_launcher_not_running() -> Result<()> {
    if is_minecraft_launcher_running() {
        Err(anyhow!(
            "마인크래프트 런처가 실행 중입니다. 런처를 완전히 닫은 뒤 픽셀몬 프렌즈 설치 도우미를 다시 실행해주세요."
        ))
    } else {
        Ok(())
    }
}
