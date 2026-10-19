//! Base d'exercices en vue galerie (lecture seule).
//! Une carte par exercice avec trois chiffres mis en avant :
//! reps max, poids max, répétitions totales. Un clic double la taille de la carte
//! et affiche le détail des dernières séances.

use super::*;
use crate::model::{parse_number, ExerciseStats};

const CARD_MIN_W: f32 = 210.0;
const CARD_H: f32 = 158.0;
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
    p.text(inner.left_top(), Align2::LEFT_TOP, name, theme::title(name_size), theme::fg());
    let (txt, fill, fg) = match week {
        Some(d) => (format!("tombé {}", &fr_weekday(d)[..3]), theme::gray(48), theme::fg()),
        None => ("à tirer".to_string(), theme::gray(30), theme::dim()),
    };
    let pill = Rect::from_min_size(inner.right_top() - Vec2::new(78.0, 0.0), Vec2::new(78.0, 20.0));
    p.rect_filled(pill, Rounding::same(10.0), fill);
    p.text(pill.center(), Align2::CENTER_CENTER, txt, FontId::proportional(11.0), fg);

    // Trois chiffres mis en avant
    let reps_max = st.reps_max.map(|v| v.to_string()).or(legacy.0.map(|v| format!("{v}")));
    let weight_max = if st.bodyweight_only {
        Some("PDC".to_string())
    } else {
        st.weight_max.map(|w| fmt_kg(w)).or(legacy.1.map(|v| fmt_kg(v as f32)))
    };
    let total = (st.total_reps > 0).then(|| st.total_reps.to_string());
    let tiles = [
        ("REPS MAX", reps_max, ""),
        ("POIDS MAX", weight_max.clone(), if weight_max.as_deref().is_some_and(|w| w != "PDC") { "kg" } else { "" }),
        ("RÉPÉTITIONS", total, if open { "au total" } else { "" }),
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

    // Pied de carte
    let foot = match &st.last {
        Some((d, _)) => format!("{} séance(s) · dernière le {:02}/{:02}", st.sessions, d.day(), d.month()),
        None => "Pas encore de séance enregistrée".to_string(),
    };
    if !open {
        p.text(inner.left_bottom(), Align2::LEFT_BOTTOM, foot, FontId::proportional(11.5), theme::dim());
        return;
    }

    // Vue agrandie : détails
    let mut y = top + tile_h + 18.0;
    let line = |p: &egui::Painter, y: f32, label: &str, value: String| {
        p.text(Pos2::new(inner.left(), y), Align2::LEFT_TOP, label, FontId::proportional(12.0), theme::dim());
        p.text(Pos2::new(inner.left() + 130.0, y), Align2::LEFT_TOP, value, FontId::proportional(13.0), theme::fg());
    };
    line(&p, y, "Séances", st.sessions.to_string());
    y += 20.0;
    line(&p, y, "Séries au total", st.total_sets.to_string());
    y += 20.0;
    line(
        &p,
        y,
        "Dernière fois",
        st.last.as_ref().map_or("—".into(), |(d, _)| fr_date(*d)),
    );
    y += 30.0;
    p.text(Pos2::new(inner.left(), y), Align2::LEFT_TOP, "DERNIÈRES SÉANCES", FontId::proportional(11.0), theme::dim());
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
