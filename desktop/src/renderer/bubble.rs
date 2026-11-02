use super::Pixmap;
use crate::hit_test::LogicalRect;
#[cfg(target_os = "macos")]
use std::ffi::c_void;

/// Cartoon thought-cloud bubble (`o -> O -> cloud card`) rendered purely in 2D CPU software framebuffer.
/// Includes automatic screen-edge clamping/flipping and live 5h + weekly Antigravity quota progress bars.
#[derive(Clone, Debug)]
pub struct ThoughtBubble {
    pub visible: bool,
    pub text: String,
    pub show_quota: bool,
    pub five_hour_pct: f32,
    pub weekly_pct: f32,
    pub quota_meta: String,
    pub opacity: f32,
}

impl Default for ThoughtBubble {
    fn default() -> Self {
        Self {
            visible: true,
            text: "我才不是美版豆包，哼！".to_string(),
            show_quota: true,
            five_hour_pct: 100.0,
            weekly_pct: 100.0,
            quota_meta: "Antigravity 实时额度已同步".to_string(),
            opacity: 1.0,
        }
    }
}

impl ThoughtBubble {
    /// Renders the cartoon thought cloud bubble (`o -> O -> cloud`) above or beside the cat,
    /// automatically clamping/shifting within screen boundaries so it never gets clipped.
    /// Returns the interactive logical bounding rectangle of the bubble card.
    pub fn draw(
        &self,
        pixmap: &mut Pixmap,
        cat_anchor_x: f32,
        cat_top_y: f32,
        win_x: f64,
        win_y: f64,
        mon_w: f64,
        dpi_scale: f32,
    ) -> Option<LogicalRect> {
        if !self.visible || self.opacity <= 0.02 {
            return None;
        }

        let alpha_scale = self.opacity.clamp(0.0, 1.0);
        let char_count = self.text.chars().count();
        let text_lines = ((char_count as f32) / 14.0).ceil().max(1.0) as usize;

        let card_w: f32 = 220.0;
        let text_block_h: f32 = (text_lines as f32) * 18.0 + 16.0;
        let quota_block_h: f32 = if self.show_quota { 54.0 } else { 0.0 };
        let card_h: f32 = (text_block_h + quota_block_h + 10.0).clamp(48.0, 160.0);

        let win_logical_w = (pixmap.width as f32) / dpi_scale.max(1.0);

        // Ideal centered X above the cat, clamped inside both window and monitor bounds
        let mut card_x = (cat_anchor_x - card_w * 0.5).clamp(8.0, (win_logical_w - card_w - 8.0).max(8.0));

        // Screen-edge horizontal shift if the window itself is near left or right screen edges
        let screen_card_left = (win_x as f32) + card_x;
        let screen_card_right = screen_card_left + card_w;
        if screen_card_left < 10.0 {
            card_x = (10.0 - (win_x as f32)).clamp(6.0, (win_logical_w - card_w - 6.0).max(6.0));
        } else if screen_card_right > (mon_w as f32) - 10.0 {
            let shift = screen_card_right - ((mon_w as f32) - 10.0);
            card_x = (card_x - shift).clamp(6.0, (win_logical_w - card_w - 6.0).max(6.0));
        }

        // Place bubble above the cat's head; if near the top of the screen, place slightly lower
        let space_above_screen = (win_y as f32) + cat_top_y;
        let place_below_head = space_above_screen < (card_h + 34.0);
        let card_y = if place_below_head {
            (cat_top_y + 16.0).clamp(6.0, 190.0)
        } else {
            (cat_top_y - card_h - 22.0).max(6.0)
        };

        let s = dpi_scale;
        let px_x = card_x * s;
        let px_y = card_y * s;
        let px_w = card_w * s;
        let px_h = card_h * s;
        let radius = 24.0 * s;

        let shadow_a = (36.0 * alpha_scale).round() as u8;
        let fill_a = (246.0 * alpha_scale).round() as u8;
        let border_a = (220.0 * alpha_scale).round() as u8;

        // 1. Draw the two thought-cloud connector dots (`o -> O`) between cat head and cloud card
        let dot_anchor_x = cat_anchor_x.clamp(card_x + 24.0, card_x + card_w - 24.0);
        let (small_dot_y, big_dot_y) = if place_below_head {
            (cat_top_y + 4.0, card_y - 6.0)
        } else {
            (cat_top_y - 5.0, card_y + card_h + 6.0)
        };
        let small_dot_x = dot_anchor_x + if dot_anchor_x >= card_x + card_w * 0.5 { 5.0 } else { -5.0 };
        let big_dot_x = dot_anchor_x;

        // Small dot `o` (radius 3.8px)
        pixmap.draw_rounded_rect_shadow(
            (small_dot_x - 3.8) * s,
            (small_dot_y - 3.8) * s,
            7.6 * s,
            7.6 * s,
            3.8 * s,
            4.0 * s,
            1.5 * s,
            [15, 23, 42, shadow_a / 2],
        );
        pixmap.fill_circle_sdf(
            small_dot_x * s,
            small_dot_y * s,
            3.8 * s,
            [255, 255, 255, fill_a],
            1.4 * s,
            [203, 213, 225, border_a],
        );

        // Medium dot `O` (radius 6.5px)
        pixmap.draw_rounded_rect_shadow(
            (big_dot_x - 6.5) * s,
            (big_dot_y - 6.5) * s,
            13.0 * s,
            13.0 * s,
            6.5 * s,
            5.0 * s,
            2.0 * s,
            [15, 23, 42, shadow_a / 2],
        );
        pixmap.fill_circle_sdf(
            big_dot_x * s,
            big_dot_y * s,
            6.5 * s,
            [255, 255, 255, fill_a],
            1.5 * s,
            [203, 213, 225, border_a],
        );

        // 2. Draw soft drop shadow + clean rounded thought card
        pixmap.draw_rounded_rect_shadow(
            px_x,
            px_y,
            px_w,
            px_h,
            radius,
            14.0 * s,
            4.5 * s,
            [15, 23, 42, shadow_a],
        );

        // Main rounded card body
        pixmap.fill_rounded_rect_sdf(
            px_x,
            px_y,
            px_w,
            px_h,
            radius,
            [255, 255, 255, fill_a],
            1.6 * s,
            [214, 222, 234, border_a],
        );

        // 3. Draw 5h & Weekly Quota Progress Bars inside bottom of cloud card if enabled
        if self.show_quota {
            let divider_y = (card_y + text_block_h + 2.0) * s;
            pixmap.fill_rounded_rect_sdf(
                (card_x + 14.0) * s,
                divider_y,
                (card_w - 28.0) * s,
                1.0 * s,
                0.5 * s,
                [226, 232, 240, (190.0 * alpha_scale) as u8],
                0.0,
                [0, 0, 0, 0],
            );

            // 5h bar track & fill
            let bar_x = (card_x + 54.0) * s;
            let bar_w = (card_w - 102.0) * s;
            let bar_h = 6.5 * s;

            let bar1_y = (card_y + text_block_h + 12.0) * s;
            pixmap.fill_rounded_rect_sdf(
                bar_x,
                bar1_y,
                bar_w,
                bar_h,
                3.25 * s,
                [226, 232, 240, (220.0 * alpha_scale) as u8],
                0.0,
                [0, 0, 0, 0],
            );
            let pct5 = (self.five_hour_pct / 100.0).clamp(0.0, 1.0);
            let color5 = quota_bar_color(self.five_hour_pct, alpha_scale);
            if pct5 > 0.01 {
                pixmap.fill_rounded_rect_sdf(
                    bar_x,
                    bar1_y,
                    (bar_w * pct5).max(6.0 * s),
                    bar_h,
                    3.25 * s,
                    color5,
                    0.0,
                    [0, 0, 0, 0],
                );
            }

            // Weekly bar track & fill
            let bar2_y = (card_y + text_block_h + 28.0) * s;
            pixmap.fill_rounded_rect_sdf(
                bar_x,
                bar2_y,
                bar_w,
                bar_h,
                3.25 * s,
                [226, 232, 240, (220.0 * alpha_scale) as u8],
                0.0,
                [0, 0, 0, 0],
            );
            let pct_w = (self.weekly_pct / 100.0).clamp(0.0, 1.0);
            let color_w = quota_bar_color(self.weekly_pct, alpha_scale);
            if pct_w > 0.01 {
                pixmap.fill_rounded_rect_sdf(
                    bar_x,
                    bar2_y,
                    (bar_w * pct_w).max(6.0 * s),
                    bar_h,
                    3.25 * s,
                    color_w,
                    0.0,
                    [0, 0, 0, 0],
                );
            }
        }

        // 4. Rasterize CJK & ASCII text directly into the software Pixmap
        draw_bubble_texts(
            pixmap,
            &self.text,
            self.show_quota,
            self.five_hour_pct,
            self.weekly_pct,
            &self.quota_meta,
            card_x,
            card_y,
            card_w,
            text_block_h,
            s,
            alpha_scale,
        );

        Some(LogicalRect {
            x: card_x,
            y: card_y,
            w: card_w,
            h: card_h,
        })
    }
}

fn quota_bar_color(pct: f32, alpha_scale: f32) -> [u8; 4] {
    let a = (245.0 * alpha_scale).round() as u8;
    if pct <= 15.0 {
        [239, 68, 68, a] // Red warning
    } else if pct <= 40.0 {
        [245, 158, 11, a] // Amber caution
    } else {
        [16, 185, 129, a] // Emerald healthy
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_bubble_texts(
    pixmap: &mut Pixmap,
    dialogue: &str,
    show_quota: bool,
    five_hour_pct: f32,
    weekly_pct: f32,
    quota_meta: &str,
    card_x: f32,
    card_y: f32,
    card_w: f32,
    text_block_h: f32,
    dpi_scale: f32,
    alpha_scale: f32,
) {
    #[cfg(target_os = "macos")]
    unsafe {
        macos_text::draw_text_into_pixmap(
            pixmap,
            dialogue,
            (card_x + 14.0) * dpi_scale,
            (card_y + 11.0) * dpi_scale,
            (card_w - 28.0) * dpi_scale,
            (text_block_h - 8.0) * dpi_scale,
            13.0 * dpi_scale,
            0.55,
            [30, 41, 59],
            alpha_scale,
        );

        if show_quota {
            let row1_y = (card_y + text_block_h + 7.5) * dpi_scale;
            let row2_y = (card_y + text_block_h + 23.5) * dpi_scale;
            let meta_y = (card_y + text_block_h + 39.5) * dpi_scale;

            macos_text::draw_text_into_pixmap(
                pixmap,
                "5h额度",
                (card_x + 14.0) * dpi_scale,
                row1_y,
                42.0 * dpi_scale,
                16.0 * dpi_scale,
                10.5 * dpi_scale,
                0.5,
                [100, 116, 139],
                alpha_scale,
            );
            let pct5_str = format!("{:.0}%", five_hour_pct);
            macos_text::draw_text_into_pixmap(
                pixmap,
                &pct5_str,
                (card_x + card_w - 42.0) * dpi_scale,
                row1_y,
                34.0 * dpi_scale,
                16.0 * dpi_scale,
                10.5 * dpi_scale,
                0.65,
                [51, 65, 85],
                alpha_scale,
            );

            macos_text::draw_text_into_pixmap(
                pixmap,
                "周额度",
                (card_x + 14.0) * dpi_scale,
                row2_y,
                42.0 * dpi_scale,
                16.0 * dpi_scale,
                10.5 * dpi_scale,
                0.5,
                [100, 116, 139],
                alpha_scale,
            );
            let pctw_str = format!("{:.0}%", weekly_pct);
            macos_text::draw_text_into_pixmap(
                pixmap,
                &pctw_str,
                (card_x + card_w - 42.0) * dpi_scale,
                row2_y,
                34.0 * dpi_scale,
                16.0 * dpi_scale,
                10.5 * dpi_scale,
                0.65,
                [51, 65, 85],
                alpha_scale,
            );

            if !quota_meta.is_empty() {
                macos_text::draw_text_into_pixmap(
                    pixmap,
                    quota_meta,
                    (card_x + 14.0) * dpi_scale,
                    meta_y,
                    (card_w - 28.0) * dpi_scale,
                    15.0 * dpi_scale,
                    9.5 * dpi_scale,
                    0.3,
                    [148, 163, 184],
                    alpha_scale,
                );
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (
            dialogue,
            show_quota,
            five_hour_pct,
            weekly_pct,
            quota_meta,
            card_x,
            card_y,
            card_w,
            text_block_h,
            dpi_scale,
            alpha_scale,
            pixmap,
        );
    }
}

#[cfg(target_os = "macos")]
mod macos_text {
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
    }

    #[inline]
    unsafe fn sel(name: &[u8]) -> *mut c_void {
        sel_registerName(name.as_ptr())
    }

    /// Rasterizes UTF-8 CJK / English text directly into `pixmap` using CoreText / NSString
    /// over a flipped `CGBitmapContext` wrapping `pixmap.data` in-place (zero extra heap copy).
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

        // Flip coordinate system vertically so top-left (0,0) matches our Pixmap layout
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_rendering() {
        let mut pixmap = Pixmap::new(200, 200);
        #[cfg(target_os = "macos")]
        unsafe {
            macos_text::draw_text_into_pixmap(
                &mut pixmap,
                "测试文字",
                10.0,
                10.0,
                150.0,
                30.0,
                14.0,
                0.5,
                [30, 41, 59],
                1.0,
            );
        }
        let mut non_zero_count = 0;
        let mut min_y = 200;
        let mut max_y = 0;
        for y in 0..200 {
            for x in 0..200 {
                let alpha = pixmap.alpha_at_physical(x, y);
                if alpha > 0 {
                    non_zero_count += 1;
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }
        println!("test_text_rendering: count={}, min_y={}, max_y={}", non_zero_count, min_y, max_y);
        assert!(non_zero_count > 0, "Text must produce non-zero pixels!");
        assert!(min_y >= 8 && max_y <= 45, "Text pixels should be between y=8 and y=45, got min_y={}, max_y={}", min_y, max_y);
    }
}

