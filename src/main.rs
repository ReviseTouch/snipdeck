#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod annotate;
mod app;
mod atomic_file;
mod autostart;
mod capture;
mod clipboard;
mod collage;
mod color;
mod context_menu;
mod dxgi_capture;
mod elevate;
mod floating;
/// Snipdeck on the session bus: Super+Alt+drag snips from the GNOME Shell
/// extension, and a second launch raising the first. Linux-only.
#[cfg(target_os = "linux")]
mod bus_service;
mod hotkeys;
mod i18n;
mod image_files;
mod loupe;
mod mouse_trigger;
mod native_mouse_trigger;
mod ocr;
mod os;
mod paths;
/// libpipewire, loaded at run time for the ScreenCast capture. Linux-only.
#[cfg(target_os = "linux")]
mod pipewire_dl;
/// Wayland global shortcuts. Linux-only: every other platform has a working
/// `global-hotkey` backend and never reaches for the portal.
#[cfg(target_os = "linux")]
mod portal_hotkeys;
mod pump;
/// Raw Wayland input-region control. Linux-only: every other platform can
/// place and stack its own windows and never needs this.
#[cfg(target_os = "linux")]
mod wayland_input_region;
mod screen;
mod settings;
mod share;
mod single_instance;
mod slint_overlay;
mod snip;
mod translate;
mod translate_popup;
mod tray;
mod upload;
mod window_env;
#[cfg(not(target_os = "windows"))]
mod wayland_screencast;
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

/// Append every panic (any thread) to `<data_dir>/crash.log` with a
/// backtrace. As a `windows_subsystem = "windows"` app we have no console:
/// without this, a panic kills the process silently — no Event Log entry,
/// no WER report, nothing to diagnose from. The previous handler still runs
/// afterwards so `RUST_LOG`-piped runs keep the stderr message too.
fn install_crash_log() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // The Linux tray dlopens libappindicator and PANICS when it is absent
        // — an expected "this desktop has no tray" outcome that `tray::linux`
        // catches and degrades from. Logging a backtrace for it would leave a
        // crash.log on a perfectly healthy run and teach the user to ignore
        // the file that exists to report real crashes.
        if info.to_string().contains("appindicator") {
            previous(info);
            return;
        }
        let backtrace = std::backtrace::Backtrace::force_capture();
        let thread = std::thread::current();
        let entry = format!(
            "==== panic at {} (thread {:?}, v{}) ====\n{info}\n{backtrace}\n\n",
            chrono::Local::now().to_rfc3339(),
            thread.name().unwrap_or("<unnamed>"),
            env!("CARGO_PKG_VERSION"),
        );
        let path = paths::data_dir().join("crash.log");
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = f.write_all(entry.as_bytes());
        }
        previous(info);
    }));
}

fn main() -> Result<(), slint::PlatformError> {
    install_crash_log();

    // FIRST, before anything that might want to explain itself.
    //
    // This used to sit below the backend installation, which meant every
    // `log::info!` and `log::warn!` in that block was written to a logger that
    // did not exist yet and thrown away. The failure that hid behind it: when
    // `Backend::new_with_renderer_by_name` cannot build a renderer it has
    // already created winit's event loop, so `set_platform` never runs, Slint
    // later builds its own backend, and the only thing reaching the terminal is
    // "EventLoop can't be recreated" — an error about the second attempt, from
    // which the actual cause cannot be recovered. The line naming the real
    // failure was being written the whole time, into nothing.
    //
    // zbus is held to warnings: its object server logs every incoming call at
    // info, with the whole message, which for a snip from the GNOME extension
    // is a screenful per capture. RUST_LOG still overrides all of it.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,zbus=warn"))
        .init();

    // Install the winit backend with the SOFTWARE renderer EXPLICITLY via
    // set_platform, rather than selecting it with the SLINT_BACKEND env var.
    // Two reasons:
    //
    //   1. Software rendering (not femtovg/OpenGL): the transparent capture
    //      overlay must composite identically across multi-monitor / mixed-GPU
    //      setups. OpenGL window transparency is per-adapter DWM-dependent —
    //      the overlay came up opaque on the primary monitor and transparent
    //      on a secondary. Software rendering goes through DWM's standard
    //      composition path and behaves identically on every monitor. The UI
    //      is static (no animation/video/3D) so the perf hit is invisible and
    //      RAM drops because no GL context is created.
    //
    //   2. set_platform — NOT the env var — is what unlocks the
    //      WinitWindowAccessor API (on_winit_window_event / winit_window). We
    //      need it to force a fresh frame on restore: the software renderer
    //      otherwise wakes blank because a timer-driven request_redraw isn't
    //      serviced until a real winit window event arrives. The default
    //      selector would pick the same backend, but the external API stays
    //      inaccessible unless we set the platform ourselves.
    //
    // Cargo.toml compiles the software renderer in for Windows only. Asked for
    // a renderer it was built without, Slint falls back to the default one
    // with nothing louder than a line on stderr — which is how Windows ran on
    // femtovg for two months, once femtovg became the build's only renderer.
    #[cfg(target_os = "windows")]
    {
        match i_slint_backend_winit::Backend::new_with_renderer_by_name(Some("software")) {
            Ok(backend) => match slint::platform::set_platform(Box::new(backend)) {
                Ok(()) => log::info!("winit-software backend installed via set_platform"),
                Err(e) => {
                    log::warn!("set_platform failed ({e:?}); falling back to env-var backend");
                    std::env::set_var("SLINT_BACKEND", "winit-software");
                }
            },
            Err(e) => {
                log::warn!("winit-software backend init failed ({e:?}); falling back to env var");
                std::env::set_var("SLINT_BACKEND", "winit-software");
            }
        }
    }
    // Linux: femtovg, installed the same way and for the same second reason —
    // `set_platform` is what unlocks `WinitWindowAccessor`, which the Wayland
    // paths depend on for `drag_window()` and for reaching the raw `wl_surface`
    // to set an input region.
    //
    // The renderer is NOT software here, and asking for it by that name was an
    // active bug: this line still said "winit-software" after the build moved to
    // femtovg (Wayland transparency is impossible on the software path — see
    // Cargo.toml). Slint answered "unrecognized renderer software, falling back
    // to FemtoVG", which worked under Wayland but left the main window unmapped
    // under XWayland — the app came up invisible with only a tray icon.
    #[cfg(not(target_os = "windows"))]
    {
        match i_slint_backend_winit::Backend::new_with_renderer_by_name(Some("femtovg")) {
            Ok(backend) => match slint::platform::set_platform(Box::new(backend)) {
                Ok(()) => log::info!("winit-femtovg backend installed via set_platform"),
                Err(e) => log::warn!("set_platform failed ({e:?}); using the default backend"),
            },
            Err(e) => log::warn!("winit-femtovg backend init failed ({e:?}); using the default"),
        }
    }


    // Claim an xdg application id. Must happen after `set_platform` (there is no
    // context to store it in before that) and before the first window is shown —
    // winit reads it once, while building the window attributes.
    //
    // Slint sends `xdg_toplevel.set_app_id` ONLY when this has been set; leave it
    // alone and every Snipdeck window reaches the compositor anonymous. What this
    // buys is the shell tying our windows to snipdeck.desktop — the icon, the
    // window list, the application switcher entry.
    //
    // NOT what identifies a snip to the shell extension: that is the window
    // title. `meta_window_get_wm_class()` reports the X11 WM_CLASS property and
    // a Wayland toplevel need not have one, so matching on it there is a gate
    // that may never open. See packaging/gnome-extension.
    //
    // The string has to stay "snipdeck", matching snipdeck.desktop's basename and
    // its StartupWMClass, or the association breaks.
    #[cfg(not(target_os = "windows"))]
    match slint::set_xdg_app_id("snipdeck") {
        Ok(()) => log::info!("xdg app id set to \"snipdeck\""),
        Err(e) => log::warn!(
            "could not set the xdg app id ({e:?}); the GNOME extension will not see our windows"
        ),
    }

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
        // Launching it again is how people look for a window they cannot find
        // — hidden, with no tray to bring it back. Ask the running one to show
        // it rather than exiting without a word.
        #[cfg(target_os = "linux")]
        if bus_service::show_running_instance() {
            log::info!("asked the running instance to show its window");
        }
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
    // Temporaries a crash left half-written. Safe only now: the single-instance
    // guard above means no other Snipdeck can be mid-write in these folders.
    atomic_file::sweep(&paths::images_dir());
    atomic_file::sweep(&paths::thumbs_dir());
    autostart::refresh();

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
    //
    // With no tray there is nothing to bring the window back, so the X must
    // quit rather than strand a headless process. On Linux the tray can fail
    // to appear at runtime (no StatusNotifierHost), so this is asked of
    // `tray::can_hide_to_tray` each time rather than decided at compile time.
    main_window.window().on_close_requested(|| {
        if !tray::can_hide_to_tray() {
            let _ = slint::quit_event_loop();
        }
        slint::CloseRequestResponse::HideWindow
    });

    // The Slint software renderer can wake from minimize/restore (or tray
    // hide/show) with a blank or stale frame: a redraw requested from our
    // 8ms timer only sets a flag and isn't serviced until a *real* window
    // event arrives — which is exactly why hovering the mouse "fixes" it.
    // Hook the actual winit events instead and request the redraw from inside
    // the event loop's own dispatch, so a fresh frame is painted immediately
    // on restore. The hook is installed from spawn_local AFTER winit_window()
    // resolves, because the WinitWindowAccessor API only attaches once the
    // native window exists (and only because we installed the backend via
    // set_platform above). Mirrors RustWailsChat's lifecycle redraw hook.
    #[cfg(target_os = "windows")]
    {
        let weak_redraw = main_window.as_weak();
        let _ = slint::spawn_local(async move {
            use i_slint_backend_winit::winit::event::WindowEvent;
            use i_slint_backend_winit::{EventResult, WinitWindowAccessor};
            let Some(w) = weak_redraw.upgrade() else { return };
            if w.window().winit_window().await.is_err() {
                log::warn!("winit_window().await failed; restore-redraw hook not installed");
                return;
            }
            w.window().on_winit_window_event(|win, ev| {
                match ev {
                    WindowEvent::Focused(true)
                    | WindowEvent::Resized(_)
                    | WindowEvent::Occluded(false) => {
                        win.request_redraw();
                    }
                    _ => {}
                }
                // Any main-window event (restore, focus, mouse, keys)
                // means the user is interacting — wake the pump so its
                // housekeeping (and the UI action queue) runs. Coalesced,
                // so it's a cheap atomic while already awake.
                crate::pump::wake();
                EventResult::Propagate
            });
        });
    }

    let mut state = app::AppState::new(&main_window);
    state.wire_callbacks(&main_window);
    state.refresh_gallery(&main_window);

    // Drive the cross-thread event pumps (hotkeys, mouse triggers) from
    // the event-driven pump (see `mod pump`). It runs this housekeeping at
    // an 8 ms cadence *while there is work* — short enough that native
    // mouse-hook events reach the overlay without a visible drag/click
    // delay — then stops the timer so the process sleeps when idle.
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
    // each iteration until the window is found, then latched off.
    let window_icon_set = std::cell::Cell::new(false);
    // Tracks whether the main window was on screen last iteration, so we can
    // release RAM to the standby list the moment it drops to the tray. (The
    // blank-frame redraw on restore is handled by the winit event hook above.)
    // Windows-only: both the standby-list trim and the blank-on-restore fix
    // it drives are Win32 concerns.
    #[cfg(target_os = "windows")]
    let prev_shown = std::cell::Cell::new(false);
    pump::install(std::time::Duration::from_millis(8), move || {
        // `active` reports whether this iteration wants the pump to keep
        // running. It's OR-ed from work done here plus `AppState::tick`,
        // which adds its own "wants to stay awake" conditions.
        let mut active = false;

        if !window_icon_set.get() {
            window_icon_set.set(tray::set_main_window_icon());
            // Keep ticking until the icon latches (the HWND may not exist yet).
            active = true;
        }

        // When the window drops to the tray / is minimized, release RAM to
        // the standby list so it costs little while parked yet comes back
        // fast. The blank-frame redraw on restore is handled by the winit
        // event hook (Focused/Resized/Occluded) set up before the loop — a
        // request_redraw from this pump isn't serviced until a real window
        // event arrives, which is why it used to wake blank until hover.
        #[cfg(target_os = "windows")]
        {
            let now_shown = tray::is_main_window_shown();
            if now_shown && !prev_shown.get() {
                // Restored from tray/minimize. On Windows the winit-software
                // renderer presents via GDI gated on WM_PAINT, which winit
                // drops for a plain request_redraw OR a raw SetWindowPos. The
                // only thing that forces a fresh present is a size change
                // routed THROUGH Slint (so its redraw path observes it) and
                // split across two event-loop iterations so winit registers
                // two distinct sizes: grow 1px now, restore on the next tick.
                // (Slint #8592 / #8823 — software renderer blank-on-restore.)
                if let Some(window) = weak.upgrade() {
                    let s = window.window().size();
                    log::info!("restore detected -> set_size toggle {}x{}", s.width, s.height);
                    window
                        .window()
                        .set_size(slint::PhysicalSize::new(s.width + 1, s.height));
                    let weak_restore = window.as_weak();
                    slint::Timer::single_shot(std::time::Duration::ZERO, move || {
                        if let Some(w) = weak_restore.upgrade() {
                            w.window().set_size(s);
                        }
                    });
                }
                active = true;
            }
            if !now_shown && prev_shown.get() {
                tray::trim_working_set();
            }
            prev_shown.set(now_shown);
        }

        while let Ok(cmd) = tray_rx.try_recv() {
            active = true;
            match cmd {
                tray::TrayCommand::Show => {
                    if let Some(window) = weak.upgrade() {
                        let _ = window.show();
                    }
                    tray::focus_main_window();
                }
                // Route through the action queue, NOT straight to arm_snip():
                // arming alone waits for a global mouse hook to deliver the
                // drag, which exists on Windows and nowhere else. Off Windows
                // the click never came and the menu item did nothing.
                tray::TrayCommand::NewSnip => {
                    if !app::queue_action(app::Action::NewSnip {
                        to_clip: false,
                        keep_float: true,
                        to_gallery: true,
                    }) {
                        // Before the UI is wired there is nothing to queue into;
                        // fall back to the old behaviour so Windows still arms.
                        mouse_trigger::arm_snip();
                    }
                }
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
            // try_borrow_mut, NOT borrow_mut: a pump step can re-enter. Some
            // tick work pumps the Win32 message queue on this thread (e.g.
            // the share sheet's WinRT `.get()`), which can dispatch a queued
            // `pump::wake()` invoke — and that invoke runs another step while
            // the outer tick still holds the borrow. Skipping the nested
            // iteration is always safe (the outer one is mid-flight); paniced
            // BorrowMutError here would kill the app silently instead.
            if let Ok(mut state) = state_for_timer.try_borrow_mut() {
                if state.tick(&window) {
                    active = true;
                }
            } else {
                // Re-entrant call: report "active" so the pump stays armed
                // and the skipped work runs on the next regular tick.
                active = true;
            }
        }

        active
    });

    main_window.show()?;
    slint::run_event_loop_until_quit()
}
