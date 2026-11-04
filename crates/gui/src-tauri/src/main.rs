// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Some(code) =
        azerothkit_core::tls::handle_ca_install_request()
    {
        std::process::exit(code);
    }

    azerothkit_gui_lib::run();
}