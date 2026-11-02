use super::Pixmap;
use std::io::Cursor;
use std::path::Path;

/// Embedded default fat-cat sprite (`skins/fat-cat/frames/idle.png`) so the binary runs standalone.
const DEFAULT_CAT_PNG: &[u8] = include_bytes!("../../../skins/fat-cat/frames/idle.png");

/// Mipmapped premultiplied RGBA sprite for high-quality, shimmer-free CPU 2D affine rendering.
#[derive(Clone)]
pub struct CatSprite {
    /// Mip level 0 (e.g. 512x512 or original)
    pub base_width: u32,
    pub base_height: u32,
    pub base_premul: Vec<u8>,
    /// Pre-filtered 256x256 mip level for crisp display at desktop pet sizes (100px..280px)
    pub mip_width: u32,
    pub mip_height: u32,
    pub mip_premul: Vec<u8>,
}

/// Transform parameters for drawing the cat sprite in logical window coordinates.
#[derive(Clone, Copy, Debug)]
pub struct SpriteTransform {
    /// Bottom-center X of the cat in logical coordinates
    pub anchor_x: f32,
    /// Bottom-center Y (feet level) of the cat in logical coordinates
    pub anchor_y: f32,
    /// Logical display width before squish/stretch (default 136.0 * zoom)
    pub base_size: f32,
    /// Horizontal squish/stretch factor (`1.0` normal, `> 1.0` wider when compressed)
    pub scale_x: f32,
    /// Vertical squish/stretch factor (`1.0` normal, `< 1.0` shorter when compressed)
    pub scale_y: f32,
    /// Signed horizontal flip factor (`-1.0` facing right, `1.0` facing left, smooth interpolation)
    pub flip_x: f32,
    /// Rotation angle in radians around the sprite center
    pub rotation_rad: f32,
    /// Subtle gaze/head offset in logical pixels
    pub gaze_offset_x: f32,
    pub gaze_offset_y: f32,
    /// Overall opacity (`0.0..=1.0`, e.g. `0.85` in DND mode or `1.0` normal)
    pub opacity: f32,
    /// Draw ground shadow ellipse under feet
    pub draw_ground_shadow: bool,
}

impl Default for SpriteTransform {
    fn default() -> Self {
        Self {
            anchor_x: 170.0,
            anchor_y: 386.0,
            base_size: 136.0,
            scale_x: 1.0,
            scale_y: 1.0,
            flip_x: 1.0,
            rotation_rad: 0.0,
            gaze_offset_x: 0.0,
            gaze_offset_y: 0.0,
            opacity: 1.0,
            draw_ground_shadow: true,
        }
    }
}

impl CatSprite {
    /// Loads the embedded default fat-cat PNG sprite.
    pub fn load_default() -> Self {
        Self::from_png_bytes(DEFAULT_CAT_PNG).unwrap_or_else(|_| Self::fallback_sprite())
    }

    /// Loads a custom skin PNG from disk, falling back to the embedded sprite on error.
    pub fn load_from_path(path: &Path) -> Self {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(sprite) = Self::from_png_bytes(&bytes) {
                return sprite;
            }
        }
        Self::load_default()
    }

    pub fn from_png_bytes(bytes: &[u8]) -> Result<Self, String> {
        let decoder = png::Decoder::new(Cursor::new(bytes));
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;

        let width = info.width;
        let height = info.height;
        let total_px = (width as usize) * (height as usize);
        let mut premul = vec![0u8; total_px * 4];

        match info.color_type {
            png::ColorType::Rgba => {
                for i in 0..total_px {
                    let o = i * 4;
                    let r = buf[o] as u32;
                    let g = buf[o + 1] as u32;
                    let b = buf[o + 2] as u32;
                    let a = buf[o + 3] as u32;
                    premul[o] = ((r * a + 127) / 255) as u8;
                    premul[o + 1] = ((g * a + 127) / 255) as u8;
                    premul[o + 2] = ((b * a + 127) / 255) as u8;
                    premul[o + 3] = a as u8;
                }
            }
            png::ColorType::Rgb => {
                for i in 0..total_px {
                    let src_o = i * 3;
                    let dst_o = i * 4;
                    premul[dst_o] = buf[src_o];
                    premul[dst_o + 1] = buf[src_o + 1];
                    premul[dst_o + 2] = buf[src_o + 2];
                    premul[dst_o + 3] = 255;
                }
            }
            _ => return Err("Unsupported PNG color type".to_string()),
        }

        // Precompute a clean box-filtered mipmap (~272x272) so downsampling 1024->136 has zero aliasing
        let (mip_w, mip_h, mip_data) = if width > 320 && height > 320 {
            let mw = (width / 4).max(128);
            let mh = (height / 4).max(128);
            let data = downsample_box(&premul, width, height, mw, mh);
            (mw, mh, data)
        } else {
            (width, height, premul.clone())
        };

        Ok(Self {
            base_width: width,
            base_height: height,
            base_premul: premul,
            mip_width: mip_w,
            mip_height: mip_h,
            mip_premul: mip_data,
        })
    }

    fn fallback_sprite() -> Self {
        let w = 128;
        let h = 128;
        let mut premul = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let dx = x as f32 - 64.0;
                let dy = y as f32 - 64.0;
                if dx * dx + dy * dy <= 52.0 * 52.0 {
                    let o = ((y * w + x) * 4) as usize;
                    premul[o] = 240;
                    premul[o + 1] = 242;
                    premul[o + 2] = 248;
                    premul[o + 3] = 255;
                }
            }
        }
        Self {
            base_width: w,
            base_height: h,
            base_premul: premul.clone(),
            mip_width: w,
            mip_height: h,
            mip_premul: premul,
        }
    }

    /// Renders the cat sprite onto `pixmap` using inverse affine transformation and bilinear texture sampling.
    /// Returns the physical bounding box `(min_x, min_y, max_x, max_y)` of drawn cat pixels.
    pub fn draw(&self, pixmap: &mut Pixmap, tf: &SpriteTransform, dpi_scale: f32) -> (i32, i32, i32, i32) {
        let aspect = (self.base_height as f32) / (self.base_width.max(1) as f32);
        let draw_w = tf.base_size * dpi_scale;
        let draw_h = draw_w * aspect;

        let sx = (tf.scale_x * tf.flip_x.signum() * tf.flip_x.abs().max(0.18)) * draw_w;
        let sy = tf.scale_y.max(0.2) * draw_h;

        let anchor_px = (tf.anchor_x + tf.gaze_offset_x) * dpi_scale;
        let anchor_py = (tf.anchor_y + tf.gaze_offset_y) * dpi_scale;

        if tf.draw_ground_shadow {
            let shadow_rx = (draw_w * 0.34 * tf.scale_x.clamp(0.8, 1.25)).max(8.0);
            let shadow_ry = (6.5 * dpi_scale).max(3.0);
            pixmap.draw_ground_ellipse_shadow(
                anchor_px,
                anchor_py - 3.0 * dpi_scale,
                shadow_rx,
                shadow_ry,
                (56.0 * tf.opacity) as u8,
            );
        }

        // Center of sprite given bottom-center anchor
        let cx = anchor_px;
        let cy = anchor_py - (sy * 0.5);

        let cos_r = tf.rotation_rad.cos();
        let sin_r = tf.rotation_rad.sin();

        let half_diag = 0.55 * (sx.abs().hypot(sy.abs()));
        let min_x = (cx - half_diag).floor().max(0.0) as i32;
        let max_x = (cx + half_diag).ceil().min(pixmap.width as f32 - 1.0) as i32;
        let min_y = (cy - half_diag).floor().max(0.0) as i32;
        let max_y = (cy + half_diag).ceil().min(pixmap.height as f32 - 1.0) as i32;

        // Choose mipmap level if target size is close to or smaller than mipmap
        let (tex_w, tex_h, tex_data) = if draw_w <= (self.mip_width as f32) * 1.35 {
            (self.mip_width, self.mip_height, &self.mip_premul)
        } else {
            (self.base_width, self.base_height, &self.base_premul)
        };

        let inv_sx = 1.0 / sx;
        let inv_sy = 1.0 / sy;
        let tw_f = (tex_w - 1) as f32;
        let th_f = (tex_h - 1) as f32;
        let opacity_u32 = (tf.opacity.clamp(0.0, 1.0) * 255.0).round() as u32;

        for py in min_y..=max_y {
            let dy = (py as f32 + 0.5) - cy;
            for px in min_x..=max_x {
                let dx = (px as f32 + 0.5) - cx;

                // Inverse rotate & scale into normalized [-0.5, 0.5] UV space
                let rx = dx * cos_r + dy * sin_r;
                let ry = -dx * sin_r + dy * cos_r;

                let u = rx * inv_sx + 0.5;
                let v = ry * inv_sy + 0.5;

                if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                    continue;
                }

                let tx = u * tw_f;
                let ty = v * th_f;
                let [mut pr, mut pg, mut pb, mut pa] =
                    sample_bilinear_premul(tex_data, tex_w, tex_h, tx, ty);

                if pa == 0 {
                    continue;
                }
                if opacity_u32 < 255 {
                    pr = ((pr as u32 * opacity_u32 + 127) / 255) as u8;
                    pg = ((pg as u32 * opacity_u32 + 127) / 255) as u8;
                    pb = ((pb as u32 * opacity_u32 + 127) / 255) as u8;
                    pa = ((pa as u32 * opacity_u32 + 127) / 255) as u8;
                }

                pixmap.blend_premultiplied(px, py, pr, pg, pb, pa);
            }
        }

        (min_x, min_y, max_x, max_y)
    }
}

#[inline]
fn sample_bilinear_premul(data: &[u8], w: u32, h: u32, tx: f32, ty: f32) -> [u8; 4] {
    let x0 = (tx.floor() as i32).clamp(0, w as i32 - 1) as usize;
    let y0 = (ty.floor() as i32).clamp(0, h as i32 - 1) as usize;
    let x1 = (x0 + 1).min(w as usize - 1);
    let y1 = (y0 + 1).min(h as usize - 1);

    let fx = ((tx - x0 as f32) * 256.0) as u32;
    let fy = ((ty - y0 as f32) * 256.0) as u32;
    let inv_fx = 256 - fx;
    let inv_fy = 256 - fy;

    let w00 = inv_fx * inv_fy;
    let w10 = fx * inv_fy;
    let w01 = inv_fx * fy;
    let w11 = fx * fy;

    let stride = (w as usize) * 4;
    let i00 = y0 * stride + x0 * 4;
    let i10 = y0 * stride + x1 * 4;
    let i01 = y1 * stride + x0 * 4;
    let i11 = y1 * stride + x1 * 4;

    let r = (data[i00] as u32 * w00
        + data[i10] as u32 * w10
        + data[i01] as u32 * w01
        + data[i11] as u32 * w11
        + 32768)
        >> 16;
    let g = (data[i00 + 1] as u32 * w00
        + data[i10 + 1] as u32 * w10
        + data[i01 + 1] as u32 * w01
        + data[i11 + 1] as u32 * w11
        + 32768)
        >> 16;
    let b = (data[i00 + 2] as u32 * w00
        + data[i10 + 2] as u32 * w10
        + data[i01 + 2] as u32 * w01
        + data[i11 + 2] as u32 * w11
        + 32768)
        >> 16;
    let a = (data[i00 + 3] as u32 * w00
        + data[i10 + 3] as u32 * w10
        + data[i01 + 3] as u32 * w01
        + data[i11 + 3] as u32 * w11
        + 32768)
        >> 16;

    [r as u8, g as u8, b as u8, a as u8]
}

fn downsample_box(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];
    let scale_x = (src_w as f32) / (dst_w as f32);
    let scale_y = (src_h as f32) / (dst_h as f32);

    for dy in 0..dst_h {
        let sy0 = ((dy as f32) * scale_y).floor() as u32;
        let sy1 = (((dy + 1) as f32) * scale_y).ceil().min(src_h as f32) as u32;
        for dx in 0..dst_w {
            let sx0 = ((dx as f32) * scale_x).floor() as u32;
            let sx1 = (((dx + 1) as f32) * scale_x).ceil().min(src_w as f32) as u32;

            let mut r_sum = 0u32;
            let mut g_sum = 0u32;
            let mut b_sum = 0u32;
            let mut a_sum = 0u32;
            let mut count = 0u32;

            for sy in sy0..sy1.max(sy0 + 1) {
                for sx in sx0..sx1.max(sx0 + 1) {
                    let idx = ((sy * src_w + sx) * 4) as usize;
                    r_sum += src[idx] as u32;
                    g_sum += src[idx + 1] as u32;
                    b_sum += src[idx + 2] as u32;
                    a_sum += src[idx + 3] as u32;
                    count += 1;
                }
            }

            let out_idx = ((dy * dst_w + dx) * 4) as usize;
            if count > 0 {
                out[out_idx] = (r_sum / count) as u8;
                out[out_idx + 1] = (g_sum / count) as u8;
                out[out_idx + 2] = (b_sum / count) as u8;
                out[out_idx + 3] = (a_sum / count) as u8;
            }
        }
    }
    out
}
