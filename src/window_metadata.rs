use serde::{Deserialize, Serialize};

use crate::snip::ScreenRect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub platform: String,
    pub window_id: String,
    pub title: Option<String>,
    pub app_name: Option<String>,
    pub process_name: Option<String>,
    pub pid: Option<u32>,
    pub exe_path: Option<String>,
    pub bounds: ScreenRect,
    pub confidence: f32,
}

pub fn capture_at_point(x: i32, y: i32) -> Option<WindowInfo> {
    #[cfg(target_os = "windows")]
    {
        imp_windows::capture_at_point(x, y).or_else(|| capture_with_xcap(x, y))
    }
    #[cfg(not(target_os = "windows"))]
    {
        capture_with_xcap(x, y)
    }
}

pub fn with_selection_confidence(mut info: WindowInfo, selection: ScreenRect) -> WindowInfo {
    info.confidence = selection_confidence(info.bounds, selection);
    info
}

pub fn selection_confidence(window: ScreenRect, selection: ScreenRect) -> f32 {
    let overlap = overlap_area(window, selection) as f32;
    if overlap <= 0.0 {
        return 0.25;
    }

    let selection_area = rect_area(selection).max(1) as f32;
    let window_area = rect_area(window).max(1) as f32;
    let selection_inside_window = overlap / selection_area;
    let window_covered = (overlap / window_area).min(0.35);

    (0.30 + selection_inside_window * 0.55 + window_covered * 0.45).min(1.0)
}

/// The window under a point, from xcap's window list — the fallback where no
/// native lookup answered. Better no name than a wrong one: a snip's window
/// details are searchable and shown, and a wrong title files the snip under
/// the wrong thing.
///
/// - On Windows the list is in z-order, topmost first, so the first window
///   under the point is the one on top — once Snipdeck's own windows (the
///   overlay above all) are left out, which is exactly when this fallback
///   runs: the native lookup declines our own windows.
/// - On X11 it is `_NET_CLIENT_LIST`, in the order windows were mapped, not
///   stacked; with more than one window under the point there is no telling
///   which is on top, and the first was simply the oldest.
/// - On Wayland it holds only XWayland windows, and the window actually
///   under the point may not be in it at all.
fn capture_with_xcap(x: i32, y: i32) -> Option<WindowInfo> {
    #[cfg(target_os = "linux")]
    if crate::window_env::caps().backend == crate::window_env::Backend::Wayland {
        return None;
    }
    let mut hits = xcap::Window::all()
        .ok()?
        .into_iter()
        .filter(|window| !window.is_minimized())
        .filter(|window| window.width() > 0 && window.height() > 0)
        .filter(|window| {
            rect_contains(
                window.x(),
                window.y(),
                window.width(),
                window.height(),
                x,
                y,
            )
        });
    #[cfg(target_os = "windows")]
    {
        hits.find(|window| window.process_id() != std::process::id())
            .map(window_from_xcap)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let only = hits.next()?;
        if hits.next().is_some() {
            return None;
        }
        Some(window_from_xcap(only))
    }
}

fn window_from_xcap(window: xcap::Window) -> WindowInfo {
    #[cfg(target_os = "windows")]
    let pid = Some(window.process_id());
    #[cfg(not(target_os = "windows"))]
    let pid = None;

    WindowInfo {
        platform: std::env::consts::OS.to_string(),
        window_id: window.id().to_string(),
        title: non_empty(window.title()),
        app_name: non_empty(window.app_name()),
        process_name: None,
        pid,
        exe_path: None,
        bounds: ScreenRect::new(window.x(), window.y(), window.width(), window.height()),
        confidence: 0.55,
    }
}

fn rect_contains(x: i32, y: i32, w: u32, h: u32, px: i32, py: i32) -> bool {
    let right = x.saturating_add(w as i32);
    let bottom = y.saturating_add(h as i32);
    px >= x && px < right && py >= y && py < bottom
}

fn overlap_area(a: ScreenRect, b: ScreenRect) -> u64 {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = rect_right(a).min(rect_right(b));
    let bottom = rect_bottom(a).min(rect_bottom(b));
    if right <= left || bottom <= top {
        return 0;
    }
    (right - left) as u64 * (bottom - top) as u64
}

fn rect_area(rect: ScreenRect) -> u64 {
    rect.w as u64 * rect.h as u64
}

fn rect_right(rect: ScreenRect) -> i32 {
    rect.x.saturating_add(rect.w as i32)
}

fn rect_bottom(rect: ScreenRect) -> i32 {
    rect.y.saturating_add(rect.h as i32)
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(target_os = "windows")]
mod imp_windows {
    use std::path::Path;

    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, HWND, POINT, RECT};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetAncestor, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible, WindowFromPoint, GA_ROOT,
    };

    use super::{non_empty, WindowInfo};
    use crate::snip::ScreenRect;

    pub fn capture_at_point(x: i32, y: i32) -> Option<WindowInfo> {
        unsafe {
            let hwnd = root_window(WindowFromPoint(POINT { x, y }))?;
            if !IsWindowVisible(hwnd).as_bool() {
                return None;
            }

            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 || pid == GetCurrentProcessId() {
                return None;
            }

            let bounds = window_bounds(hwnd)?;
            let exe_path = process_exe_path(pid);
            let process_name = exe_path
                .as_deref()
                .and_then(|path| Path::new(path).file_name())
                .and_then(|name| name.to_str())
                .map(|name| name.to_string());

            Some(WindowInfo {
                platform: "windows".to_string(),
                window_id: format!("{:p}", hwnd.0),
                title: non_empty(&window_title(hwnd)),
                app_name: process_name.clone(),
                process_name,
                pid: Some(pid),
                exe_path,
                bounds,
                confidence: 0.70,
            })
        }
    }

    unsafe fn root_window(hwnd: HWND) -> Option<HWND> {
        if hwnd.0.is_null() {
            return None;
        }
        let root = GetAncestor(hwnd, GA_ROOT);
        if root.0.is_null() {
            Some(hwnd)
        } else {
            Some(root)
        }
    }

    unsafe fn window_bounds(hwnd: HWND) -> Option<ScreenRect> {
        let mut rect = RECT::default();
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )
        .is_err()
        {
            GetWindowRect(hwnd, &mut rect).ok()?;
        }
        rect_to_screen_rect(rect)
    }

    fn rect_to_screen_rect(rect: RECT) -> Option<ScreenRect> {
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w <= 0 || h <= 0 {
            return None;
        }
        Some(ScreenRect::new(rect.left, rect.top, w as u32, h as u32))
    }

    unsafe fn window_title(hwnd: HWND) -> String {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len as usize + 1];
        let n = GetWindowTextW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf[..n as usize])
    }

    unsafe fn process_exe_path(pid: u32) -> Option<String> {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u16; 32768];
        let mut len = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        result
            .ok()
            .map(|_| String::from_utf16_lossy(&buf[..len as usize]))
            .and_then(|path| non_empty(&path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_inside_window_scores_high() {
        let window = ScreenRect::new(0, 0, 1000, 800);
        let selection = ScreenRect::new(100, 100, 200, 150);
        assert!(selection_confidence(window, selection) > 0.85);
    }

    #[test]
    fn non_overlapping_selection_keeps_low_confidence() {
        let window = ScreenRect::new(0, 0, 100, 100);
        let selection = ScreenRect::new(200, 200, 20, 20);
        assert_eq!(selection_confidence(window, selection), 0.25);
    }
}
