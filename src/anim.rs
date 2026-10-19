//! Animations communes : courbe "ease-in-out" (départ et arrivée en douceur).
//!
//! egui interpole ses valeurs animées de façon linéaire ; ici on garde pour chaque
//! valeur animée son point de départ, sa cible et l'instant du changement, et on
//! applique une courbe cubique douce aux deux extrémités.

use eframe::egui::{Context, Id};

/// Courbe ease-in-out cubique : vitesse nulle au départ et à l'arrivée.
pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Progression adoucie d'une animation de `duration` secondes commencée il y a `elapsed` s.
pub fn progress(elapsed: f32, duration: f32) -> f32 {
    if duration <= 0.0 {
        1.0
    } else {
        ease_in_out(elapsed / duration)
    }
}

#[derive(Clone, Copy)]
struct Tween {
    from: f32,
    to: f32,
    start: f64,
}

impl Tween {
    fn value(&self, now: f64, duration: f32) -> f32 {
        let k = progress((now - self.start) as f32, duration);
        self.from + (self.to - self.from) * k
    }
}

/// Valeur animée vers `target` en ease-in-out sur `duration` secondes.
/// Si la cible change en cours de route, l'animation repart de la valeur actuelle.
pub fn smooth(ctx: &Context, id: Id, target: f32, duration: f32) -> f32 {
    let now = ctx.input(|i| i.time);
    let value = ctx.data_mut(|d| {
        let tw = d.get_temp_mut_or_insert_with(id, || Tween { from: target, to: target, start: now });
        if tw.to != target {
            let current = tw.value(now, duration);
            *tw = Tween { from: current, to: target, start: now };
        }
        tw.value(now, duration)
    });
    if (value - target).abs() > f32::EPSILON {
        ctx.request_repaint();
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_is_soft_at_both_ends() {
        assert_eq!(ease_in_out(0.0), 0.0);
        assert_eq!(ease_in_out(1.0), 1.0);
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-6);
        // pente quasi nulle au départ et à l'arrivée, maximale au milieu
        let h = 0.01;
        let start = ease_in_out(h) / h;
        let end = (1.0 - ease_in_out(1.0 - h)) / h;
        let mid = (ease_in_out(0.5 + h) - ease_in_out(0.5 - h)) / (2.0 * h);
        assert!(start < 0.01 && end < 0.01, "start={start} end={end}");
        assert!(mid > 1.4, "mid={mid}");
        // monotone
        let mut prev = 0.0;
        for i in 1..=100 {
            let v = ease_in_out(i as f32 / 100.0);
            assert!(v >= prev);
            prev = v;
        }
    }
}
