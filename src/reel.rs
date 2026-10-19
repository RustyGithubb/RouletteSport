//! Cinématique d'une roue verticale (défilement du haut vers le bas).
//!
//! Positions exprimées en "lignes" : la ligne `k` est au centre de la fenêtre
//! quand `pos == k`. Quand `pos` augmente, chaque ligne descend.
//! La vitesse est plafonnée pour éviter l'effet stroboscopique
//! (au-delà d'une demi-ligne par image, l'œil perçoit la roue tourner à l'envers).

/// Vitesse de croisière (lignes / seconde). 12 l/s = 0,2 ligne par image à 60 i/s.
const V_MAX: f32 = 12.0;
/// Durée de la mise en rotation (s).
const ACCEL: f32 = 0.30;
/// Écart angulaire entre deux lignes sur le cylindre (radians).
pub const ROW_ANGLE: f32 = 0.62;

/// Profil de mouvement d'une roue qui s'arrête à l'instant `stop`.
#[derive(Clone, Copy, Debug)]
pub struct Reel {
    accel: f32,
    cruise: f32,
    decel: f32,
    v: f32,
    /// Index entier de la ligne finale (le nom tiré).
    pub target: i64,
}

impl Reel {
    pub fn new(stop: f32) -> Self {
        let accel = ACCEL.min(stop * 0.3);
        let decel = (stop * 0.45).clamp(0.4, 1.2);
        let cruise = (stop - accel - decel).max(0.0);
        let shape = accel / 2.0 + cruise + decel / 2.0;
        // On arrondit la distance à une ligne entière, puis on ajuste la vitesse
        // pour atterrir pile sur le nom tiré.
        let target = (V_MAX * shape).round().max(1.0) as i64;
        let v = target as f32 / shape;
        Self { accel, cruise, decel, v, target }
    }

    #[cfg(test)]
    pub fn duration(&self) -> f32 {
        self.accel + self.cruise + self.decel
    }

    /// Position (en lignes) à l'instant `t`.
    pub fn pos(&self, t: f32) -> f32 {
        let (a, c, d, v) = (self.accel, self.cruise, self.decel, self.v);
        if t <= 0.0 {
            0.0
        } else if t < a {
            v * t * t / (2.0 * a)
        } else if t < a + c {
            v * (a / 2.0 + (t - a))
        } else if t < a + c + d {
            let u = t - a - c;
            v * (a / 2.0 + c + u - u * u / (2.0 * d))
        } else {
            self.target as f32
        }
    }

    /// Vitesse (lignes / s) à l'instant `t`.
    #[cfg(test)]
    pub fn speed(&self, t: f32) -> f32 {
        let (a, c, d, v) = (self.accel, self.cruise, self.decel, self.v);
        if t <= 0.0 || t >= a + c + d {
            0.0
        } else if t < a {
            v * t / a
        } else if t < a + c {
            v
        } else {
            v * (1.0 - (t - a - c) / d)
        }
    }
}

/// Décalage vertical (fraction du rayon, -1 = haut, +1 = bas) et profondeur
/// (1 = face, 0 = tranche) de la ligne `k` quand la roue est à `pos`.
/// `None` si la ligne est sur la face cachée du cylindre.
pub fn place(pos: f32, k: i64) -> Option<(f32, f32)> {
    let angle = (pos - k as f32) * ROW_ANGLE;
    if angle.abs() >= std::f32::consts::FRAC_PI_2 {
        return None;
    }
    Some((angle.sin(), angle.cos()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lands_exactly_on_target_at_stop() {
        for stop in [1.0, 1.6, 2.3, 3.0, 4.5] {
            let r = Reel::new(stop);
            assert!((r.duration() - stop).abs() < 1e-4);
            assert!((r.pos(stop - 1e-4) - r.target as f32).abs() < 0.01, "stop={stop}");
            assert_eq!(r.pos(stop + 1.0), r.target as f32);
        }
    }

    #[test]
    fn moves_downward_and_never_backward() {
        let r = Reel::new(2.3);
        let mut prev_y = place(r.pos(0.0), 0).unwrap().0;
        // On suit la ligne 0 tant qu'elle est visible : elle doit descendre.
        for i in 1..200 {
            let t = i as f32 / 240.0;
            let Some((y, _)) = place(r.pos(t), 0) else { break };
            assert!(y >= prev_y, "la ligne remonte à t={t}");
            prev_y = y;
        }
        // La position ne recule jamais.
        let mut prev = 0.0;
        for i in 0..=1000 {
            let p = r.pos(i as f32 * r.duration() / 1000.0);
            assert!(p >= prev - 1e-5);
            prev = p;
        }
    }

    #[test]
    fn no_stroboscopic_reversal() {
        // À 60 i/s comme à 30 i/s, une image ne doit jamais avancer
        // de plus d'une demi-ligne, sinon la roue paraît tourner à l'envers.
        for stop in [1.6, 2.3, 3.0] {
            let r = Reel::new(stop);
            for fps in [60.0, 30.0] {
                let dt = 1.0 / fps;
                let mut t = 0.0;
                while t < stop {
                    let step = r.pos(t + dt) - r.pos(t);
                    assert!(step < 0.5, "stop={stop} fps={fps} t={t} step={step}");
                    t += dt;
                }
            }
        }
    }

    #[test]
    fn speed_matches_position() {
        let r = Reel::new(2.3);
        for i in 1..100 {
            let t = i as f32 * r.duration() / 100.0;
            let h = 1e-3;
            let num = (r.pos(t + h) - r.pos(t - h)) / (2.0 * h);
            assert!((num - r.speed(t)).abs() < 0.2, "t={t}");
        }
    }
}
