use crate::renderer::Pixmap;

/// Logical bounding rectangle (used for thought bubble or auxiliary interactive regions).
#[derive(Clone, Copy, Debug, Default)]
pub struct LogicalRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl LogicalRect {
    #[inline]
    pub fn contains(&self, lx: f32, ly: f32) -> bool {
        self.w > 0.0
            && self.h > 0.0
            && lx >= self.x
            && lx <= self.x + self.w
            && ly >= self.y
            && ly <= self.y + self.h
    }
}

/// Zero-IPC, `< 1μs` direct memory RGBA alpha hit-tester for the Pure CPU 2D software renderer.
pub struct PixelHitTester {
    /// Minimum alpha threshold (`0..=255`) considered interactive (default `20` ~ 8% opacity).
    pub alpha_threshold: u8,
    /// Optional active thought-bubble rectangle in logical window coordinates.
    pub bubble_rect: Option<LogicalRect>,
}

impl Default for PixelHitTester {
    fn default() -> Self {
        Self {
            alpha_threshold: 20,
            bubble_rect: None,
        }
    }
}

impl PixelHitTester {
    /// Tests whether logical window coordinate `(logical_x, logical_y)` hits a non-transparent
    /// pixel of the rendered frame (`pixmap`) or an active thought-cloud bubble.
    ///
    /// Note on DND mode:
    /// In DND ("免打扰模式"), we still allow hit-testing on the cat body so the user can
    /// right-click the cat at any time to open the native context menu and exit DND mode,
    /// without needing any extra floating wake button.
    #[inline]
    pub fn is_interactive_at(
        &self,
        pixmap: &Pixmap,
        logical_x: f64,
        logical_y: f64,
        dpi_scale: f64,
        is_dragging: bool,
        dnd_mode: bool,
    ) -> bool {
        if is_dragging {
            return true;
        }
        if logical_x < 0.0 || logical_y < 0.0 {
            return false;
        }

        // Check active thought bubble rectangle first (when not in DND mode)
        if !dnd_mode {
            if let Some(rect) = self.bubble_rect {
                if rect.contains(logical_x as f32, logical_y as f32) {
                    return true;
                }
            }
        }

        // Direct O(1) lookup in the rendered RGBA software framebuffer
        let px = (logical_x * dpi_scale).round() as i32;
        let py = (logical_y * dpi_scale).round() as i32;
        pixmap.alpha_at_physical(px, py) >= self.alpha_threshold
    }
}
