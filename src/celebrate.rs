//! Célébration quand la séance du jour est terminée :
//! vibration de l'écran, fond assombri, confettis qui tombent et message de félicitations.

use crate::theme;
use eframe::egui::{self, Align2, Color32, FontId, Id, Order, Pos2, Sense, Shape, Stroke, Vec2};
use rand::Rng;
use std::time::Instant;

const SHAKE_DURATION: f32 = 0.7;
const FADE_IN: f32 = 0.45;
/// Les confettis sont lâchés pendant cette durée (s).
const DROP_WINDOW: f32 = 2.8;
/// On peut fermer la célébration après ce délai (s).
const CLOSABLE_AFTER: f32 = 1.0;

struct Piece {
    x: f32,
    delay: f32,
    speed: f32,
    sway: f32,
    sway_freq: f32,
    phase: f32,
    size: Vec2,
    spin: f32,
    flip: f32,
    color: Color32,
}

pub struct Celebration {
    start: Instant,
    streak: u32,
    pieces: Vec<Piece>,
}

impl Celebration {
    pub fn new(streak: u32) -> Self {
        let mut rng = rand::thread_rng();
        let palette = [
            theme::gray(245),
            theme::gray(190),
            theme::gray(130),
            Color32::from_rgb(60, 190, 110),
            Color32::from_rgb(96, 214, 136),
            Color32::from_rgb(170, 110, 114),
        ];
        let pieces = (0..220)
            .map(|_| Piece {
                x: rng.gen_range(0.0..1.0),
                delay: rng.gen_range(0.0..DROP_WINDOW).powf(1.3) * 0.75,
                speed: rng.gen_range(170.0..380.0),
                sway: rng.gen_range(8.0..34.0),
                sway_freq: rng.gen_range(1.5..4.5),
                phase: rng.gen_range(0.0..std::f32::consts::TAU),
                size: Vec2::new(rng.gen_range(6.0..11.0), rng.gen_range(10.0..17.0)),
                spin: rng.gen_range(-6.0..6.0),
                flip: rng.gen_range(3.0..9.0),
                color: palette[rng.gen_range(0..palette.len())],
            })
            .collect();
        Self { start: Instant::now(), streak, pieces }
    }

    fn t(&self) -> f32 {
        self.start.elapsed().as_secs_f32()
    }

    /// Décalage à appliquer à la page pour la faire vibrer.
    pub fn shake_offset(&self) -> Vec2 {
        let t = self.t();
        if t >= SHAKE_DURATION {
            return Vec2::ZERO;
        }
        let k = (1.0 - t / SHAKE_DURATION).powi(2);
        Vec2::new((t * 83.0).sin() * 14.0 * k, (t * 61.0 + 1.3).sin() * 9.0 * k)
    }

    /// Dessine la célébration par-dessus l'app. Renvoie `false` quand elle doit se fermer.
    pub fn show(&self, ctx: &egui::Context) -> bool {
        let t = self.t();
        let screen = ctx.screen_rect();
        let mut keep = true;

        egui::Area::new(Id::new("celebration"))
            .order(Order::Tooltip)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                // Capte les clics : rien n'est cliquable dessous pendant la fête.
                let resp = ui.allocate_rect(screen, Sense::click());
                let p = ui.painter();

                // Fond assombri
                let a = crate::anim::progress(t, FADE_IN);
                p.rect_filled(screen, 0.0, Color32::from_black_alpha((240.0 * a) as u8));

                // Message
                let m = crate::anim::progress(t - 0.25, 0.5);
                if m > 0.0 {
                    let c = screen.center() + Vec2::new(0.0, -30.0 + 18.0 * (1.0 - m));
                    let alpha = |g: u8| theme::gray(g).gamma_multiply(m);
                    p.text(c + Vec2::new(0.0, -95.0), Align2::CENTER_CENTER, "BRAVO !", theme::title(54.0), alpha(250));
                    p.text(c + Vec2::new(0.0, -48.0), Align2::CENTER_CENTER, "Séance du jour terminée", FontId::proportional(17.0), alpha(150));
                    p.text(
                        c + Vec2::new(0.0, 25.0),
                        Align2::CENTER_CENTER,
                        self.streak.to_string(),
                        theme::numbers(96.0),
                        Color32::from_rgb(96, 214, 136).gamma_multiply(m),
                    );
                    let label = if self.streak > 1 { "jours de suite complétés" } else { "jour complété — la série commence" };
                    p.text(c + Vec2::new(0.0, 92.0), Align2::CENTER_CENTER, label, FontId::proportional(19.0), alpha(220));
                    if t > CLOSABLE_AFTER {
                        let h = crate::anim::progress(t - CLOSABLE_AFTER, 0.5) * m;
                        p.text(
                            Pos2::new(screen.center().x, screen.bottom() - 40.0),
                            Align2::CENTER_CENTER,
                            "Cliquer pour continuer",
                            FontId::proportional(13.0),
                            theme::gray(140).gamma_multiply(h),
                        );
                    }
                }

                // Confettis : tombent du haut vers le bas en se balançant et en tournant.
                for c in &self.pieces {
                    let tp = t - c.delay;
                    if tp < 0.0 {
                        continue;
                    }
                    let y = screen.top() - 20.0 + c.speed * tp;
                    if y > screen.bottom() + 20.0 {
                        continue;
                    }
                    let x = screen.left() + c.x * screen.width() + c.sway * (c.sway_freq * tp + c.phase).sin();
                    let angle = c.spin * tp + c.phase;
                    let half = Vec2::new(c.size.x * 0.5 * (c.flip * tp).cos().abs().max(0.15), c.size.y * 0.5);
                    let (s, co) = angle.sin_cos();
                    let rot = |v: Vec2| Vec2::new(v.x * co - v.y * s, v.x * s + v.y * co);
                    let center = Pos2::new(x, y);
                    let pts = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                        .iter()
                        .map(|(a, b)| center + rot(Vec2::new(half.x * a, half.y * b)))
                        .collect();
                    p.add(Shape::convex_polygon(pts, c.color, Stroke::NONE));
                }

                let esc = ui.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Enter));
                if t > CLOSABLE_AFTER && (resp.clicked() || esc) {
                    keep = false;
                }
            });
        keep
    }
}
