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
        let text_lines = ((char_count as f32) / 13.5).ceil().max(1.0) as usize;

        // Exact v0.2.x proportions (width 204px, pill rounded-rectangle)
        let card_w: f32 = 204.0;
        let text_block_h: f32 = (text_lines as f32) * 17.5 + 16.0;
        let quota_block_h: f32 = if self.show_quota { 50.0 } else { 0.0 };
        let card_h: f32 = (text_block_h + quota_block_h + 8.0).clamp(46.0, 160.0);

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
        let radius = 26.0 * s;

        // v0.2.x authentic colors: #283666 deep navy outline, rgba(252, 251, 255, 0.96) creamy background, soft navy shadow
        let shadow_a = (46.0 * alpha_scale).round() as u8;
        let fill_a = (246.0 * alpha_scale).round() as u8;
        let border_a = (230.0 * alpha_scale).round() as u8;
        let border_rgba = [40, 54, 102, border_a];
        let fill_rgba = [252, 251, 255, fill_a];

        // 1. Draw the two thought-cloud connector dots (`o -> O`) between cat head and cloud card
        let dot_anchor_x = cat_anchor_x.clamp(card_x + 24.0, card_x + card_w - 24.0);
        let (small_dot_y, big_dot_y) = if place_below_head {
            (cat_top_y + 4.0, card_y - 7.0)
        } else {
            (cat_top_y - 4.0, card_y + card_h + 7.0)
        };
        let small_dot_x = dot_anchor_x - 3.0;
        let big_dot_x = dot_anchor_x + 6.0;

        // Small dot `o` (diameter ~6.4px, radius 3.2px)
        pixmap.draw_rounded_rect_shadow(
            (small_dot_x - 3.2) * s,
            (small_dot_y - 3.2) * s,
            6.4 * s,
            6.4 * s,
            3.2 * s,
            3.5 * s,
            1.2 * s,
            [28, 36, 75, (36.0 * alpha_scale) as u8],
        );
        pixmap.fill_circle_sdf(
            small_dot_x * s,
            small_dot_y * s,
            3.2 * s,
            fill_rgba,
            1.8 * s,
            border_rgba,
        );

        // Medium dot `O` (diameter 10px, radius 5.0px)
        pixmap.draw_rounded_rect_shadow(
            (big_dot_x - 5.0) * s,
            (big_dot_y - 5.0) * s,
            10.0 * s,
            10.0 * s,
            5.0 * s,
            5.0 * s,
            1.8 * s,
            [28, 36, 75, (40.0 * alpha_scale) as u8],
        );
        pixmap.fill_circle_sdf(
            big_dot_x * s,
            big_dot_y * s,
            5.0 * s,
            fill_rgba,
            2.0 * s,
            border_rgba,
        );

        // 2. Draw soft drop shadow + clean rounded thought card (0 8px 20px rgba(28, 36, 75, 0.18))
        pixmap.draw_rounded_rect_shadow(
            px_x,
            px_y,
            px_w,
            px_h,
            radius,
            16.0 * s,
            6.0 * s,
            [28, 36, 75, shadow_a],
        );

        // Main rounded card body with 2.4px #283666 stroke
        pixmap.fill_rounded_rect_sdf(
            px_x,
            px_y,
            px_w,
            px_h,
            radius,
            fill_rgba,
            2.4 * s,
            border_rgba,
        );

        // 3. Draw 5h & Weekly Quota Progress Bars inside bottom of cloud card if enabled
        if self.show_quota {
            let divider_y = (card_y + text_block_h + 3.0) * s;
            pixmap.fill_rounded_rect_sdf(
                (card_x + 14.0) * s,
                divider_y,
                (card_w - 28.0) * s,
                1.2 * s,
                0.6 * s,
                [40, 54, 102, (38.0 * alpha_scale) as u8],
                0.0,
                [0, 0, 0, 0],
            );

            // 5h bar track & fill: calibrated to eliminate overlap with label (14..52) and pct (164..190)
            let bar_x = (card_x + 56.0) * s;
            let bar_w = 104.0 * s;
            let bar_h = 5.0 * s;

            let bar1_y = (card_y + text_block_h + 12.0) * s;
            pixmap.fill_rounded_rect_sdf(
                bar_x,
                bar1_y,
                bar_w,
                bar_h,
                2.5 * s,
                [124, 77, 255, (36.0 * alpha_scale) as u8],
                0.0,
                [0, 0, 0, 0],
            );
            let pct5 = (self.five_hour_pct / 100.0).clamp(0.0, 1.0);
            let (start5, end5) = quota_bar_gradient(self.five_hour_pct, false, alpha_scale);
            if pct5 > 0.01 {
                pixmap.fill_rounded_rect_gradient_sdf(
                    bar_x,
                    bar1_y,
                    (bar_w * pct5).max(5.0 * s),
                    bar_h,
                    2.5 * s,
                    start5,
                    end5,
                );
            }

            // Weekly bar track & fill
            let bar2_y = (card_y + text_block_h + 26.0) * s;
            pixmap.fill_rounded_rect_sdf(
                bar_x,
                bar2_y,
                bar_w,
                bar_h,
                2.5 * s,
                [124, 77, 255, (36.0 * alpha_scale) as u8],
                0.0,
                [0, 0, 0, 0],
            );
            let pct_w = (self.weekly_pct / 100.0).clamp(0.0, 1.0);
            let (start_w, end_w) = quota_bar_gradient(self.weekly_pct, true, alpha_scale);
            if pct_w > 0.01 {
                pixmap.fill_rounded_rect_gradient_sdf(
                    bar_x,
                    bar2_y,
                    (bar_w * pct_w).max(5.0 * s),
                    bar_h,
                    2.5 * s,
                    start_w,
                    end_w,
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

fn quota_bar_gradient(pct: f32, is_weekly: bool, alpha_scale: f32) -> ([u8; 4], [u8; 4]) {
    let a = (248.0 * alpha_scale).round() as u8;
    if pct <= 15.0 {
        // Red warning (danger)
        ([255, 23, 68, a], [245, 0, 87, a])
    } else if pct <= 40.0 {
        // Amber warning (caution)
        ([255, 145, 0, a], [255, 171, 0, a])
    } else if is_weekly {
        // Weekly cyan-to-emerald gradient
        ([0, 176, 255, a], [0, 230, 118, a])
    } else {
        // 5h Gemini purple-to-blue gradient
        ([124, 77, 255, a], [41, 121, 255, a])
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
        // Dialogue text: centered, #1f2642 deep ink color, 12px, weight 0.58 (semibold)
        macos_text::draw_text_into_pixmap(
            pixmap,
            dialogue,
            (card_x + 12.0) * dpi_scale,
            (card_y + 10.0) * dpi_scale,
            (card_w - 24.0) * dpi_scale,
            (text_block_h - 6.0) * dpi_scale,
            12.0 * dpi_scale,
            0.58,
            [31, 38, 66],
            alpha_scale,
            1, // Center alignment
        );

        if show_quota {
            let row1_y = (card_y + text_block_h + 7.0) * dpi_scale;
            let row2_y = (card_y + text_block_h + 21.0) * dpi_scale;
            let meta_y = (card_y + text_block_h + 35.5) * dpi_scale;

            // 5h label: left-aligned, #5c6484, 9.8px
            macos_text::draw_text_into_pixmap(
                pixmap,
                "5h额度",
                (card_x + 14.0) * dpi_scale,
                row1_y,
                38.0 * dpi_scale,
                15.0 * dpi_scale,
                9.8 * dpi_scale,
                0.5,
                [92, 100, 132],
                alpha_scale,
                0, // Left alignment
            );

            // 5h percentage: right-aligned, #1f2642, bold
            let pct5_str = format!("{:.0}%", five_hour_pct);
            macos_text::draw_text_into_pixmap(
                pixmap,
                &pct5_str,
                (card_x + card_w - 40.0) * dpi_scale,
                row1_y,
                26.0 * dpi_scale,
                15.0 * dpi_scale,
                9.8 * dpi_scale,
                0.65,
                [31, 38, 66],
                alpha_scale,
                2, // Right alignment
            );

            // Weekly label: left-aligned, #5c6484, 9.8px
            macos_text::draw_text_into_pixmap(
                pixmap,
                "周额度",
                (card_x + 14.0) * dpi_scale,
                row2_y,
                38.0 * dpi_scale,
                15.0 * dpi_scale,
                9.8 * dpi_scale,
                0.5,
                [92, 100, 132],
                alpha_scale,
                0, // Left alignment
            );

            // Weekly percentage: right-aligned, #1f2642, bold
            let pctw_str = format!("{:.0}%", weekly_pct);
            macos_text::draw_text_into_pixmap(
                pixmap,
                &pctw_str,
                (card_x + card_w - 40.0) * dpi_scale,
                row2_y,
                26.0 * dpi_scale,
                15.0 * dpi_scale,
                9.8 * dpi_scale,
                0.65,
                [31, 38, 66],
                alpha_scale,
                2, // Right alignment
            );

            // Quota meta: centered, #5c6484, 9px
            if !quota_meta.is_empty() {
                macos_text::draw_text_into_pixmap(
                    pixmap,
                    quota_meta,
                    (card_x + 10.0) * dpi_scale,
                    meta_y,
                    (card_w - 20.0) * dpi_scale,
                    14.0 * dpi_scale,
                    9.0 * dpi_scale,
                    0.3,
                    [92, 100, 132],
                    alpha_scale,
                    1, // Center alignment
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
        alignment: isize, // 0 = Left, 1 = Center, 2 = Right in Cocoa AppKit
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
                                // Release +1 retain count from mutableCopy to avoid memory leak
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
                1,
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

    #[test]
    fn test_bubble_geometry_no_overlap() {
        let bubble = ThoughtBubble::default();
        let mut pixmap = Pixmap::new(400, 400);
        let rect = bubble.draw(&mut pixmap, 200.0, 200.0, 500.0, 500.0, 1440.0, 1.0);
        assert!(rect.is_some());
        let r = rect.unwrap();
        assert_eq!(r.w, 204.0, "Card width must be exactly 204.0");

        // Verify quota row horizontal geometry:
        // Left margin (14.0) -> Label (14..52) -> Gap (4.0) -> Bar (56..160) -> Gap (4.0) -> Pct (164..190) -> Right margin (14.0)
        let label_left = 14.0_f32;
        let label_w = 38.0_f32;
        let label_right = label_left + label_w; // 52.0

        let bar_left = 56.0_f32;
        let bar_w = 104.0_f32;
        let bar_right = bar_left + bar_w; // 160.0

        let pct_left = 164.0_f32;
        let pct_w = 26.0_f32;
        let pct_right = pct_left + pct_w; // 190.0

        assert!(label_right < bar_left, "Label right ({}) must be strictly less than bar left ({})", label_right, bar_left);
        assert!(bar_right < pct_left, "Bar right ({}) must be strictly less than pct left ({})", bar_right, pct_left);
        assert_eq!(pct_right + 14.0, 204.0, "Symmetric margins must add up to exact card width");
    }
}

