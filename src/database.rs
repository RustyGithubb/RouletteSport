//! Base d'exercices en vue galerie (lecture seule).
//! Une carte par exercice avec trois chiffres mis en avant :
//! reps max, poids max, moyenne de reps par série ; puis les totaux, un petit
//! graphique de la moyenne de reps par séance et une pastille de couleur du jour
//! où l'exercice est tombé cette semaine. Un clic double la taille de la carte.

use super::*;
use crate::model::{parse_number, ExerciseStats};

const CARD_MIN_W: f32 = 210.0;
const CARD_H: f32 = 270.0;
const GAP: f32 = 12.0;

impl RouletteApp {
    pub(super) fn ui_database(&mut self, ui: &mut egui::Ui, today: NaiveDate) {
        ui.horizontal(|ui| {
            section(ui, "BASE D'EXERCICES");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(egui::TextEdit::singleline(&mut self.db_search).hint_text("Rechercher…").desired_width(160.0));
            });
        });
        ui.label(
            RichText::new("Clic : agrandir. Clic droit : renommer / supprimer. Les chiffres viennent des séances validées (lecture seule).")
                .color(theme::dim())
                .size(12.0),
        );
        ui.add_space(6.0);
        // Légende : couleur du jour où l'exercice est tombé cette semaine.
        ui.horizontal(|ui| {
            ui.label(RichText::new("Tombé cette semaine :").color(theme::dim()).size(11.5));
            for (i, d) in ["lun", "mar", "mer", "jeu", "ven", "sam", "dim"].iter().enumerate() {
                let (r, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                ui.painter().circle_filled(r.center(), 5.0, day_color(i));
                ui.label(RichText::new(*d).color(theme::dim()).size(11.5));
            }
            let (r, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
            ui.painter().circle_stroke(r.center(), 4.5, Stroke::new(1.0_f32, theme::line()));
            ui.label(RichText::new("pas encore").color(theme::dim()).size(11.5));
        });
        ui.add_space(12.0);

        let ws = week_start(today);
        let q = self.db_search.trim().to_lowercase();
        let items: Vec<(String, ExerciseStats, Option<NaiveDate>)> = self
            .data
            .pool
            .iter()
            .filter(|n| q.is_empty() || n.to_lowercase().contains(&q))
            .map(|n| {
                let week = self
                    .data
                    .active_days()
                    .find(|r| week_start(r.date) == ws && r.exercises.iter().any(|e| e == n))
                    .map(|r| r.date);
                (n.clone(), self.data.exercise_stats(n), week)
            })
            .collect();

        // --- placement en grille (une carte ouverte occupe 2 x 2 cases)
        let avail = ui.available_width();
        let cols = (((avail + GAP) / (CARD_MIN_W + GAP)).floor() as usize).max(1);
        let cw = (avail - GAP * (cols as f32 - 1.0)) / cols as f32;
        let mut grid: Vec<Vec<bool>> = Vec::new();
        let mut placed = Vec::new();
        for (name, _, _) in &items {
            let open = self.gallery_open.as_deref() == Some(name.as_str());
            let (sw, sh) = if open { (2.min(cols), 2) } else { (1, 1) };
            let (r, c) = find_slot(&mut grid, cols, sw, sh);
            for rr in r..r + sh {
                for cc in c..c + sw {
                    grid[rr][cc] = true;
                }
            }
            placed.push((r, c, sw, sh));
        }
        // Carte "+" pour ajouter un exercice, toujours en dernier.
        let (plus_r, plus_c) = find_slot(&mut grid, cols, 1, 1);
        grid[plus_r][plus_c] = true;
        let rows = grid.len();
        let total_h = rows as f32 * CARD_H + (rows as f32 - 1.0).max(0.0) * GAP;
        let (area, _) = ui.allocate_exact_size(Vec2::new(avail, total_h), egui::Sense::hover());

        let ctx = ui.ctx().clone();
        let mut toggle = None;
        let mut menu = None;
        for ((name, st, week), (r, c, sw, sh)) in items.iter().zip(placed) {
            let target = Rect::from_min_size(
                Pos2::new(area.left() + c as f32 * (cw + GAP), area.top() + r as f32 * (CARD_H + GAP)),
                Vec2::new(sw as f32 * cw + (sw as f32 - 1.0) * GAP, sh as f32 * CARD_H + (sh as f32 - 1.0) * GAP),
            );
            // Transition douce de position / taille
            let anim = |k: &str, v: f32| crate::anim::smooth(&ctx, egui::Id::new(("card", name.as_str(), k)), v, 0.32);
            // Position animée relative à la grille (et non à l'écran) : quand la page défile,
            // les cartes suivent instantanément ; seule la réorganisation est animée.
            let rect = Rect::from_min_size(
                area.min + Vec2::new(anim("x", target.left() - area.left()), anim("y", target.top() - area.top())),
                Vec2::new(anim("w", target.width()), anim("h", target.height())),
            );
            let open = self.gallery_open.as_deref() == Some(name.as_str());
            if open && self.gallery_scroll {
                ui.scroll_to_rect(target, None);
            }
            let resp = ui.interact(rect, egui::Id::new(("card_click", name.as_str())), egui::Sense::click());
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                toggle = Some(name.clone());
            }
            let legacy = (parse_number(self.data.stat(name, "Reps max")), parse_number(self.data.stat(name, "Poids max (kg)")));
            paint_card(ui, rect, name, st, *week, open, resp.hovered(), legacy);
            resp.context_menu(|ui| {
                if ui.button("Renommer").clicked() {
                    menu = Some(Modal::RenameExercise(name.clone(), name.clone()));
                    ui.close_menu();
                }
                if ui.button("Supprimer").clicked() {
                    menu = Some(Modal::DeleteExercise(name.clone()));
                    ui.close_menu();
                }
            });
        }

        // --- carte "+"
        let target = Rect::from_min_size(
            Pos2::new(area.left() + plus_c as f32 * (cw + GAP), area.top() + plus_r as f32 * (CARD_H + GAP)),
            Vec2::new(cw, CARD_H),
        );
        let anim = |k: &str, v: f32| crate::anim::smooth(&ctx, egui::Id::new(("card_plus", k)), v, 0.32);
        let rect = Rect::from_min_size(area.min + Vec2::new(anim("x", target.left() - area.left()), anim("y", target.top() - area.top())), target.size());
        if let Some(name) = self.ui_add_card(ui, rect) {
            if self.data.add_exercise(&name) {
                self.save();
                self.gallery_adding = None;
            } else {
                self.message = Some("Nom vide ou exercice déjà présent.".into());
            }
        }
        if let Some(m) = menu {
            self.open_modal(m);
        }
        self.gallery_scroll = false;
        if let Some(name) = toggle {
            self.gallery_scroll = true;
            self.gallery_open = if self.gallery_open.as_deref() == Some(name.as_str()) { None } else { Some(name) };
        }
    }
}

impl RouletteApp {
    /// Carte "+" : clic pour saisir un nouvel exercice. Renvoie le nom validé.
    fn ui_add_card(&mut self, ui: &mut egui::Ui, rect: Rect) -> Option<String> {
        let id = egui::Id::new("card_add");
        match &mut self.gallery_adding {
            None => {
                let resp = ui.interact(rect, id, egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                let p = ui.painter();
                let col = if resp.hovered() { theme::fg() } else { theme::gray(90) };
                // bordure en pointillés
                let r = rect.shrink(1.0);
                let dash = |a: Pos2, b: Pos2| p.add(egui::Shape::dashed_line(&[a, b], Stroke::new(1.2_f32, col), 6.0, 5.0));
                dash(r.left_top(), r.right_top());
                dash(r.right_top(), r.right_bottom());
                dash(r.right_bottom(), r.left_bottom());
                dash(r.left_bottom(), r.left_top());
                let c = rect.center() - Vec2::new(0.0, 12.0);
                let s = Stroke::new(2.0_f32, col);
                p.line_segment([c - Vec2::new(14.0, 0.0), c + Vec2::new(14.0, 0.0)], s);
                p.line_segment([c - Vec2::new(0.0, 14.0), c + Vec2::new(0.0, 14.0)], s);
                p.text(rect.center() + Vec2::new(0.0, 26.0), Align2::CENTER_CENTER, "Ajouter un exercice", FontId::proportional(13.0), col);
                if resp.clicked() {
                    self.gallery_adding = Some(String::new());
                }
                None
            }
            Some(buf) => {
                ui.painter().rect(rect, Rounding::same(10.0), theme::surface(), Stroke::new(2.0_f32, theme::fg()));
                let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(14.0)).layout(Layout::top_down(Align::Min)));
                inner.label(RichText::new("NOUVEL EXERCICE").size(11.0).color(theme::dim()));
                inner.add_space(8.0);
                let r = inner.add(egui::TextEdit::singleline(buf).hint_text("Nom…").desired_width(f32::INFINITY).font(FontId::proportional(16.0)));
                if !r.has_focus() && !r.lost_focus() && buf.is_empty() {
                    r.request_focus();
                }
                let enter = r.lost_focus() && inner.input(|i| i.key_pressed(egui::Key::Enter));
                let esc = inner.input(|i| i.key_pressed(egui::Key::Escape));
                let name = buf.clone();
                inner.add_space(12.0);
                let mut out = None;
                let mut cancel = esc;
                inner.horizontal(|ui| {
                    if ui.button("Annuler").clicked() {
                        cancel = true;
                    }
                    let add = egui::Button::new(RichText::new("AJOUTER").color(theme::bg())).fill(theme::fg());
                    if ui.add(add).clicked() || enter {
                        out = Some(name.clone());
                    }
                });
                if cancel {
                    self.gallery_adding = None;
                    return None;
                }
                out.filter(|n| !n.trim().is_empty())
            }
        }
    }
}

/// Première position libre (ligne, colonne) pouvant accueillir sw x sh cases.
fn find_slot(grid: &mut Vec<Vec<bool>>, cols: usize, sw: usize, sh: usize) -> (usize, usize) {
    let mut r = 0;
    loop {
        while grid.len() < r + sh {
            grid.push(vec![false; cols]);
        }
        for c in 0..=(cols - sw) {
            if (r..r + sh).all(|rr| (c..c + sw).all(|cc| !grid[rr][cc])) {
                return (r, c);
            }
        }
        r += 1;
    }
}

fn fmt_kg(w: f32) -> String {
    if w.fract() == 0.0 {
        format!("{w:.0}")
    } else {
        format!("{w:.1}").replace('.', ",")
    }
}

pub(super) fn fmt_log(l: &ExerciseLog) -> String {
    let reps: Vec<String> = l.sets.iter().map(|s| s.reps.to_string()).collect();
    let charge = if l.bodyweight {
        "poids du corps".to_string()
    } else {
        let ws: Vec<f32> = l.sets.iter().filter_map(|s| s.weight).collect();
        match (ws.iter().cloned().reduce(f32::min), ws.iter().cloned().reduce(f32::max)) {
            (Some(a), Some(b)) if a == b => format!("{} kg", fmt_kg(a)),
            (Some(a), Some(b)) => format!("{}–{} kg", fmt_kg(a), fmt_kg(b)),
            _ => "—".into(),
        }
    };
    format!("{} séries · {} reps · {}", l.sets.len(), reps.join("/"), charge)
}

#[allow(clippy::too_many_arguments)]
fn paint_card(
    ui: &egui::Ui,
    rect: Rect,
    name: &str,
    st: &ExerciseStats,
    week: Option<NaiveDate>,
    open: bool,
    hovered: bool,
    legacy: (Option<f64>, Option<f64>),
) {
    let p = ui.painter().with_clip_rect(rect.expand(3.0).intersect(ui.clip_rect()));
    let stroke = if open {
        Stroke::new(2.0_f32, theme::fg())
    } else if hovered {
        Stroke::new(1.0_f32, theme::gray(110))
    } else {
        Stroke::new(1.0_f32, theme::line())
    };
    theme::panel(&p, rect, 10.0, theme::surface());
    p.rect_stroke(rect, Rounding::same(10.0), Stroke::new(theme::border(stroke.width), stroke.color));
    let pad = 14.0;
    let inner = rect.shrink(pad);

    // En-tête : nom + pastille semaine
    let name_size = if open { 22.0 } else { 16.0 };
    let name_clip = Rect::from_min_max(inner.left_top() - Vec2::new(2.0, 2.0), Pos2::new(inner.right() - 56.0, inner.top() + name_size + 8.0));
    p.with_clip_rect(name_clip.intersect(p.clip_rect())).text(inner.left_top(), Align2::LEFT_TOP, name, theme::title(name_size), theme::fg());
    // Pastille du jour : couleur du jour où il est tombé cette semaine, sinon cercle vide.
    let pill = Rect::from_min_size(inner.right_top() - Vec2::new(48.0, 0.0), Vec2::new(48.0, 20.0));
    match week {
        Some(d) => {
            let i = d.weekday().num_days_from_monday() as usize;
            let c = day_color(i);
            p.rect_filled(pill, Rounding::same(10.0), c);
            p.text(pill.center(), Align2::CENTER_CENTER, &fr_weekday(d)[..3], FontId::proportional(11.0), ink_on(c));
        }
        None => {
            p.rect_stroke(pill, Rounding::same(10.0), Stroke::new(1.0_f32, theme::line()));
            p.text(pill.center(), Align2::CENTER_CENTER, "—", FontId::proportional(11.0), theme::line());
        }
    }

    // Trois chiffres mis en avant
    let reps_max = st.reps_max.map(|v| v.to_string()).or(legacy.0.map(|v| format!("{v}")));
    let weight_max = if st.bodyweight_only {
        Some("PDC".to_string())
    } else {
        st.weight_max.map(|w| fmt_kg(w)).or(legacy.1.map(|v| fmt_kg(v as f32)))
    };
    let avg = st.avg_reps.map(fmt_avg);
    let tiles = [
        ("REPS MAX", reps_max, ""),
        ("POIDS MAX", weight_max.clone(), if weight_max.as_deref().is_some_and(|w| w != "PDC") { "kg" } else { "" }),
        ("MOY. / SÉRIE", avg, if open { "reps" } else { "" }),
    ];
    let top = inner.top() + if open { 44.0 } else { 34.0 };
    let tile_h = if open { 92.0 } else { 70.0 };
    let tgap = 8.0;
    let tw = (inner.width() - 2.0 * tgap) / 3.0;
    for (k, (label, value, unit)) in tiles.iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(inner.left() + k as f32 * (tw + tgap), top), Vec2::new(tw, tile_h));
        p.rect_filled(r, Rounding::same(7.0), theme::gray(30));
        p.text(r.left_top() + Vec2::new(9.0, 8.0), Align2::LEFT_TOP, *label, FontId::proportional(if open { 11.0 } else { 9.5 }), theme::dim());
        let (v, col) = match value {
            Some(v) => (v.clone(), theme::fg()),
            None => ("—".to_string(), theme::line()),
        };
        let vs = if open { 36.0 } else { 24.0 };
        let g = p.text(r.left_bottom() + Vec2::new(9.0, -8.0), Align2::LEFT_BOTTOM, v, theme::numbers(vs), col);
        if value.is_some() && !unit.is_empty() {
            p.text(Pos2::new(g.right() + 4.0, g.bottom() - 4.0), Align2::LEFT_BOTTOM, *unit, FontId::proportional(11.0), theme::dim());
        }
    }

    // Statistiques complètes sous les chiffres
    let mut y = top + tile_h + 12.0;
    let per_session = (st.sessions > 0).then(|| fmt_avg(st.total_sets as f32 / st.sessions as f32));
    let stats = [
        ("SÉRIES", st.total_sets.to_string()),
        ("SÉANCES", st.sessions.to_string()),
        ("SÉRIES / SÉANCE", per_session.unwrap_or_else(|| "—".into())),
        ("REPS TOTALES", st.total_reps.to_string()),
    ];
    let shown = if open { 4 } else { 3 };
    let sw = inner.width() / shown as f32;
    for (k, (label, value)) in stats.iter().take(shown).enumerate() {
        let x = inner.left() + k as f32 * sw;
        p.text(Pos2::new(x, y), Align2::LEFT_TOP, *label, FontId::proportional(if open { 10.5 } else { 9.0 }), theme::dim());
        p.text(Pos2::new(x, y + 13.0), Align2::LEFT_TOP, value, theme::numbers(if open { 20.0 } else { 15.0 }), theme::fg());
    }
    y += if open { 44.0 } else { 38.0 };

    // Petit graphique : moyenne de reps par série, séance après séance
    let chart_h = if open { 180.0 } else { inner.bottom() - 16.0 - y };
    let chart = Rect::from_min_size(Pos2::new(inner.left(), y), Vec2::new(inner.width(), chart_h.max(30.0)));
    paint_avg_chart(&p, chart, &st.avg_history, st.avg_reps, open);
    y = chart.bottom() + 8.0;

    let foot = match &st.last {
        Some((d, _)) => format!("Dernière séance le {:02}/{:02}", d.day(), d.month()),
        None => "Pas encore de séance enregistrée".to_string(),
    };
    if !open {
        p.text(inner.left_bottom(), Align2::LEFT_BOTTOM, foot, FontId::proportional(11.0), theme::dim());
        return;
    }

    // Vue agrandie : dernières séances
    y += 14.0;
    p.text(Pos2::new(inner.left(), y), Align2::LEFT_TOP, format!("DERNIÈRES SÉANCES  ·  {}", foot.to_lowercase()), FontId::proportional(11.0), theme::dim());
    y += 18.0;
    if st.recent.is_empty() {
        p.text(Pos2::new(inner.left(), y), Align2::LEFT_TOP, "Valide l'exercice avec « fait » pour enregistrer séries, reps et poids.", FontId::proportional(12.0), theme::dim());
    }
    for (d, l) in &st.recent {
        if y > inner.bottom() - 14.0 {
            break;
        }
        p.text(Pos2::new(inner.left(), y), Align2::LEFT_TOP, format!("{:02}/{:02}", d.day(), d.month()), FontId::proportional(12.0), theme::dim());
        p.text(Pos2::new(inner.left() + 52.0, y), Align2::LEFT_TOP, fmt_log(l), FontId::proportional(12.5), theme::fg());
        y += 19.0;
    }
}

/// Moyenne avec une décimale au besoin (« 9 », « 12,5 »).
fn fmt_avg(v: f32) -> String {
    let r = (v * 10.0).round() / 10.0;
    if r.fract() == 0.0 {
        format!("{r:.0}")
    } else {
        format!("{r:.1}").replace('.', ",")
    }
}

/// Couleur de chaque jour de la semaine (lundi = 0).
pub(super) fn day_color(i: usize) -> Color32 {
    const C: [[u8; 3]; 7] = [
        [229, 115, 115], // lundi : rouge
        [255, 167, 77],  // mardi : orange
        [238, 214, 90],  // mercredi : jaune
        [120, 196, 124], // jeudi : vert
        [79, 179, 230],  // vendredi : bleu
        [149, 117, 205], // samedi : violet
        [236, 112, 160], // dimanche : rose
    ];
    let [r, g, b] = C[i % 7];
    Color32::from_rgb(r, g, b)
}

/// Texte noir ou blanc selon la clarté du fond.
fn ink_on(c: Color32) -> Color32 {
    let l = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    if l > 150.0 {
        Color32::from_gray(20)
    } else {
        Color32::WHITE
    }
}

/// Barres : moyenne de reps par série de chaque séance (la plus récente à droite),
/// avec la moyenne générale en pointillés.
fn paint_avg_chart(p: &egui::Painter, rect: Rect, data: &[(NaiveDate, f32)], avg: Option<f32>, open: bool) {
    p.rect_filled(rect, Rounding::same(7.0), theme::gray(30));
    let label_h = if open { 16.0 } else { 0.0 };
    let head = if open { 22.0 } else { 18.0 };
    let plot = Rect::from_min_max(rect.min + Vec2::new(8.0, head + (if open { 14.0 } else { 4.0 })), rect.max - Vec2::new(8.0, 6.0 + label_h));
    let fs = if open { 10.5 } else { 9.0 };
    p.text(rect.left_top() + Vec2::new(8.0, 6.0), Align2::LEFT_TOP, if open { "MOY. REPS / SÉRIE PAR SÉANCE" } else { "REPS / SÉRIE" }, FontId::proportional(fs), theme::dim());
    if let Some(a) = avg {
        p.text(rect.right_top() + Vec2::new(-8.0, 6.0), Align2::RIGHT_TOP, format!("moy. {}", fmt_avg(a)), FontId::proportional(fs), theme::fg().gamma_multiply(0.8));
    }
    if data.is_empty() {
        p.hline(plot.x_range(), plot.bottom(), Stroke::new(1.0_f32, theme::line()));
        p.text(plot.center(), Align2::CENTER_CENTER, "pas encore de données", FontId::proportional(11.0), theme::line());
        return;
    }
    let max = data.iter().map(|(_, v)| *v).fold(avg.unwrap_or(0.0), f32::max).max(1.0) * 1.15;
    let slots = data.len().max(if open { 8 } else { 6 });
    let slot = plot.width() / slots as f32;
    let bw = (slot * 0.62).min(26.0);
    let y_of = |v: f32| plot.bottom() - plot.height() * (v / max);
    p.hline(plot.x_range(), plot.bottom(), Stroke::new(1.0_f32, theme::line()));
    for (k, (d, v)) in data.iter().enumerate() {
        // Les barres sont alignées à droite : la dernière séance est toujours au bout.
        let cx = plot.right() - (data.len() - 1 - k) as f32 * slot - slot / 2.0;
        let bar = Rect::from_min_max(Pos2::new(cx - bw / 2.0, y_of(*v)), Pos2::new(cx + bw / 2.0, plot.bottom()));
        let last = k + 1 == data.len();
        p.rect_filled(bar, Rounding { nw: 3.0, ne: 3.0, sw: 0.0, se: 0.0 }, if last { theme::fg() } else { theme::gray(110) });
        if open {
            p.text(Pos2::new(cx, bar.top() - 2.0), Align2::CENTER_BOTTOM, fmt_avg(*v), FontId::proportional(10.5), theme::dim());
            p.text(Pos2::new(cx, plot.bottom() + 3.0), Align2::CENTER_TOP, format!("{:02}/{:02}", d.day(), d.month()), FontId::proportional(9.5), theme::dim());
        }
    }
    if let Some(a) = avg {
        let y = y_of(a);
        p.add(egui::Shape::dashed_line(&[Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)], Stroke::new(1.0_f32, theme::fg().gamma_multiply(0.55)), 4.0, 3.0));
    }
}
