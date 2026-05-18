use crate::ui_events;

#[allow(dead_code)]
pub fn print_completion_message() {
    ui_events::log("다 깔렸어요! 으!!!!!@!!!@ ENTER 눌러서 종료하던지 말던지 제 알빠는 아닌데요 ");
}

#[allow(dead_code)]
fn read_enter() {
    use std::io;

    let mut input = String::new();
    let _ignored = io::stdin().read_line(&mut input);
}

#[allow(dead_code)]
pub fn wait_for_enter() {
    ui_events::log("");
    ui_events::log("창을 닫으려면 Enter 키를 눌러주세요.");
    read_enter();
}

#[allow(dead_code)]
pub fn wait_for_enter_silent() {
    read_enter();
}

pub fn print_installed_message(name: &str) {
    ui_events::log(format!("{name}이 깔렸어요 으!!@!"));
}
