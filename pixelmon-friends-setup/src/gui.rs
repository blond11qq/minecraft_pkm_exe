use crate::{run_installation, ui_events, ui_events::InstallEvent};
use eframe::egui;
use std::{
    any::Any,
    fs,
    path::PathBuf,
    panic::{self, AssertUnwindSafe},
    sync::{
        atomic::{AtomicI32, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const SUCCESS_MESSAGE: &str = "다 깔렸어요! 으!!!!!@!!!@ ENTER 눌러서 종료하던지 말던지 제 알빠는 아닌데요 ";
const EXIT_BUTTON_DELAY: Duration = Duration::from_secs(2);

pub fn run() -> i32 {
    let log_path = ui_events::init_log_file();
    install_panic_hook();

    // GUI 앱은 실패도 창 안에서 보여주므로, Windows Program Compatibility Assistant가
    // "제대로 설치되지 않았을 수 있습니다"를 다시 띄우지 않도록 기본 종료 코드는 0으로 둡니다.
    let exit_code = Arc::new(AtomicI32::new(0));
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_title("Pixelmon Friends")
            .with_inner_size([720.0, 560.0])
            .with_resizable(false),
        ..Default::default()
    };

    let app_exit_code = Arc::clone(&exit_code);
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        eframe::run_native(
            "Pixelmon Friends",
            native_options,
            Box::new(move |creation_context| {
                Box::new(SetupApp::new(
                    &creation_context.egui_ctx,
                    Arc::clone(&app_exit_code),
                    log_path.clone(),
                ))
            }),
        )
    }));

    match result {
        Ok(Ok(())) => exit_code.load(Ordering::SeqCst),
        Ok(Err(error)) => {
            let message = format!("GUI를 시작할 수 없습니다: {error}");
            ui_events::diagnostic(&message);
            show_fatal_error(&message);
            1
        }
        Err(payload) => {
            let message = format!(
                "GUI에서 내부 오류가 발생했습니다.\n{}",
                panic_payload_to_string(payload.as_ref())
            );
            ui_events::diagnostic(&message);
            show_fatal_error(&message);
            1
        }
    }
}

struct SetupApp {
    receiver: Receiver<InstallEvent>,
    install_thread: Option<JoinHandle<()>>,
    exit_code: Arc<AtomicI32>,
    logs: Vec<String>,
    status: String,
    progress: f32,
    done: bool,
    success: bool,
    finished_at: Option<Instant>,
    allow_close: bool,
    close_warning_logged: bool,
    log_path: Option<PathBuf>,
}

impl SetupApp {
    fn new(ctx: &egui::Context, exit_code: Arc<AtomicI32>, log_path: Option<PathBuf>) -> Self {
        install_korean_font(ctx);

        let (sender, receiver) = mpsc::channel();
        ui_events::set_sender(sender);

        let install_thread = thread::spawn(|| {
            let result = panic::catch_unwind(AssertUnwindSafe(run_installation));
            match result {
                Ok(Ok(())) => ui_events::finished(),
                Ok(Err(error)) => ui_events::failed(format!("{error:#}")),
                Err(payload) => ui_events::failed(format!(
                    "설치 중 내부 오류가 발생했습니다.\n{}",
                    panic_payload_to_string(payload.as_ref())
                )),
            }
        });

        let mut logs = vec!["픽셀몬 프렌즈 설치 도우미를 시작했어요.".to_string()];
        if let Some(path) = &log_path {
            logs.push(format!("문제가 생기면 로그 파일을 확인해주세요: {}", path.display()));
        }

        Self {
            receiver,
            install_thread: Some(install_thread),
            exit_code,
            logs,
            status: "설치를 준비하는 중...".to_string(),
            progress: 0.03,
            done: false,
            success: false,
            finished_at: None,
            allow_close: false,
            close_warning_logged: false,
            log_path,
        }
    }

    fn drain_events(&mut self) {
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                InstallEvent::Log(message) => self.logs.push(message),
                InstallEvent::Progress { value, message } => {
                    self.progress = value.clamp(0.0, 1.0);
                    self.status = message.clone();
                    self.logs.push(message);
                }
                InstallEvent::Finished => self.mark_finished(),
                InstallEvent::Failed(error) => self.mark_failed(error),
            }
        }
    }

    fn check_install_thread(&mut self) {
        if self.done {
            return;
        }

        let Some(handle) = self.install_thread.as_ref() else {
            return;
        };
        if !handle.is_finished() {
            return;
        }

        let Some(handle) = self.install_thread.take() else {
            return;
        };
        match handle.join() {
            Ok(()) => {
                self.drain_events();
                if !self.done {
                    self.mark_failed(
                        "설치 작업이 결과를 보내지 않고 종료되었습니다. installer.log를 확인해주세요."
                            .to_string(),
                    );
                }
            }
            Err(payload) => self.mark_failed(format!(
                "설치 스레드가 예기치 않게 종료되었습니다.\n{}",
                panic_payload_to_string(payload.as_ref())
            )),
        }
    }

    fn mark_finished(&mut self) {
        self.done = true;
        self.success = true;
        self.progress = 1.0;
        self.status = SUCCESS_MESSAGE.to_string();
        self.logs.push(SUCCESS_MESSAGE.to_string());
        self.logs
            .push("창은 자동으로 닫히지 않아요. 확인하고 종료 버튼을 눌러주세요.".to_string());
        self.finished_at = Some(Instant::now());
        self.exit_code.store(0, Ordering::SeqCst);
    }

    fn mark_failed(&mut self, error: String) {
        self.done = true;
        self.success = false;
        self.status = "설치를 계속할 수 없습니다.".to_string();
        self.logs.push("설치를 계속할 수 없습니다:".to_string());
        self.logs.push(error);
        if let Some(path) = &self.log_path {
            self.logs
                .push(format!("자세한 로그: {}", path.display()));
        }
        self.logs
            .push("창은 자동으로 닫히지 않아요. 로그를 확인하고 종료 버튼을 눌러주세요.".to_string());
        self.finished_at = Some(Instant::now());
        // GUI에서는 오류 내용을 창에 남기므로 PCA 팝업 방지를 위해 0으로 종료합니다.
        self.exit_code.store(0, Ordering::SeqCst);
    }

    fn can_close(&self) -> bool {
        self.finished_at
            .map(|finished_at| finished_at.elapsed() >= EXIT_BUTTON_DELAY)
            .unwrap_or(false)
    }
}

impl eframe::App for SetupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|input| input.viewport().close_requested()) && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if !self.close_warning_logged {
                let message = if self.done {
                    "아직 종료 버튼이 활성화되지 않았어요. 잠깐만 기다렸다가 종료 버튼을 눌러주세요."
                } else {
                    "설치 중에는 창을 자동으로 닫지 않아요. 설치가 끝난 뒤 종료 버튼을 눌러주세요."
                };
                self.logs.push(message.to_string());
                self.close_warning_logged = true;
            }
        }

        self.drain_events();
        self.check_install_thread();
        ctx.request_repaint_after(Duration::from_millis(120));

        egui::CentralPanel::default()
            .frame(egui::Frame::default().inner_margin(egui::Margin::same(24.0)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Pixelmon Friends");
                    ui.add_space(4.0);
                    ui.label("픽셀몬 모드, 최적화 모드, 런처 프로필, 서버 목록을 자동으로 설치합니다.");
                });

                ui.add_space(20.0);
                egui::Frame::group(ui.style())
                    .rounding(egui::Rounding::same(14.0))
                    .inner_margin(egui::Margin::same(18.0))
                    .show(ui, |ui| {
                        let status_prefix = if self.done && self.success {
                            "완료"
                        } else if self.done {
                            "실패"
                        } else {
                            "진행 중"
                        };

                        ui.horizontal(|ui| {
                            ui.strong(status_prefix);
                            ui.separator();
                            ui.label(&self.status);
                        });
                        ui.add_space(12.0);
                        let progress_width = ui.available_width().max(120.0);
                        ui.add_sized(
                            [progress_width, 22.0],
                            egui::ProgressBar::new(self.progress).show_percentage(),
                        );
                    });

                ui.add_space(18.0);
                ui.strong("설치 로그");
                ui.add_space(8.0);

                egui::Frame::group(ui.style())
                    .rounding(egui::Rounding::same(14.0))
                    .inner_margin(egui::Margin::same(14.0))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .max_height(260.0)
                            .show(ui, |ui| {
                                for log in &self.logs {
                                    ui.label(log);
                                }
                            });
                    });

                ui.add_space(16.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_close = self.can_close();
                    let button_label = if can_close {
                        "종료"
                    } else if self.done {
                        "잠시만요..."
                    } else {
                        "설치 중..."
                    };
                    let button = egui::Button::new(button_label).min_size(egui::vec2(104.0, 36.0));
                    if ui.add_enabled(can_close, button).clicked() {
                        self.allow_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
    }
}

fn install_korean_font(ctx: &egui::Context) {
    let Some(path) = malgun_gothic_path() else {
        return;
    };
    let Ok(bytes) = fs::read(path) else {
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("malgun-gothic".to_string(), egui::FontData::from_owned(bytes));

    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "malgun-gothic".to_string());
    }

    ctx.set_fonts(fonts);
}

fn malgun_gothic_path() -> Option<PathBuf> {
    let windir = std::env::var_os("WINDIR")?;
    let fonts_dir = PathBuf::from(windir).join("Fonts");
    for filename in ["malgun.ttf", "malgunbd.ttf", "malgunsl.ttf"] {
        let path = fonts_dir.join(filename);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn install_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        ui_events::diagnostic(format!("panic: {info}"));
        default_hook(info);
    }));
}

fn panic_payload_to_string(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "알 수 없는 내부 오류입니다.".to_string()
}

#[cfg(windows)]
fn show_fatal_error(message: &str) {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MB_TASKMODAL,
    };

    fn wide(text: &str) -> Vec<u16> {
        OsStr::new(text).encode_wide().chain(Some(0)).collect()
    }

    let title = wide("Pixelmon Friends 오류");
    let body = if let Some(path) = ui_events::current_log_path() {
        format!("{message}\n\n로그 파일:\n{}", path.display())
    } else {
        message.to_string()
    };
    let body = wide(&body);

    unsafe {
        MessageBoxW(
            0,
            body.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_TASKMODAL | MB_SETFOREGROUND,
        );
    }
}

#[cfg(not(windows))]
fn show_fatal_error(message: &str) {
    eprintln!("{message}");
}
