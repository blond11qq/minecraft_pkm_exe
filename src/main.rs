use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const APP_NAME: &str = "Pixelmon Friends Client";
const PROFILE_NAME: &str = "Pixelmon Friends";
const PROFILE_ID: &str = "pixelmon-friends";
const OLD_PROFILE_ID_UNDERSCORE: &str = "pixelmon_friends";
const GAME_DIR_NAME: &str = ".minecraft-pixelmon-friends";
const MINECRAFT_VERSION: &str = "1.21.1";
const NEOFORGE_VERSION: &str = "21.1.219";
const NEOFORGE_VERSION_ID: &str = "neoforge-21.1.219";
const OLD_NEOFORGE_VERSION: &str = "21.1.200";
const OLD_NEOFORGE_VERSION_ID: &str = "neoforge-21.1.200";
const PIXELMON_VERSION: &str = "9.3.16";
const SERVER_NAME: &str = "뭐해 포켓몬 모드 서버";
const SERVER_ADDRESS: &str = "34.64.32.34:25565";
const USER_AGENT: &str = "PixelmonFriendsClient/0.3.2 (private friend modpack client tool)";

#[derive(Clone)]
struct Logger {
    file: Arc<Mutex<File>>,
}

impl Logger {
    fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("로그 폴더 생성 실패: {}", parent.display()))?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("로그 파일 열기 실패: {}", path.display()))?;

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }

    fn line(&self, message: impl AsRef<str>) {
        let message = message.as_ref();
        println!("{}", message);
        if let Ok(mut file) = self.file.lock() {
            let _ = writeln!(file, "{} {}", Utc::now().to_rfc3339(), message);
            let _ = file.flush();
        }
    }

    fn blank(&self) {
        self.line("");
    }
}

#[derive(Clone, Debug)]
struct ModSpec {
    project_ref: &'static str,
    display_name: &'static str,
    exact_version: Option<&'static str>,
    required: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct ModrinthVersion {
    id: String,
    project_id: String,
    name: String,
    version_number: String,
    files: Vec<ModrinthFile>,
    #[serde(default)]
    dependencies: Vec<ModrinthDependency>,
}

#[derive(Debug, Deserialize, Clone)]
struct ModrinthFile {
    url: String,
    filename: String,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    hashes: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ModrinthDependency {
    #[serde(default)]
    version_id: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
    dependency_type: String,
}

#[derive(Default)]
struct ModInstallState {
    installed_version_ids: HashSet<String>,
    installed_project_ids: HashSet<String>,
    installed_filenames: HashSet<String>,
}

fn main() {
    let log_path = default_log_path();
    let logger = match Logger::new(&log_path) {
        Ok(logger) => logger,
        Err(err) => {
            eprintln!("로그 초기화 실패: {err:#}");
            wait_for_user_exit_without_logger();
            return;
        }
    };

    let panic_logger = logger.clone();
    std::panic::set_hook(Box::new(move |panic_info| {
        panic_logger.blank();
        panic_logger.line("==================== PANIC ====================");
        panic_logger.line(format!("{panic_info}"));
        panic_logger.line("================================================");
        panic_logger.blank();
    }));

    let result = std::panic::catch_unwind(|| run(logger.clone()));

    match result {
        Ok(Ok(())) => {
            logger.blank();
            logger.line("✅ 설치/정리 완료");
            logger.line("Minecraft Launcher를 새로 열고 'Pixelmon Friends' 프로필을 실행하세요.");
            logger.line(format!("프로필 버전: {NEOFORGE_VERSION_ID}"));
            logger.line(format!("서버 주소: {SERVER_ADDRESS}"));
        }
        Ok(Err(err)) => {
            logger.blank();
            logger.line("❌ 설치 중 오류 발생");
            logger.line(format!("{err:#}"));
            logger.line(format!("로그 파일: {}", log_path.display()));
        }
        Err(_) => {
            logger.blank();
            logger.line("❌ 프로그램 내부 오류 발생. 위 panic 로그를 확인하세요.");
            logger.line(format!("로그 파일: {}", log_path.display()));
        }
    }

    wait_for_user_exit(&logger);
}

fn run(logger: Logger) -> Result<()> {
    logger.line("========================================");
    logger.line(APP_NAME);
    logger.line("========================================");
    logger.line("이 빌드는 기존 21.1.200 설치/캐시/프로필을 지우고 21.1.219로 다시 설치합니다.");
    logger.line("이 창은 자동으로 닫히지 않습니다. 끝난 뒤 q 입력 후 Enter로만 종료됩니다.");
    logger.blank();

    let appdata = appdata_roaming()?;
    let minecraft_dir = appdata.join(".minecraft");
    let game_dir = appdata.join(GAME_DIR_NAME);
    let mods_dir = game_dir.join("mods");
    let installer_cache = appdata.join(".pixelmon-friends-client-cache");

    logger.line(format!("APPDATA: {}", appdata.display()));
    logger.line(format!("Minecraft 폴더: {}", minecraft_dir.display()));
    logger.line(format!("전용 게임 폴더: {}", game_dir.display()));
    logger.line(format!("설치할 NeoForge: {NEOFORGE_VERSION_ID}"));
    logger.blank();

    fs::create_dir_all(&minecraft_dir).context(".minecraft 폴더 생성 실패")?;

    logger.line("[1/8] Minecraft Launcher / javaw 프로세스 종료 중...");
    kill_known_minecraft_processes(&logger);
    logger.blank();

    logger.line("[2/8] 기존 Pixelmon Friends 폴더/모드/크래시/캐시 제거 중...");
    clean_pixelmon_game_dir(&game_dir, &logger)?;
    fs::create_dir_all(&mods_dir).with_context(|| format!("mods 폴더 생성 실패: {}", mods_dir.display()))?;
    logger.blank();

    logger.line("[3/8] 기존 NeoForge 21.1.200/21.1.219 캐시 제거 중...");
    clean_neoforge_versions_and_libraries(&minecraft_dir, &installer_cache, &logger)?;
    logger.blank();

    logger.line("[4/8] 런처 프로필에서 기존 Pixelmon Friends / NeoForge 자동 프로필 제거 중...");
    clean_launcher_profiles(&minecraft_dir, &logger)?;
    logger.blank();

    logger.line("[5/8] Java 21 이상 확인 중...");
    check_java(&logger)?;
    logger.blank();

    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(180))
        .build()
        .context("HTTP 클라이언트 생성 실패")?;

    logger.line("[6/8] NeoForge 21.1.219 새로 설치 중...");
    fs::create_dir_all(&installer_cache)
        .with_context(|| format!("installer cache 생성 실패: {}", installer_cache.display()))?;
    install_neoforge_219(&client, &minecraft_dir, &installer_cache, &logger)?;
    logger.blank();

    logger.line("[7/8] Pixelmon / 최적화 모드 새로 다운로드 중...");
    download_mods(&client, &mods_dir, &logger)?;
    logger.blank();

    logger.line("[8/8] 런처 프로필/서버 메모를 21.1.219로 새로 생성 중...");
    logger.line("NeoForge installer가 만든 일반 NeoForge 프로필은 제거하고 Pixelmon Friends만 남깁니다.");
    write_launcher_profiles(&minecraft_dir, &game_dir, &logger)?;
    write_server_note(&game_dir, &logger)?;
    verify_profile_version(&minecraft_dir, &logger)?;
    logger.blank();

    logger.line("완료. 이제 Minecraft Launcher를 새로 열고 Pixelmon Friends를 실행하세요.");
    logger.line(format!("정상 로그 첫 줄에는 --version, {NEOFORGE_VERSION_ID} 가 떠야 합니다."));

    Ok(())
}

fn default_log_path() -> PathBuf {
    match env::var_os("APPDATA") {
        Some(appdata) => PathBuf::from(appdata)
            .join(".pixelmon-friends-client-logs")
            .join("installer.log"),
        None => PathBuf::from("PixelmonFriendsClient.log"),
    }
}

fn appdata_roaming() -> Result<PathBuf> {
    env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("APPDATA 환경 변수를 찾을 수 없습니다. Windows에서 실행해 주세요."))
}

fn kill_known_minecraft_processes(logger: &Logger) {
    for process in ["MinecraftLauncher.exe", "Minecraft.exe", "javaw.exe", "java.exe"] {
        let status = Command::new("taskkill")
            .args(["/IM", process, "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match status {
            Ok(status) if status.success() => logger.line(format!("종료됨: {process}")),
            Ok(_) => logger.line(format!("실행 중 아님 또는 종료 불필요: {process}")),
            Err(_) => logger.line(format!("taskkill 사용 불가 또는 Windows가 아님: {process}")),
        }
    }
}

fn check_java(logger: &Logger) -> Result<()> {
    let output = Command::new("java")
        .arg("-version")
        .output()
        .context("Java를 찾을 수 없습니다. Java 21을 설치한 뒤 다시 실행하세요. 예: winget install -e --id Microsoft.OpenJDK.21")?;

    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));

    logger.line(text.trim());

    if !output.status.success() {
        bail!("java -version 실행 실패. Java 21을 설치한 뒤 다시 실행하세요.");
    }

    if !looks_like_java_21_or_newer(&text) {
        bail!("Java는 있지만 21 이상으로 보이지 않습니다. Minecraft 1.21.1/NeoForge용 Java 21을 설치하세요.");
    }

    Ok(())
}

fn looks_like_java_21_or_newer(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    for major in 21..=40 {
        if text.contains(&format!("version \"{major}"))
            || text.contains(&format!("openjdk {major}"))
            || text.contains(&format!("java version {major}"))
        {
            return true;
        }
    }
    false
}

fn clean_pixelmon_game_dir(game_dir: &Path, logger: &Logger) -> Result<()> {
    if game_dir.exists() {
        logger.line(format!("삭제: {}", game_dir.display()));
        remove_path_aggressive(game_dir).with_context(|| format!("전용 게임 폴더 삭제 실패: {}", game_dir.display()))?;
    }
    Ok(())
}

fn clean_neoforge_versions_and_libraries(minecraft_dir: &Path, installer_cache: &Path, logger: &Logger) -> Result<()> {
    let targets = vec![
        minecraft_dir.join("versions").join(OLD_NEOFORGE_VERSION_ID),
        minecraft_dir.join("versions").join(NEOFORGE_VERSION_ID),
        minecraft_dir.join("libraries").join("net").join("neoforged").join("neoforge").join(OLD_NEOFORGE_VERSION),
        minecraft_dir.join("libraries").join("net").join("neoforged").join("neoforge").join(NEOFORGE_VERSION),
        installer_cache.to_path_buf(),
    ];

    for target in targets {
        if target.exists() {
            logger.line(format!("삭제: {}", target.display()));
            remove_path_aggressive(&target).with_context(|| format!("삭제 실패: {}", target.display()))?;
        } else {
            logger.line(format!("없음: {}", target.display()));
        }
    }

    Ok(())
}

fn remove_path_aggressive(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if cfg!(windows) {
        if path.is_dir() {
            let status = Command::new("cmd")
                .args(["/C", "rmdir", "/S", "/Q"])
                .arg(path)
                .status();
            if let Ok(status) = status {
                if status.success() || !path.exists() {
                    return Ok(());
                }
            }
        } else {
            let status = Command::new("cmd")
                .args(["/C", "del", "/F", "/Q"])
                .arg(path)
                .status();
            if let Ok(status) = status {
                if status.success() || !path.exists() {
                    return Ok(());
                }
            }
        }
    }

    clear_readonly_recursive(path).ok();
    if path.is_dir() {
        fs::remove_dir_all(path).with_context(|| format!("remove_dir_all 실패: {}", path.display()))?;
    } else {
        fs::remove_file(path).with_context(|| format!("remove_file 실패: {}", path.display()))?;
    }
    Ok(())
}

fn clear_readonly_recursive(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let metadata = fs::metadata(path)?;
    let mut permissions = metadata.permissions();
    if permissions.readonly() {
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)?;
    }

    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            clear_readonly_recursive(&entry.path()).ok();
        }
    }

    Ok(())
}

fn install_neoforge_219(client: &Client, minecraft_dir: &Path, installer_cache: &Path, logger: &Logger) -> Result<()> {
    let version_json = minecraft_dir
        .join("versions")
        .join(NEOFORGE_VERSION_ID)
        .join(format!("{NEOFORGE_VERSION_ID}.json"));

    if version_json.exists() {
        logger.line(format!("이미 설치 확인됨: {}", version_json.display()));
        return Ok(());
    }

    let installer_name = format!("neoforge-{NEOFORGE_VERSION}-installer.jar");
    let installer_url = format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{NEOFORGE_VERSION}/{installer_name}"
    );
    let installer_path = installer_cache.join(&installer_name);

    logger.line(format!("다운로드: {installer_url}"));
    download_file(client, &installer_url, &installer_path, logger)
        .context("NeoForge installer 다운로드 실패")?;

    let cli_attempts: &[&[&str]] = &[
        &["-jar", "__INSTALLER__", "--install-client"],
        &["-jar", "__INSTALLER__", "--installClient"],
        &["-jar", "__INSTALLER__", "--install", "client"],
    ];

    for args_template in cli_attempts {
        let mut args: Vec<String> = Vec::new();
        for arg in *args_template {
            if *arg == "__INSTALLER__" {
                args.push(installer_path.to_string_lossy().to_string());
            } else {
                args.push((*arg).to_string());
            }
        }

        logger.line(format!("실행: java {}", args.join(" ")));
        let status = Command::new("java")
            .args(&args)
            .status()
            .with_context(|| format!("NeoForge installer CLI 실행 실패: java {}", args.join(" ")))?;

        if status.success() && version_json.exists() {
            logger.line(format!("NeoForge 설치 완료: {NEOFORGE_VERSION_ID}"));
            return Ok(());
        }

        logger.line(format!("CLI 시도 실패 또는 확인 실패: exit={:?}", status.code()));
    }

    logger.blank();
    logger.line("자동 CLI 설치가 실패했습니다.");
    logger.line("이제 NeoForge GUI installer를 열겠습니다.");
    logger.line("GUI에서 Install client → Proceed → 완료 후 이 콘솔에 done 입력.");

    Command::new("javaw")
        .arg("-jar")
        .arg(&installer_path)
        .spawn()
        .or_else(|_| Command::new("java").arg("-jar").arg(&installer_path).spawn())
        .context("NeoForge GUI installer 실행 실패")?;

    wait_for_done(logger, "NeoForge GUI 설치가 끝났으면 done 입력 후 Enter: ")?;

    if version_json.exists() {
        logger.line(format!("NeoForge 설치 확인 완료: {NEOFORGE_VERSION_ID}"));
        return Ok(());
    }

    bail!(
        "NeoForge 설치 확인 실패: {} 가 없습니다. GUI에서 Install client를 완료했는지 확인하세요.",
        version_json.display()
    );
}

fn download_mods(client: &Client, mods_dir: &Path, logger: &Logger) -> Result<()> {
    let specs = vec![
        ModSpec {
            project_ref: "pixelmon",
            display_name: "Pixelmon",
            exact_version: Some(PIXELMON_VERSION),
            required: true,
        },
        ModSpec {
            project_ref: "sodium",
            display_name: "Sodium",
            exact_version: Some("mc1.21.1-0.6.13-neoforge"),
            required: true,
        },
        ModSpec {
            project_ref: "iris",
            display_name: "Iris Shaders",
            exact_version: Some("1.8.12+1.21.1-neoforge"),
            required: false,
        },
        ModSpec {
            project_ref: "wi-zoom",
            display_name: "WI Zoom",
            exact_version: Some("1.6-MC1.21.1-NeoForge"),
            required: false,
        },
        ModSpec {
            project_ref: "modernfix",
            display_name: "ModernFix",
            exact_version: None,
            required: true,
        },
        ModSpec {
            project_ref: "ferrite-core",
            display_name: "FerriteCore",
            exact_version: None,
            required: true,
        },
        ModSpec {
            project_ref: "lithium",
            display_name: "Lithium",
            exact_version: None,
            required: false,
        },
        ModSpec {
            project_ref: "clumps",
            display_name: "Clumps",
            exact_version: None,
            required: false,
        },
        ModSpec {
            project_ref: "entityculling",
            display_name: "Entity Culling",
            exact_version: None,
            required: false,
        },
        ModSpec {
            project_ref: "immediatelyfast",
            display_name: "ImmediatelyFast",
            exact_version: None,
            required: false,
        },
        ModSpec {
            project_ref: "dynamic-fps",
            display_name: "Dynamic FPS",
            exact_version: None,
            required: false,
        },
    ];

    let mut state = ModInstallState::default();

    for spec in specs {
        logger.line(format!("모드 처리: {}", spec.display_name));
        match install_project_version(client, mods_dir, logger, &spec, &mut state) {
            Ok(()) => {}
            Err(err) if spec.required => return Err(err).with_context(|| format!("필수 모드 실패: {}", spec.display_name)),
            Err(err) => logger.line(format!("선택 모드 건너뜀: {} ({err:#})", spec.display_name)),
        }
    }

    Ok(())
}

fn install_project_version(
    client: &Client,
    mods_dir: &Path,
    logger: &Logger,
    spec: &ModSpec,
    state: &mut ModInstallState,
) -> Result<()> {
    let versions = query_modrinth_versions(client, spec.project_ref)?;
    let selected = select_version(&versions, spec.exact_version).ok_or_else(|| {
        anyhow!(
            "Modrinth에서 Minecraft {MINECRAFT_VERSION} + NeoForge용 버전을 찾지 못했습니다: {}",
            spec.project_ref
        )
    })?;

    install_modrinth_version(client, mods_dir, logger, selected, state)
}

fn query_modrinth_versions(client: &Client, project_ref: &str) -> Result<Vec<ModrinthVersion>> {
    let url = format!(
        "https://api.modrinth.com/v2/project/{project_ref}/version?loaders=%5B%22neoforge%22%5D&game_versions=%5B%22{MINECRAFT_VERSION}%22%5D"
    );

    let versions = client
        .get(url)
        .send()
        .context("Modrinth version query 실패")?
        .error_for_status()
        .context("Modrinth version query HTTP 오류")?
        .json::<Vec<ModrinthVersion>>()
        .context("Modrinth version 응답 파싱 실패")?;

    Ok(versions)
}

fn query_modrinth_version_by_id(client: &Client, version_id: &str) -> Result<ModrinthVersion> {
    let url = format!("https://api.modrinth.com/v2/version/{version_id}");
    let version = client
        .get(url)
        .send()
        .context("Modrinth version-id query 실패")?
        .error_for_status()
        .context("Modrinth version-id query HTTP 오류")?
        .json::<ModrinthVersion>()
        .context("Modrinth version-id 응답 파싱 실패")?;
    Ok(version)
}

fn select_version(versions: &[ModrinthVersion], exact_version: Option<&str>) -> Option<ModrinthVersion> {
    if let Some(exact) = exact_version {
        versions
            .iter()
            .find(|version| version.version_number == exact)
            .cloned()
    } else {
        versions.first().cloned()
    }
}

fn install_modrinth_version(
    client: &Client,
    mods_dir: &Path,
    logger: &Logger,
    version: ModrinthVersion,
    state: &mut ModInstallState,
) -> Result<()> {
    if state.installed_version_ids.contains(&version.id) || state.installed_project_ids.contains(&version.project_id) {
        logger.line(format!("이미 처리됨: {} {}", version.name, version.version_number));
        return Ok(());
    }

    let file = version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .cloned()
        .ok_or_else(|| anyhow!("download file not found for {} {}", version.name, version.version_number))?;

    if !file.filename.to_ascii_lowercase().ends_with(".jar") {
        bail!("primary file is not a jar: {}", file.filename);
    }

    logger.line(format!(
        "다운로드: {} {} -> {}",
        version.name, version.version_number, file.filename
    ));

    if !state.installed_filenames.contains(&file.filename) {
        let dest = mods_dir.join(&file.filename);
        download_file(client, &file.url, &dest, logger)
            .with_context(|| format!("모드 파일 다운로드 실패: {}", file.filename))?;

        if let Some(sha1) = file.hashes.get("sha1") {
            logger.line(format!("sha1: {sha1}"));
        }

        state.installed_filenames.insert(file.filename.clone());
    }

    state.installed_version_ids.insert(version.id.clone());
    state.installed_project_ids.insert(version.project_id.clone());

    for dep in version.dependencies.iter() {
        if dep.dependency_type != "required" {
            continue;
        }

        if let Some(version_id) = &dep.version_id {
            if state.installed_version_ids.contains(version_id) {
                continue;
            }
            logger.line(format!("필수 의존성 처리(version): {version_id}"));
            let dep_version = query_modrinth_version_by_id(client, version_id)?;
            install_modrinth_version(client, mods_dir, logger, dep_version, state)?;
        } else if let Some(project_id) = &dep.project_id {
            if state.installed_project_ids.contains(project_id) {
                continue;
            }
            logger.line(format!("필수 의존성 처리(project): {project_id}"));
            let dep_versions = query_modrinth_versions(client, project_id)?;
            let dep_version = dep_versions.first().cloned().ok_or_else(|| {
                anyhow!("required dependency has no compatible NeoForge version: {project_id}")
            })?;
            install_modrinth_version(client, mods_dir, logger, dep_version, state)?;
        }
    }

    Ok(())
}

fn download_file(client: &Client, url: &str, dest: &Path, logger: &Logger) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).with_context(|| format!("폴더 생성 실패: {}", parent.display()))?;
    }

    let mut response = client
        .get(url)
        .send()
        .with_context(|| format!("GET 실패: {url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP 오류: {url}"))?;

    let tmp = dest.with_extension("download-tmp");
    let mut out = File::create(&tmp).with_context(|| format!("임시 파일 생성 실패: {}", tmp.display()))?;
    let bytes = response
        .copy_to(&mut out)
        .with_context(|| format!("다운로드 저장 실패: {}", tmp.display()))?;
    out.flush().ok();
    drop(out);

    if dest.exists() {
        remove_path_aggressive(dest).ok();
    }
    fs::rename(&tmp, dest).with_context(|| format!("임시 파일 이동 실패: {}", dest.display()))?;
    logger.line(format!("저장 완료: {} ({} bytes)", dest.display(), bytes));
    Ok(())
}

fn clean_launcher_profiles(minecraft_dir: &Path, logger: &Logger) -> Result<()> {
    for profile_path in launcher_profile_paths(minecraft_dir) {
        if profile_path.exists() {
            logger.line(format!("기존 프로필 정리: {}", profile_path.display()));
            update_launcher_profile_file(&profile_path, None, logger)?;
        }
    }
    Ok(())
}

fn write_launcher_profiles(minecraft_dir: &Path, game_dir: &Path, logger: &Logger) -> Result<()> {
    for profile_path in launcher_profile_paths(minecraft_dir) {
        logger.line(format!("프로필 쓰기: {}", profile_path.display()));
        update_launcher_profile_file(&profile_path, Some(game_dir), logger)?;
    }
    Ok(())
}

fn launcher_profile_paths(minecraft_dir: &Path) -> Vec<PathBuf> {
    vec![
        minecraft_dir.join("launcher_profiles.json"),
        minecraft_dir.join("launcher_profiles_microsoft_store.json"),
    ]
}

fn update_launcher_profile_file(profile_path: &Path, game_dir: Option<&Path>, logger: &Logger) -> Result<()> {
    if let Some(parent) = profile_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("프로필 폴더 생성 실패: {}", parent.display()))?;
    }

    let mut root: Value = if profile_path.exists() {
        let mut text = String::new();
        File::open(profile_path)
            .with_context(|| format!("프로필 파일 열기 실패: {}", profile_path.display()))?
            .read_to_string(&mut text)
            .with_context(|| format!("프로필 파일 읽기 실패: {}", profile_path.display()))?;
        serde_json::from_str(&text).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    if !root.is_object() {
        root = json!({});
    }

    if profile_path.exists() {
        let backup_path = profile_path.with_extension(format!(
            "json.bak-{}",
            Utc::now().format("%Y%m%d-%H%M%S")
        ));
        fs::copy(profile_path, &backup_path).with_context(|| {
            format!(
                "프로필 백업 실패: {} -> {}",
                profile_path.display(),
                backup_path.display()
            )
        })?;
        logger.line(format!("백업 생성: {}", backup_path.display()));
    }

    let obj = root.as_object_mut().ok_or_else(|| anyhow!("launcher profile root가 object가 아님"))?;
    if !obj.contains_key("profiles") || !obj.get("profiles").map(|v| v.is_object()).unwrap_or(false) {
        obj.insert("profiles".to_string(), json!({}));
    }

    let profiles = obj
        .get_mut("profiles")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| anyhow!("profiles object 없음"))?;

    remove_managed_launcher_profiles(profiles);

    if let Some(game_dir) = game_dir {
        let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        profiles.insert(
            PROFILE_ID.to_string(),
            json!({
                "created": now,
                "gameDir": game_dir.to_string_lossy().to_string(),
                "icon": "Grass",
                "javaArgs": "-Xmx6144M -XX:+UseG1GC",
                "lastUsed": now,
                "lastVersionId": NEOFORGE_VERSION_ID,
                "name": PROFILE_NAME,
                "type": "custom"
            }),
        );
    }

    if !obj.contains_key("settings") {
        obj.insert("settings".to_string(), json!({}));
    }

    let pretty = serde_json::to_string_pretty(&root).context("프로필 JSON 직렬화 실패")?;
    fs::write(profile_path, pretty).with_context(|| format!("프로필 파일 쓰기 실패: {}", profile_path.display()))?;

    Ok(())
}

fn remove_managed_launcher_profiles(profiles: &mut Map<String, Value>) {
    let keys_to_remove: Vec<String> = profiles
        .iter()
        .filter_map(|(key, value)| {
            let profile_name = value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("");
            let last_version_id = value
                .get("lastVersionId")
                .and_then(Value::as_str)
                .unwrap_or("");
            let game_dir = value
                .get("gameDir")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();

            let is_pixelmon_friends_profile = profile_name.eq_ignore_ascii_case(PROFILE_NAME)
                || key == PROFILE_ID
                || key == OLD_PROFILE_ID_UNDERSCORE
                || ((last_version_id == OLD_NEOFORGE_VERSION_ID || last_version_id == NEOFORGE_VERSION_ID)
                    && game_dir.contains(GAME_DIR_NAME));

            let is_neoforge_auto_profile = profile_name.eq_ignore_ascii_case("NeoForge")
                && (last_version_id == OLD_NEOFORGE_VERSION_ID || last_version_id == NEOFORGE_VERSION_ID);

            if is_pixelmon_friends_profile || is_neoforge_auto_profile {
                Some(key.clone())
            } else {
                None
            }
        })
        .collect();

    for key in keys_to_remove {
        profiles.remove(&key);
    }
}

fn verify_profile_version(minecraft_dir: &Path, logger: &Logger) -> Result<()> {
    for profile_path in launcher_profile_paths(minecraft_dir) {
        if !profile_path.exists() {
            continue;
        }

        let text = fs::read_to_string(&profile_path)
            .with_context(|| format!("프로필 검증 읽기 실패: {}", profile_path.display()))?;
        let root: Value = serde_json::from_str(&text)
            .with_context(|| format!("프로필 검증 JSON 파싱 실패: {}", profile_path.display()))?;

        let profiles = root
            .get("profiles")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("프로필 검증 실패: profiles object 없음"))?;

        let profile = profiles
            .get(PROFILE_ID)
            .ok_or_else(|| anyhow!("프로필 검증 실패: {} 안에 {} 프로필이 없습니다.", profile_path.display(), PROFILE_ID))?;

        let version = profile
            .get("lastVersionId")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("프로필 검증 실패: lastVersionId 없음"))?;

        if version != NEOFORGE_VERSION_ID {
            bail!(
                "프로필 검증 실패: {} lastVersionId가 {} 입니다. 기대값: {}",
                profile_path.display(),
                version,
                NEOFORGE_VERSION_ID
            );
        }

        for (key, value) in profiles.iter() {
            let profile_name = value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("");
            let last_version_id = value
                .get("lastVersionId")
                .and_then(Value::as_str)
                .unwrap_or("");

            if profile_name.eq_ignore_ascii_case("NeoForge")
                && (last_version_id == OLD_NEOFORGE_VERSION_ID || last_version_id == NEOFORGE_VERSION_ID)
            {
                bail!(
                    "프로필 검증 실패: {} 안에 일반 NeoForge 프로필({})이 남아 있습니다.",
                    profile_path.display(),
                    key
                );
            }
        }

        logger.line(format!("검증 OK: {} -> {} / 일반 NeoForge 프로필 제거됨", profile_path.display(), NEOFORGE_VERSION_ID));
    }
    Ok(())
}

fn write_server_note(game_dir: &Path, logger: &Logger) -> Result<()> {
    fs::create_dir_all(game_dir).with_context(|| format!("게임 폴더 생성 실패: {}", game_dir.display()))?;
    let note = format!(
        "{SERVER_NAME}\n\nServer address:\n{SERVER_ADDRESS}\n\nLauncher profile:\n{PROFILE_NAME}\n\nMinecraft: {MINECRAFT_VERSION}\nNeoForge: {NEOFORGE_VERSION}\nPixelmon: {PIXELMON_VERSION}\n"
    );
    let path = game_dir.join("SERVER_ADDRESS.txt");
    fs::write(&path, note).with_context(|| format!("서버 주소 파일 쓰기 실패: {}", path.display()))?;
    logger.line(format!("서버 주소 파일: {}", path.display()));
    Ok(())
}

fn wait_for_done(logger: &Logger, prompt: &str) -> Result<()> {
    loop {
        logger.line(prompt);
        let mut input = String::new();
        io::stdin().read_line(&mut input).context("입력 읽기 실패")?;
        let normalized = input.trim().to_ascii_lowercase();
        if normalized == "done" || normalized == "완료" || normalized == "d" {
            return Ok(());
        }
        logger.line("아직 계속 대기합니다. 완료했으면 done 입력 후 Enter.");
    }
}

fn wait_for_user_exit(logger: &Logger) {
    logger.blank();
    logger.line("========================================");
    logger.line("자동 종료 방지 대기 중");
    logger.line("종료하려면 q 입력 후 Enter를 누르세요.");
    logger.line("그 외 입력/빈 Enter는 계속 대기합니다.");
    logger.line("========================================");

    loop {
        print!("exit? q + Enter: ");
        let _ = io::stdout().flush();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            continue;
        }
        let normalized = input.trim().to_ascii_lowercase();
        if normalized == "q" || normalized == "quit" || normalized == "exit" || normalized == "종료" {
            break;
        }
        logger.line("계속 대기합니다. 종료하려면 q 입력 후 Enter.");
    }
}

fn wait_for_user_exit_without_logger() {
    eprintln!("종료하려면 q 입력 후 Enter를 누르세요.");
    loop {
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let normalized = input.trim().to_ascii_lowercase();
            if normalized == "q" || normalized == "quit" || normalized == "exit" || normalized == "종료" {
                break;
            }
        }
    }
}
