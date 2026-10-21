#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|a| a == openray_lib::hotkey::HELPER_ARG) {
        openray_lib::hotkey::run_helper();
    } else {
        openray_lib::run()
    }
}
