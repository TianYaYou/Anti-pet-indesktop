use super::Pixmap;
#[cfg(target_os = "macos")]
use std::ffi::c_void;

/// Universal text alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlignment {
    Left = 0,
    Center = 1,
    Right = 2,
}

/// Renders CJK / English text into the `pixmap` software buffer using native OS typography.
/// macOS: CoreText / NSString / AppKit font rendering directly into the Pixmap bitmap.
/// Windows: Win32 GDI DrawTextW with system typography.
#[allow(clippy::too_many_arguments)]
pub fn draw_text_into_pixmap(
    pixmap: &mut Pixmap,
    text: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    font_size_px: f32,
    font_weight: f64,
    rgb: [u8; 3],
    alpha: f32,
    alignment: TextAlignment,
) {
    if text.is_empty() || w <= 1.0 || h <= 1.0 || alpha <= 0.01 {
        return;
    }

    #[cfg(target_os = "macos")]
    unsafe {
        macos_text::draw_text_into_pixmap(
            pixmap,
            text,
            x,
            y,
            w,
            h,
            font_size_px,
            font_weight,
            rgb,
            alpha,
            alignment as isize,
        );
    }

    #[cfg(target_os = "windows")]
    unsafe {
        windows_text::draw_text_into_pixmap(
            pixmap,
            text,
            x,
            y,
            w,
            h,
            font_size_px,
            font_weight,
            rgb,
            alpha,
            alignment as isize,
        );
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (pixmap, text, x, y, w, h, font_size_px, font_weight, rgb, alpha, alignment);
    }
}

#[cfg(target_os = "macos")]
pub mod macos_text {
    use super::{c_void, Pixmap};
    use std::ffi::CString;

    const K_CG_IMAGE_ALPHA_PREMULTIPLIED_LAST: u32 = 1;
    const K_CG_BITMAP_BYTE_ORDER_32_BIG: u32 = 4 << 12;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NSPoint {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NSSize {
        width: f64,
        height: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NSRect {
        origin: NSPoint,
        size: NSSize,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGAffineTransform {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        tx: f64,
        ty: f64,
    }

    extern "C" {
        fn objc_getClass(name: *const u8) -> *mut c_void;
        fn sel_registerName(name: *const u8) -> *mut c_void;
        fn objc_msgSend();
        fn objc_autoreleasePoolPush() -> *mut c_void;
        fn objc_autoreleasePoolPop(pool: *mut c_void);

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
        fn CGContextRelease(context: *mut c_void);
        fn CGContextTranslateCTM(c: *mut c_void, tx: f64, ty: f64);
        fn CGContextScaleCTM(c: *mut c_void, sx: f64, sy: f64);
        fn CGContextSetTextMatrix(c: *mut c_void, t: CGAffineTransform);
        fn CGContextSetAllowsAntialiasing(c: *mut c_void, allows: bool);
        fn CGContextSetShouldAntialias(c: *mut c_void, should: bool);
        fn CGContextSetAllowsFontSmoothing(c: *mut c_void, allows: bool);
        fn CGContextSetShouldSmoothFonts(c: *mut c_void, should: bool);
        fn CGContextSetAllowsFontSubpixelPositioning(c: *mut c_void, allows: bool);
        fn CGContextSetShouldSubpixelPositionFonts(c: *mut c_void, should: bool);
    }

    #[inline]
    unsafe fn sel(name: &[u8]) -> *mut c_void {
        sel_registerName(name.as_ptr())
    }

    #[allow(clippy::too_many_arguments)]
    pub unsafe fn draw_text_into_pixmap(
        pixmap: &mut Pixmap,
        text: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        font_size_px: f32,
        font_weight: f64,
        rgb: [u8; 3],
        alpha: f32,
        alignment: isize,
    ) {
        if text.is_empty() || w <= 1.0 || h <= 1.0 || alpha <= 0.01 {
            return;
        }
        let Ok(c_str) = CString::new(text) else {
            return;
        };

        let color_space = CGColorSpaceCreateDeviceRGB();
        if color_space.is_null() {
            return;
        }

        let bitmap_info = K_CG_IMAGE_ALPHA_PREMULTIPLIED_LAST | K_CG_BITMAP_BYTE_ORDER_32_BIG;
        let cg_ctx = CGBitmapContextCreate(
            pixmap.data.as_mut_ptr() as *mut c_void,
            pixmap.width as usize,
            pixmap.height as usize,
            8,
            (pixmap.width as usize) * 4,
            color_space,
            bitmap_info,
        );

        if cg_ctx.is_null() {
            CGColorSpaceRelease(color_space);
            return;
        }

        let pool = objc_autoreleasePoolPush();

        CGContextSetAllowsAntialiasing(cg_ctx, true);
        CGContextSetShouldAntialias(cg_ctx, true);
        CGContextSetAllowsFontSmoothing(cg_ctx, false);
        CGContextSetShouldSmoothFonts(cg_ctx, false);
        CGContextSetAllowsFontSubpixelPositioning(cg_ctx, true);
        CGContextSetShouldSubpixelPositionFonts(cg_ctx, true);

        CGContextTranslateCTM(cg_ctx, 0.0, pixmap.height as f64);
        CGContextScaleCTM(cg_ctx, 1.0, -1.0);
        CGContextSetTextMatrix(
            cg_ctx,
            CGAffineTransform {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: -1.0,
                tx: 0.0,
                ty: 0.0,
            },
        );

        let ns_gfx_cls = objc_getClass(b"NSGraphicsContext\0".as_ptr());
        let ns_str_cls = objc_getClass(b"NSString\0".as_ptr());
        let ns_font_cls = objc_getClass(b"NSFont\0".as_ptr());
        let ns_color_cls = objc_getClass(b"NSColor\0".as_ptr());
        let ns_dict_cls = objc_getClass(b"NSMutableDictionary\0".as_ptr());
        let ns_pstyle_cls = objc_getClass(b"NSMutableParagraphStyle\0".as_ptr());

        if !ns_gfx_cls.is_null()
            && !ns_str_cls.is_null()
            && !ns_font_cls.is_null()
            && !ns_color_cls.is_null()
            && !ns_dict_cls.is_null()
        {
            let msg_gfx: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, i8) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_void: unsafe extern "C" fn(*mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_set_ctx: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());

            let gfx_ctx = msg_gfx(
                ns_gfx_cls,
                sel(b"graphicsContextWithCGContext:flipped:\0"),
                cg_ctx,
                1,
            );

            if !gfx_ctx.is_null() {
                msg_void(ns_gfx_cls, sel(b"saveGraphicsState\0"));
                msg_set_ctx(ns_gfx_cls, sel(b"setCurrentContext:\0"), gfx_ctx);

                let msg_utf8: unsafe extern "C" fn(*mut c_void, *mut c_void, *const i8) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let ns_text = msg_utf8(ns_str_cls, sel(b"stringWithUTF8String:\0"), c_str.as_ptr());

                let msg_font: unsafe extern "C" fn(*mut c_void, *mut c_void, f64, f64) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let font = msg_font(
                    ns_font_cls,
                    sel(b"systemFontOfSize:weight:\0"),
                    font_size_px as f64,
                    font_weight,
                );

                let msg_color: unsafe extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    f64,
                    f64,
                    f64,
                    f64,
                ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const ());
                let color = msg_color(
                    ns_color_cls,
                    sel(b"colorWithSRGBRed:green:blue:alpha:\0"),
                    (rgb[0] as f64) / 255.0,
                    (rgb[1] as f64) / 255.0,
                    (rgb[2] as f64) / 255.0,
                    alpha as f64,
                );

                let msg_dict_new: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let attrs = msg_dict_new(ns_dict_cls, sel(b"dictionary\0"));

                let key_font = msg_utf8(
                    ns_str_cls,
                    sel(b"stringWithUTF8String:\0"),
                    b"NSFont\0".as_ptr() as *const i8,
                );
                let key_color = msg_utf8(
                    ns_str_cls,
                    sel(b"stringWithUTF8String:\0"),
                    b"NSColor\0".as_ptr() as *const i8,
                );

                let msg_set_obj: unsafe extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    *mut c_void,
                    *mut c_void,
                ) = std::mem::transmute(objc_msgSend as *const ());
                if !attrs.is_null() {
                    if !font.is_null() && !key_font.is_null() {
                        msg_set_obj(attrs, sel(b"setObject:forKey:\0"), font, key_font);
                    }
                    if !color.is_null() && !key_color.is_null() {
                        msg_set_obj(attrs, sel(b"setObject:forKey:\0"), color, key_color);
                    }

                    if !ns_pstyle_cls.is_null() {
                        let pstyle_default = msg_dict_new(ns_pstyle_cls, sel(b"defaultParagraphStyle\0"));
                        if !pstyle_default.is_null() {
                            let pstyle_mut = msg_dict_new(pstyle_default, sel(b"mutableCopy\0"));
                            if !pstyle_mut.is_null() {
                                let msg_set_align: unsafe extern "C" fn(*mut c_void, *mut c_void, isize) =
                                    std::mem::transmute(objc_msgSend as *const ());
                                msg_set_align(pstyle_mut, sel(b"setAlignment:\0"), alignment);
                                let key_pstyle = msg_utf8(
                                    ns_str_cls,
                                    sel(b"stringWithUTF8String:\0"),
                                    b"NSParagraphStyle\0".as_ptr() as *const i8,
                                );
                                if !key_pstyle.is_null() {
                                    msg_set_obj(attrs, sel(b"setObject:forKey:\0"), pstyle_mut, key_pstyle);
                                }
                                msg_void(pstyle_mut, sel(b"release\0"));
                            }
                        }
                    }
                }

                if !ns_text.is_null() {
                    let draw_rect = NSRect {
                        origin: NSPoint {
                            x: x as f64,
                            y: y as f64,
                        },
                        size: NSSize {
                            width: w as f64,
                            height: h as f64,
                        },
                    };
                    let msg_draw_rect: unsafe extern "C" fn(
                        *mut c_void,
                        *mut c_void,
                        NSRect,
                        *mut c_void,
                    ) = std::mem::transmute(objc_msgSend as *const ());
                    msg_draw_rect(
                        ns_text,
                        sel(b"drawInRect:withAttributes:\0"),
                        draw_rect,
                        attrs,
                    );
                }

                msg_void(ns_gfx_cls, sel(b"restoreGraphicsState\0"));
            }
        }

        CGContextRelease(cg_ctx);
        CGColorSpaceRelease(color_space);
        objc_autoreleasePoolPop(pool);
    }
}

#[cfg(target_os = "windows")]
pub mod windows_text {
    use super::Pixmap;
    use std::ffi::c_void;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{COLORREF, RECT};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, DrawTextW,
        SelectObject, SetBkMode, SetTextColor, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        DIB_RGB_COLORS, DT_CENTER, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_WORDBREAK,
        TRANSPARENT,
    };

    #[allow(clippy::too_many_arguments)]
    pub unsafe fn draw_text_into_pixmap(
        pixmap: &mut Pixmap,
        text: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        font_size_px: f32,
        font_weight: f64,
        rgb: [u8; 3],
        alpha: f32,
        alignment: isize, // 0 = Left, 1 = Center, 2 = Right
    ) {
        if text.is_empty() || w <= 1.0 || h <= 1.0 || alpha <= 0.01 {
            return;
        }

        let bw = (w.ceil() as i32).max(1);
        let bh = (h.ceil() as i32).max(1);

        let hdc = CreateCompatibleDC(None);
        if hdc.is_invalid() {
            return;
        }

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: bw,
                biHeight: -bh, // Top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        if let Ok(dib) = CreateDIBSection(
            Some(hdc),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits_ptr,
            None,
            0,
        ) {
            if !bits_ptr.is_null() {
                std::ptr::write_bytes(bits_ptr, 0, (bw * bh * 4) as usize);

                let old_bmp = SelectObject(hdc, dib.into());

                let font_name: Vec<u16> = "Microsoft YaHei UI\0".encode_utf16().collect();
                let weight = if font_weight >= 0.62 {
                    700 // FW_BOLD (only for percentage numbers)
                } else {
                    400 // FW_NORMAL (native un-synthesized YaHei glyphs, zero 1px faux bold doubling)
                };

                let hfont = CreateFontW(
                    -(font_size_px.round() as i32),
                    0,
                    0,
                    0,
                    weight,
                    0,
                    0,
                    0,
                    windows::Win32::Graphics::Gdi::FONT_CHARSET(1), // DEFAULT_CHARSET
                    windows::Win32::Graphics::Gdi::FONT_OUTPUT_PRECISION(0), // OUT_DEFAULT_PRECIS
                    windows::Win32::Graphics::Gdi::FONT_CLIP_PRECISION(0), // CLIP_DEFAULT_PRECIS
                    windows::Win32::Graphics::Gdi::FONT_QUALITY(4), // ANTIALIASED_QUALITY: Crisp grayscale anti-aliasing
                    0,
                    PCWSTR(font_name.as_ptr()),
                );
                let old_font = SelectObject(hdc, hfont.into());

                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, COLORREF(0x00FFFFFF)); // White text

                let mut text_w: Vec<u16> = text.encode_utf16().collect();
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: bw,
                    bottom: bh,
                };
                let align_flag = match alignment {
                    1 => DT_CENTER,
                    2 => DT_RIGHT,
                    _ => DT_LEFT,
                };
                let flags = DT_NOPREFIX | DT_WORDBREAK | align_flag;
                DrawTextW(hdc, &mut text_w, &mut rect, flags);

                let dst = bits_ptr as *const u8;
                let alpha_mult = (alpha * 255.0).round().clamp(0.0, 255.0) as u32;

                for py in 0..bh {
                    let dest_y = (y as i32) + py;
                    if dest_y < 0 || dest_y >= pixmap.height as i32 {
                        continue;
                    }
                    for px in 0..bw {
                        let dest_x = (x as i32) + px;
                        if dest_x < 0 || dest_x >= pixmap.width as i32 {
                            continue;
                        }

                        let o = ((py * bw + px) * 4) as usize;
                        let b = *dst.add(o) as u32;
                        let g = *dst.add(o + 1) as u32;
                        let r = *dst.add(o + 2) as u32;

                        // ANTIALIASED_QUALITY produces clean grayscale pixels where r == g == b.
                        let intensity = (r + g + b) / 3;
                        if intensity > 0 {
                            let glyph_a = ((intensity * alpha_mult) / 255) as u8;
                            if glyph_a > 0 {
                                pixmap.blend_pixel(
                                    dest_x,
                                    dest_y,
                                    rgb[0],
                                    rgb[1],
                                    rgb[2],
                                    glyph_a,
                                );
                            }
                        }
                    }
                }

                SelectObject(hdc, old_font);
                let _ = DeleteObject(hfont.into());
                SelectObject(hdc, old_bmp);
            }
            let _ = DeleteObject(dib.into());
        }

        let _ = DeleteDC(hdc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_rendering() {
        let mut pixmap = Pixmap::new(200, 200);
        draw_text_into_pixmap(
            &mut pixmap,
            "测试文字 Anti-Pet",
            10.0,
            10.0,
            150.0,
            30.0,
            14.0,
            0.4,
            [255, 255, 255],
            1.0,
            TextAlignment::Left,
        );
    }
}
