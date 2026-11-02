use super::Pixmap;
use crate::hit_test::LogicalRect;

/// Thought cloud bubble state and renderer (expanded in Milestone 2: `feat/cpu-2d-bubble-ui`).
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
            text: "大肥猫已就位！随时为你护航代码~".to_string(),
            show_quota: true,
            five_hour_pct: 100.0,
            weekly_pct: 100.0,
            quota_meta: "Antigravity 实时额度已同步".to_string(),
            opacity: 1.0,
        }
    }
}

impl ThoughtBubble {
    pub fn draw(
        &self,
        _pixmap: &mut Pixmap,
        _cat_anchor_x: f32,
        _cat_top_y: f32,
        _win_x: f64,
        _win_y: f64,
        _mon_w: f64,
        _dpi_scale: f32,
    ) -> Option<LogicalRect> {
        None
    }
}
