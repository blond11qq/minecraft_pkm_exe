use crate::{launcher, manifest::Manifest, paths::InstallPaths};
use anyhow::{anyhow, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn upsert_launcher_profile(manifest: &Manifest, paths: &InstallPaths) -> Result<PathBuf> {
    let profile_path = find_launcher_profile(paths)?;
    backup_profile(&profile_path, &paths.backups_dir)?;
    upsert_launcher_profile_file(&profile_path, manifest, &paths.game_dir)?;
    launcher::print_installed_message("런처 프로필");
    Ok(profile_path)
}

fn find_launcher_profile(paths: &InstallPaths) -> Result<PathBuf> {
    paths
        .launcher_profile_candidates
        .iter()
        .find(|path| path.is_file())
        .cloned()
        .ok_or_else(|| {
            anyhow!(
                "launcher_profiles.json 파일을 찾을 수 없습니다. 공식 마인크래프트 런처를 한 번 실행한 뒤 완전히 닫고, 이 설치 도우미를 다시 실행해주세요."
            )
        })
}

fn backup_profile(profile_path: &Path, backups_dir: &Path) -> Result<()> {
    fs::create_dir_all(backups_dir)
        .with_context(|| format!("백업 폴더를 만들 수 없습니다: {}", backups_dir.display()))?;
    let base = profile_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("launcher_profiles.json");
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let backup_path = backups_dir.join(format!("{base}.backup-{timestamp}"));
    fs::copy(profile_path, &backup_path).with_context(|| {
        format!(
            "{} 파일을 {} 위치로 백업할 수 없습니다",
            profile_path.display(),
            backup_path.display()
        )
    })?;
    Ok(())
}

pub fn upsert_launcher_profile_file(
    profile_path: &Path,
    manifest: &Manifest,
    game_dir: &Path,
) -> Result<()> {
    let data = fs::read_to_string(profile_path)
        .with_context(|| format!("파일을 읽을 수 없습니다: {}", profile_path.display()))?;
    let mut root: Value = serde_json::from_str(&data)
        .with_context(|| format!("JSON을 해석할 수 없습니다: {}", profile_path.display()))?;
    upsert_launcher_profile_value(&mut root, manifest, game_dir)?;

    let parent = profile_path
        .parent()
        .ok_or_else(|| anyhow!("런처 프로필 경로의 상위 폴더를 찾을 수 없습니다"))?;
    let temp_path = parent.join(format!(
        ".{}.tmp",
        profile_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("launcher_profiles.json")
    ));
    let pretty = serde_json::to_string_pretty(&root)
        .context("런처 프로필 JSON을 저장 형식으로 변환할 수 없습니다")?;
    fs::write(&temp_path, pretty)
        .with_context(|| format!("임시 프로필 파일을 쓸 수 없습니다: {}", temp_path.display()))?;
    fs::rename(&temp_path, profile_path).with_context(|| {
        format!(
            "{} 파일을 업데이트된 런처 프로필로 교체할 수 없습니다",
            profile_path.display()
        )
    })?;
    Ok(())
}

pub fn upsert_launcher_profile_value(
    root: &mut Value,
    manifest: &Manifest,
    game_dir: &Path,
) -> Result<()> {
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    if !root.is_object() {
        *root = json!({});
    }

    let root_obj = root.as_object_mut().expect("root was just made an object");
    let profiles_value = root_obj
        .entry("profiles".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !profiles_value.is_object() {
        *profiles_value = Value::Object(Map::new());
    }

    let profiles = profiles_value
        .as_object_mut()
        .expect("profiles was just made an object");
    let existing_created = profiles
        .get(&manifest.profile_id)
        .and_then(|profile| profile.get("created"))
        .cloned()
        .unwrap_or_else(|| Value::String(now.clone()));

    remove_neoforge_installer_profiles(profiles, manifest);

    profiles.insert(
        manifest.profile_id.clone(),
        json!({
            "name": manifest.profile_name,
            "type": "custom",
            "created": existing_created,
            "lastUsed": now,
            "lastVersionId": manifest.neoforge_version_id,
            "gameDir": game_dir.to_string_lossy().to_string(),
            "javaArgs": format!("-Xmx{}M -XX:+UseG1GC", manifest.ram_mb),
            "launcherVisibilityOnGameClose": "keep the launcher open"
        }),
    );
    Ok(())
}

fn remove_neoforge_installer_profiles(profiles: &mut Map<String, Value>, manifest: &Manifest) {
    let neoforge_version_id = manifest.neoforge_version_id.to_ascii_lowercase();
    let neoforge_name = format!("neoforge {}", manifest.neoforge_version).to_ascii_lowercase();

    profiles.retain(|profile_id, profile| {
        if profile_id == &manifest.profile_id {
            return true;
        }

        let profile_id_lower = profile_id.to_ascii_lowercase();
        if profile_id_lower == neoforge_version_id {
            return false;
        }

        let name_lower = profile
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase();
        let last_version_id_lower = profile
            .get("lastVersionId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase();

        let is_neoforge_name = name_lower == neoforge_version_id || name_lower == neoforge_name;
        let is_neoforge_profile =
            last_version_id_lower == neoforge_version_id && name_lower.contains("neoforge");

        !(is_neoforge_name || is_neoforge_profile)
    });
}

#[cfg(test)]
mod tests {
    use super::{upsert_launcher_profile_file, upsert_launcher_profile_value};
    use crate::manifest::load_manifest;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn preserves_unrelated_profiles_and_adds_pixelmon() {
        let manifest = load_manifest();
        let game_dir = PathBuf::from(r"C:\Users\Test\AppData\Roaming\.minecraft-pixelmon-friends");
        let mut root = json!({
            "clientToken": "leave-me-alone",
            "profiles": {
                "vanilla": {
                    "name": "Vanilla",
                    "lastVersionId": "1.21.1"
                }
            }
        });

        upsert_launcher_profile_value(&mut root, &manifest, &game_dir).unwrap();

        assert_eq!(root["clientToken"], "leave-me-alone");
        assert_eq!(root["profiles"]["vanilla"]["name"], "Vanilla");
        assert_eq!(
            root["profiles"]["pixelmon-friends"]["name"],
            "Pixelmon Friends"
        );
        assert_eq!(
            root["profiles"]["pixelmon-friends"]["lastVersionId"],
            "neoforge-21.1.200"
        );
    }

    #[test]
    fn keeps_existing_created_field_when_updating() {
        let manifest = load_manifest();
        let game_dir = PathBuf::from(r"C:\GameDir");
        let mut root = json!({
            "profiles": {
                "pixelmon-friends": {
                    "created": "2024-01-01T00:00:00.000Z",
                    "name": "Old"
                }
            }
        });

        upsert_launcher_profile_value(&mut root, &manifest, &game_dir).unwrap();

        assert_eq!(
            root["profiles"]["pixelmon-friends"]["created"],
            "2024-01-01T00:00:00.000Z"
        );
        assert_eq!(
            root["profiles"]["pixelmon-friends"]["name"],
            "Pixelmon Friends"
        );
    }

    #[test]
    fn upserts_temp_json_file() {
        let manifest = load_manifest();
        let temp = tempfile::tempdir().unwrap();
        let profile_path = temp.path().join("launcher_profiles.json");
        let game_dir = temp.path().join(".minecraft-pixelmon-friends");
        fs::write(
            &profile_path,
            r#"{"profiles":{"vanilla":{"name":"Vanilla"}},"settings":{"keep":true}}"#,
        )
        .unwrap();

        upsert_launcher_profile_file(&profile_path, &manifest, &game_dir).unwrap();

        let updated: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&profile_path).unwrap()).unwrap();
        assert_eq!(updated["profiles"]["vanilla"]["name"], "Vanilla");
        assert_eq!(
            updated["profiles"]["pixelmon-friends"]["gameDir"],
            game_dir.to_string_lossy().to_string()
        );
        assert_eq!(updated["settings"]["keep"], true);
    }

    #[test]
    fn removes_neoforge_installer_profile_but_keeps_pixelmon() {
        let manifest = load_manifest();
        let game_dir = PathBuf::from(r"C:\GameDir");
        let mut root = json!({
            "profiles": {
                "neoforge-21.1.200": {
                    "name": "NeoForge 21.1.200",
                    "lastVersionId": "neoforge-21.1.200"
                },
                "vanilla": {
                    "name": "Vanilla",
                    "lastVersionId": "1.21.1"
                }
            }
        });

        upsert_launcher_profile_value(&mut root, &manifest, &game_dir).unwrap();

        assert!(root["profiles"]["neoforge-21.1.200"].is_null());
        assert_eq!(root["profiles"]["vanilla"]["name"], "Vanilla");
        assert_eq!(
            root["profiles"]["pixelmon-friends"]["lastVersionId"],
            "neoforge-21.1.200"
        );
    }
}
