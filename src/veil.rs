//! Voile de fin de séance sur la page principale :
//! flou (copies décalées du contenu déjà dessiné), vignette plus sombre sur les bords
//! qu'au centre.

use crate::anim;
use crate::theme;
use eframe::egui::{self, epaint, Color32, Context, LayerId, Mesh, Rect, Shape, Vec2};

/// Durée d'apparition du voile (s).
pub const FADE: f32 = 0.7;

/// Floute les formes `[start, end)` du calque de fond : chaque forme est remplacée par
/// plusieurs copies décalées et transparentes (flou "multi-échantillons").
/// `skip_first` : ne pas flouter la première forme (fond du panneau).
pub fn blur_range(ctx: &Context, start: usize, end: usize, radius: f32) {
    if radius < 0.3 || end <= start {
        return;
    }
    // 8 directions sur le rayon complet + 4 sur un demi-rayon : 12 échantillons.
    let mut offsets: Vec<Vec2> = (0..8)
        .map(|k| {
            let a = k as f32 * std::f32::consts::TAU / 8.0;
            Vec2::new(a.cos(), a.sin()) * radius
        })
        .collect();
    offsets.extend((0..4).map(|k| {
        let a = (k as f32 + 0.5) * std::f32::consts::TAU / 4.0;
        Vec2::new(a.cos(), a.sin()) * radius * 0.5
    }));
    // Opacité de chaque copie : 12 copies superposées ≈ 90 % de couverture.
    let alpha = 1.0 - (0.1_f32).powf(1.0 / offsets.len() as f32);

    ctx.graphics_mut(|g| {
        let list = g.entry(LayerId::background());
        let originals: Vec<epaint::ClippedShape> = list.all_entries().skip(start).take(end - start).cloned().collect();
        let mut blurred = Vec::with_capacity(originals.len() * offsets.len());
        for (i, cs) in originals.iter().enumerate() {
            // Le fond plein du panneau reste net (sinon il "salit" les copies suivantes).
            if i == 0 {
                continue;
            }
            for o in &offsets {
                let mut shape = cs.shape.clone();
                shape.translate(*o);
                epaint::shape_transform::adjust_colors(&mut shape, move |c| {
                    if *c != Color32::PLACEHOLDER {
                        *c = c.gamma_multiply(alpha);
                    }
                });
                blurred.push((cs.clip_rect, shape));
            }
        }
        for idx in (start + 1)..end {
            list.reset_shape(egui::layers::ShapeIdx(idx));
        }
        for (clip, shape) in blurred {
            list.add(clip, shape);
        }
    });
}

/// Dessine la vignette sur `rect`. `k` = progression d'apparition (0..1).
pub fn paint(p: &egui::Painter, rect: Rect, k: f32) {
    let base = theme::bg();
    // --- vignette : anneaux concentriques, plus opaques vers les bords
    let rings: [(f32, f32); 4] = [(0.0, 0.42), (0.55, 0.58), (0.85, 0.80), (1.0, 0.93)];
    let seg = 64;
    let c = rect.center();
    // point sur le bord du rectangle dans la direction `a`, à la fraction `f`
    let at = |a: f32, f: f32| {
        let d = Vec2::new(a.cos(), a.sin());
        let tx = if d.x.abs() > 1e-6 { (rect.width() / 2.0) / d.x.abs() } else { f32::INFINITY };
        let ty = if d.y.abs() > 1e-6 { (rect.height() / 2.0) / d.y.abs() } else { f32::INFINITY };
        let edge = tx.min(ty);
        // les anneaux intérieurs sont des ellipses, l'extérieur épouse le rectangle
        let ell = Vec2::new(d.x * rect.width() / 2.0, d.y * rect.height() / 2.0).length();
        let r = if f >= 1.0 { edge * 1.08 } else { ell * f };
        c + d * r
    };
    let mut m = Mesh::default();
    let color = |alpha: f32| {
        let a = (alpha * k).clamp(0.0, 1.0);
        let dark = Color32::from_rgb(base.r() / 3, base.g() / 3, base.b() / 3);
        dark.gamma_multiply(a)
    };
    // centre
    m.colored_vertex(c, color(rings[0].1));
    for ring in 1..rings.len() {
        for s in 0..seg {
            let a = s as f32 * std::f32::consts::TAU / seg as f32;
            m.colored_vertex(at(a, rings[ring].0), color(rings[ring].1));
        }
    }
    // triangles centre -> anneau 1
    for s in 0..seg as u32 {
        m.add_triangle(0, 1 + s, 1 + (s + 1) % seg as u32);
    }
    for ring in 1..(rings.len() as u32 - 1) {
        let a0 = 1 + (ring - 1) * seg as u32;
        let b0 = 1 + ring * seg as u32;
        for s in 0..seg as u32 {
            let s1 = (s + 1) % seg as u32;
            m.add_triangle(a0 + s, b0 + s, b0 + s1);
            m.add_triangle(a0 + s, b0 + s1, a0 + s1);
        }
    }
    p.add(Shape::mesh(m));
}

/// Progression adoucie du voile depuis `since` secondes.
pub fn progress(since: f32) -> f32 {
    anim::progress(since, FADE)
}
