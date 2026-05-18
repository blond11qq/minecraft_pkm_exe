use anyhow::{anyhow, Context, Result};
use std::process::Command;

pub fn ensure_java(required_major: u32) -> Result<()> {
    let output = Command::new("java")
        .arg("-version")
        .output()
        .context("Java를 찾을 수 없습니다. NeoForge 설치 프로그램을 실행하려면 Java 21이 필요합니다. Java 21을 설치한 뒤 다시 실행해주세요.")?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}\n{stdout}");
    let major = parse_java_major_version(&combined).ok_or_else(|| {
        anyhow!("Java 버전을 확인할 수 없습니다. NeoForge 설치 프로그램을 실행하려면 Java 21이 필요합니다. Java 21을 설치한 뒤 다시 실행해주세요.")
    })?;

    if major >= required_major {
        Ok(())
    } else {
        Err(anyhow!(
            "Java {major}이(가) 감지되었지만, NeoForge 설치 프로그램을 실행하려면 Java {required_major} 이상이 필요합니다. Java {required_major}을 설치한 뒤 다시 실행해주세요."
        ))
    }
}

pub fn parse_java_major_version(output: &str) -> Option<u32> {
    for line in output.lines() {
        let Some(start) = line.find('"') else {
            continue;
        };
        let rest = &line[start + 1..];
        let Some(end) = rest.find('"') else {
            continue;
        };
        let version = &rest[..end];
        let first = version.split('.').next()?;
        if first == "1" {
            return version.split('.').nth(1)?.parse().ok();
        }
        return first.parse().ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::parse_java_major_version;

    #[test]
    fn parses_openjdk_21() {
        let output = r#"openjdk version "21.0.4" 2024-07-16"#;
        assert_eq!(parse_java_major_version(output), Some(21));
    }

    #[test]
    fn parses_java_17() {
        let output = r#"java version "17.0.10" 2024-01-16 LTS"#;
        assert_eq!(parse_java_major_version(output), Some(17));
    }

    #[test]
    fn invalid_output_is_none() {
        assert_eq!(parse_java_major_version("not java output"), None);
    }
}
