//! Filtre « VHS » sur toute la fenêtre : grain qui scintille, lignes de balayage,
//! bande de tracking qui descend, parasites horizontaux. Tout est réglable
//! (Réglages › Développement › Filtre VHS) et dessiné sur le calque le plus haut,
//! sans intercepter les clics.

use eframe::egui::{self, Color32, Context, Id, LayerId, Mesh, Order, Pos2, Rect, Shape, TextureHandle, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct VhsFilter {
    pub enabled: bool,
    /// Grain (0..1).
    pub grain: f32,
    /// Lignes de balayage (0..1).
    pub scanlines: f32,
    /// Bande de tracking qui défile (0..1).
    pub tracking: f32,
    /// Parasites horizontaux aléatoires (0..1).
    pub glitches: f32,
    /// Vitesse de l'animation (0 = figé, 1 = normal, 2 = rapide).
    pub speed: f32,
}

impl Default for VhsFilter {
    fn default() -> Self {
        Self { enabled: false, grain: 0.22, scanlines: 0.0, tracking: 0.0, glitches: 0.13, speed: 0.6 }
    }
}

impl VhsFilter {
    /// Le filtre a besoin d'être redessiné en continu.
    pub fn animated(&self) -> bool {
        self.enabled && self.speed > 0.0 && (self.grain > 0.0 || self.tracking > 0.0 || self.glitches > 0.0)
    }
}

/// Textures du filtre (créées une fois).
pub struct VhsTextures {
    noise: TextureHandle,
    lines: TextureHandle,
}

const NOISE: usize = 256;

impl VhsTextures {
    pub fn new(ctx: &Context) -> Self {
        let mut seed: u32 = 0x1234_5678;
        let pixels: Vec<Color32> = (0..NOISE * NOISE)
            .map(|_| {
                let v = (xorshift(&mut seed) & 0xFF) as u8;
                Color32::from_rgba_unmultiplied(v, v, v, 255)
            })
            .collect();
        let rep = egui::TextureOptions { wrap_mode: egui::TextureWrapMode::Repeat, ..egui::TextureOptions::NEAREST };
        let noise = ctx.load_texture("vhs_noise", egui::ColorImage { size: [NOISE, NOISE], pixels }, rep);
        // Motif de balayage sur 4 pixels : une ligne sombre, trois claires.
        let lines = ctx.load_texture(
            "vhs_lines",
            egui::ColorImage {
                size: [1, 4],
                pixels: vec![Color32::from_black_alpha(255), Color32::TRANSPARENT, Color32::TRANSPARENT, Color32::from_black_alpha(90)],
            },
            rep,
        );
        Self { noise, lines }
    }
}

fn xorshift(s: &mut u32) -> u32 {
    *s ^= *s << 13;
    *s ^= *s >> 17;
    *s ^= *s << 5;
    *s
}

/// Nombre pseudo-aléatoire 0..1 déterministe pour (image, n).
fn rand01(frame: u64, n: u32) -> f32 {
    let mut s = (frame as u32).wrapping_mul(0x9E37_79B9) ^ n.wrapping_mul(0x85EB_CA6B) ^ 0xC2B2_AE35;
    xorshift(&mut s);
    xorshift(&mut s);
    (s & 0xFFFF) as f32 / 65535.0
}

fn tex_rect(tex: &TextureHandle, rect: Rect, uv: Rect, tint: Color32) -> Shape {
    let mut m = Mesh::with_texture(tex.id());
    m.add_rect_with_uv(rect, uv, tint);
    Shape::mesh(m)
}

/// Calque des filtres plein écran (au-dessus de tout, VHS puis bitmap).
pub fn layer() -> LayerId {
    LayerId::new(Order::Debug, Id::new("post_filters"))
}

/// Dessine le filtre au-dessus de toute l'interface.
pub fn paint(ctx: &Context, f: &VhsFilter, tex: &VhsTextures) {
    if !f.enabled {
        return;
    }
    let screen = ctx.screen_rect();
    let p = ctx.layer_painter(layer());
    let t = ctx.input(|i| i.time) * f.speed.max(0.0) as f64;
    // Le grain change ~24 fois par seconde, comme une image vidéo.
    let frame = if f.speed > 0.0 { (t * 24.0) as u64 } else { 0 };
    let (w, h) = (screen.width(), screen.height());

    // --- grain : la texture de bruit est décalée au hasard à chaque image
    if f.grain > 0.0 {
        let off = Vec2::new(rand01(frame, 1), rand01(frame, 2));
        let scale = 1.0 / NOISE as f32;
        let uv = Rect::from_min_size(Pos2::new(off.x, off.y), Vec2::new(w * scale, h * scale));
        // Grain clair puis grain sombre (motif inversé), pour un bruit "neutre".
        let a = (f.grain.clamp(0.0, 1.0) * 0.16 * 255.0) as u8;
        p.add(tex_rect(&tex.noise, screen, uv, Color32::from_rgba_premultiplied(a, a, a, a)));
        let uv2 = uv.translate(Vec2::new(0.37, 0.61));
        p.add(tex_rect(&tex.noise, screen, uv2, Color32::from_black_alpha((f.grain.clamp(0.0, 1.0) * 0.22 * 255.0) as u8)));
    }

    // --- lignes de balayage (défilent très lentement vers le bas)
    if f.scanlines > 0.0 {
        let crawl = ((t * 6.0) % 4.0) as f32 / 4.0;
        let uv = Rect::from_min_size(Pos2::new(0.0, -crawl), Vec2::new(1.0, h / 4.0));
        let a = (f.scanlines.clamp(0.0, 1.0) * 0.45 * 255.0) as u8;
        p.add(tex_rect(&tex.lines, screen, uv, Color32::from_white_alpha(255).gamma_multiply(a as f32 / 255.0)));
    }

    // --- bande de tracking : bande de bruit plus fort qui descend en boucle
    if f.tracking > 0.0 {
        let band_h = 26.0 + 50.0 * f.tracking;
        let period = h + band_h * 2.0;
        let y = (((t * 70.0) as f32) % period) - band_h;
        let band = Rect::from_min_size(Pos2::new(screen.left(), screen.top() + y), Vec2::new(w, band_h));
        let k = f.tracking.clamp(0.0, 1.0);
        // léger voile clair en dégradé (fort au centre de la bande)
        let mut m = Mesh::default();
        let mid = band.center().y;
        for (yy, a) in [(band.top(), 0.0), (mid, 0.10 * k), (band.bottom(), 0.0)] {
            let c = Color32::from_rgba_premultiplied((255.0 * a) as u8, (255.0 * a) as u8, (255.0 * a) as u8, (255.0 * a) as u8);
            m.colored_vertex(Pos2::new(band.left(), yy), c);
            m.colored_vertex(Pos2::new(band.right(), yy), c);
        }
        m.add_triangle(0, 1, 3);
        m.add_triangle(0, 3, 2);
        m.add_triangle(2, 3, 5);
        m.add_triangle(2, 5, 4);
        p.add(Shape::mesh(m));
        // bruit plus dense dans la bande, avec un décalage horizontal saccadé
        let jitter = (rand01(frame, 7) - 0.5) * 0.2;
        let uv = Rect::from_min_size(Pos2::new(rand01(frame, 3) + jitter, rand01(frame, 4)), Vec2::new(w / NOISE as f32 * 0.25, band_h / NOISE as f32));
        let a = (k * 0.35 * 255.0) as u8;
        p.add(tex_rect(&tex.noise, band, uv, Color32::from_rgba_premultiplied(a, a, a, a)));
    }

    // --- parasites : quelques traits horizontaux aléatoires
    if f.glitches > 0.0 {
        let n = (f.glitches * 6.0).round() as u32;
        for i in 0..n {
            if rand01(frame, 100 + i) > 0.35 + 0.4 * (1.0 - f.glitches) {
                continue;
            }
            let y = screen.top() + rand01(frame, 200 + i) * h;
            let x0 = screen.left() + rand01(frame, 300 + i) * w * 0.7;
            let len = w * (0.08 + 0.5 * rand01(frame, 400 + i));
            let thick = 1.0 + 2.0 * rand01(frame, 500 + i);
            let a = 0.10 + 0.25 * rand01(frame, 600 + i) * f.glitches;
            p.rect_filled(
                Rect::from_min_size(Pos2::new(x0, y), Vec2::new(len, thick)),
                0.0,
                Color32::from_rgba_premultiplied((255.0 * a) as u8, (255.0 * a) as u8, (255.0 * a) as u8, (255.0 * a) as u8),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rand_is_deterministic_and_bounded() {
        for f in 0..50u64 {
            for n in 0..20u32 {
                let a = rand01(f, n);
                assert!((0.0..=1.0).contains(&a));
                assert_eq!(a, rand01(f, n));
            }
        }
        assert_ne!(rand01(1, 1), rand01(2, 1));
    }

    #[test]
    fn animation_flag() {
        let mut f = VhsFilter { enabled: true, ..VhsFilter::default() };
        assert!(f.animated());
        f.speed = 0.0;
        assert!(!f.animated());
        f.speed = 1.0;
        f.enabled = false;
        assert!(!f.animated());
    }
}
