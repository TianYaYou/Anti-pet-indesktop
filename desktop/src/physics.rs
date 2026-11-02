use crate::renderer::sprite::SpriteTransform;
use std::time::Instant;

pub const COMPACT_WIN_W: f64 = 340.0;
pub const COMPACT_WIN_H: f64 = 400.0;
pub const DEFAULT_CAT_WIDTH: f32 = 136.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PetMotionState {
    Idle,
    Thinking,
    Working,
    Success,
    Error,
    Petting,
    Dragging,
    Pressed,
    Rebound,
}

/// Complete 60 FPS spring-damper, gravity fall, Dock/screen-edge snapping, and petting gesture physics engine.
pub struct PetPhysics {
    /// Window outer position in logical desktop coordinates (for Compact Widget Mode)
    pub win_x: f64,
    pub win_y: f64,
    /// Cat anchor (bottom-center) in logical window coordinates
    pub local_anchor_x: f32,
    pub local_anchor_y: f32,

    /// Gravity velocity when released in air
    pub vx: f64,
    pub vy: f64,
    pub falling: bool,
    pub bounces: u8,
    pub target_floor_y: f64,

    /// User preferences
    pub gravity_enabled: bool,
    pub floor_mode: String, // "bottom", "taskbar", "free", "auto"
    pub zoom_scale: f32,    // 0.60 ..= 1.80

    /// Elastic spring state for squish/stretch and tilt
    pub scale_x: f32,
    pub scale_y: f32,
    pub target_scale_x: f32,
    pub target_scale_y: f32,
    pub vel_scale_x: f32,
    pub vel_scale_y: f32,

    /// Horizontal facing (`1.0` = left, `-1.0` = right)
    pub flip_x: f32,
    pub target_flip_x: f32,

    /// Rotation angle in radians
    pub rotation_rad: f32,
    pub target_rotation_rad: f32,

    /// Subtle gaze tracking offset toward cursor
    pub gaze_x: f32,
    pub gaze_y: f32,

    /// Screen edge magnetic peek state
    pub edge_dock: Option<EdgeSide>,

    /// Current motion state & timer
    pub motion: PetMotionState,
    pub motion_until: Option<Instant>,
    pub start_instant: Instant,

    /// Petting (head rub) gesture detector
    last_pet_cursor_x: f64,
    last_pet_dir: i8,
    pet_stroke_count: u8,
    last_pet_stroke_time: Instant,
}

impl PetPhysics {
    pub fn new(init_win_x: f64, init_win_y: f64) -> Self {
        let now = Instant::now();
        Self {
            win_x: init_win_x,
            win_y: init_win_y,
            local_anchor_x: (COMPACT_WIN_W * 0.5) as f32,
            local_anchor_y: COMPACT_WIN_H as f32,
            vx: 0.0,
            vy: 0.0,
            falling: false,
            bounces: 0,
            target_floor_y: init_win_y,
            gravity_enabled: true,
            floor_mode: "bottom".to_string(),
            zoom_scale: 1.0,
            scale_x: 1.0,
            scale_y: 1.0,
            target_scale_x: 1.0,
            target_scale_y: 1.0,
            vel_scale_x: 0.0,
            vel_scale_y: 0.0,
            flip_x: 1.0,
            target_flip_x: 1.0,
            rotation_rad: 0.0,
            target_rotation_rad: 0.0,
            gaze_x: 0.0,
            gaze_y: 0.0,
            edge_dock: None,
            motion: PetMotionState::Idle,
            motion_until: None,
            start_instant: now,
            last_pet_cursor_x: -1.0,
            last_pet_dir: 0,
            pet_stroke_count: 0,
            last_pet_stroke_time: now,
        }
    }

    pub fn cat_logical_size(&self) -> f32 {
        (DEFAULT_CAT_WIDTH * self.zoom_scale).clamp(80.0, 260.0)
    }

    /// Returns the generous interactive bounding box of the cat bust.
    pub fn cat_bounding_rect(&self) -> crate::hit_test::LogicalRect {
        let cat_w = self.cat_logical_size();
        let cat_h = cat_w * 0.835;
        let pad = 12.0_f32;
        let left = (self.local_anchor_x - cat_w * 0.5 - pad).max(0.0);
        let top = (self.local_anchor_y - cat_h - pad).max(0.0);
        let width = cat_w + pad * 2.0;
        let height = cat_h + pad * 2.0;
        crate::hit_test::LogicalRect {
            x: left,
            y: top,
            w: width,
            h: height,
        }
    }

    /// Triggers Q-elastic press compression (`compress 0.84`).
    pub fn on_pointer_down(&mut self) {
        self.falling = false;
        self.motion = PetMotionState::Pressed;
        self.motion_until = None;
        self.target_scale_x = 1.14;
        self.target_scale_y = 0.84;
    }

    /// Triggers duang rebound when released without dragging.
    pub fn trigger_duang_rebound(&mut self, duration_ms: u64) {
        self.motion = PetMotionState::Rebound;
        self.motion_until = Some(Instant::now() + std::time::Duration::from_millis(duration_ms));
        self.vel_scale_x = -0.36;
        self.vel_scale_y = 0.44;
        self.target_scale_x = 1.0;
        self.target_scale_y = 1.0;
        self.target_rotation_rad = 0.0;
    }

    /// Moves the window during user drag and updates tilt/facing.
    pub fn on_drag_move(&mut self, dx: f64, dy: f64, mon_w: f64, mon_h: f64) {
        self.falling = false;
        self.edge_dock = None;
        self.motion = PetMotionState::Dragging;
        self.motion_until = None;

        // When picked up: do not gaze track the mouse
        self.gaze_x = 0.0;
        self.gaze_y = 0.0;

        let max_drag_y = (mon_h - COMPACT_WIN_H).max(0.0);
        self.win_x = (self.win_x + dx).clamp(-80.0, mon_w - COMPACT_WIN_W + 80.0);
        self.win_y = (self.win_y + dy).clamp(0.0, max_drag_y);

        self.vx = dx;
        self.vy = dy;

        // Facing direction only changes if dragging with clear horizontal velocity
        if dx.abs() > 2.5 {
            self.target_flip_x = if dx > 0.0 { -1.0 } else { 1.0 };
        }
        self.target_scale_x = 0.94;
        self.target_scale_y = 1.07;
        self.target_rotation_rad = ((dx as f32) * 0.015).clamp(-0.20, 0.20);
    }

    /// Releases the pet from drag and computes target floor or left/right screen-edge magnetic snap.
    pub fn on_drag_release(
        &mut self,
        mon_w: f64,
        mon_h: f64,
        dock_bottom: f64,
        dock_left: f64,
        dock_right: f64,
    ) {
        self.target_scale_x = 1.0;
        self.target_scale_y = 1.0;
        self.target_rotation_rad = 0.0;
        self.gaze_x = 0.0;
        self.gaze_y = 0.0;

        let true_bottom_y = (mon_h - COMPACT_WIN_H).max(0.0);
        let dock_ledge_y = (mon_h - COMPACT_WIN_H - dock_bottom.max(0.0)).max(0.0);

        let target_floor_y = if self.floor_mode == "bottom" || dock_bottom <= 2.0 {
            true_bottom_y
        } else if self.floor_mode == "taskbar" || self.floor_mode == "dock" {
            dock_ledge_y
        } else if self.floor_mode == "free" || !self.gravity_enabled {
            self.win_y
        } else if self.win_y >= dock_ledge_y + 18.0 {
            true_bottom_y
        } else {
            dock_ledge_y
        };

        let cat_w = self.cat_logical_size() as f64;
        let cat_pad_x = ((COMPACT_WIN_W - cat_w) * 0.5).max(0.0);
        let min_win_x = dock_left.max(0.0) - cat_pad_x;
        let max_win_x = mon_w - dock_right.max(0.0) - COMPACT_WIN_W + cat_pad_x;
        let edge_snap_dist = 36.0;

        // Check left/right screen edge magnetic peeking when above the floor
        if self.win_x <= min_win_x + edge_snap_dist && self.win_y < target_floor_y - 28.0 {
            self.win_x = min_win_x - cat_w * 0.36;
            self.edge_dock = Some(EdgeSide::Left);
            self.target_flip_x = -1.0;
            self.target_rotation_rad = 0.20;
            self.falling = false;
            self.motion = PetMotionState::Idle;
            return;
        }
        if self.win_x >= max_win_x - edge_snap_dist && self.win_y < target_floor_y - 28.0 {
            self.win_x = max_win_x + cat_w * 0.36;
            self.edge_dock = Some(EdgeSide::Right);
            self.target_flip_x = 1.0;
            self.target_rotation_rad = -0.20;
            self.falling = false;
            self.motion = PetMotionState::Idle;
            return;
        }

        self.edge_dock = None;
        self.win_x = self.win_x.clamp(min_win_x, max_win_x);

        if !self.gravity_enabled || self.floor_mode == "free" || self.win_y >= target_floor_y - 2.0
        {
            self.win_y = if !self.gravity_enabled || self.floor_mode == "free" {
                self.win_y.clamp(0.0, true_bottom_y)
            } else {
                target_floor_y
            };
            self.falling = false;
            self.trigger_duang_rebound(440);
        } else {
            self.falling = true;
            self.vx *= 0.35;
            self.vy = 0.0;
            self.bounces = 0;
            self.target_floor_y = target_floor_y;
            self.motion = PetMotionState::Idle;
            // Lock facing direction in the direction of the throw velocity, never flip while falling:
            if self.vx.abs() > 0.5 {
                self.target_flip_x = if self.vx > 0.0 { -1.0 } else { 1.0 };
            }
        }
    }

    /// Updates cursor gaze tracking and detects rapid back-and-forth head petting gestures.
    /// Returns `true` if a new petting gesture was just triggered.
    pub fn on_cursor_moved(&mut self, local_x: f64, local_y: f64, over_cat: bool) -> bool {
        // While falling or dragged: do not gaze track the mouse and do not flip
        if self.falling || matches!(self.motion, PetMotionState::Dragging | PetMotionState::Pressed) {
            self.gaze_x = 0.0;
            self.gaze_y = 0.0;
            return false;
        }

        let dx = (local_x as f32) - self.local_anchor_x;
        let cat_h = self.cat_logical_size();
        let center_y = self.local_anchor_y - cat_h * 0.5;
        let dy = (local_y as f32) - center_y;

        // Subtle gaze shift toward cursor only when stationary and hovered over cat
        if over_cat {
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let factor = (dist / 180.0).min(1.0) * 3.2;
            self.gaze_x = (dx / dist) * factor;
            self.gaze_y = (dy / dist) * factor * 0.65;

            // Only flip when stationary on ground and cursor is clearly on one side (with generous deadband)
            if self.edge_dock.is_none() && dx.abs() > 38.0 {
                self.target_flip_x = if dx > 0.0 { -1.0 } else { 1.0 };
            }
        } else {
            self.gaze_x *= 0.8;
            self.gaze_y *= 0.8;
        }

        // Detect petting gesture on top 55% of the cat sprite
        let head_top = self.local_anchor_y - cat_h * 0.95;
        let head_bottom = self.local_anchor_y - cat_h * 0.35;
        let in_head_zone = over_cat
            && (local_y as f32) >= head_top
            && (local_y as f32) <= head_bottom
            && dx.abs() <= cat_h * 0.45;

        if in_head_zone {
            let now = Instant::now();
            if now.duration_since(self.last_pet_stroke_time).as_millis() > 900 {
                self.pet_stroke_count = 0;
                self.last_pet_dir = 0;
            }
            if self.last_pet_cursor_x >= 0.0 {
                let delta_x = local_x - self.last_pet_cursor_x;
                if delta_x.abs() >= 9.0 {
                    let dir = if delta_x > 0.0 { 1 } else { -1 };
                    if dir != self.last_pet_dir {
                        self.last_pet_dir = dir;
                        self.pet_stroke_count = self.pet_stroke_count.saturating_add(1);
                        self.last_pet_stroke_time = now;
                        if self.pet_stroke_count >= 4 {
                            self.pet_stroke_count = 0;
                            self.motion = PetMotionState::Petting;
                            self.motion_until =
                                Some(now + std::time::Duration::from_millis(2200));
                            self.vel_scale_x = 0.18;
                            self.vel_scale_y = -0.18;
                            return true;
                        }
                    }
                }
            }
            self.last_pet_cursor_x = local_x;
        } else {
            self.last_pet_cursor_x = -1.0;
        }

        false
    }

    /// Steps 1 frame (60 FPS) of gravity, spring-damper deformation, and idle breathing.
    /// Returns `(window_moved, needs_next_frame_tick)`.
    pub fn step_frame(
        &mut self,
        mon_w: f64,
        dock_left: f64,
        dock_right: f64,
    ) -> (bool, bool) {
        let now = Instant::now();
        if let Some(until) = self.motion_until {
            if now >= until {
                self.motion = PetMotionState::Idle;
                self.motion_until = None;
                self.target_scale_x = 1.0;
                self.target_scale_y = 1.0;
                self.target_rotation_rad = 0.0;
            }
        }

        let mut window_moved = false;

        // 1. Gravity drop & floor bounce physics
        if self.falling {
            let cat_w = self.cat_logical_size() as f64;
            let cat_pad_x = ((COMPACT_WIN_W - cat_w) * 0.5).max(0.0);
            let min_win_x = dock_left.max(0.0) - cat_pad_x;
            let max_win_x = mon_w - dock_right.max(0.0) - COMPACT_WIN_W + cat_pad_x;

            self.vy += 1.15;
            self.win_y += self.vy;
            self.win_x = (self.win_x + self.vx).clamp(min_win_x, max_win_x);
            self.vx *= 0.94;
            self.gaze_x = 0.0;
            self.gaze_y = 0.0;

            // Stretch slightly while falling fast
            let stretch = ((self.vy as f32) * 0.006).clamp(0.0, 0.12);
            self.target_scale_x = 1.0 - stretch * 0.6;
            self.target_scale_y = 1.0 + stretch;

            if self.win_y >= self.target_floor_y {
                self.win_y = self.target_floor_y;
                self.falling = false;
                self.vy = 0.0;
                self.vx = 0.0;
                self.bounces = 0;
                self.trigger_duang_rebound(440);
            }
            window_moved = true;
        }

        // 2. Motion-specific procedural animation (breathing / thinking / working / petting)
        let t = now.duration_since(self.start_instant).as_secs_f32();
        let (anim_sx, anim_sy, anim_rot) = match self.motion {
            PetMotionState::Idle => {
                let breath = (t * 2.0).sin() * 0.014;
                (1.0 - breath * 0.5, 1.0 + breath, 0.0)
            }
            PetMotionState::Thinking => {
                let sway = (t * 3.6).sin() * 0.055;
                let pulse = (t * 4.5).cos() * 0.02;
                (1.0 - pulse, 1.0 + pulse, sway)
            }
            PetMotionState::Working => {
                let hop = (t * 8.5).sin().abs() * 0.045;
                let tilt = (t * 8.5).cos() * 0.05;
                (1.0 + hop * 0.4, 1.0 - hop * 0.6, tilt)
            }
            PetMotionState::Petting => {
                let nuzzle = (t * 11.0).sin() * 0.075;
                (1.06, 0.93, nuzzle)
            }
            PetMotionState::Success => {
                let cheer = (t * 9.0).sin() * 0.06;
                (1.0 - cheer * 0.5, 1.0 + cheer, (t * 4.5).sin() * 0.06)
            }
            PetMotionState::Error => {
                let shiver = (t * 18.0).sin() * 0.03;
                (1.03, 0.96, shiver)
            }
            PetMotionState::Pressed | PetMotionState::Dragging | PetMotionState::Rebound => {
                (self.target_scale_x, self.target_scale_y, self.target_rotation_rad)
            }
        };

        let goal_sx = if matches!(
            self.motion,
            PetMotionState::Pressed | PetMotionState::Dragging
        ) {
            self.target_scale_x
        } else {
            anim_sx
        };
        let goal_sy = if matches!(
            self.motion,
            PetMotionState::Pressed | PetMotionState::Dragging
        ) {
            self.target_scale_y
        } else {
            anim_sy
        };
        let goal_rot = if self.edge_dock.is_some()
            || matches!(self.motion, PetMotionState::Dragging)
        {
            self.target_rotation_rad
        } else {
            anim_rot
        };

        // 3. Calm breathing in Idle, Spring-damper for dynamic actions
        if matches!(self.motion, PetMotionState::Idle) && !self.falling && self.edge_dock.is_none() {
            let breath = (t * 2.0).sin() * 0.014;
            self.scale_x = 1.0 - breath * 0.5;
            self.scale_y = 1.0 + breath;
            self.vel_scale_x = 0.0;
            self.vel_scale_y = 0.0;
            self.rotation_rad = 0.0;
        } else {
            let stiffness = 0.22;
            let damping = 0.74;
            self.vel_scale_x = (self.vel_scale_x + (goal_sx - self.scale_x) * stiffness) * damping;
            self.vel_scale_y = (self.vel_scale_y + (goal_sy - self.scale_y) * stiffness) * damping;
            self.scale_x += self.vel_scale_x;
            self.scale_y += self.vel_scale_y;
            self.rotation_rad += (goal_rot - self.rotation_rad) * 0.22;
            if self.rotation_rad.abs() < 0.001 {
                self.rotation_rad = 0.0;
            }
        }

        self.flip_x += (self.target_flip_x - self.flip_x) * 0.25;
        if (self.flip_x - self.target_flip_x).abs() < 0.005 {
            self.flip_x = self.target_flip_x;
        }

        let active_anim = self.falling
            || self.vel_scale_x.abs() > 0.002
            || self.vel_scale_y.abs() > 0.002
            || self.rotation_rad.abs() > 0.002
            || (self.scale_x - goal_sx).abs() > 0.003
            || (self.scale_y - goal_sy).abs() > 0.003
            || (self.flip_x - self.target_flip_x).abs() > 0.005
            || !matches!(self.motion, PetMotionState::Idle);

        (window_moved, active_anim)
    }

    pub fn current_sprite_transform(&self, dnd_enabled: bool) -> SpriteTransform {
        SpriteTransform {
            anchor_x: self.local_anchor_x,
            anchor_y: self.local_anchor_y,
            base_size: self.cat_logical_size(),
            scale_x: self.scale_x,
            scale_y: self.scale_y,
            flip_x: self.flip_x,
            rotation_rad: self.rotation_rad,
            gaze_offset_x: self.gaze_x,
            gaze_offset_y: self.gaze_y,
            opacity: if dnd_enabled { 0.88 } else { 1.0 },
            draw_ground_shadow: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_landing_duang_rebound() {
        let mut physics = PetPhysics::new(100.0, 200.0);
        physics.falling = true;
        physics.vy = 10.0;
        physics.target_floor_y = 500.0;
        physics.win_y = 495.0;

        // Step frame: moves win_y to 495 + 10 + 1.15 = 506.15 >= 500.0
        let (moved, _) = physics.step_frame(1440.0, 0.0, 0.0);
        assert!(moved);
        assert_eq!(physics.win_y, 500.0);
        assert!(!physics.falling, "Falling must stop immediately on floor landing");
        assert_eq!(physics.bounces, 0, "No double physical bounces allowed");
        assert_eq!(physics.motion, PetMotionState::Rebound);
    }

    #[test]
    fn test_floor_geometry_never_falls_out_of_screen() {
        let mon_w = 1440.0;
        let mon_h = 900.0;
        let dock_bottom = 72.0;

        let mut physics = PetPhysics::new(100.0, 100.0);
        assert_eq!(physics.floor_mode, "bottom");

        // 1. Release in taskbar mode with 72px dock
        physics.floor_mode = "taskbar".to_string();
        physics.win_y = 200.0;
        physics.on_drag_release(mon_w, mon_h, dock_bottom, 0.0, 0.0);
        let expected_dock_y = mon_h - COMPACT_WIN_H - dock_bottom; // 900 - 400 - 72 = 428.0
        assert_eq!(physics.target_floor_y, expected_dock_y);
        assert!(physics.target_floor_y + COMPACT_WIN_H <= mon_h - dock_bottom);

        // 2. Release in bottom mode
        physics.floor_mode = "bottom".to_string();
        physics.win_y = 200.0;
        physics.on_drag_release(mon_w, mon_h, dock_bottom, 0.0, 0.0);
        let expected_bottom_y = mon_h - COMPACT_WIN_H; // 900 - 400 = 500.0
        assert_eq!(physics.target_floor_y, expected_bottom_y);
        assert_eq!(physics.target_floor_y + COMPACT_WIN_H, mon_h);

        // 3. Drag move never allows win_y past screen bottom
        physics.on_drag_move(0.0, 1000.0, mon_w, mon_h);
        assert!(physics.win_y <= mon_h - COMPACT_WIN_H);
    }
}
