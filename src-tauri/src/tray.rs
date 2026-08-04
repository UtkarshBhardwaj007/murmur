use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Runtime,
};

pub const TRAY_ID: &str = "murmur-tray";

pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<TrayIcon<R>> {
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Murmur", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &separator, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().cloned().expect("bundled icon"))
        .tooltip("Murmur — idle")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => crate::show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)
}
