#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod annotate;
mod app;
mod autostart;
mod capture;
mod clipboard;
mod collage;
mod context_menu;
mod dxgi_capture;
mod elevate;
mod floating;
mod hotkeys;
mod i18n;
mod mouse_trigger;
mod native_mouse_trigger;
mod ocr;
mod paths;
mod settings;
mod share;
mod single_instance;
mod slint_overlay;
mod snip;
mod translate;
mod translate_popup;
mod tray;
mod upload;
mod window_metadata;
mod window_tamer;

slint::include_modules!();

#[cfg(target_os = "windows")]
fn bump_process_priority() {
    use windows::Win32::System::Threading::{
        GetCurrentProcess, SetPriorityClass, HIGH_PRIORITY_CLASS,
    };
    unsafe {
        let _ = SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS);
    }
}

#[cfg(not(target_os = "windows"))]
fn bump_process_priority() {}

fn main() -> Result<(), slint::PlatformError> {
    // Force Slint's software renderer rather than the default femtovg
    // (OpenGL) one. femtovg gives us the prettier renderer but its
    // window-level transparency depends on per-GPU DWM behaviour — on
    // multi-monitor setups with different adapters the overlay window
    // ends up fully opaque on the primary monitor while transparent on
    // a secondary. Software rendering goes through DWM's standard
    // composition path and behaves identically on every monitor.
    //
    // The UI is static (no animations / video / 3D) so the perf hit
    // is invisible while RAM drops because no GL context is created.
    std::env::set_var("SLINT_BACKEND", "winit-software");

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Only one Snipdeck per session. A second launch would fight the first
    // over the global hotkey and the process-wide input hooks; bail out
    // early (and raise the running window) instead.
    //
    // The exception is the tray "Run as administrator" handoff: this elevated
    // instance was just relaunched by a medium instance that is exiting right
    // now, so it must wait for that lock to be released and take it over rather
    // than quitting as a duplicate.
    let elevated_takeover = std::env::args().any(|a| a == "--elevated-takeover");
    let acquired = if elevated_takeover {
        single_instance::acquire_after_handoff()
    } else {
        single_instance::acquire()
    };
    if !acquired {
        log::info!("Another Snipdeck instance is already running; exiting.");
        return Ok(());
    }

    // Bump our process to "high" priority on Windows. Low-level mouse
    // and keyboard hooks (WH_MOUSE_LL / WH_KEYBOARD_LL) have a strict
    // per-callback timeout — if our process is at normal priority and
    // sits idle, Windows can suspend the hook chain and drop the first
    // event after a quiet period (this is what causes "Win+LMouse on
    // the secondary monitor doesn't trigger the first time"). HIGH
    // priority keeps the hook callback responsive.
    bump_process_priority();

    // Same Win11 fade-in / window-tame infra as the eframe build.
    window_tamer::start();

    let _ = paths::ensure_dirs();

    // Resolve the UI language from settings + OS locale and apply it to the
    // Rust-side catalog now (used by any early `i18n::t()` strings).
    let ui_language = {
        let settings = settings::Settings::load();
        let lang = i18n::resolve_language(&settings.language);
        i18n::set_language(lang);
        lang
    };

    let main_window = MainWindow::new()?;

    // Select the bundled Slint UI translation AFTER the window exists. Selecting
    // it earlier (before the backend is initialized) does not reach the first
    // render, so the UI would come up in English until the user re-picked the
    // language. This matches Slint's documented order (create component, then
    // `select_bundled_translation`, then run).
    if let Err(e) = slint::select_bundled_translation(ui_language) {
        log::warn!("select_bundled_translation({ui_language}) failed: {e:?}");
    }

    // Closing with the window's X hides Snipdeck to the tray instead of
    // quitting. The tray menu (and the global hotkeys) bring it back; the
    // tray's "Exit" is the only thing that actually exits. Paired with
    // `run_event_loop_until_quit()` below so the loop survives with no
    // window shown.
    main_window
        .window()
        .on_close_requested(|| slint::CloseRequestResponse::HideWindow);

    let mut state = app::AppState::new(&main_window);
    state.wire_callbacks(&main_window);
    state.refresh_gallery(&main_window);

    // Drive the cross-thread event pumps (hotkeys, mouse triggers) from
    // a Slint timer. Slint owns the main-thread event loop; keep this
    // short so native mouse-hook events reach the overlay without a
    // visible drag/click delay.
    let timer = slint::Timer::default();
    let weak = main_window.as_weak();
    let state = std::rc::Rc::new(std::cell::RefCell::new(state));

    // Re-chunk the gallery synchronously when the column count changes. This
    // is wired here (against the shared state) rather than in wire_callbacks
    // because it must run during a live window-resize, when the timer-driven
    // `tick` is frozen by the Win32 modal resize loop — a queued action would
    // only fire once the user released the mouse. try_borrow_mut keeps it
    // safe if it ever races with the tick.
    {
        let state_cols = state.clone();
        let weak_cols = main_window.as_weak();
        main_window.on_gallery_cols_changed(move |n| {
            if let Some(win) = weak_cols.upgrade() {
                if let Ok(mut st) = state_cols.try_borrow_mut() {
                    st.set_gallery_cols(n.max(1) as usize, &win);
                }
            }
        });
    }

    // System-tray icon + menu. Commands arrive on a channel we drain each
    // tick (same pattern as the mouse-trigger service) so all UI work stays
    // on the main thread.
    let tray = tray::start();
    let tray_rx = tray.rx.clone();

    let state_for_timer = state.clone();
    // winit doesn't apply the exe's embedded icon to the window's own title
    // bar / Alt-Tab entry, so set it natively once the HWND exists. Retried
    // each tick until the window is found, then latched off.
    let mut window_icon_set = false;
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(8),
        move || {
            if !window_icon_set {
                window_icon_set = tray::set_main_window_icon();
            }
            while let Ok(cmd) = tray_rx.try_recv() {
                match cmd {
                    tray::TrayCommand::Show => {
                        if let Some(window) = weak.upgrade() {
                            let _ = window.show();
                        }
                        tray::focus_main_window();
                    }
                    tray::TrayCommand::NewSnip => mouse_trigger::arm_snip(),
                    tray::TrayCommand::ToggleAutostart => match autostart::toggle() {
                        Ok(on) => {
                            log::info!("launch-at-login set to {on}");
                            if let Some(window) = weak.upgrade() {
                                window.set_autostart_enabled(on);
                            }
                        }
                        Err(e) => log::warn!("autostart toggle failed: {e}"),
                    },
                    tray::TrayCommand::RunAsAdmin => match elevate::relaunch_as_admin() {
                        // Launched an elevated instance — remove the tray icon
                        // and exit so it takes over the single-instance lock.
                        Ok(true) => {
                            tray::shutdown();
                            std::process::exit(0);
                        }
                        // User dismissed the UAC prompt — stay as we are.
                        Ok(false) => log::info!("run-as-administrator cancelled by user"),
                        Err(e) => log::warn!("run-as-administrator failed: {e}"),
                    },
                    tray::TrayCommand::Exit => {
                        let _ = slint::quit_event_loop();
                    }
                }
            }
            if let Some(window) = weak.upgrade() {
                state_for_timer.borrow_mut().tick(&window);
            }
        },
    );

    main_window.show()?;
    slint::run_event_loop_until_quit()
}
