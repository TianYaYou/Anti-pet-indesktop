pub mod bubble;
pub mod sprite;

#[cfg(target_os = "macos")]
use std::ffi::c_void;

/// Premultiplied RGBA8888 software pixel buffer (`[R, G, B, A]` per pixel in little-endian `u32`).
/// Designed for zero-GPU, low-memory (< 500 KB for compact widget) CPU 2D rasterization.
#[derive(Clone)]
pub struct Pixmap {
    pub width: u32,
    pub height: u32,
    /// Stored as premultiplied `[R, G, B, A]` bytes (4 bytes per pixel).
    pub data: Vec<u8>,
}

impl Pixmap {
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        Self {
            width,
            height,
            data: vec![0u8; len],
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        let len = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        self.data.resize(len, 0);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    /// Returns the alpha value (0..=255) at physical pixel coordinates `(px, py)`.
    #[inline]
    pub fn alpha_at_physical(&self, px: i32, py: i32) -> u8 {
        if px < 0 || py < 0 || px >= self.width as i32 || py >= self.height as i32 {
            return 0;
        }
        let idx = ((py as usize) * (self.width as usize) + (px as usize)) * 4;
        self.data.get(idx + 3).copied().unwrap_or(0)
    }

    /// Blends a straight (unpremultiplied) RGBA color `(r, g, b, a)` onto `(x, y)` using Porter-Duff `SrcOver`.
    #[inline]
    pub fn blend_pixel(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
        if a == 0 || x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        let sa = a as u32;
        let sr = (r as u32 * sa + 127) / 255;
        let sg = (g as u32 * sa + 127) / 255;
        let sb = (b as u32 * sa + 127) / 255;

        if sa == 255 {
            self.data[idx] = sr as u8;
            self.data[idx + 1] = sg as u8;
            self.data[idx + 2] = sb as u8;
            self.data[idx + 3] = 255;
            return;
        }

        let inv_sa = 255 - sa;
        let dr = self.data[idx] as u32;
        let dg = self.data[idx + 1] as u32;
        let db = self.data[idx + 2] as u32;
        let da = self.data[idx + 3] as u32;

        self.data[idx] = (sr + (dr * inv_sa + 127) / 255).min(255) as u8;
        self.data[idx + 1] = (sg + (dg * inv_sa + 127) / 255).min(255) as u8;
        self.data[idx + 2] = (sb + (db * inv_sa + 127) / 255).min(255) as u8;
        self.data[idx + 3] = (sa + (da * inv_sa + 127) / 255).min(255) as u8;
    }

    /// Blends an already-premultiplied RGBA pixel `(pr, pg, pb, pa)` onto `(x, y)`.
    #[inline]
    pub fn blend_premultiplied(&mut self, x: i32, y: i32, pr: u8, pg: u8, pb: u8, pa: u8) {
        if pa == 0 || x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        if pa == 255 {
            self.data[idx] = pr;
            self.data[idx + 1] = pg;
            self.data[idx + 2] = pb;
            self.data[idx + 3] = 255;
            return;
        }
        let inv_sa = 255 - (pa as u32);
        let dr = self.data[idx] as u32;
        let dg = self.data[idx + 1] as u32;
        let db = self.data[idx + 2] as u32;
        let da = self.data[idx + 3] as u32;

        self.data[idx] = (pr as u32 + (dr * inv_sa + 127) / 255).min(255) as u8;
        self.data[idx + 1] = (pg as u32 + (dg * inv_sa + 127) / 255).min(255) as u8;
        self.data[idx + 2] = (pb as u32 + (db * inv_sa + 127) / 255).min(255) as u8;
        self.data[idx + 3] = (pa as u32 + (da * inv_sa + 127) / 255).min(255) as u8;
    }

    /// Draws an anti-aliased SDF rounded rectangle with optional border.
    pub fn fill_rounded_rect_sdf(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        fill_rgba: [u8; 4],
        border_width: f32,
        border_rgba: [u8; 4],
    ) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let r = radius.clamp(0.0, (w.min(h)) * 0.5);
        let half_w = w * 0.5;
        let half_h = h * 0.5;
        let cx = x + half_w;
        let cy = y + half_h;

        let min_x = (x - border_width - 2.0).floor().max(0.0) as i32;
        let max_x = (x + w + border_width + 2.0)
            .ceil()
            .min(self.width as f32 - 1.0) as i32;
        let min_y = (y - border_width - 2.0).floor().max(0.0) as i32;
        let max_y = (y + h + border_width + 2.0)
            .ceil()
            .min(self.height as f32 - 1.0) as i32;

        let bx = half_w - r;
        let by = half_h - r;

        for py in min_y..=max_y {
            let fy = py as f32 + 0.5 - cy;
            let qy = fy.abs() - by;
            for px in min_x..=max_x {
                let fx = px as f32 + 0.5 - cx;
                let qx = fx.abs() - bx;

                let out_x = qx.max(0.0);
                let out_y = qy.max(0.0);
                let dist_outside = (out_x * out_x + out_y * out_y).sqrt();
                let dist_inside = qx.max(qy).min(0.0);
                let sdf = dist_outside + dist_inside - r;

                // Outer edge coverage (smoothstep over 1.1px for crisp anti-aliasing)
                let outer_cov = (0.5 - sdf).clamp(0.0, 1.0);
                if outer_cov <= 0.0 {
                    continue;
                }

                if border_width > 0.0 && border_rgba[3] > 0 {
                    let inner_sdf = sdf + border_width;
                    let inner_cov = (0.5 - inner_sdf).clamp(0.0, 1.0);
                    let border_cov = (outer_cov - inner_cov).max(0.0);

                    if inner_cov > 0.0 && fill_rgba[3] > 0 {
                        let fa = ((fill_rgba[3] as f32) * inner_cov).round() as u8;
                        self.blend_pixel(px, py, fill_rgba[0], fill_rgba[1], fill_rgba[2], fa);
                    }
                    if border_cov > 0.0 {
                        let ba = ((border_rgba[3] as f32) * border_cov).round() as u8;
                        self.blend_pixel(
                            px,
                            py,
                            border_rgba[0],
                            border_rgba[1],
                            border_rgba[2],
                            ba,
                        );
                    }
                } else if fill_rgba[3] > 0 {
                    let fa = ((fill_rgba[3] as f32) * outer_cov).round() as u8;
                    self.blend_pixel(px, py, fill_rgba[0], fill_rgba[1], fill_rgba[2], fa);
                }
            }
        }
    }

    /// Draws a soft Gaussian-approximated SDF drop shadow for a rounded rectangle.
    pub fn draw_rounded_rect_shadow(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        blur: f32,
        offset_y: f32,
        shadow_rgba: [u8; 4],
    ) {
        if w <= 0.0 || h <= 0.0 || blur <= 0.0 || shadow_rgba[3] == 0 {
            return;
        }
        let r = radius.clamp(0.0, (w.min(h)) * 0.5);
        let half_w = w * 0.5;
        let half_h = h * 0.5;
        let cx = x + half_w;
        let cy = y + offset_y + half_h;

        let min_x = (cx - half_w - blur - 2.0).floor().max(0.0) as i32;
        let max_x = (cx + half_w + blur + 2.0)
            .ceil()
            .min(self.width as f32 - 1.0) as i32;
        let min_y = (cy - half_h - blur - 2.0).floor().max(0.0) as i32;
        let max_y = (cy + half_h + blur + 2.0)
            .ceil()
            .min(self.height as f32 - 1.0) as i32;

        let bx = half_w - r;
        let by = half_h - r;
        let inv_blur = 1.0 / blur.max(1.0);

        for py in min_y..=max_y {
            let fy = py as f32 + 0.5 - cy;
            let qy = fy.abs() - by;
            for px in min_x..=max_x {
                let fx = px as f32 + 0.5 - cx;
                let qx = fx.abs() - bx;
                let out_x = qx.max(0.0);
                let out_y = qy.max(0.0);
                let sdf = (out_x * out_x + out_y * out_y).sqrt() + qx.max(qy).min(0.0) - r;
                if sdf >= blur {
                    continue;
                }
                let t = (1.0 - (sdf.max(0.0) * inv_blur)).clamp(0.0, 1.0);
                let falloff = t * t * (3.0 - 2.0 * t);
                let sa = ((shadow_rgba[3] as f32) * falloff).round() as u8;
                if sa > 0 {
                    self.blend_pixel(px, py, shadow_rgba[0], shadow_rgba[1], shadow_rgba[2], sa);
                }
            }
        }
    }

    /// Draws an anti-aliased SDF rounded rectangle with horizontal linear gradient fill from start_rgba to end_rgba.
    pub fn fill_rounded_rect_gradient_sdf(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        start_rgba: [u8; 4],
        end_rgba: [u8; 4],
    ) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let r = radius.clamp(0.0, (w.min(h)) * 0.5);
        let half_w = w * 0.5;
        let half_h = h * 0.5;
        let cx = x + half_w;
        let cy = y + half_h;

        let min_x = (x - 2.0).floor().max(0.0) as i32;
        let max_x = (x + w + 2.0)
            .ceil()
            .min(self.width as f32 - 1.0) as i32;
        let min_y = (y - 2.0).floor().max(0.0) as i32;
        let max_y = (y + h + 2.0)
            .ceil()
            .min(self.height as f32 - 1.0) as i32;

        let bx = half_w - r;
        let by = half_h - r;
        let inv_w = 1.0 / w.max(1.0);

        for py in min_y..=max_y {
            let fy = py as f32 + 0.5 - cy;
            let qy = fy.abs() - by;
            for px in min_x..=max_x {
                let fx = px as f32 + 0.5 - cx;
                let qx = fx.abs() - bx;

                let out_x = qx.max(0.0);
                let out_y = qy.max(0.0);
                let dist_outside = (out_x * out_x + out_y * out_y).sqrt();
                let dist_inside = qx.max(qy).min(0.0);
                let sdf = dist_outside + dist_inside - r;

                let outer_cov = (0.5 - sdf).clamp(0.0, 1.0);
                if outer_cov <= 0.0 {
                    continue;
                }

                let t = ((px as f32 + 0.5 - x) * inv_w).clamp(0.0, 1.0);
                let inv_t = 1.0 - t;
                let r_col = (start_rgba[0] as f32 * inv_t + end_rgba[0] as f32 * t).round() as u8;
                let g_col = (start_rgba[1] as f32 * inv_t + end_rgba[1] as f32 * t).round() as u8;
                let b_col = (start_rgba[2] as f32 * inv_t + end_rgba[2] as f32 * t).round() as u8;
                let a_base = start_rgba[3] as f32 * inv_t + end_rgba[3] as f32 * t;
                let fa = (a_base * outer_cov).round() as u8;

                if fa > 0 {
                    self.blend_pixel(px, py, r_col, g_col, b_col, fa);
                }
            }
        }
    }

    /// Draws an anti-aliased SDF circle with optional border.
    pub fn fill_circle_sdf(
        &mut self,
        cx: f32,
        cy: f32,
        radius: f32,
        fill_rgba: [u8; 4],
        border_width: f32,
        border_rgba: [u8; 4],
    ) {
        self.fill_rounded_rect_sdf(
            cx - radius,
            cy - radius,
            radius * 2.0,
            radius * 2.0,
            radius,
            fill_rgba,
            border_width,
            border_rgba,
        );
    }

    /// Draws a soft elliptical ground shadow beneath the cat sprite.
    pub fn draw_ground_ellipse_shadow(
        &mut self,
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        max_alpha: u8,
    ) {
        if rx <= 1.0 || ry <= 1.0 || max_alpha == 0 {
            return;
        }
        let min_x = (cx - rx - 2.0).floor().max(0.0) as i32;
        let max_x = (cx + rx + 2.0).ceil().min(self.width as f32 - 1.0) as i32;
        let min_y = (cy - ry - 2.0).floor().max(0.0) as i32;
        let max_y = (cy + ry + 2.0).ceil().min(self.height as f32 - 1.0) as i32;

        let inv_rx = 1.0 / rx;
        let inv_ry = 1.0 / ry;

        for py in min_y..=max_y {
            let dy = (py as f32 + 0.5 - cy) * inv_ry;
            let dy2 = dy * dy;
            if dy2 >= 1.0 {
                continue;
            }
            for px in min_x..=max_x {
                let dx = (px as f32 + 0.5 - cx) * inv_rx;
                let d2 = dx * dx + dy2;
                if d2 >= 1.0 {
                    continue;
                }
                let t = 1.0 - d2.sqrt();
                let smooth = t * t;
                let a = ((max_alpha as f32) * smooth).round() as u8;
                if a > 0 {
                    self.blend_pixel(px, py, 15, 23, 42, a);
                }
            }
        }
    }
}

/// Cross-platform software framebuffer presenter that blits a transparent `Pixmap`
/// directly to the native OS window without WebKit or GPU shaders.
pub struct SoftwareSurface {
    #[cfg(target_os = "macos")]
    ns_window: *mut c_void,
    #[cfg(target_os = "windows")]
    hwnd: isize,
}

impl SoftwareSurface {
    #[cfg(target_os = "macos")]
    pub fn new_macos(ns_window: *mut c_void) -> Self {
        unsafe {
            macos_surface::configure_transparent_layer_backed_view(ns_window);
        }
        Self { ns_window }
    }

    #[cfg(target_os = "windows")]
    pub fn new_windows(hwnd: isize) -> Self {
        Self { hwnd }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    pub fn new_unix() -> Self {
        Self {}
    }

    /// Presents the premultiplied RGBA `Pixmap` to the window.
    pub fn present(&self, pixmap: &Pixmap, _scale_factor: f64) {
        if pixmap.width == 0 || pixmap.height == 0 {
            return;
        }
        #[cfg(target_os = "macos")]
        unsafe {
            macos_surface::present_pixmap_to_nswindow(self.ns_window, pixmap);
        }

        #[cfg(target_os = "windows")]
        unsafe {
            windows_surface::present_pixmap_to_hwnd(self.hwnd, pixmap);
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_surface {
    use super::{c_void, Pixmap};

    const K_CG_IMAGE_ALPHA_PREMULTIPLIED_LAST: u32 = 1;
    const K_CG_BITMAP_BYTE_ORDER_32_BIG: u32 = 4 << 12;

    extern "C" {
        fn objc_getClass(name: *const u8) -> *mut c_void;
        fn sel_registerName(name: *const u8) -> *mut c_void;
        fn objc_msgSend();

        fn CGColorSpaceCreateDeviceRGB() -> *mut c_void;
        fn CGColorSpaceRelease(space: *mut c_void);
        fn CGBitmapContextCreate(
            data: *mut c_void,
            width: usize,
            height: usize,
            bits_per_component: usize,
            bytes_per_row: usize,
            space: *mut c_void,
            bitmap_info: u32,
        ) -> *mut c_void;
        fn CGBitmapContextCreateImage(context: *mut c_void) -> *mut c_void;
        fn CGContextRelease(context: *mut c_void);
        fn CGImageRelease(image: *mut c_void);
    }

    #[inline]
    unsafe fn sel(name: &[u8]) -> *mut c_void {
        sel_registerName(name.as_ptr())
    }

    pub unsafe fn configure_transparent_layer_backed_view(ns_window: *mut c_void) {
        if ns_window.is_null() {
            return;
        }
        let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_void_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_void_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
            std::mem::transmute(objc_msgSend as *const ());

        // Ensure NSWindow is non-opaque with clearColor background
        msg_void_bool(ns_window, sel(b"setOpaque:\0"), 0);
        let color_cls = objc_getClass(b"NSColor\0".as_ptr());
        if !color_cls.is_null() {
            let clear_color = msg_ptr(color_cls, sel(b"clearColor\0"));
            if !clear_color.is_null() {
                msg_void_ptr(ns_window, sel(b"setBackgroundColor:\0"), clear_color);
            }
        }

        let content_view = msg_ptr(ns_window, sel(b"contentView\0"));
        if !content_view.is_null() {
            msg_void_bool(content_view, sel(b"setWantsLayer:\0"), 1);
            let layer = msg_ptr(content_view, sel(b"layer\0"));
            if !layer.is_null() {
                msg_void_bool(layer, sel(b"setOpaque:\0"), 0);
            }
        }
    }

    pub unsafe fn present_pixmap_to_nswindow(ns_window: *mut c_void, pixmap: &Pixmap) {
        if ns_window.is_null() {
            return;
        }
        let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_void_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_void_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
            std::mem::transmute(objc_msgSend as *const ());

        let content_view = msg_ptr(ns_window, sel(b"contentView\0"));
        if content_view.is_null() {
            return;
        }
        msg_void_bool(content_view, sel(b"setWantsLayer:\0"), 1);
        let layer = msg_ptr(content_view, sel(b"layer\0"));
        if layer.is_null() {
            return;
        }
        msg_void_bool(layer, sel(b"setOpaque:\0"), 0);

        let color_space = CGColorSpaceCreateDeviceRGB();
        if color_space.is_null() {
            return;
        }

        let bitmap_info = K_CG_IMAGE_ALPHA_PREMULTIPLIED_LAST | K_CG_BITMAP_BYTE_ORDER_32_BIG;
        let ctx = CGBitmapContextCreate(
            pixmap.data.as_ptr() as *mut c_void,
            pixmap.width as usize,
            pixmap.height as usize,
            8,
            (pixmap.width as usize) * 4,
            color_space,
            bitmap_info,
        );

        if !ctx.is_null() {
            let cg_image = CGBitmapContextCreateImage(ctx);
            if !cg_image.is_null() {
                let ca_trans_cls = objc_getClass(b"CATransaction\0".as_ptr());
                if !ca_trans_cls.is_null() {
                    let msg_void: unsafe extern "C" fn(*mut c_void, *mut c_void) =
                        std::mem::transmute(objc_msgSend as *const ());
                    msg_void(ca_trans_cls, sel(b"begin\0"));
                    msg_void_bool(ca_trans_cls, sel(b"setDisableActions:\0"), 1);
                    msg_void_ptr(layer, sel(b"setContents:\0"), cg_image);
                    msg_void(ca_trans_cls, sel(b"commit\0"));
                } else {
                    msg_void_ptr(layer, sel(b"setContents:\0"), cg_image);
                }
                CGImageRelease(cg_image);
            }
            CGContextRelease(ctx);
        }
        CGColorSpaceRelease(color_space);
    }
}

#[cfg(target_os = "windows")]
mod windows_surface {
    use super::Pixmap;
    use windows::Win32::Foundation::{ HWND, POINT, SIZE };
    use windows::Win32::Graphics::Gdi::{
        AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
        CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC,
        ReleaseDC, SelectObject,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongW, SetWindowLongW, ULW_ALPHA, UpdateLayeredWindow, WS_EX_LAYERED,
    };

    pub unsafe fn present_pixmap_to_hwnd(hwnd_isize: isize, pixmap: &Pixmap) {
        let hwnd = HWND(hwnd_isize as *mut _);
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        if (ex_style & (WS_EX_LAYERED.0 as i32)) == 0 {
            SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | (WS_EX_LAYERED.0 as i32));
        }

        let screen_dc = GetDC(None);
        let mem_dc = CreateCompatibleDC(Some(screen_dc));

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: pixmap.width as i32,
                biHeight: -(pixmap.height as i32), // Top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        if let Ok(dib) = CreateDIBSection(
            Some(mem_dc),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits_ptr,
            None,
            0,
        ) {
            if !bits_ptr.is_null() {
                let total_px = (pixmap.width as usize) * (pixmap.height as usize);
                let dst = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, total_px * 4);
                // Convert RGBA premultiplied -> BGRA premultiplied for Win32 UpdateLayeredWindow
                for i in 0..total_px {
                    let o = i * 4;
                    dst[o] = pixmap.data[o + 2];     // B
                    dst[o + 1] = pixmap.data[o + 1]; // G
                    dst[o + 2] = pixmap.data[o];     // R
                    dst[o + 3] = pixmap.data[o + 3]; // A
                }

                let old_obj = SelectObject(mem_dc, dib.into());
                let size = SIZE {
                    cx: pixmap.width as i32,
                    cy: pixmap.height as i32,
                };
                let pt_src = POINT { x: 0, y: 0 };
                let blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                };
                let _ = UpdateLayeredWindow(
                    hwnd,
                    Some(screen_dc),
                    None,
                    Some(&size),
                    Some(mem_dc),
                    Some(&pt_src),
                    None,
                    Some(&blend),
                    ULW_ALPHA,
                );
                SelectObject(mem_dc, old_obj);
            }
            let _ = DeleteObject(dib.into());
        }

        let _ = DeleteDC(mem_dc);
        ReleaseDC(None, screen_dc);
    }
}
