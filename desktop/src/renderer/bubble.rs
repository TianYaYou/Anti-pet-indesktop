use super::text::{draw_text_into_pixmap, TextAlignment, VerticalAlignment};
use super::Pixmap;
use crate::hit_test::LogicalRect;

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
    pub is_dark: bool,
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
            is_dark: false,
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

        // Exact proportions (width 204px, pill rounded-rectangle)
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

        // Light vs Dark mode styling
        let shadow_a = if self.is_dark {
            (75.0 * alpha_scale).round() as u8
        } else {
            (46.0 * alpha_scale).round() as u8
        };
        let fill_a = (246.0 * alpha_scale).round() as u8;
        let border_a = if self.is_dark {
            (220.0 * alpha_scale).round() as u8
        } else {
            (230.0 * alpha_scale).round() as u8
        };

        // Dark mode: dark slate card with glowing indigo outline
        // Light mode: creamy white card with deep navy outline
        let fill_rgba = if self.is_dark {
            [32, 34, 42, fill_a]
        } else {
            [252, 251, 255, fill_a]
        };
        let border_rgba = if self.is_dark {
            [86, 104, 152, border_a]
        } else {
            [40, 54, 102, border_a]
        };
        let shadow_color = if self.is_dark {
            [0, 0, 0, shadow_a]
        } else {
            [28, 36, 75, shadow_a]
        };
        let dot_shadow_color = if self.is_dark {
            [0, 0, 0, (60.0 * alpha_scale) as u8]
        } else {
            [28, 36, 75, (36.0 * alpha_scale) as u8]
        };

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
            dot_shadow_color,
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
            dot_shadow_color,
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
            shadow_color,
        );

        // Main rounded card body with 2.4px stroke
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
            let div_color = if self.is_dark {
                [255, 255, 255, (26.0 * alpha_scale) as u8]
            } else {
                [40, 54, 102, (38.0 * alpha_scale) as u8]
            };
            pixmap.fill_rounded_rect_sdf(
                (card_x + 14.0) * s,
                divider_y,
                (card_w - 28.0) * s,
                1.2 * s,
                0.6 * s,
                div_color,
                0.0,
                [0, 0, 0, 0],
            );

            // 5h bar track & fill: calibrated to eliminate overlap with label (14..52) and pct (164..190)
            let bar_x = (card_x + 56.0) * s;
            let bar_w = 104.0 * s;
            let bar_h = 5.0 * s;

            let bar1_y = (card_y + text_block_h + 12.0) * s;
            let track_color = if self.is_dark {
                [255, 255, 255, (28.0 * alpha_scale) as u8]
            } else {
                [124, 77, 255, (36.0 * alpha_scale) as u8]
            };
            pixmap.fill_rounded_rect_sdf(
                bar_x,
                bar1_y,
                bar_w,
                bar_h,
                2.5 * s,
                track_color,
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
                track_color,
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
            self.is_dark,
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
    is_dark: bool,
) {
    let text_pri = if is_dark { [242, 244, 250] } else { [31, 38, 66] };
    let text_sec = if is_dark { [160, 168, 196] } else { [92, 100, 132] };

    // Dialogue text: centered, 12px, semibold/normal weight
    draw_text_into_pixmap(
        pixmap,
        dialogue,
        (card_x + 12.0) * dpi_scale,
        (card_y + 10.0) * dpi_scale,
        (card_w - 24.0) * dpi_scale,
        (text_block_h - 6.0) * dpi_scale,
        12.0 * dpi_scale,
        0.5,
        text_pri,
        alpha_scale,
        TextAlignment::Center,
        VerticalAlignment::Middle,
    );

    if show_quota {
        let row1_y = (card_y + text_block_h + 7.0) * dpi_scale;
        let row2_y = (card_y + text_block_h + 21.0) * dpi_scale;
        let meta_y = (card_y + text_block_h + 35.5) * dpi_scale;

        // 5h label: left-aligned, 9.8px
        draw_text_into_pixmap(
            pixmap,
            "5h额度",
            (card_x + 14.0) * dpi_scale,
            row1_y,
            38.0 * dpi_scale,
            15.0 * dpi_scale,
            9.8 * dpi_scale,
            0.4,
            text_sec,
            alpha_scale,
            TextAlignment::Left,
            VerticalAlignment::Middle,
        );

        // 5h percentage: right-aligned, bold
        let pct5_str = format!("{:.0}%", five_hour_pct);
        draw_text_into_pixmap(
            pixmap,
            &pct5_str,
            (card_x + card_w - 40.0) * dpi_scale,
            row1_y,
            26.0 * dpi_scale,
            15.0 * dpi_scale,
            9.8 * dpi_scale,
            0.7,
            text_pri,
            alpha_scale,
            TextAlignment::Right,
            VerticalAlignment::Middle,
        );

        // Weekly label: left-aligned, 9.8px
        draw_text_into_pixmap(
            pixmap,
            "周额度",
            (card_x + 14.0) * dpi_scale,
            row2_y,
            38.0 * dpi_scale,
            15.0 * dpi_scale,
            9.8 * dpi_scale,
            0.4,
            text_sec,
            alpha_scale,
            TextAlignment::Left,
            VerticalAlignment::Middle,
        );

        // Weekly percentage: right-aligned, bold
        let pctw_str = format!("{:.0}%", weekly_pct);
        draw_text_into_pixmap(
            pixmap,
            &pctw_str,
            (card_x + card_w - 40.0) * dpi_scale,
            row2_y,
            26.0 * dpi_scale,
            15.0 * dpi_scale,
            9.8 * dpi_scale,
            0.7,
            text_pri,
            alpha_scale,
            TextAlignment::Right,
            VerticalAlignment::Middle,
        );

        // Quota meta: centered, 9.5px
        if !quota_meta.is_empty() {
            draw_text_into_pixmap(
                pixmap,
                quota_meta,
                (card_x + 8.0) * dpi_scale,
                meta_y,
                (card_w - 16.0) * dpi_scale,
                14.0 * dpi_scale,
                9.5 * dpi_scale,
                0.4,
                text_sec,
                alpha_scale,
                TextAlignment::Center,
                VerticalAlignment::Middle,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_bubble_dark_and_light_mode_rendering() {
        let mut bubble = ThoughtBubble::default();
        let mut pixmap_light = Pixmap::new(400, 400);
        let mut pixmap_dark = Pixmap::new(400, 400);

        bubble.is_dark = false;
        let r_light = bubble.draw(&mut pixmap_light, 200.0, 200.0, 500.0, 500.0, 1440.0, 1.0);
        assert!(r_light.is_some());

        bubble.is_dark = true;
        let r_dark = bubble.draw(&mut pixmap_dark, 200.0, 200.0, 500.0, 500.0, 1440.0, 1.0);
        assert!(r_dark.is_some());

        assert_eq!(r_light.unwrap(), r_dark.unwrap());
    }
}
