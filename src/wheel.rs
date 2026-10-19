//! Roue de sélection de nombre (façon sélecteur iOS) :
//! molette, glisser-déposer avec inertie, clic sur une valeur, flèches haut/bas.
//! Les valeurs s'estompent vers le haut et le bas (dégradé d'opacité).

use crate::theme;
use crate::anim;
use eframe::egui::{self, Align2, Color32, Mesh, Pos2, Rect, Rounding, Sense, Shape, Stroke, Vec2};

/// Hauteur d'une ligne de la roue.
pub const ROW_H: f32 = 30.0;
/// Durée du recalage sur une valeur (s).
const SNAP: f32 = 0.24;

#[derive(Clone, Copy)]
struct WheelState {
    from: f32,
    target: f32,
    start: f64,
    drag: Option<f32>,
    velocity: f32,
    scroll_acc: f32,
}

impl WheelState {
    fn eased(&self, now: f64) -> f32 {
        self.from + (self.target - self.from) * anim::progress((now - self.start) as f32, SNAP)
    }
    fn current(&self, now: f64) -> f32 {
        self.drag.unwrap_or_else(|| self.eased(now))
    }
    fn go_to(&mut self, idx: f32, now: f64) {
        self.from = self.current(now);
        self.target = idx;
        self.start = now;
    }
}

/// Affiche une roue pour choisir un index dans `0..count`. `fmt` donne le texte de chaque index.
/// Renvoie true si la valeur a changé.
pub fn number_wheel(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    value: &mut usize,
    count: usize,
    size: Vec2,
    enabled: bool,
    colors: (Color32, Color32, Color32), // (fond, bande centrale, texte)
    fmt: impl Fn(usize) -> String,
) -> bool {
    let id = ui.make_persistent_id(id_salt);
    let sense = if enabled { Sense::click_and_drag() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    let now = ui.input(|i| i.time);
    let max = count.saturating_sub(1) as f32;
    let mut st = ui.data(|d| d.get_temp::<WheelState>(id)).unwrap_or(WheelState {
        from: *value as f32,
        target: *value as f32,
        start: now - 10.0,
        drag: None,
        velocity: 0.0,
        scroll_acc: 0.0,
    });
    // Valeur changée de l'extérieur (pré-remplissage, nouvelle étape…)
    if st.drag.is_none() && (st.target - *value as f32).abs() > 0.01 {
        st.go_to(*value as f32, now);
    }

    let before = st.target;
    if enabled {
        // Glisser : la liste suit le pointeur, puis se recale avec un peu d'inertie.
        if resp.drag_started() {
            st.drag = Some(st.current(now));
            st.velocity = 0.0;
        }
        if resp.dragged() {
            let dy = resp.drag_delta().y;
            let dt = ui.input(|i| i.stable_dt).max(1e-3);
            let p = (st.drag.unwrap_or(st.target) - dy / ROW_H).clamp(-0.45, max + 0.45);
            st.drag = Some(p);
            st.velocity = st.velocity * 0.6 + (-dy / ROW_H / dt) * 0.4;
        }
        if resp.drag_stopped() {
            let p = st.drag.take().unwrap_or(st.target);
            let projected = (p + st.velocity * 0.12).round().clamp(0.0, max);
            st.from = p;
            st.target = projected;
            st.start = now;
        }
        if resp.hovered() {
            // Molette : une valeur par cran ; on empêche la page de défiler en même temps.
            let dy = ui.input_mut(|i| {
                let d = i.raw_scroll_delta.y;
                i.raw_scroll_delta = Vec2::ZERO;
                i.smooth_scroll_delta = Vec2::ZERO;
                d
            });
            // Molette classique : 1 cran = 1 valeur (l'amplitude d'un cran varie selon
            // le système : 40, 80, 120 px…). Pavé tactile : petits deltas cumulés.
            if dy.abs() >= 20.0 {
                let notches = (dy.abs() / 80.0).floor().max(1.0);
                st.scroll_acc += -dy.signum() * notches;
            } else {
                st.scroll_acc += -dy / 40.0;
            }
            let steps = st.scroll_acc.trunc();
            if steps != 0.0 {
                st.scroll_acc -= steps;
                let t = (st.target + steps).clamp(0.0, max);
                st.go_to(t, now);
            }
            let (up, down) = ui.input(|i| (i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown)));
            if up || down {
                let t = (st.target + if down { 1.0 } else { -1.0 }).clamp(0.0, max);
                st.go_to(t, now);
            }
        }
        // Clic sur une valeur au-dessus / au-dessous du centre : on y va.
        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let rel = ((pos.y - rect.center().y) / ROW_H).round();
                let t = (st.target + rel).clamp(0.0, max);
                st.go_to(t, now);
            }
        }
    }
    let changed = st.target != before;
    *value = st.target.round().clamp(0.0, max) as usize;

    // ---------- dessin
    let (bg, band, text) = colors;
    let p = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    p.rect_filled(rect, Rounding::same(8.0), bg);
    let band_r = Rect::from_center_size(rect.center(), Vec2::new(rect.width() - 6.0, ROW_H));
    p.rect(band_r, Rounding::same(6.0), band, Stroke::new(1.0_f32, theme::gray(70)));

    let pos = st.current(now);
    let half = rect.height() / 2.0;
    let first = (pos - half / ROW_H - 1.0).floor().max(0.0) as usize;
    let last = ((pos + half / ROW_H + 1.0).ceil() as usize).min(count.saturating_sub(1));
    for i in first..=last {
        let dy = (i as f32 - pos) * ROW_H;
        if dy.abs() > half + ROW_H {
            continue;
        }
        let t = (dy.abs() / half).min(1.0);
        // Dégradé d'opacité : net au centre, s'efface vers les bords.
        let alpha = 1.0 - 0.85 * t.powf(1.4);
        let size = 20.0 - 6.0 * t;
        let col = if enabled { text } else { theme::gray(90) };
        p.text(
            Pos2::new(rect.center().x, rect.center().y + dy),
            Align2::CENTER_CENTER,
            fmt(i),
            crate::theme::numbers(size),
            col.gamma_multiply(alpha),
        );
    }
    // Voile en dégradé en haut et en bas, pour renforcer l'effet de profondeur.
    let fade_h = rect.height() * 0.32;
    let fade = |top: bool| {
        let (y0, y1) = if top { (rect.top(), rect.top() + fade_h) } else { (rect.bottom(), rect.bottom() - fade_h) };
        let opaque = bg;
        let clear = bg.gamma_multiply(0.0);
        let mut m = Mesh::default();
        m.colored_vertex(Pos2::new(rect.left(), y0), opaque);
        m.colored_vertex(Pos2::new(rect.right(), y0), opaque);
        m.colored_vertex(Pos2::new(rect.right(), y1), clear);
        m.colored_vertex(Pos2::new(rect.left(), y1), clear);
        m.add_triangle(0, 1, 2);
        m.add_triangle(0, 2, 3);
        Shape::mesh(m)
    };
    p.add(fade(true));
    p.add(fade(false));
    if resp.hovered() && enabled {
        p.rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0_f32, theme::gray(80)));
    }

    let animating = st.drag.is_some() || (now - st.start) < SNAP as f64;
    ui.data_mut(|d| d.insert_temp(id, st));
    if animating {
        ui.ctx().request_repaint();
    }
    let _ = resp;
    changed
}
