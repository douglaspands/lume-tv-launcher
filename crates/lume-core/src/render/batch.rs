//! Turns a [`DisplayList`] into vertex data and draw batches.
//!
//! Painter's order is preserved: consecutive commands that use the same shader program
//! and the same clip rectangle share one batch, and a change of either starts a new one.
//! GLES2 has no instancing, so every primitive becomes a quad of four vertices that the
//! backend draws with a shared index pattern.

use super::geometry::Rect;
use super::list::{Cmd, DisplayList, RectCmd, RectKind, TexturedCmd};
use crate::theme::{Color, Paint};
use bytemuck::{Pod, Zeroable};

/// Most quads one batch may hold, so vertex indices fit in `u16`.
pub const MAX_QUADS_PER_BATCH: u32 = 4096;

/// Indices of the two triangles of a quad, relative to its first vertex.
pub const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];

/// Pixels of antialiasing margin added around every rectangle quad.
const AA_MARGIN: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Program {
    /// Rounded rectangles drawn from a signed distance function.
    Rect,
    /// Glyphs and icons sampled from the 8-bit atlas.
    Text,
    /// RGBA images.
    Image,
}

/// The texture an image batch samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Texture {
    /// The RGBA atlas of app icons (also what the other programs use, which ignore it).
    Icons,
    /// The opaque RGB wallpaper.
    Wallpaper,
}

/// One draw call: `quad_count` quads of `program`, starting at `first_quad` in that
/// program's vertex buffer, clipped to `clip` (logical units) when set. Image batches sample
/// `texture`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Batch {
    pub program: Program,
    pub texture: Texture,
    pub clip: Option<Rect>,
    pub first_quad: u32,
    pub quad_count: u32,
}

/// Vertex of the rectangle program. Every field is an `f32` so the struct has no padding,
/// and the fields form seven consecutive `vec4` shader attributes. The same values are
/// repeated on the four vertices of a quad.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RectVertex {
    /// Attribute 0, xy: position on the canvas (logical units).
    pub position: [f32; 2],
    /// Attribute 0, zw: position relative to the rectangle center.
    pub local: [f32; 2],
    /// Attribute 1, xy: half of the rectangle size.
    pub half_size: [f32; 2],
    /// Attribute 1, z: corner radius.
    pub radius: f32,
    /// Attribute 1, w: border width for rings; 0 for filled shapes.
    pub ring: f32,
    /// Attribute 2, x: shadow softness; 0 for hard edges.
    pub softness: f32,
    /// Attribute 2, yz: gradient axis already divided by the gradient length:
    /// `t = dot(local, gradient) + 0.5`.
    pub gradient: [f32; 2],
    /// Attribute 2, w: 1.0 when the fill is a gradient.
    pub is_gradient: f32,
    /// Attribute 3, xyz: gradient stop positions.
    pub stops: [f32; 3],
    /// Attribute 3, w: unused.
    pub _padding: f32,
    /// Attributes 4 to 6: premultiplied RGBA of up to three stops (solid fills use the first).
    pub colors: [[f32; 4]; 3],
}

/// Vertex of the text and image programs. Four vec4 attributes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct TexVertex {
    /// Attribute 0, xy: position on the canvas (logical units).
    pub position: [f32; 2],
    /// Attribute 0, zw: texture coordinates.
    pub uv: [f32; 2],
    /// Attribute 1: premultiplied RGBA tint.
    pub color: [f32; 4],
    /// Attribute 2, xy: position relative to the quad center.
    pub local: [f32; 2],
    /// Attribute 2, zw: half of the quad size.
    pub half_size: [f32; 2],
    /// Attribute 3, x: corner radius.
    pub radius: f32,
    /// Attribute 3, yzw: unused padding to complete vec4.
    pub _padding: [f32; 3],
}

/// Everything a backend needs to draw one frame.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub rect_vertices: Vec<RectVertex>,
    pub text_vertices: Vec<TexVertex>,
    pub image_vertices: Vec<TexVertex>,
    pub batches: Vec<Batch>,
}

impl Frame {
    /// Builds the frame for `list`.
    pub fn build(list: &DisplayList) -> Frame {
        let mut builder = Builder::default();
        for cmd in list.cmds() {
            builder.add(cmd);
        }
        builder.frame
    }

    /// Whether any batch samples the wallpaper texture.
    pub fn uses_wallpaper(&self) -> bool {
        self.batches
            .iter()
            .any(|b| b.program == Program::Image && b.texture == Texture::Wallpaper)
    }

    /// Vertices of `program`.
    pub fn quad_count(&self, program: Program) -> usize {
        match program {
            Program::Rect => self.rect_vertices.len() / 4,
            Program::Text => self.text_vertices.len() / 4,
            Program::Image => self.image_vertices.len() / 4,
        }
    }
}

#[derive(Default)]
struct Builder {
    frame: Frame,
    /// Active clip rectangles; the effective clip is their intersection.
    clips: Vec<Rect>,
    /// Effective clip; `Some(None)` when it is empty and nothing can be seen.
    effective: Option<Option<Rect>>,
}

impl Builder {
    fn add(&mut self, cmd: &Cmd) {
        match cmd {
            Cmd::PushClip(rect) => {
                self.clips.push(*rect);
                self.recompute_clip();
            }
            Cmd::PopClip => {
                self.clips.pop();
                self.recompute_clip();
            }
            Cmd::Rect(rect) => self.add_rect(rect),
            Cmd::Glyph(quad) => self.add_textured(Program::Text, Texture::Icons, quad),
            Cmd::Image(quad) => self.add_textured(Program::Image, Texture::Icons, quad),
            Cmd::Wallpaper(quad) => self.add_textured(Program::Image, Texture::Wallpaper, quad),
        }
    }

    fn recompute_clip(&mut self) {
        let mut clip: Option<Rect> = None;
        let mut empty = false;
        for rect in &self.clips {
            clip = match clip {
                None => Some(*rect),
                Some(current) => {
                    let inter = current.intersection(rect);
                    empty |= inter.is_none();
                    inter.or(Some(Rect::default()))
                }
            };
            empty |= rect.is_empty();
        }
        self.effective = if self.clips.is_empty() {
            None
        } else if empty {
            Some(None)
        } else {
            Some(clip)
        };
    }

    /// The clip to attach to a batch, or `None` when everything is clipped away.
    fn visible_clip(&self) -> Option<Option<Rect>> {
        match self.effective {
            None => Some(None),
            Some(None) => None,
            Some(Some(rect)) => Some(Some(rect)),
        }
    }

    fn start_quad(&mut self, program: Program, texture: Texture, clip: Option<Rect>) -> u32 {
        let first_quad = self.frame.quad_count(program) as u32;
        match self.frame.batches.last_mut() {
            Some(batch)
                if batch.program == program
                    && batch.texture == texture
                    && batch.clip == clip
                    && batch.quad_count < MAX_QUADS_PER_BATCH
                    && batch.first_quad + batch.quad_count == first_quad =>
            {
                batch.quad_count += 1;
            }
            _ => self.frame.batches.push(Batch {
                program,
                texture,
                clip,
                first_quad,
                quad_count: 1,
            }),
        }
        first_quad
    }

    fn add_textured(&mut self, program: Program, texture: Texture, cmd: &TexturedCmd) {
        let Some(clip) = self.visible_clip() else {
            return;
        };
        if cmd.dest.is_empty() || clip.is_some_and(|c| !c.intersects(&cmd.dest)) {
            return;
        }
        self.start_quad(program, texture, clip);
        let color = premultiplied(cmd.color, 1.0);
        let (d, uv) = (cmd.dest, cmd.uv);
        let half_size = [d.w / 2.0, d.h / 2.0];
        let radius = cmd.radius.min(half_size[0]).min(half_size[1]).max(0.0);
        let corners = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        let target = match program {
            Program::Image => &mut self.frame.image_vertices,
            _ => &mut self.frame.text_vertices,
        };
        for (cx, cy) in corners {
            target.push(TexVertex {
                position: [d.x + cx * d.w, d.y + cy * d.h],
                uv: [uv.x + cx * uv.w, uv.y + cy * uv.h],
                color,
                local: [(cx - 0.5) * d.w, (cy - 0.5) * d.h],
                half_size,
                radius,
                _padding: [0.0; 3],
            });
        }
    }

    fn add_rect(&mut self, cmd: &RectCmd) {
        let Some(clip) = self.visible_clip() else {
            return;
        };
        let (softness, grow) = match cmd.kind {
            RectKind::Shadow { blur, .. } => (blur * 0.5, blur + AA_MARGIN),
            _ => (0.0, AA_MARGIN),
        };
        let quad = cmd.rect.grow(grow);
        if cmd.rect.is_empty() || clip.is_some_and(|c| !c.intersects(&quad)) {
            return;
        }
        self.start_quad(Program::Rect, Texture::Icons, clip);

        let mut vertex = RectVertex {
            half_size: [cmd.rect.w / 2.0, cmd.rect.h / 2.0],
            radius: cmd
                .radius
                .min(cmd.rect.w / 2.0)
                .min(cmd.rect.h / 2.0)
                .max(0.0),
            softness,
            ..RectVertex::default()
        };
        match cmd.kind {
            RectKind::Fill(Paint::Solid(color)) => {
                vertex.colors[0] = premultiplied(color, cmd.opacity);
            }
            RectKind::Fill(Paint::Gradient(gradient)) => {
                vertex.is_gradient = 1.0;
                let theta = gradient.angle_deg.to_radians();
                let (sin, cos) = theta.sin_cos();
                let length = (cmd.rect.w * sin).abs() + (cmd.rect.h * cos).abs();
                let inv = if length > 0.0 { 1.0 / length } else { 0.0 };
                vertex.gradient = [sin * inv, -cos * inv];
                let stops = gradient.stops();
                for i in 0..3 {
                    // Missing stops repeat the last one.
                    let stop = stops[i.min(stops.len() - 1)];
                    vertex.colors[i] = premultiplied(stop.color, cmd.opacity);
                    vertex.stops[i] = stop.position;
                }
            }
            RectKind::Ring { width, color } => {
                vertex.ring = width;
                vertex.colors[0] = premultiplied(color, cmd.opacity);
            }
            RectKind::Shadow { color, .. } => {
                vertex.colors[0] = premultiplied(color, cmd.opacity);
            }
        }

        let (cx, cy) = cmd.rect.center();
        for (corner_x, corner_y) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
            let x = quad.x + corner_x * quad.w;
            let y = quad.y + corner_y * quad.h;
            self.frame.rect_vertices.push(RectVertex {
                position: [x, y],
                local: [x - cx, y - cy],
                ..vertex
            });
        }
    }
}

fn premultiplied(color: Color, opacity: f32) -> [f32; 4] {
    let a = f32::from(color.a) / 255.0 * opacity.clamp(0.0, 1.0);
    [
        f32::from(color.r) / 255.0 * a,
        f32::from(color.g) / 255.0 * a,
        f32::from(color.b) / 255.0 * a,
        a,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Gradient, GradientStop};

    const WHITE: Color = Color::WHITE;
    const R: Rect = Rect::new(10.0, 10.0, 100.0, 50.0);

    fn solid() -> Paint {
        Paint::Solid(Color::rgb(255, 0, 0))
    }

    fn glyph_at(list: &mut DisplayList, x: f32) {
        list.glyph(
            Rect::new(x, 0.0, 10.0, 10.0),
            Rect::new(0.0, 0.0, 0.1, 0.1),
            WHITE,
        );
    }

    #[test]
    fn vertex_structs_have_no_padding() {
        assert_eq!(std::mem::size_of::<RectVertex>(), 28 * 4);
        assert_eq!(std::mem::size_of::<TexVertex>(), 16 * 4);
    }

    #[test]
    fn image_rounded_records_radius_and_half_size_in_vertices() {
        let mut list = DisplayList::new();
        list.image_rounded(
            Rect::new(10.0, 20.0, 64.0, 64.0),
            Rect::new(0.0, 0.0, 1.0, 1.0),
            Color::WHITE,
            32.0,
        );
        let frame = Frame::build(&list);
        assert_eq!(frame.image_vertices.len(), 4);
        for v in &frame.image_vertices {
            assert_eq!(v.half_size, [32.0, 32.0]);
            assert_eq!(v.radius, 32.0);
        }
        assert_eq!(frame.image_vertices[0].local, [-32.0, -32.0]);
        assert_eq!(frame.image_vertices[3].local, [32.0, 32.0]);
    }

    #[test]
    fn empty_list_builds_an_empty_frame() {
        let frame = Frame::build(&DisplayList::new());
        assert!(frame.batches.is_empty() && frame.rect_vertices.is_empty());
    }

    #[test]
    fn consecutive_rects_share_one_batch_in_order() {
        let mut list = DisplayList::new();
        list.fill(R, 8.0, solid());
        list.fill(R.translate(120.0, 0.0), 8.0, solid());
        list.ring(R, 8.0, 2.0, WHITE);
        let frame = Frame::build(&list);
        assert_eq!(frame.batches.len(), 1);
        assert_eq!(
            frame.batches[0],
            Batch {
                program: Program::Rect,
                texture: Texture::Icons,
                clip: None,
                first_quad: 0,
                quad_count: 3
            }
        );
        assert_eq!(frame.rect_vertices.len(), 12);
        // Painter's order: the second quad is to the right of the first, the ring is last.
        assert!(frame.rect_vertices[4].position[0] > frame.rect_vertices[0].position[0] + 100.0);
        assert_eq!(frame.rect_vertices[8].ring, 2.0);
    }

    fn image_at(list: &mut DisplayList, x: f32, wallpaper: bool) {
        let (dest, uv) = (Rect::new(x, 0.0, 10.0, 10.0), Rect::new(0.0, 0.0, 1.0, 1.0));
        if wallpaper {
            list.wallpaper(dest, uv, WHITE);
        } else {
            list.image(dest, uv, WHITE);
        }
    }

    #[test]
    fn a_texture_change_breaks_the_image_batch_and_only_then() {
        let mut list = DisplayList::new();
        image_at(&mut list, 0.0, true);
        list.fill(R, 0.0, solid());
        image_at(&mut list, 20.0, false);
        image_at(&mut list, 30.0, false);
        image_at(&mut list, 40.0, true);
        let frame = Frame::build(&list);
        let shape: Vec<_> = frame
            .batches
            .iter()
            .map(|b| (b.program, b.texture, b.quad_count))
            .collect();
        assert_eq!(
            shape,
            [
                (Program::Image, Texture::Wallpaper, 1),
                (Program::Rect, Texture::Icons, 1),
                (Program::Image, Texture::Icons, 2),
                (Program::Image, Texture::Wallpaper, 1),
            ]
        );
        // Both textures share the image vertex buffer, in painter's order.
        assert_eq!(frame.image_vertices.len(), 16);
        assert!(frame.uses_wallpaper());
    }

    #[test]
    fn consecutive_wallpaper_quads_share_a_batch_and_icons_alone_do_not_use_it() {
        let mut list = DisplayList::new();
        image_at(&mut list, 0.0, true);
        image_at(&mut list, 20.0, true);
        assert_eq!(Frame::build(&list).batches.len(), 1);
        let mut icons = DisplayList::new();
        image_at(&mut icons, 0.0, false);
        assert!(!Frame::build(&icons).uses_wallpaper());
    }

    #[test]
    fn program_change_starts_a_new_batch_and_keeps_the_order() {
        let mut list = DisplayList::new();
        list.fill(R, 8.0, solid());
        glyph_at(&mut list, 20.0);
        glyph_at(&mut list, 30.0);
        list.fill(R, 8.0, solid());
        let frame = Frame::build(&list);
        let programs: Vec<_> = frame
            .batches
            .iter()
            .map(|b| (b.program, b.first_quad, b.quad_count))
            .collect();
        assert_eq!(
            programs,
            [
                (Program::Rect, 0, 1),
                (Program::Text, 0, 2),
                (Program::Rect, 1, 1)
            ]
        );
    }

    #[test]
    fn same_program_after_another_continues_numbering_per_program() {
        let mut list = DisplayList::new();
        glyph_at(&mut list, 0.0);
        list.fill(R, 0.0, solid());
        glyph_at(&mut list, 20.0);
        let frame = Frame::build(&list);
        assert_eq!(frame.batches[2].first_quad, 1);
        assert_eq!(frame.text_vertices.len(), 8);
    }

    #[test]
    fn clips_split_batches_and_restore_after_pop() {
        let clip = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut list = DisplayList::new();
        list.fill(R, 0.0, solid());
        list.push_clip(clip);
        list.fill(R, 0.0, solid());
        list.fill(R.translate(5.0, 5.0), 0.0, solid());
        list.pop_clip();
        list.fill(R, 0.0, solid());
        let frame = Frame::build(&list);
        let shape: Vec<_> = frame
            .batches
            .iter()
            .map(|b| (b.clip, b.quad_count))
            .collect();
        assert_eq!(shape, [(None, 1), (Some(clip), 2), (None, 1)]);
    }

    #[test]
    fn nested_clips_intersect() {
        let mut list = DisplayList::new();
        list.push_clip(Rect::new(0.0, 0.0, 100.0, 100.0));
        list.push_clip(Rect::new(50.0, 50.0, 100.0, 100.0));
        list.fill(Rect::new(60.0, 60.0, 10.0, 10.0), 0.0, solid());
        list.pop_clip();
        list.fill(Rect::new(60.0, 60.0, 10.0, 10.0), 0.0, solid());
        let frame = Frame::build(&list);
        assert_eq!(
            frame.batches[0].clip,
            Some(Rect::new(50.0, 50.0, 50.0, 50.0))
        );
        assert_eq!(
            frame.batches[1].clip,
            Some(Rect::new(0.0, 0.0, 100.0, 100.0))
        );
    }

    #[test]
    fn primitives_outside_the_clip_are_culled() {
        let mut list = DisplayList::new();
        list.push_clip(Rect::new(0.0, 0.0, 100.0, 100.0));
        list.fill(Rect::new(500.0, 500.0, 50.0, 50.0), 0.0, solid());
        glyph_at(&mut list, 400.0);
        list.fill(Rect::new(10.0, 10.0, 20.0, 20.0), 0.0, solid());
        let frame = Frame::build(&list);
        assert_eq!(frame.batches.len(), 1);
        assert_eq!(frame.batches[0].quad_count, 1);
        assert!(frame.text_vertices.is_empty());
    }

    #[test]
    fn disjoint_nested_clips_hide_everything_until_popped() {
        let mut list = DisplayList::new();
        list.push_clip(Rect::new(0.0, 0.0, 10.0, 10.0));
        list.push_clip(Rect::new(100.0, 100.0, 10.0, 10.0));
        list.fill(Rect::new(0.0, 0.0, 200.0, 200.0), 0.0, solid());
        list.pop_clip();
        list.fill(Rect::new(0.0, 0.0, 5.0, 5.0), 0.0, solid());
        let frame = Frame::build(&list);
        assert_eq!(frame.batches.len(), 1);
        assert_eq!(frame.batches[0].clip, Some(Rect::new(0.0, 0.0, 10.0, 10.0)));
    }

    #[test]
    fn quads_are_premultiplied_and_carry_local_coordinates() {
        let mut list = DisplayList::new();
        list.fill(
            R,
            8.0,
            Paint::Solid(Color {
                r: 255,
                g: 0,
                b: 0,
                a: 128,
            }),
        );
        let frame = Frame::build(&list);
        let v = frame.rect_vertices[0];
        assert!((v.colors[0][0] - 128.0 / 255.0).abs() < 1e-4);
        assert!((v.colors[0][3] - 128.0 / 255.0).abs() < 1e-4);
        assert_eq!(v.half_size, [50.0, 25.0]);
        // The quad covers the rect plus the antialiasing margin, centered on it.
        assert_eq!(v.position, [R.x - AA_MARGIN, R.y - AA_MARGIN]);
        assert_eq!(v.local, [-50.0 - AA_MARGIN, -25.0 - AA_MARGIN]);
    }

    #[test]
    fn radius_is_limited_to_half_the_shorter_side() {
        let mut list = DisplayList::new();
        list.fill(Rect::new(0.0, 0.0, 40.0, 20.0), 999.0, solid());
        assert_eq!(Frame::build(&list).rect_vertices[0].radius, 10.0);
    }

    #[test]
    fn gradient_axis_follows_the_css_angle() {
        const STOPS: [GradientStop; 2] = [
            GradientStop {
                color: Color::rgb(0, 0, 0),
                position: 0.0,
            },
            GradientStop {
                color: Color::rgb(255, 255, 255),
                position: 1.0,
            },
        ];
        let stops = STOPS;
        let mut list = DisplayList::new();
        // 90deg runs left to right: the axis is horizontal and spans the width.
        list.fill(
            Rect::new(0.0, 0.0, 200.0, 100.0),
            0.0,
            Paint::Gradient(Gradient::new(90.0, &stops)),
        );
        // 180deg runs top to bottom.
        list.fill(
            Rect::new(0.0, 0.0, 200.0, 100.0),
            0.0,
            Paint::Gradient(Gradient::new(180.0, &stops)),
        );
        let frame = Frame::build(&list);
        let (right, down) = (frame.rect_vertices[0], frame.rect_vertices[4]);
        assert!((right.gradient[0] - 1.0 / 200.0).abs() < 1e-5 && right.gradient[1].abs() < 1e-5);
        assert!(down.gradient[0].abs() < 1e-5 && (down.gradient[1] - 1.0 / 100.0).abs() < 1e-5);
        assert_eq!(right.is_gradient, 1.0);
        // Two stops: the third repeats the last.
        assert_eq!(right.stops, [0.0, 1.0, 1.0]);
        assert_eq!(right.colors[2], right.colors[1]);
    }

    #[test]
    fn shadow_quad_is_larger_and_soft() {
        let mut list = DisplayList::new();
        list.shadow(
            R,
            8.0,
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 128,
            },
            24.0,
        );
        let v = Frame::build(&list).rect_vertices[0];
        assert_eq!(v.softness, 12.0);
        assert_eq!(v.position[0], R.x - 24.0 - AA_MARGIN);
    }

    #[test]
    fn opacity_scales_alpha() {
        let mut list = DisplayList::new();
        list.push(Cmd::Rect(RectCmd {
            rect: R,
            radius: 0.0,
            kind: RectKind::Fill(Paint::Solid(WHITE)),
            opacity: 0.5,
        }));
        assert!((Frame::build(&list).rect_vertices[0].colors[0][3] - 0.5).abs() < 1e-5);
    }
}
