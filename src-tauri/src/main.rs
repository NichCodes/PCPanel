// PCPanel desktop shell.
//
// A thin Tauri window around the existing PCPanel backend. The backend is a self-contained Quarkus
// native binary that serves the Angular UI + REST + WebSocket on http://127.0.0.1:7654; this shell
// only adds a native window, a cross-platform tray, single-instance handling, and ownership of the
// backend's process lifecycle. All application logic stays in the backend, reached over localhost.
//
// The UI is shown only in this native window — closing the window hides it to the tray (the backend
// keeps running), and Quit (from the tray or the settings page) tears everything down.

// Hide the extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, RunEvent, WindowEvent};

// The shell always talks to the backend API on :7654 (health, quit, quitting).
const BACKEND_URL: &str = "http://127.0.0.1:7654";
const QUIT_URL: &str = "http://127.0.0.1:7654/api/system/quit";
const QUITTING_URL: &str = "http://127.0.0.1:7654/api/system/quitting";
// Where the window loads the UI from — different in dev vs. release. In `cargo tauri dev` (a debug
// build) the UI is served by the Angular/Vite dev server on :4200; loading it there (rather than from
// the backend's Quinoa-forwarded :7654) is what lets Vite's hot-reload websocket connect to its own
// origin instead of failing forever and spamming the console with red WebSocket errors. The dev server
// proxies /api + /ws back to :7654. A packaged release loads the built UI straight from the backend.
#[cfg(debug_assertions)]
const UI_URL: &str = "http://localhost:4200";
#[cfg(debug_assertions)]
const SETTINGS_URL: &str = "http://localhost:4200/settings";
#[cfg(not(debug_assertions))]
const UI_URL: &str = "http://127.0.0.1:7654";
#[cfg(not(debug_assertions))]
const SETTINGS_URL: &str = "http://127.0.0.1:7654/settings";
const MAIN_WINDOW: &str = "main";
// Cheap localhost poll for the supervisor's backend-health / quit check.
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Shared shell state: the backend child process (so we can watch it and kill it on exit) and a guard
/// so the shutdown sequence only runs once.
struct ShellState {
    backend: Mutex<Option<Child>>,
    shutting_down: AtomicBool,
}

fn main() {
    tauri::Builder::default()
        .manage(ShellState {
            backend: Mutex::new(None),
            shutting_down: AtomicBool::new(false),
        })
        // A second launch focuses the running instance instead of starting a new one.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_window_at(app, UI_URL);
        }))
        .setup(|app| {
            let handle = app.handle().clone();
            spawn_backend(&handle);
            build_tray(&handle)?;
            run_supervisor(handle);
            Ok(())
        })
        // Closing the window hides it to the tray; the backend keeps running. Quit is via the tray.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building the PCPanel shell")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                kill_backend(app);
            }
        });
}

/// Launch the bundled backend as a child process. Skipped when the bundled binary is absent — that is
/// the dev case (`cargo tauri dev`), where the backend is started separately via `./mvnw quarkus:dev`.
fn spawn_backend(app: &AppHandle) {
    let bin = if cfg!(windows) { "PCPanel.exe" } else { "PCPanel" };
    // PCPANEL_BACKEND_DIR lets a hand-rolled layout (e.g. the Flatpak) point at the backend explicitly
    // instead of relying on Tauri's resource-dir heuristics. Otherwise it is bundled under resources/.
    let backend_dir = match std::env::var_os("PCPANEL_BACKEND_DIR") {
        Some(dir) => std::path::PathBuf::from(dir),
        None => match app.path().resource_dir() {
            Ok(dir) => dir.join("backend"),
            Err(e) => {
                log::error!("could not resolve resource dir: {e}");
                return;
            }
        },
    };
    let exe = backend_dir.join(bin);
    if !exe.exists() {
        log::info!("no bundled backend at {exe:?}; assuming dev mode (start it with ./mvnw quarkus:dev)");
        return;
    }

    // skipfilecheck: this shell owns single-instance, so the backend's FileChecker must stand down.
    // PCPANEL_DISABLE_TRAY: the shell owns the tray.
    // Env vars (not -D props) are the launcher-robust path for the native image.
    match Command::new(&exe)
        .current_dir(&backend_dir)
        .arg("skipfilecheck")
        .env("PCPANEL_DISABLE_TRAY", "1")
        .spawn()
    {
        Ok(child) => {
            log::info!("started backend: {exe:?} (pid {})", child.id());
            if let Some(state) = app.try_state::<ShellState>() {
                *state.backend.lock().unwrap() = Some(child);
            }
        }
        Err(e) => log::error!("failed to start backend {exe:?}: {e}"),
    }
}

/// Terminate the backend child if it is still running (fallback — a tray Quit asks it to exit first).
fn kill_backend(app: &AppHandle) {
    if let Some(state) = app.try_state::<ShellState>() {
        if let Some(mut child) = state.backend.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open PCPanel", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Go to Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &quit])?;

    TrayIconBuilder::with_id("pcpanel-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("PCPanel")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_window_at(app, UI_URL),
            "settings" => show_window_at(app, SETTINGS_URL),
            "quit" => initiate_shutdown(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window_at(tray.app_handle(), UI_URL);
            }
        })
        .build(app)?;
    Ok(())
}

/// Background supervisor: wait for the backend to come up, reveal the window, then watch for a Quit so
/// it tears the whole shell down too. A Quit is detected three ways, covering every launch mode:
///   * the backend reports `quitting=true` (it was asked to quit — works even in dev, where the shell
///     did not spawn the backend and so has no child process to watch),
///   * the spawned backend child has exited (production), or
///   * the backend went unreachable after having been up and we have no child (dev, server stopped).
fn run_supervisor(app: AppHandle) {
    std::thread::spawn(move || {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_millis(1500))
            .timeout_read(Duration::from_secs(3))
            .build();

        wait_for_backend(&agent);
        // The backend (:7654) is up; in dev the Quinoa-managed dev server (:4200) comes up before it,
        // so UI_URL is ready to load by now.
        show_window_at(&app, UI_URL);

        let mut was_up = false;
        let mut down_streak = 0u32;
        loop {
            if backend_has_exited(&app) {
                log::info!("backend process exited; shutting down the shell");
                initiate_shutdown(&app);
                return;
            }
            match fetch_quitting(&agent) {
                Some(true) => {
                    log::info!("backend reported a quit; shutting down the shell");
                    initiate_shutdown(&app);
                    return;
                }
                Some(false) => {
                    was_up = true;
                    down_streak = 0;
                }
                None if was_up && !backend_child_present(&app) => {
                    // Dev mode has no spawned child to watch (the backend runs separately via
                    // `mvnw quarkus:dev`), so also treat going unreachable after being up as a Quit.
                    // Require a few consecutive misses so a brief live-reload restart doesn't count.
                    down_streak += 1;
                    if down_streak >= 6 {
                        log::info!("backend unreachable after running; shutting down the shell");
                        initiate_shutdown(&app);
                        return;
                    }
                }
                None => {}
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    });
}

/// Read /api/system/quitting (a bare JSON boolean). `None` when the backend is unreachable.
fn fetch_quitting(agent: &ureq::Agent) -> Option<bool> {
    let body = agent.get(QUITTING_URL).call().ok()?.into_string().ok()?;
    Some(body.contains("true"))
}

/// Tear the shell down for good, from any thread and any launch mode. Hides the UI at once (so the app
/// visibly disappears), asks a still-running backend to shut down gracefully and gives it a moment to
/// flush, then guarantees the process is gone with `std::process::exit` — which removes the window, the
/// Dock icon and the tray icon together (plain `AppHandle::exit` from a background thread does not
/// reliably terminate on macOS). Runs at most once.
fn initiate_shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<ShellState>() {
        if state.shutting_down.swap(true, Ordering::SeqCst) {
            return; // already shutting down
        }
    }

    // Make the app disappear immediately: hide the window and (macOS) drop the Dock icon.
    let visual = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = visual.get_webview_window(MAIN_WINDOW) {
            let _ = window.hide();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = visual.set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
    });

    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200)); // let the hide / Dock-icon change apply
        if !backend_has_exited(&app) {
            http_post(QUIT_URL); // ask the backend to shut down gracefully (no-op if already asked/down)
        }
        // Give a spawned backend up to ~4s to exit on its own (graceful Quarkus shutdown), then kill it.
        for _ in 0..40 {
            if !backend_child_present(&app) || backend_has_exited(&app) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        kill_backend(&app);
        std::process::exit(0);
    });
}

/// True once the spawned backend child has terminated (for any reason — a user Quit, a crash). Returns
/// false when there is no child (dev mode), so the shell keeps running there.
fn backend_has_exited(app: &AppHandle) -> bool {
    let Some(state) = app.try_state::<ShellState>() else {
        return false;
    };
    let mut guard = state.backend.lock().unwrap();
    match guard.as_mut() {
        Some(child) => !matches!(child.try_wait(), Ok(None)), // Ok(Some)=exited, Err=unknown → treat as gone
        None => false,
    }
}

/// Whether the shell spawned and still holds a backend child (production); false in dev.
fn backend_child_present(app: &AppHandle) -> bool {
    app.try_state::<ShellState>()
        .map(|s| s.backend.lock().unwrap().is_some())
        .unwrap_or(false)
}

/// Poll the backend root until it answers (or we give up after ~60s and let the webview surface the
/// error). Cheap GETs against localhost, so a tight 250ms cadence is fine.
fn wait_for_backend(agent: &ureq::Agent) {
    for _ in 0..240 {
        if agent.get(BACKEND_URL).call().is_ok() {
            log::info!("backend is up");
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    log::error!("backend did not become reachable on {BACKEND_URL}");
}

/// Navigate the native window to `url` and bring it to the front.
fn show_window_at(app: &AppHandle, url: &str) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        if let Ok(parsed) = tauri::Url::parse(url) {
            let _ = window.navigate(parsed);
        }
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn http_post(url: &str) {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(1500))
        .timeout_read(Duration::from_secs(3))
        .build();
    if let Err(e) = agent.post(url).call() {
        log::error!("POST {url} failed: {e}");
    }
}
