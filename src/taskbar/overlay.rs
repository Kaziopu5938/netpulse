use std::ffi::c_void;
use std::sync::{Arc, Once};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush, DeleteDC,
    DeleteObject, EndPaint, FillRect, GetDC, GetMonitorInfoW, GetTextExtentPoint32W,
    GetTextMetricsW, MapWindowPoints, MonitorFromWindow, ReleaseDC, SelectObject, SetBkMode,
    SetTextAlign, SetTextColor, TextOutW, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, BLENDFUNCTION, CLIP_DEFAULT_PRECIS, DIB_RGB_COLORS, FONT_CHARSET, HFONT, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, OUT_DEFAULT_PRECIS, ANTIALIASED_QUALITY, TA_BASELINE, TA_LEFT,
    TEXTMETRICW, TRANSPARENT,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetParent, GetWindowLongPtrW, IsWindow, RegisterClassW,
    SetParent, SetWindowLongPtrW, SetWindowPos, ShowWindow, UpdateLayeredWindow, CREATESTRUCTW,
    GWLP_USERDATA, GWL_STYLE, SWP_NOACTIVATE, SW_HIDE, SW_SHOWNA, ULW_ALPHA, WINDOW_EX_STYLE,
    WM_ERASEBKGND, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WNDCLASSW, WS_CHILD, WS_CLIPSIBLINGS,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    HWND_TOPMOST, HWND_TOP,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

use crate::platform::wide;
use crate::settings::{Background, Config, DockMode, Layout, Position};
use crate::state::{Shared, METER_CLASS};
use crate::taskbar::{display, taskbar_is_vertical, taskbar_window, tray_window_of, window_rect};

const PAD_X: i32 = 5;
const PAD_Y: i32 = 2;
const FONT_WEIGHT_SEMIBOLD: i32 = 600;

static CLASS_REGISTERED: Once = Once::new();

struct Meter {
    hwnd: HWND,
    parent: HWND,
    config: Config,
    font: HFONT,
    measure_font: HFONT,
    font_px: i32,
    ascent: i32,
    vertical: bool,
    lines: Vec<Vec<u16>>,
    line_widths: Vec<i32>,
    width: i32,
    height: i32,
    visible: bool,
    /// Font family, size, DPI and orientation of the current layout.
    layout_key: String,
}

impl Meter {
    /// Refreshes text, font and geometry. The window is sized to the text itself,
    /// so its right edge stays anchored next to the tray without a phantom gap.
    unsafe fn apply(&mut self, config: &Config, lines: &[String], hwnd: HWND) {
        self.config = config.clone();
        self.vertical = taskbar_is_vertical();

        let mut texts: Vec<String> = match config.layout {
            Layout::Horizontal => vec![lines.join(display::GAP)],
            Layout::Vertical => lines.to_vec(),
        };
        if self.vertical {
            texts = vec![lines.join(display::GAP)];
        }
        self.lines = texts.iter().map(|line| line.encode_utf16().collect()).collect();

        let dpi = GetDpiForWindow(hwnd).max(96) as i32;
        let key = format!(
            "{}|{}|{}|{}",
            config.font_family,
            config.font_size,
            dpi,
            self.vertical as u8
        );
        if key != self.layout_key {
            self.layout_key = key;
            let font_px = ((config.font_size as i32) * dpi / 96).clamp(8, 64);
            let face = wide(&config.font_family);
            let escapement = if self.vertical { 900 } else { 0 };

            let font = CreateFontW(
                -font_px,
                0,
                escapement,
                0,
                FONT_WEIGHT_SEMIBOLD,
                0,
                0,
                0,
                FONT_CHARSET(1),
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                ANTIALIASED_QUALITY,
                0,
                PCWSTR(face.as_ptr()),
            );
            let measure_font = if self.vertical {
                CreateFontW(
                    -font_px,
                    0,
                    0,
                    0,
                    FONT_WEIGHT_SEMIBOLD,
                    0,
                    0,
                    0,
                    FONT_CHARSET(1),
                    OUT_DEFAULT_PRECIS,
                    CLIP_DEFAULT_PRECIS,
                    ANTIALIASED_QUALITY,
                    0,
                    PCWSTR(face.as_ptr()),
                )
            } else {
                HFONT(std::ptr::null_mut())
            };

            if !self.font.is_invalid() {
                let _ = DeleteObject(self.font.into());
            }
            if !self.measure_font.is_invalid() {
                let _ = DeleteObject(self.measure_font.into());
            }
            self.font = font;
            self.measure_font = measure_font;
            self.font_px = font_px;

            let hdc = GetDC(None);
            let mut metrics = TEXTMETRICW::default();
            let _ = GetTextMetricsW(hdc, &mut metrics);
            self.ascent = metrics.tmAscent;
            let _ = ReleaseDC(None, hdc);
        }

        self.measure_lines();
        let widest = self.line_widths.iter().copied().max().unwrap_or(0);
        if self.vertical {
            self.width = self.font_px + 2 * PAD_Y;
            self.height = widest + 2 * PAD_X;
        } else {
            self.width = widest + 2 * PAD_X;
            let count = self.lines.len().max(1) as i32;
            self.height = count * self.font_px + 2 * PAD_Y;
        }
    }

    /// Measures each visible line with the current font.
    unsafe fn measure_lines(&mut self) {
        self.line_widths.clear();
        if self.font.is_invalid() || self.lines.is_empty() {
            return;
        }
        let hdc = GetDC(None);
        let font = if self.measure_font.is_invalid() {
            self.font
        } else {
            self.measure_font
        };
        let previous = SelectObject(hdc, font.into());
        for line in &self.lines {
            let mut extent = SIZE::default();
            let _ = GetTextExtentPoint32W(hdc, line, &mut extent);
            self.line_widths.push(extent.cx);
        }
        SelectObject(hdc, previous);
        let _ = ReleaseDC(None, hdc);
    }

    unsafe fn place(&mut self) {
        let Some(taskbar) = taskbar_window() else {
            self.hide();
            return;
        };
        let Some(taskbar_rect) = window_rect(taskbar) else {
            self.hide();
            return;
        };
        if taskbar_rect.right <= taskbar_rect.left || taskbar_rect.bottom <= taskbar_rect.top {
            self.hide();
            return;
        }
        if taskbar_offscreen(taskbar, &taskbar_rect) {
            self.hide();
            return;
        }
        let tray = tray_window_of(taskbar).unwrap_or(taskbar);
        let tray_rect = window_rect(tray).unwrap_or(taskbar_rect);

        let horizontal = !taskbar_is_vertical();
        let (screen_x, screen_y) = if horizontal {
            let y = tray_rect.top + ((tray_rect.bottom - tray_rect.top) - self.height) / 2;
            let x = match self.config.position {
                Position::LeftOfTray => tray_rect.left - self.width - self.config.offset,
                Position::RightOfTray => tray_rect.right + self.config.offset,
            };
            (x, y)
        } else {
            let x = tray_rect.left + ((tray_rect.right - tray_rect.left) - self.width) / 2;
            let y = match self.config.position {
                Position::LeftOfTray => tray_rect.top - self.height - self.config.offset,
                Position::RightOfTray => tray_rect.bottom + self.config.offset,
            };
            (x, y)
        };

        let (x, y) = if self.parent.is_invalid() {
            (screen_x, screen_y)
        } else {
            let point = POINT {
                x: screen_x,
                y: screen_y,
            };
            let mut points = [point];
            MapWindowPoints(None, Some(self.parent), &mut points);
            (points[0].x, points[0].y)
        };

        let insert_after = if self.parent.is_invalid() {
            Some(HWND_TOPMOST)
        } else {
            Some(HWND_TOP)
        };
        let _ = SetWindowPos(
            self.hwnd,
            insert_after,
            x,
            y,
            self.width,
            self.height,
            SWP_NOACTIVATE,
        );
    }

    unsafe fn set_visible(&mut self, visible: bool) {
        if visible == self.visible {
            return;
        }
        self.visible = visible;
        let _ = ShowWindow(self.hwnd, if visible { SW_SHOWNA } else { SW_HIDE });
    }

    unsafe fn hide(&mut self) {
        self.set_visible(false);
    }

    /// Draws the meter into a 32-bit DIB and pushes it to the layered window with
    /// per-pixel alpha (no colour-key fringing, real background opacity).
    unsafe fn render(&mut self, hwnd: HWND) {
        let width = self.width.max(1);
        let height = self.height.max(1);
        let screen = GetDC(None);
        let memory = CreateCompatibleDC(Some(screen));

        let mut info = BITMAPINFO::default();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width;
        info.bmiHeader.biHeight = -height;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB.0;

        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(
            Some(memory),
            &info,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        ) {
            Ok(bitmap) if !bits.is_null() => bitmap,
            _ => {
                let _ = DeleteDC(memory);
                let _ = ReleaseDC(None, screen);
                return;
            }
        };
        let previous = SelectObject(memory, bitmap.into());

        // Text is drawn as a grayscale mask on black and turned into alpha below.
        let black = CreateSolidBrush(COLORREF(0));
        let client = RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };
        FillRect(memory, &client, black);
        let _ = DeleteObject(black.into());

        let _ = SetBkMode(memory, TRANSPARENT);
        let _ = SetTextAlign(memory, TA_BASELINE | TA_LEFT);
        SetTextColor(memory, COLORREF(0x00FF_FFFF));
        let previous_font = SelectObject(memory, self.font.into());
        if self.vertical {
            if let Some(first) = self.lines.first() {
                let _ = TextOutW(memory, PAD_Y + self.ascent, height - PAD_X, first);
            }
        } else {
            for (index, line) in self.lines.iter().enumerate() {
                let line_width = self.line_widths.get(index).copied().unwrap_or(0);
                let x = ((width - line_width) / 2).max(PAD_X);
                let baseline = PAD_Y + self.ascent + index as i32 * self.font_px;
                let _ = TextOutW(memory, x, baseline, line);
            }
        }
        SelectObject(memory, previous_font);

        let text = display::colorref(&self.config.text_color);
        let (text_r, text_g, text_b) = (
            text.0 & 0xFF,
            (text.0 >> 8) & 0xFF,
            (text.0 >> 16) & 0xFF,
        );
        let background = display::colorref(&self.config.background_color);
        let (bg_r, bg_g, bg_b) = (
            background.0 & 0xFF,
            (background.0 >> 8) & 0xFF,
            (background.0 >> 16) & 0xFF,
        );
        let opacity = if self.config.background == Background::Solid {
            self.config.background_opacity
        } else {
            0
        };

        let count = (width * height) as usize;
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u8, count * 4);
        let radius = 6.0f64.min(height as f64 / 2.0);
        for y in 0..height {
            for x in 0..width {
                let index = ((y * width + x) * 4) as usize;
                let mask = pixels[index]
                    .max(pixels[index + 1])
                    .max(pixels[index + 2]) as u32;
                let coverage = if self.config.background == Background::Solid {
                    rounded_coverage(x as f64, y as f64, width as f64, height as f64, radius)
                } else {
                    0.0
                };
                let back_alpha = (opacity as f64 * coverage) as u32;
                let inverse = 255 - mask;
                let effective_back = back_alpha * inverse / 255;
                let out_alpha = mask + effective_back;
                if out_alpha == 0 {
                    pixels[index..index + 4].fill(0);
                    continue;
                }
                let red = (text_r * mask + bg_r * effective_back) / 255;
                let green = (text_g * mask + bg_g * effective_back) / 255;
                let blue = (text_b * mask + bg_b * effective_back) / 255;
                pixels[index] = blue as u8;
                pixels[index + 1] = green as u8;
                pixels[index + 2] = red as u8;
                pixels[index + 3] = out_alpha as u8;
            }
        }

        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let size = SIZE {
            cx: width,
            cy: height,
        };
        let source = POINT { x: 0, y: 0 };
        let _ = UpdateLayeredWindow(
            hwnd,
            Some(screen),
            None,
            Some(&size),
            Some(memory),
            Some(&source),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        SelectObject(memory, previous);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        let _ = ReleaseDC(None, screen);
    }
}

fn taskbar_offscreen(taskbar: HWND, taskbar_rect: &RECT) -> bool {
    unsafe {
        let monitor = MonitorFromWindow(taskbar, MONITOR_DEFAULTTONEAREST);
        if monitor.is_invalid() {
            return true;
        }
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return false;
        }
        let area = info.rcMonitor;
        taskbar_rect.right <= area.left
            || taskbar_rect.bottom <= area.top
            || taskbar_rect.left >= area.right
            || taskbar_rect.top >= area.bottom
    }
}

unsafe fn meter_of(hwnd: HWND) -> Option<&'static mut Meter> {
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Meter;
    if pointer.is_null() {
        None
    } else {
        Some(&mut *pointer)
    }
}

unsafe extern "system" fn meter_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            let meter = create.lpCreateParams as *mut Meter;
            if meter.is_null() {
                return LRESULT(0);
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, meter as isize);
            (*meter).hwnd = hwnd;
            LRESULT(1)
        }
        WM_NCDESTROY => {
            let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Meter;
            if !pointer.is_null() {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Box::from_raw(pointer));
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            // Layered surfaces are pushed with UpdateLayeredWindow; the paint
            // message only has to validate the region.
            let mut paint = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut paint);
            let _ = EndPaint(hwnd, &paint);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn ensure_class() {
    CLASS_REGISTERED.call_once(|| unsafe {
        let Ok(instance) = GetModuleHandleW(None) else {
            return;
        };
        let class_name = wide(METER_CLASS);
        let class = WNDCLASSW {
            style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0x0003),
            lpfnWndProc: Some(meter_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: windows::Win32::Foundation::HINSTANCE(instance.0),
            hIcon: Default::default(),
            hCursor: Default::default(),
            hbrBackground: Default::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
        };
        RegisterClassW(&class);
    });
}

/// Creates the meter window and takes ownership of the [`Meter`] state.
pub unsafe fn create(shared: &Arc<Shared>) -> Option<HWND> {
    ensure_class();
    let config = shared.config.read().ok()?.clone();
    let sample = shared.sample.read().ok()?.clone();
    let lines = display::meter_lines(&config, &sample);

    let state = Box::new(Meter {
        hwnd: HWND::default(),
        parent: HWND::default(),
        config: config.clone(),
        font: HFONT(std::ptr::null_mut()),
        measure_font: HFONT(std::ptr::null_mut()),
        font_px: 16,
        ascent: 14,
        vertical: taskbar_is_vertical(),
        lines: Vec::new(),
        line_widths: Vec::new(),
        width: 140,
        height: 22,
        visible: false,
        layout_key: String::new(),
    });
    let state = Box::into_raw(state);

    let class_name = wide(METER_CLASS);
    let title = wide("NetPulse");
    let instance = match GetModuleHandleW(None) {
        Ok(instance) => instance,
        Err(_) => {
            drop(Box::from_raw(state));
            return None;
        }
    };
    let ex_style = WINDOW_EX_STYLE(
        WS_EX_LAYERED.0
            | WS_EX_TRANSPARENT.0
            | WS_EX_TOOLWINDOW.0
            | WS_EX_NOACTIVATE.0
            | WS_EX_TOPMOST.0,
    );

    let created = CreateWindowExW(
        ex_style,
        PCWSTR(class_name.as_ptr()),
        PCWSTR(title.as_ptr()),
        WS_POPUP,
        0,
        0,
        140,
        22,
        None,
        None,
        Some(windows::Win32::Foundation::HINSTANCE(instance.0)),
        Some(state as *const c_void),
    );

    let hwnd = match created {
        Ok(hwnd) => hwnd,
        Err(_) => {
            drop(Box::from_raw(state));
            return None;
        }
    };

    if config.dock_mode == DockMode::Taskbar {
        if let Some(taskbar) = taskbar_window() {
            if dock_into(hwnd, taskbar) {
                (*state).parent = taskbar;
            }
        }
    }

    (*state).apply(&config, &lines, hwnd);
    Some(hwnd)
}

/// Reparents the meter into Explorer's taskbar. Cross-process `CreateWindowExW`
/// with a foreign parent does not produce a visible child, so the window is
/// created top-level and converted afterwards.
unsafe fn dock_into(hwnd: HWND, taskbar: HWND) -> bool {
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    let child_style = (style & !WS_POPUP.0) | WS_CHILD.0 | WS_CLIPSIBLINGS.0;
    SetWindowLongPtrW(hwnd, GWL_STYLE, child_style as isize);
    let _ = SetParent(hwnd, Some(taskbar));
    if GetParent(hwnd).ok() == Some(taskbar) {
        return true;
    }
    let _ = SetParent(hwnd, None);
    SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);
    false
}

pub unsafe fn destroy(hwnd: HWND) {
    if !hwnd.is_invalid() && IsWindow(Some(hwnd)).as_bool() {
        let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
    }
}

/// Recomputes text, font, geometry and visibility for the current state.
pub unsafe fn sync(hwnd: HWND, shared: &Shared) {
    if hwnd.is_invalid() || !IsWindow(Some(hwnd)).as_bool() {
        return;
    }
    let Some(meter) = meter_of(hwnd) else {
        return;
    };
    let config = match shared.config.read() {
        Ok(config) => config.clone(),
        Err(_) => return,
    };
    let sample = match shared.sample.read() {
        Ok(sample) => sample.clone(),
        Err(_) => return,
    };
    let lines = display::meter_lines(&config, &sample);
    meter.apply(&config, &lines, hwnd);
    meter.place();
    let visible = config.visible && !lines.is_empty();
    meter.set_visible(visible);
    if visible {
        meter.render(hwnd);
    }
}

/// Coverage of a rounded rectangle at a pixel centre (0.0 ..= 1.0).
fn rounded_coverage(x: f64, y: f64, width: f64, height: f64, radius: f64) -> f64 {
    let dx = if x < radius {
        radius - x
    } else if x > width - radius {
        x - (width - radius)
    } else {
        0.0
    };
    let dy = if y < radius {
        radius - y
    } else if y > height - radius {
        y - (height - radius)
    } else {
        0.0
    };
    if dx == 0.0 && dy == 0.0 {
        return 1.0;
    }
    let distance = (dx * dx + dy * dy).sqrt();
    (radius - distance + 0.5).clamp(0.0, 1.0)
}
