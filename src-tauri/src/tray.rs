use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Runtime,
};

pub const TRAY_ID: &str = "murmur-tray";

pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<TrayIcon<R>> {
    // The dictation item doubles as the fallback for environments where
    // global hotkeys are unavailable (e.g. some Wayland compositors).
    let dictate = MenuItem::with_id(app, "dictate", "Start/Stop Dictation", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Murmur", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&dictate, &settings, &separator, &quit])?;

    let idle_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-idle.png"))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(idle_icon)
        .tooltip("Murmur — idle")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "dictate" => crate::dictation::toggle(app),
            "settings" => crate::show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)
}
