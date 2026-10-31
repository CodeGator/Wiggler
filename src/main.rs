#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod autostart;
mod engine;
mod icon;
mod input;
mod quit;
mod settings;
mod theme;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(windows))]
    compile_error!("Wiggler is Windows-only");

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--nudge-once") {
        return input::nudge_once();
    }
    if args.iter().any(|arg| arg == "--quit") {
        quit::running_instances();
        return Ok(());
    }
    let start_hidden = args.iter().any(|arg| arg == "--autostart");
    app::run(start_hidden)
}
