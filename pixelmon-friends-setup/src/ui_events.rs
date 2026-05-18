use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{mpsc::Sender, Mutex, OnceLock},
};

#[derive(Clone, Debug)]
pub enum InstallEvent {
    Log(String),
    Progress { value: f32, message: String },
    Finished,
    Failed(String),
}

static SENDER: OnceLock<Mutex<Sender<InstallEvent>>> = OnceLock::new();
static LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn init_log_file() -> Option<PathBuf> {
    let path = diagnostic_log_path();
    if let Some(parent) = path.parent() {
        let _ignored = fs::create_dir_all(parent);
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()?;

    if LOG_FILE.set(Mutex::new(file)).is_ok() {
        let _ignored = LOG_PATH.set(path.clone());
        diagnostic(format!("=== Pixelmon Friends started: {} ===", timestamp()));
    }

    Some(path)
}

pub fn current_log_path() -> Option<PathBuf> {
    LOG_PATH.get().cloned()
}

pub fn diagnostic(message: impl AsRef<str>) {
    write_log_line(message.as_ref());
}

pub fn set_sender(sender: Sender<InstallEvent>) {
    if SENDER.set(Mutex::new(sender)).is_err() {
        log("GUI 로그 연결을 이미 사용 중입니다.");
    }
}

pub fn log(message: impl Into<String>) {
    let message = message.into();
    write_log_line(&message);

    if let Some(sender) = SENDER.get() {
        if let Ok(sender) = sender.lock() {
            let _ignored = sender.send(InstallEvent::Log(message));
            return;
        }
    }

    println!("{message}");
}

pub fn progress(value: f32, message: impl Into<String>) {
    let message = message.into();
    write_log_line(&format!("[{:.0}%] {message}", value.clamp(0.0, 1.0) * 100.0));

    if let Some(sender) = SENDER.get() {
        if let Ok(sender) = sender.lock() {
            let _ignored = sender.send(InstallEvent::Progress { value, message });
            return;
        }
    }

    println!("{message}");
}

pub fn finished() {
    write_log_line("Installation finished successfully.");
    if let Some(sender) = SENDER.get() {
        if let Ok(sender) = sender.lock() {
            let _ignored = sender.send(InstallEvent::Finished);
        }
    }
}

pub fn failed(error: impl Into<String>) {
    let error = error.into();
    write_log_line(&format!("Installation failed: {error}"));

    if let Some(sender) = SENDER.get() {
        if let Ok(sender) = sender.lock() {
            let _ignored = sender.send(InstallEvent::Failed(error));
            return;
        }
    }

    eprintln!("{error}");
}

fn write_log_line(message: &str) {
    let Some(file) = LOG_FILE.get() else {
        return;
    };
    let Ok(mut file) = file.lock() else {
        return;
    };
    let _ignored = writeln!(file, "{} {message}", timestamp());
    let _ignored = file.flush();
}

fn diagnostic_log_path() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata)
            .join(".minecraft-pixelmon-friends")
            .join("logs")
            .join("installer.log");
    }

    std::env::temp_dir()
        .join("pixelmon-friends")
        .join("installer.log")
}

fn timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
