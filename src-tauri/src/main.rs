#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
fn attach_console() {
    extern "system" {
        fn AttachConsole(dwProcessId: u32) -> i32;
    }
    unsafe {
        AttachConsole(0xFFFFFFFF); // ATTACH_PARENT_PROCESS
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    // If there is more than just the executable name, run in CLI mode
    if args.len() > 1 {
        #[cfg(windows)]
        attach_console();

        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        if let Err(e) = rt.block_on(tenet_lib::cli::run_cli()) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    } else {
        // Run GUI mode
        tenet_lib::run();
    }
}
