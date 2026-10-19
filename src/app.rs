//! Interface egui monochrome.

#[path = "database.rs"]
mod database;

use crate::theme;
use crate::veil;
use crate::anim;
use crate::wheel;
use crate::celebrate::Celebration;
use crate::reel::{self, Reel, ROW_ANGLE};
use crate::model::{fr_date, fr_month, fr_weekday, week_start, Data, DayKind, DayStatus, ExerciseLog, SetLog};
use chrono::{Datelike, Duration, Local, NaiveDate};
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Layout, Margin, Pos2, Rect, RichText, Rounding, Stroke, Vec2,
};
use std::collections::HashMap;
use std::time::Instant;


/// Délai (s) avant que la roue `i` ne s'arrête.
fn reveal_at(i: usize) -> f32 {
    1.6 + 0.7 * i as f32
}
/// Durée de la vibration à la validation d'une case.
const SHAKE_TIME: f32 = 0.45;
/// Côté maximal d'une roulette carrée.
const MAX_SLOT: f32 = 320.0;
/// Largeur de l'aperçu des réglages.
const PREVIEW_W: f32 = 260.0;
/// Taille du grand titre de la page principale (3 × 34).
const HERO_SIZE: f32 = 102.0;
/// Hauteur de l'en-tête flottant (logo, onglets, réglages).
/// Hauteur d'origine de l'en-tête (référence pour l'échelle).
const HEADER_REF: f32 = 74.0;
/// Durée de la transition entre onglets (s).
const TAB_FADE: f32 = 0.32;
/// Hauteur de la zone de questions quand une case est agrandie.
const WIZ_H: f32 = 352.0;

/// Nom de remplissage pour la position `k` de la roue `slot` (jamais le nom final).
fn filler(pool: &[String], k: i64, slot: usize, seed: usize, final_name: &str) -> String {
    if pool.is_empty() {
        return String::new();
    }
    let len = pool.len() as i64;
    let mut idx = (k * 7 + (slot * 3 + seed) as i64).rem_euclid(len) as usize;
    if pool[idx] == final_name && pool.len() > 1 {
        idx = (idx + 1) % pool.len();
    }
    pool[idx].clone()
}

/// Fenêtres de validation.
enum Modal {
    ResetCalendar,
    DeleteExercise(String),
    /// (nom actuel, saisie)
    RenameExercise(String, String),
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum LogStep {
    Sets,
    Reps,
    Weight,
}

enum WizardResult {
    Cancel,
    Done(ExerciseLog),
}

/// Saisie guidée d'un exercice validé. Les valeurs sont des index de roue.
struct LogWizard {
    slot: usize,
    step: LogStep,
    /// séries = index + 1
    sets: usize,
    /// reps = index + 1
    reps: Vec<usize>,
    /// poids = index × 0,5 kg
    weights: Vec<usize>,
    bodyweight: bool,
}

const MAX_SETS: usize = 12;
const MAX_REPS: usize = 100;
const WEIGHT_STEP: f32 = 0.5;
const WEIGHT_COUNT: usize = 601; // 0 à 300 kg
const DEFAULT_REPS: usize = 9; // 10 reps
const DEFAULT_WEIGHT: usize = 40; // 20 kg

fn fmt_weight(idx: usize) -> String {
    let w = idx as f32 * WEIGHT_STEP;
    if w.fract() == 0.0 { format!("{w:.0}") } else { format!("{w:.1}").replace('.', ",") }
}

impl LogWizard {
    /// Pré-remplit avec la dernière séance de cet exercice, s'il y en a une.
    fn new(slot: usize, last: Option<&ExerciseLog>) -> Self {
        let w_idx = |w: Option<f32>| w.map_or(DEFAULT_WEIGHT, |w| ((w / WEIGHT_STEP).round() as usize).min(WEIGHT_COUNT - 1));
        Self {
            slot,
            step: LogStep::Sets,
            sets: last.map_or(2, |l| l.sets.len().clamp(1, MAX_SETS) - 1),
            reps: last.map_or(vec![], |l| l.sets.iter().map(|s| (s.reps as usize).clamp(1, MAX_REPS) - 1).collect()),
            weights: last.map_or(vec![], |l| l.sets.iter().map(|s| w_idx(s.weight)).collect()),
            bodyweight: last.is_some_and(|l| l.bodyweight),
        }
    }

    fn back(&mut self) -> Option<WizardResult> {
        match self.step {
            LogStep::Sets => return Some(WizardResult::Cancel),
            LogStep::Reps => self.step = LogStep::Sets,
            LogStep::Weight => self.step = LogStep::Reps,
        }
        None
    }

    fn next(&mut self) -> Option<WizardResult> {
        match self.step {
            LogStep::Sets => {
                // Si le nombre de séries change, on complète avec la dernière valeur.
                let n = self.sets + 1;
                let last_r = self.reps.last().copied().unwrap_or(DEFAULT_REPS);
                let last_w = self.weights.last().copied().unwrap_or(DEFAULT_WEIGHT);
                self.reps.resize(n, last_r);
                self.weights.resize(n, last_w);
                self.step = LogStep::Reps;
            }
            LogStep::Reps => self.step = LogStep::Weight,
            LogStep::Weight => {
                let sets = self
                    .reps
                    .iter()
                    .zip(&self.weights)
                    .map(|(r, w)| SetLog {
                        reps: *r as u32 + 1,
                        weight: if self.bodyweight { None } else { Some(*w as f32 * WEIGHT_STEP) },
                    })
                    .collect();
                return Some(WizardResult::Done(ExerciseLog { sets, bodyweight: self.bodyweight }));
            }
        }
        None
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Today,
    Week,
    Settings,
}

pub struct RouletteApp {
    data: Data,
    tab: Tab,
    spin: Option<Instant>,
    /// Animation du bouton "fait" par case : (début, nouvel état).
    done_anim: HashMap<usize, (Instant, bool)>,
    celebration: Option<Celebration>,
    modal: Option<Modal>,
    /// Voile de fin de séance : instant d'apparition, place du bouton.
    veil_since: Option<Instant>,
    done_bar: Option<Rect>,
    modal_opened: Instant,
    /// Saisie du nouvel exercice dans la carte "+" de la galerie.
    gallery_adding: Option<String>,
    /// Grand titre de la page principale : phrase affichée et instant du changement.
    hero: Option<(String, Instant)>,
    /// Position x et largeur des roulettes (pour aligner le bouton du bas).
    reels_span: Option<(f32, f32)>,
    /// Aperçu des réglages affiché à droite de l'onglet Réglages.
    preview_on: bool,
    /// Hauteur du grand titre (image précédente) pour le coller aux roulettes.
    hero_h: f32,
    /// Instant du dernier changement d'onglet (transition).
    tab_changed: Option<Instant>,
    /// Questions en cours après "fait" (séries / reps / poids).
    wizard: Option<LogWizard>,
    /// Mois affiché dans le calendrier (None = mois courant).
    cal_month: Option<NaiveDate>,
    /// Jour sélectionné dans le calendrier (détail affiché dessous).
    cal_selected: Option<NaiveDate>,
    /// Polices disponibles (réglages de développement).
    font_list: Option<Vec<String>>,
    /// Base d'exercices : tri, recherche, saisie en cours.
    db_search: String,
    /// Carte ouverte (agrandie) dans la galerie.
    gallery_open: Option<String>,
    gallery_scroll: bool,
    day_offset: i64,
    message: Option<String>,
    /// Textures du filtre VHS.
    vhs_tex: crate::vhs::VhsTextures,
    /// Ressources OpenGL du filtre bitmap.
    bitmap_gl: crate::bitmap::BitmapGl,
}

impl RouletteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let data = Data::load();
        apply_theme(&cc.egui_ctx, &data);
        Self {
            data,
            tab: Tab::Today,
            spin: None,
            done_anim: HashMap::new(),
            celebration: None,
            wizard: None,
            tab_changed: None,
            hero: None,
            reels_span: None,
            preview_on: true,
            hero_h: 0.0,
            modal: None,
            veil_since: None,
            done_bar: None,
            modal_opened: Instant::now(),
            gallery_adding: None,
            cal_month: None,
            cal_selected: None,
            font_list: None,
            db_search: String::new(),
            gallery_open: None,
            gallery_scroll: false,
            day_offset: 0,
            message: None,
            vhs_tex: crate::vhs::VhsTextures::new(&cc.egui_ctx),
            bitmap_gl: Default::default(),
        }
    }

    fn today(&self) -> NaiveDate {
        Local::now().date_naive() + Duration::days(self.day_offset)
    }

    fn save(&mut self) {
        self.message = self.data.save().err().map(|e| format!("Erreur de sauvegarde : {e}"));
    }
}

impl eframe::App for RouletteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.done_bar = None;
        let today = self.today();
        // Fond de l'app (uni ou dégradé), sous tout le reste.
        theme::paint_background(&ctx.layer_painter(egui::LayerId::background()), ctx.screen_rect());

        if self.tab == Tab::Today && self.data.day_kind(today) == DayKind::Training {
            egui::TopBottomPanel::bottom("actions")
                .frame(Frame::none().fill(Color32::TRANSPARENT).inner_margin(Margin { left: 24.0, right: 24.0, top: 12.0, bottom: 20.0 }))
                .show_separator_line(false)
                .show(ctx, |ui| self.ui_action_bar(ui, today));
        }

        let shape_count = |ctx: &egui::Context| ctx.graphics(|g| g.get(egui::LayerId::background()).map_or(0, |l| l.next_idx().0));
        let bg_start = shape_count(ctx);
        let central = egui::CentralPanel::default()
            .frame(Frame::none().fill(Color32::TRANSPARENT).inner_margin(Margin { left: 24.0, right: 24.0, top: 0.0, bottom: 16.0 }))
            .show(ctx, |ui| {
                // Transition d'onglet : fondu + léger glissement, en ease-in-out.
                let since = self.tab_changed.map_or(f32::MAX, |t| t.elapsed().as_secs_f32());
                let k = anim::progress(since, TAB_FADE);
                if k < 1.0 {
                    ui.ctx().request_repaint();
                }
                ui.set_opacity(k);
                egui::ScrollArea::vertical().id_salt(self.tab as u8).auto_shrink([false; 2]).show(ui, |ui| {
                    // La page occupe toute la fenêtre et défile sous l'en-tête :
                    // on la commence juste sous l'en-tête pour que rien ne se chevauche.
                    ui.add_space(theme::header_h() + 10.0 + 14.0 * (1.0 - k));
                    if let Some(msg) = &self.message {
                        ui.label(RichText::new(msg).color(theme::fg()).strong());
                        ui.add_space(8.0);
                    }
                    match self.tab {
                    Tab::Today => self.ui_today(ui, today),
                    Tab::Week => self.ui_week(ui, today),
                    Tab::Settings => self.ui_settings(ui, today),
                    }
                });
            });

        let bg_end = shape_count(ctx);
        self.ui_veil(ctx, today, central.response.rect, bg_start, bg_end);
        self.ui_header(ctx);
        if self.tab == Tab::Settings {
            self.ui_preview(ctx);
        }

        // Célébration : l'écran vibre, puis overlay confettis + message.
        let shake = self.celebration.as_ref().map_or(Vec2::ZERO, |c| c.shake_offset());
        ctx.set_transform_layer(egui::LayerId::background(), egui::emath::TSTransform::from_translation(shake));
        self.ui_modal(ctx, today);
        if let Some(c) = &self.celebration {
            if !c.show(ctx) {
                self.celebration = None;
            }
        }

        // Filtre VHS par-dessus tout (ne capte pas les clics).
        crate::vhs::paint(ctx, &self.data.settings.vhs, &self.vhs_tex);
        // Filtre bitmap : retraite l'image finale (VHS compris).
        crate::bitmap::paint(ctx, crate::vhs::layer(), &self.data.settings.bitmap, &self.bitmap_gl, theme::fg(), theme::bg());

        if self.spin.is_some() || self.celebration.is_some() || self.data.settings.vhs.animated() {
            ctx.request_repaint();
        } else {
            // Rafraîchit de temps en temps pour passer minuit proprement.
            ctx.request_repaint_after(std::time::Duration::from_secs(30));
        }
    }
}

impl RouletteApp {
    fn ui_today(&mut self, ui: &mut egui::Ui, today: NaiveDate) {
        let kind = self.data.day_kind(today);
        let (title, sub) = match kind {
            DayKind::Training => (progress_phrase(&self.data, today), "Tire tes exercices du jour."),
            DayKind::Rest => ("Repos".to_string(), "Jour de récupération. La roulette reprend demain."),
            DayKind::Weekend => ("Weekend".to_string(), "Pas de séance. La liste repart à zéro lundi."),
        };
        let mut date = fr_date(today);
        if self.day_offset != 0 {
            date.push_str(" (date simulée)");
        }
        let training = kind == DayKind::Training;
        // Les roulettes (au repos) sont centrées dans la fenêtre, horizontalement et
        // verticalement. Le titre prend la place disponible au-dessus.
        let n = self.data.settings.draws_per_day.max(self.data.record(today).map_or(0, |r| r.exercises.len())).max(1);
        let side = ((ui.available_width() - 10.0 * (n as f32 - 1.0)) / n as f32).min(MAX_SLOT);
        let ts = theme::todo_style();
        let block_h = side + ts.gap + side * ts.height;
        let content_top = ui.min_rect().top();
        let screen_c = ui.ctx().screen_rect().center().y;
        let desired_rel = screen_c - block_h / 2.0 - ui.clip_rect().top();
        let start_rel = ui.cursor().top() - content_top;
        // Le titre est collé juste au-dessus des roulettes (hauteur mesurée à l'image précédente).
        const HERO_GAP: f32 = 22.0;
        let room = desired_rel - start_rel - HERO_GAP;
        let hero_max = if training { room.max(40.0) } else { f32::INFINITY };
        if training {
            ui.add_space((room - self.hero_h).max(0.0));
        }
        let before = ui.cursor().top();
        self.ui_hero(ui, &title, &format!("{date}  ·  {sub}"), hero_max);
        let h = ui.cursor().top() - before;
        if (h - self.hero_h).abs() > 0.5 {
            self.hero_h = h;
            ui.ctx().request_repaint();
        }
        let rel = ui.cursor().top() - content_top;
        ui.add_space(if training { (desired_rel - rel).max(HERO_GAP) } else { 24.0 });

        if training {
            self.ui_slots(ui, today);
        }
    }

    /// Grand titre centré (3× l'ancienne taille, réduit seulement s'il dépasse 2 lignes),
    /// avec un fondu quand la phrase change.
    fn ui_hero(&mut self, ui: &mut egui::Ui, title: &str, sub: &str, max_h: f32) {
        if self.hero.as_ref().map_or(true, |(t, _)| t != title) {
            self.hero = Some((title.to_string(), Instant::now()));
        }
        let since = self.hero.as_ref().map_or(10.0, |(_, t)| t.elapsed().as_secs_f32());
        let k = anim::progress(since, 0.45);
        if k < 1.0 {
            ui.ctx().request_repaint();
        }
        let width = ui.available_width();
        let mut size = HERO_SIZE;
        let galley = loop {
            let mut job = egui::text::LayoutJob::simple(title.to_string(), theme::title(size), theme::fg(), width);
            job.halign = Align::Center;
            let g = ui.painter().layout_job(job);
            if (g.rows.len() <= 2 && g.size().y <= max_h) || size <= 30.0 {
                break g;
            }
            size *= 0.9;
        };
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, galley.size().y), egui::Sense::hover());
        let pos = Pos2::new(rect.center().x, rect.top() + 8.0 * (1.0 - k));
        ui.painter().galley(pos, galley, theme::fg().gamma_multiply(k));
        ui.add_space(4.0);
        ui.vertical_centered(|ui| ui.label(RichText::new(sub).color(theme::dim())));
    }

    /// Barre d'action fixée en bas de la page, de la largeur des trois roues.
    fn ui_action_bar(&mut self, ui: &mut egui::Ui, today: NaiveDate) {
        let spinning = self.spin.is_some();
        let record = self.data.record(today).cloned();
        // Même largeur que les trois roulettes, centré comme elles.
        let full = ui.available_width();
        let (x0, w) = match self.reels_span {
            Some((x, rw)) if rw <= full + 1.0 => ((x - ui.cursor().left()).clamp(0.0, full - rw), rw),
            _ => (0.0, full),
        };
        let (slot, _) = ui.allocate_exact_size(Vec2::new(full, 56.0), egui::Sense::hover());
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(slot.min + Vec2::new(x0, 0.0), Vec2::new(w, 56.0))));
        let ui = &mut child;
        if self.data.is_complete(today) {
            // Le bouton est dessiné au-dessus du voile (cf. update) : ici on réserve sa place.
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 56.0), egui::Sense::hover());
            self.done_bar = Some(rect);
            return;
        }
        match &record {
            None => {
                if wide_button(ui, w, "LANCER LA ROULETTE", !spinning && !self.data.pool.is_empty()) {
                    match self.data.draw(today) {
                        Ok(()) => {
                            self.save();
                            self.done_anim.clear();
                            self.wizard = None;
                            self.spin = Some(Instant::now());
                        }
                        Err(e) => self.message = Some(e.into()),
                    }
                }
            }
            Some(r) => {
                let left = self.data.settings.max_rerolls.saturating_sub(r.rerolls_used);
                let label = match left {
                    0 => "TIRAGE DÉFINITIF".to_string(),
                    1 => "RELANCER  ·  1 restante".to_string(),
                    n => format!("RELANCER  ·  {n} restantes"),
                };
                if wide_button(ui, w, &label, left > 0 && !spinning) {
                    match self.data.reroll(today) {
                        Ok(()) => {
                            self.save();
                            self.done_anim.clear();
                            self.wizard = None;
                            self.spin = Some(Instant::now());
                        }
                        Err(e) => self.message = Some(e.into()),
                    }
                }
            }
        }
    }

    fn ui_slots(&mut self, ui: &mut egui::Ui, today: NaiveDate) {
        let record = self.data.record(today).cloned();
        let names = record.as_ref().map(|r| r.exercises.clone()).unwrap_or_default();
        let done = record.as_ref().map(|r| r.done.clone()).unwrap_or_default();
        let n = self.data.settings.draws_per_day.max(names.len()).max(1);

        let elapsed = self.spin.map(|s| s.elapsed().as_secs_f32());
        if elapsed.is_some_and(|t| t > reveal_at(n - 1) + SHAKE_TIME) {
            self.spin = None;
        }
        let spinning = self.spin.is_some();
        let seed = today.ordinal() as usize + record.as_ref().map_or(0, |r| r.rerolls_used as usize * 13);

        let anims = self.done_anim.clone();
        // Journée terminée : plus possible de décocher.
        let locked = self.data.is_complete(today);
        if locked || spinning {
            self.wizard = None;
        }
        if anims.values().any(|(t0, _)| t0.elapsed().as_secs_f32() < SHAKE_TIME) {
            ui.ctx().request_repaint();
        }

        // Largeur / hauteur animées : la case en cours de saisie double de largeur
        // et s'allonge pour accueillir les questions.
        let ctx = ui.ctx().clone();
        let wizard_slot = self.wizard.as_ref().map(|w| w.slot);
        let exp: Vec<f32> = (0..n)
            .map(|i| anim::smooth(&ctx, egui::Id::new(("slot_expand", i)), if wizard_slot == Some(i) { 1.0 } else { 0.0 }, 0.35))
            .collect();
        // Roulettes carrées (largeur = hauteur), centrées dans la page.
        let gap = 10.0;
        let total_w = ui.available_width();
        let gaps = gap * (n as f32 - 1.0);
        let side = ((total_w - gaps) / n as f32).min(MAX_SLOT);
        let mut widths: Vec<f32> = exp.iter().map(|e| side * (1.0 + e)).collect();
        let sum: f32 = widths.iter().sum();
        if sum + gaps > total_w {
            let scale = (total_w - gaps) / sum;
            widths.iter_mut().for_each(|w| *w *= scale);
        }
        let row_w = widths.iter().sum::<f32>() + gaps;
        // Bouton "fait" = 1/3 de la hauteur de la roulette.
        let ts = theme::todo_style();
        let done_h = side * ts.height.clamp(0.1, 0.8);
        let bottom: Vec<f32> = exp.iter().map(|e| done_h + e * (WIZ_H - done_h)).collect();
        let row_h = side + ts.gap + bottom.iter().cloned().fold(0.0, f32::max);
        let (row, _) = ui.allocate_exact_size(Vec2::new(total_w, row_h), egui::Sense::hover());

        let mut clicks = Vec::new();
        let mut wizard_out = None;
        let mut x = row.left() + (total_w - row_w) / 2.0;
        // Largeur de référence (roulettes au repos) pour le bouton du bas.
        let rest_w = side * n as f32 + gaps;
        self.reels_span = Some((row.left() + (total_w - rest_w) / 2.0, rest_w));
        for i in 0..n {
            let w = widths[i];
            let slot_rect = Rect::from_min_size(Pos2::new(x, row.top()), Vec2::new(w, row_h));
            x += w + gap;
            let mut col = ui.new_child(egui::UiBuilder::new().max_rect(slot_rect).layout(Layout::top_down(Align::Min)));
            let col = &mut col;

            let t = elapsed.unwrap_or(f32::MAX);
            let stop = reveal_at(i);
            let since_stop = t - stop;
            let landed = names.get(i).is_some() && since_stop >= 0.0;
            // 1 au moment de l'impact -> 0 à la fin de la vibration
            let impact = if landed && since_stop < SHAKE_TIME { 1.0 - since_stop / SHAKE_TIME } else { 0.0 };

            let dx = if impact > 0.0 { (since_stop * 75.0).sin() * 7.0 * impact * impact } else { 0.0 };
            let rs = theme::reel_style();
            let (stroke_w, border) = if landed { (rs.landed_border * (1.0 + impact), theme::fg()) } else { (rs.border, theme::line()) };

            let (rect, _) = col.allocate_exact_size(Vec2::new(w, side), egui::Sense::hover());
            let r = rect.translate(Vec2::new(dx, 0.0));
            let painter = col.painter().clone();
            theme::reel_base(&painter, r);
            painter.text(r.center_top() + Vec2::new(0.0, 18.0), Align2::CENTER_CENTER, format!("{:02}", i + 1), theme::numbers(12.0), theme::dim());

            let reel = Rect::from_min_max(r.min + Vec2::new(6.0, 38.0), r.max - Vec2::new(6.0, 8.0));
            let p = painter.with_clip_rect(reel.intersect(col.clip_rect()));
            let cy = reel.center().y;
            let radius = reel.height() * 0.5;

            let Some(final_name) = names.get(i) else {
                p.text(reel.center(), Align2::CENTER_CENTER, "?", theme::reel(26.0), theme::line());
                theme::reel_overlay(&painter, r, stroke_w, border);
                col.add_space(ts.gap);
                done_button(col, done_h, false, false, false, None);
                continue;
            };

            // Roue : les noms descendent du haut vers le bas (cf. reel.rs).
            let wheel = Reel::new(stop);
            let target = wheel.target;
            let mut pos = if since_stop >= 0.0 { target as f32 } else { wheel.pos(t) };
            if impact > 0.0 {
                pos += (since_stop * 40.0).sin() * 0.10 * impact;
            }
            let row_h = radius * ROW_ANGLE.sin();
            for dy in [-row_h * 0.5, row_h * 0.5] {
                p.hline(reel.x_range().shrink(10.0), cy + dy, Stroke::new(1.0_f32, theme::gray(34).gamma_multiply(rs.guides.clamp(0.0, 1.0))));
            }
            let base = pos.round() as i64;
            for k in (base - 3)..=(base + 3) {
                let name = if k == target { final_name.clone() } else { filler(&self.data.pool, k, i, seed, final_name) };
                let Some((y, depth)) = reel::place(pos, k) else { continue };
                let color = if landed && k == target { theme::fg() } else { theme::gray((24.0 + 150.0 * depth.powi(3)) as u8) };
                p.text(
                    Pos2::new(reel.center().x, cy + radius * y),
                    Align2::CENTER_CENTER,
                    name,
                    theme::reel(24.0 * (0.5 + 0.5 * depth)),
                    color,
                );
            }
            theme::reel_overlay(&painter, r, stroke_w, border);

            col.add_space(ts.gap);
            if wizard_slot == Some(i) {
                if let Some(res) = self.ui_wizard(col, bottom[i], final_name) {
                    wizard_out = Some(res);
                }
            } else {
                let active = !spinning && i < names.len();
                let is_done = done.get(i).copied().unwrap_or(false);
                if done_button(col, done_h, is_done, active, locked, anims.get(&i).copied()) {
                    clicks.push((i, is_done));
                }
            }
        }

        // Clic sur "À FAIRE" : ouvre les questions. Clic sur "FAIT" : décoche.
        for (i, is_done) in clicks {
            if is_done {
                self.done_anim.insert(i, (Instant::now(), false));
                self.data.set_done(today, i, false);
                self.save();
            } else if let Some(name) = names.get(i) {
                self.wizard = Some(LogWizard::new(i, self.data.last_log(name)));
            }
        }
        match wizard_out {
            Some(WizardResult::Cancel) => self.wizard = None,
            Some(WizardResult::Done(log)) => {
                let slot = self.wizard.take().map_or(0, |w| w.slot);
                let was_complete = self.data.is_complete(today);
                self.data.log_exercise(today, slot, log);
                self.done_anim.insert(slot, (Instant::now(), true));
                self.save();
                if !was_complete && self.data.is_complete(today) {
                    self.celebration = Some(Celebration::new(self.data.streak(today)));
                }
            }
            None => {}
        }
    }

    /// Questions après "fait" : séries -> reps par série -> poids par série.
    /// Texte centré, roues de nombres à faire défiler, boutons empilés en bas.
    fn ui_wizard(&mut self, ui: &mut egui::Ui, height: f32, exercise: &str) -> Option<WizardResult> {
        let wz = self.wizard.as_mut()?;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::hover());
        // Garde la zone de questions visible au-dessus du bouton du bas.
        ui.scroll_to_rect(rect.expand(8.0), None);
        theme::panel(ui.painter(), rect, 8.0, theme::surface());
        ui.painter().rect_stroke(rect, Rounding::same(8.0), Stroke::new(theme::border(1.5_f32), theme::fg()));
        let clip = rect.shrink(2.0).intersect(ui.clip_rect());
        let inner = rect.shrink(14.0);
        let p = ui.painter().with_clip_rect(clip);

        let (n_step, title, unit) = match wz.step {
            LogStep::Sets => (1, "Combien de séries effectuées ?", "séries"),
            LogStep::Reps => (2, "Répétitions par série", "reps"),
            LogStep::Weight => (3, "Poids par série", "kg"),
        };
        let cx = inner.center().x;
        p.text(Pos2::new(cx, inner.top() + 6.0), Align2::CENTER_CENTER, format!("ÉTAPE {n_step}/3  ·  {exercise}"), FontId::proportional(11.0), theme::dim());
        p.text(Pos2::new(cx, inner.top() + 30.0), Align2::CENTER_CENTER, title, theme::title(17.0), theme::fg());

        // Boutons empilés : SUIVANT / VALIDER au-dessus, RETOUR dessous.
        let back_r = Rect::from_min_max(Pos2::new(inner.left(), inner.bottom() - 34.0), inner.right_bottom());
        let ok_r = Rect::from_min_max(Pos2::new(inner.left(), back_r.top() - 48.0), Pos2::new(inner.right(), back_r.top() - 8.0));

        // Zone des roues (+ option poids du corps à l'étape 3)
        let mut top = inner.top() + 50.0;
        if wz.step == LogStep::Weight {
            let toggle_r = Rect::from_center_size(Pos2::new(cx, top + 13.0), Vec2::new(170.0, 26.0));
            let t = ui.interact(toggle_r, ui.id().with("wz_bw"), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            if t.clicked() {
                wz.bodyweight = !wz.bodyweight;
            }
            let on = wz.bodyweight;
            p.rect(toggle_r, Rounding::same(13.0), if on { theme::fg() } else { theme::gray(30) }, Stroke::new(1.0_f32, if on { theme::fg() } else { theme::line() }));
            p.text(toggle_r.center(), Align2::CENTER_CENTER, "Poids du corps", FontId::proportional(13.0), if on { theme::bg() } else { theme::fg() });
            top += 34.0;
        }
        let caption_h = 18.0;
        let wheels_rect = Rect::from_min_max(Pos2::new(inner.left(), top), Pos2::new(inner.right(), ok_r.top() - 10.0 - caption_h));
        p.text(Pos2::new(cx, wheels_rect.bottom() + caption_h / 2.0 + 2.0), Align2::CENTER_CENTER, unit, FontId::proportional(12.0), theme::dim());

        let colors = (theme::gray(26), theme::gray(40), theme::fg());
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(wheels_rect).layout(Layout::left_to_right(Align::Min)));
        child.set_clip_rect(wheels_rect.intersect(clip));
        match wz.step {
            LogStep::Sets => {
                let w = 120.0;
                child.add_space(((wheels_rect.width() - w) / 2.0).max(0.0));
                let wheel_size = Vec2::new(w, wheels_rect.height());
                wheel::number_wheel(&mut child, ("wz_sets", wz.slot), &mut wz.sets, MAX_SETS, wheel_size, true, colors, |i| (i + 1).to_string());
            }
            LogStep::Reps | LogStep::Weight => {
                let is_w = wz.step == LogStep::Weight;
                let n = wz.reps.len();
                let gap = 8.0;
                let label_h = 16.0;
                let ww = ((wheels_rect.width() - gap * (n as f32 - 1.0)) / n as f32).clamp(56.0, 96.0);
                let total = ww * n as f32 + gap * (n as f32 - 1.0);
                let wheel_size = Vec2::new(ww, wheels_rect.height() - label_h - (if total > wheels_rect.width() { 12.0 } else { 0.0 }));
                let enabled = !(is_w && wz.bodyweight);
                let mut draw = |ui: &mut egui::Ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for k in 0..n {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            let (lr, _) = ui.allocate_exact_size(Vec2::new(ww, label_h), egui::Sense::hover());
                            ui.painter().text(lr.center(), Align2::CENTER_CENTER, format!("S{}", k + 1), FontId::proportional(11.0), theme::dim());
                            if is_w {
                                wheel::number_wheel(ui, ("wz_w", wz.slot, k), &mut wz.weights[k], WEIGHT_COUNT, wheel_size, enabled, colors, fmt_weight);
                            } else {
                                wheel::number_wheel(ui, ("wz_r", wz.slot, k), &mut wz.reps[k], MAX_REPS, wheel_size, true, colors, |i| (i + 1).to_string());
                            }
                        });
                    }
                };
                if total <= wheels_rect.width() {
                    child.add_space((wheels_rect.width() - total) / 2.0);
                    draw(&mut child);
                } else {
                    // Trop de séries pour la largeur : défilement horizontal.
                    egui::ScrollArea::horizontal().id_salt(("wz_scroll", wz.slot, is_w)).show(&mut child, |ui| {
                        ui.horizontal(|ui| draw(ui));
                    });
                }
            }
        }

        let back = ui.interact(back_r, ui.id().with("wz_back"), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let ok = ui.interact(ok_r, ui.id().with("wz_ok"), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        p.rect(ok_r, Rounding::same(7.0), if ok.hovered() { Color32::WHITE } else { theme::fg() }, Stroke::NONE);
        let ok_label = if wz.step == LogStep::Weight { "VALIDER" } else { "SUIVANT" };
        p.text(ok_r.center(), Align2::CENTER_CENTER, ok_label, FontId::proportional(15.0), theme::bg());
        p.rect(back_r, Rounding::same(7.0), if back.hovered() { theme::gray(44) } else { theme::gray(30) }, Stroke::new(1.0_f32, theme::line()));
        p.text(back_r.center(), Align2::CENTER_CENTER, "RETOUR", FontId::proportional(13.0), theme::fg());

        let (enter, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        if back.clicked() || esc {
            return wz.back();
        }
        if ok.clicked() || enter {
            return wz.next();
        }
        None
    }

    fn ui_week_progress(&self, ui: &mut egui::Ui, today: NaiveDate) {
        let total = self.data.pool.len();
        let remaining = self.data.remaining(today).len();
        let drawn = total - remaining;

        section(ui, &format!("SEMAINE DU {}", fr_date(week_start(today)).to_uppercase()));
        ui.label(RichText::new(format!("{drawn} / {total} exercices tombés")).color(theme::fg()));
        ui.add_space(6.0);
        progress_bar(ui, if total == 0 { 0.0 } else { drawn as f32 / total as f32 });
        ui.add_space(14.0);

        // Frise lun -> ven (grille : centrée comme un seul bloc)
        center_block(ui, "frise", |ui| {
            egui::Grid::new("frise").spacing([0.0, 0.0]).show(ui, |ui| {
                let ws = week_start(today);
                for i in 0..5 {
                    let d = ws + Duration::days(i);
                    let mark = match (self.data.day_kind(d), self.data.record(d)) {
                        _ if self.data.is_complete(d) => Mark::Complete,
                        (_, Some(_)) => Mark::Drawn,
                        (DayKind::Rest, _) => Mark::Rest,
                        _ if d < today => Mark::Missed,
                        _ => Mark::Todo,
                    };
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(56.0, 44.0), egui::Sense::hover());
                    let col = if matches!(mark, Mark::Drawn | Mark::Complete) { theme::fg() } else { theme::dim() };
                    let font = if d == today { FontId::proportional(13.5) } else { FontId::proportional(13.0) };
                    let p = ui.painter();
                    let top = rect.center_top() + Vec2::new(0.0, 10.0);
                    let g = p.text(top, Align2::CENTER_CENTER, fr_weekday(d)[..3].to_uppercase(), font, if d == today { theme::fg() } else { col });
                    if d == today {
                        p.hline(g.x_range(), g.bottom() + 1.0, Stroke::new(1.0_f32, theme::fg()));
                    }
                    paint_mark(p, rect.center_top() + Vec2::new(0.0, 32.0), mark);
                }
            });
        });

        if !crate::model::is_weekend(today) {
            let capacity = self.data.open_training_days(today) * self.data.settings.draws_per_day;
            let pending = remaining;
            ui.add_space(10.0);
            if pending > capacity {
                ui.label(
                    RichText::new(format!(
                        "! {pending} exos restants pour {capacity} places avant le weekend : tout ne tombera pas."
                    ))
                    .color(theme::fg()),
                );
            } else {
                ui.label(RichText::new(format!("{pending} exos restants · {capacity} places avant le weekend")).color(theme::dim()));
            }
        }
    }

    fn ui_week(&mut self, ui: &mut egui::Ui, today: NaiveDate) {
        centered_column(ui, 1000.0, |ui| {
            self.ui_week_progress(ui, today);
            ui.add_space(28.0);
            // Calendrier et détail du jour côte à côte (empilés si la fenêtre est étroite).
            let w = ui.available_width();
            if w >= 640.0 {
                let gap = 18.0;
                let cal_w = ((w - gap) * 0.56).floor();
                let det_w = w - gap - cal_w;
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.allocate_ui_with_layout(Vec2::new(cal_w, 0.0), Layout::top_down(Align::Min), |ui| {
                        ui.set_width(cal_w);
                        self.ui_calendar(ui, today);
                    });
                    ui.add_space(gap);
                    ui.allocate_ui_with_layout(Vec2::new(det_w, 0.0), Layout::top_down(Align::Min), |ui| {
                        ui.set_width(det_w);
                        ui.add_space(30.0);
                        let d = self.cal_selected.unwrap_or(today);
                        self.ui_day_detail(ui, d, today);
                    });
                });
            } else {
                self.ui_calendar(ui, today);
                ui.add_space(12.0);
                let d = self.cal_selected.unwrap_or(today);
                self.ui_day_detail(ui, d, today);
            }
            ui.add_space(28.0);
            self.ui_database(ui, today);
        });
    }

    /// Fin de séance : la page se floute sous une vignette granuleuse,
    /// le message s'affiche au centre et le bouton reste net au-dessus.
    fn ui_veil(&mut self, ctx: &egui::Context, today: NaiveDate, central: Rect, start: usize, end: usize) {
        let active = self.tab == Tab::Today
            && self.data.day_kind(today) == DayKind::Training
            && self.data.is_complete(today)
            && self.celebration.is_none();
        if !active {
            self.veil_since = None;
            return;
        }
        let since = self.veil_since.get_or_insert_with(Instant::now).elapsed().as_secs_f32();
        let k = veil::progress(since);
        if k < 1.0 {
            ctx.request_repaint();
        }
        veil::blur_range(ctx, start, end, 5.0 * k);

        let screen = ctx.screen_rect();
        let region = Rect::from_min_max(Pos2::new(screen.left(), central.top()), screen.max);
        egui::Area::new(egui::Id::new("veil"))
            .order(egui::Order::Middle)
            .fixed_pos(region.min)
            .interactable(false)
            .show(ctx, |ui| {
                let p = ui.painter().with_clip_rect(region);
                veil::paint(&p, region, k);
                // Message centré, grand et épais
                let m = anim::progress(since - 0.25, 0.6);
                let c = region.center() + Vec2::new(0.0, 10.0 * (1.0 - m));
                let col = theme::fg().gamma_multiply(m);
                theme::bold_text(&p, c - Vec2::new(0.0, 28.0), Align2::CENTER_CENTER, "Reviens demain champion,", theme::title(42.0), col, 1.0);
                theme::bold_text(&p, c + Vec2::new(0.0, 28.0), Align2::CENTER_CENTER, "c'est bon pour aujourd'hui.", theme::title(42.0), col, 1.0);
            });
        if let Some(r) = self.done_bar {
            egui::Area::new(egui::Id::new("done_bar")).order(egui::Order::Foreground).fixed_pos(r.min).show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(r.size(), egui::Sense::hover());
                let p = ui.painter();
                // Même capsule que le bouton du bas.
                let s = theme::header_h() / HEADER_REF;
                let h = (46.0 * s).clamp(36.0, rect.height());
                let pill = Rect::from_center_size(rect.center(), Vec2::new(rect.width(), h));
                theme::panel(p, pill, h / 2.0, theme::done_fill());
                p.rect_stroke(pill, Rounding::same(h / 2.0), Stroke::new(theme::border(1.0_f32), theme::done_stroke()));
                p.text(pill.center(), Align2::CENTER_CENTER, "SÉANCE TERMINÉE", FontId::proportional((15.0 * s).clamp(12.0, 20.0)), theme::on(theme::done_fill()));
            });
        }
    }

    /// Interrupteurs de tous les effets ajoutés par-dessus les couleurs unies.
    fn ui_effects_switches(&mut self, ui: &mut egui::Ui) {
        section(ui, "DÉVELOPPEMENT · EFFETS");
        ui.label(RichText::new("Décoche un effet pour revenir à la couleur unie. Les réglages de chaque effet sont gardés.").color(theme::dim()).size(12.0));
        ui.add_space(6.0);
        let st = &mut self.data.settings;
        let mut changed = false;
        {
            let mut rows: Vec<(&str, Vec<(&str, &mut bool)>)> = vec![
                ("Page", vec![("Dégradé de fond", &mut st.background.enabled), ("Filtre VHS", &mut st.vhs.enabled), ("Filtre bitmap", &mut st.bitmap.enabled)]),
                (
                    "Roulettes",
                    vec![
                        ("Teinte", &mut st.reel_style.use_tint),
                        ("Dégradé", &mut st.reel_style.use_gradient),
                        ("Reflet", &mut st.reel_style.use_gloss),
                        ("Profondeur", &mut st.reel_style.use_depth),
                        ("Liseré", &mut st.reel_style.rim.enabled),
                    ],
                ),
                (
                    "Boutons à faire / fait",
                    vec![("Dégradé", &mut st.todo_style.use_gradient), ("Reflet", &mut st.todo_style.use_gloss), ("Liseré", &mut st.todo_style.rim.enabled)],
                ),
                ("Autres panneaux", vec![("Liseré", &mut st.glass.enabled)]),
            ];
            center_block(ui, "dev_effects", |ui| {
                egui::Grid::new("dev_effects").num_columns(2).spacing([18.0, 8.0]).show(ui, |ui| {
                    for (group, items) in rows.iter_mut() {
                        ui.label(RichText::new(*group).color(theme::dim()));
                        ui.horizontal_wrapped(|ui| {
                            for (name, flag) in items.iter_mut() {
                                changed |= ui.checkbox(flag, *name).changed();
                            }
                        });
                        ui.end_row();
                    }
                });
            });
        }
        ui.add_space(4.0);
        let set_all = |st: &mut crate::model::Settings, on: bool| {
            st.background.enabled = on;
            st.vhs.enabled = on;
            st.bitmap.enabled = on;
            st.reel_style.use_tint = on;
            st.reel_style.use_gradient = on;
            st.reel_style.use_gloss = on;
            st.reel_style.use_depth = on;
            st.reel_style.rim.enabled = on;
            st.todo_style.use_gradient = on;
            st.todo_style.use_gloss = on;
            st.todo_style.rim.enabled = on;
            st.glass.enabled = on;
        };
        ui.horizontal(|ui| {
            let w = 220.0 * 2.0 + ui.spacing().item_spacing.x;
            ui.add_space(((ui.available_width() - w) / 2.0).max(0.0));
            if ui.add_sized([220.0, 26.0], egui::Button::new("Tout désactiver (couleur unie)")).clicked() {
                set_all(st, false);
                changed = true;
            }
            if ui.add_sized([220.0, 26.0], egui::Button::new("Tout réactiver")).clicked() {
                set_all(st, true);
                changed = true;
            }
        });
        if changed {
            theme::set_background(st.background);
            theme::set_reel_style(st.reel_style);
            theme::set_todo_style(st.todo_style);
            theme::set_glass(st.glass);
            self.save();
        }
    }

    /// Outils de développement : polices par catégorie et palette de couleurs.
    fn ui_dev_theme(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.ui_effects_switches(ui);
        ui.add_space(24.0);
        section(ui, "DÉVELOPPEMENT · POLICES");
        ui.label(RichText::new("Temporaire : choisir la police de chaque catégorie de texte.").color(theme::dim()).size(12.0));
        ui.add_space(8.0);
        let fonts: Vec<String> = self.font_list.get_or_insert_with(|| theme::available_fonts().into_iter().map(|(n, _)| n).collect()).clone();
        let mut changed = false;
        center_block(ui, "dev_fonts", |ui| {
            egui::Grid::new("dev_fonts").num_columns(3).spacing([18.0, 10.0]).show(ui, |ui| {
                for (cat, sample, value) in self.data.settings.fonts.categories_mut() {
                    ui.label(cat);
                    egui::ComboBox::from_id_salt(("font", cat)).width(210.0).selected_text(value.as_str()).show_ui(ui, |ui| {
                        for f in &fonts {
                            changed |= ui.selectable_value(value, f.clone(), f.as_str()).changed();
                        }
                    });
                    let font = match cat {
                        "Titres" => theme::title(18.0),
                        "Chiffres" => theme::numbers(18.0),
                        "Roulettes" => theme::reel(18.0),
                        _ => FontId::proportional(16.0),
                    };
                    ui.label(RichText::new(sample).font(font).color(theme::fg()));
                    ui.end_row();
                }
            });
        });
        ui.add_space(6.0);
        if ui.button("Polices par défaut").clicked() {
            self.data.settings.fonts = theme::FontChoice::default();
            changed = true;
        }
        ui.add_space(10.0);
        let dir = theme::user_font_dir();
        ui.label(
            RichText::new("Tes polices : dépose des fichiers .ttf / .otf dans le dossier « polices » (ou dans RouletteApp\\polices), puis actualise. Elles apparaissent avec « (perso) ».")
                .color(theme::dim())
                .size(12.0),
        );
        ui.label(RichText::new(dir.display().to_string()).color(theme::dim()).size(11.0));
        ui.horizontal(|ui| {
            let w = 190.0 * 2.0 + ui.spacing().item_spacing.x;
            ui.add_space(((ui.available_width() - w) / 2.0).max(0.0));
            if ui.add_sized([190.0, 26.0], egui::Button::new("Ouvrir le dossier")).clicked() {
                let _ = std::fs::create_dir_all(&dir);
                let opener = if cfg!(windows) {
                    "explorer"
                } else if cfg!(target_os = "macos") {
                    "open"
                } else {
                    "xdg-open"
                };
                let _ = std::process::Command::new(opener).arg(&dir).spawn();
            }
            if ui.add_sized([190.0, 26.0], egui::Button::new("Actualiser la liste")).clicked() {
                self.font_list = None;
                changed = true;
            }
        });
        if changed {
            theme::apply_fonts(&ctx, &self.data.settings.fonts);
            self.save();
        }

        ui.add_space(24.0);
        section(ui, "DÉVELOPPEMENT · PALETTE");
        ui.label(RichText::new("Temporaire : choisir chaque teinte de l'app, ou partir d'un préréglage.").color(theme::dim()).size(12.0));
        ui.add_space(8.0);
        let mut pal_changed = false;
        center_block(ui, "presets", |ui| {
            egui::Grid::new("presets").spacing([8.0, 0.0]).show(ui, |ui| {
                for (name, preset) in theme::Palette::presets() {
                    let current = self.data.settings.palette == preset;
                    let resp = ui.add(egui::Button::new(RichText::new(name).color(if current { theme::bg() } else { theme::fg() })).fill(if current { theme::fg() } else { theme::gray(28) }));
                    // pastilles de la palette
                    let r = resp.rect;
                    for (k, c) in [preset.bg, preset.surface, preset.fg, preset.todo_fill, preset.done_fill].iter().enumerate() {
                        ui.painter().circle_filled(Pos2::new(r.left() + 8.0 + k as f32 * 7.0, r.bottom() + 5.0), 2.5, theme::rgb(*c));
                    }
                    if resp.clicked() {
                        self.data.settings.palette = preset;
                        pal_changed = true;
                    }
                }
            });
        });
        ui.add_space(14.0);
        center_block(ui, "dev_palette", |ui| {
            egui::Grid::new("dev_palette").num_columns(4).spacing([18.0, 8.0]).show(ui, |ui| {
                for (k, (name, rgb)) in self.data.settings.palette.roles_mut().into_iter().enumerate() {
                    pal_changed |= egui::color_picker::color_edit_button_srgb(ui, rgb).changed();
                    ui.label(name);
                    if k % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
        });
        if pal_changed {
            theme::set_palette(self.data.settings.palette);
            apply_style(&ctx);
            self.save();
        }

        ui.add_space(18.0);
        section(ui, "DÉVELOPPEMENT · DÉGRADÉ DE FOND");
        let g = &mut self.data.settings.background;
        let mut g_changed = ui.checkbox(&mut g.enabled, "Activer le dégradé").changed();
        ui.add_enabled_ui(g.enabled, |ui| {
            center_block(ui, "dev_gradient", |ui| {
                egui::Grid::new("dev_gradient").num_columns(2).spacing([18.0, 10.0]).show(ui, |ui| {
                    ui.label("Couleur d'arrivée");
                    g_changed |= egui::color_picker::color_edit_button_srgb(ui, &mut g.color).changed();
                    ui.end_row();
                    ui.label("Sens");
                    egui::ComboBox::from_id_salt("grad_dir").width(220.0).selected_text(g.direction.label()).show_ui(ui, |ui| {
                        for d in theme::GradientDir::ALL {
                            g_changed |= ui.selectable_value(&mut g.direction, d, d.label()).changed();
                        }
                    });
                    ui.end_row();
                    ui.label("Intensité");
                    ui.spacing_mut().slider_width = 220.0;
                    g_changed |= ui
                        .add(egui::Slider::new(&mut g.strength, 0.0..=1.0).custom_formatter(|v, _| format!("{:.0} %", v * 100.0)))
                        .changed();
                    ui.end_row();
                });
            });
        });
        if g_changed {
            theme::set_background(self.data.settings.background);
            self.save();
        }

        ui.add_space(18.0);
        section(ui, "DÉVELOPPEMENT · DIMENSIONS");
        let m = &mut self.data.settings.metrics;
        let mut m_changed = false;
        center_block(ui, "dev_metrics", |ui| {
            egui::Grid::new("dev_metrics").num_columns(2).spacing([18.0, 10.0]).show(ui, |ui| {
                ui.spacing_mut().slider_width = 220.0;
                ui.label("Hauteur de l'en-tête");
                m_changed |= ui.add(egui::Slider::new(&mut m.header_h, 52.0..=130.0).custom_formatter(|v, _| format!("{v:.0} px"))).changed();
                ui.end_row();
                ui.label("Épaisseur des bordures");
                m_changed |= ui.add(egui::Slider::new(&mut m.border, 0.0..=4.0).step_by(0.25).custom_formatter(|v, _| format!("{v:.2} px"))).changed();
                ui.end_row();
            });
        });
        if ui.button("Dimensions par défaut").clicked() {
            *m = theme::Metrics::default();
            m_changed = true;
        }
        if m_changed {
            theme::set_metrics(self.data.settings.metrics);
            self.save();
        }

        ui.add_space(18.0);
        section(ui, "DÉVELOPPEMENT · PANNEAUX DES ROULETTES");
        let rs = &mut self.data.settings.reel_style;
        let mut r_changed = false;
        let pct = |v: f64, _| format!("{:.0} %", v * 100.0);
        ui.label(RichText::new("Forme").color(theme::dim()).size(12.0));
        center_block(ui, "dev_reel_shape", |ui| {
            egui::Grid::new("dev_reel_shape").num_columns(2).spacing([18.0, 10.0]).show(ui, |ui| {
                ui.spacing_mut().slider_width = 220.0;
                ui.label("Rayon des coins");
                r_changed |= ui.add(egui::Slider::new(&mut rs.radius, 0.0..=60.0).step_by(1.0).custom_formatter(|v, _| format!("{v:.0} px"))).changed();
                ui.end_row();
                ui.label("Bordure pendant le tirage");
                r_changed |= ui.add(egui::Slider::new(&mut rs.border, 0.0..=5.0).step_by(0.25).custom_formatter(|v, _| format!("{v:.2} px"))).changed();
                ui.end_row();
                ui.label("Bordure à l'arrêt");
                r_changed |= ui.add(egui::Slider::new(&mut rs.landed_border, 0.0..=8.0).step_by(0.25).custom_formatter(|v, _| format!("{v:.2} px"))).changed();
                ui.end_row();
                ui.label("Lignes guides");
                r_changed |= ui.add(egui::Slider::new(&mut rs.guides, 0.0..=1.0).custom_formatter(pct)).changed();
                ui.end_row();
            });
        });
        ui.add_space(8.0);
        ui.label(RichText::new("Verre").color(theme::dim()).size(12.0));
        center_block(ui, "dev_reel_glass", |ui| {
            egui::Grid::new("dev_reel_glass").num_columns(2).spacing([18.0, 10.0]).show(ui, |ui| {
                ui.spacing_mut().slider_width = 220.0;
                r_changed |= ui.checkbox(&mut rs.use_tint, "Teinte").changed();
                ui.add_enabled_ui(rs.use_tint, |ui| {
                    ui.horizontal(|ui| {
                        r_changed |= ui.color_edit_button_srgb(&mut rs.tint).changed();
                        ui.spacing_mut().slider_width = 180.0;
                        r_changed |= ui.add(egui::Slider::new(&mut rs.tint_mix, 0.0..=1.0).custom_formatter(pct)).changed();
                    });
                });
                ui.end_row();
                ui.label("Opacité du fond");
                r_changed |= ui.add(egui::Slider::new(&mut rs.opacity, 0.0..=1.0).custom_formatter(pct)).changed();
                ui.end_row();
                r_changed |= ui.checkbox(&mut rs.use_gradient, "Dégradé vertical").changed();
                r_changed |= ui.add_enabled(rs.use_gradient, egui::Slider::new(&mut rs.gradient, -1.0..=1.0).step_by(0.05).custom_formatter(|v, _| format!("{:+.0} %", v * 100.0))).changed();
                ui.end_row();
                r_changed |= ui.checkbox(&mut rs.use_gloss, "Reflet brillant").changed();
                r_changed |= ui.add_enabled(rs.use_gloss, egui::Slider::new(&mut rs.gloss, 0.0..=1.0).custom_formatter(pct)).changed();
                ui.end_row();
                ui.label("Hauteur du reflet");
                r_changed |= ui.add_enabled(rs.use_gloss, egui::Slider::new(&mut rs.gloss_height, 0.05..=1.0).custom_formatter(pct)).changed();
                ui.end_row();
                r_changed |= ui.checkbox(&mut rs.use_depth, "Profondeur (tambour)").changed();
                r_changed |= ui.add_enabled(rs.use_depth, egui::Slider::new(&mut rs.depth, 0.0..=1.0).custom_formatter(pct)).changed();
                ui.end_row();
                ui.label("Taille de l'ombre");
                r_changed |= ui.add_enabled(rs.use_depth, egui::Slider::new(&mut rs.depth_size, 0.02..=0.5).custom_formatter(pct)).changed();
                ui.end_row();
            });
        });
        ui.add_space(8.0);
        ui.label(RichText::new("Liseré lumineux").color(theme::dim()).size(12.0));
        r_changed |= rim_controls(ui, "dev_reel_rim", &mut rs.rim);
        if ui.button("Roulettes par défaut").clicked() {
            *rs = theme::ReelStyle::default();
            r_changed = true;
        }
        if r_changed {
            theme::set_reel_style(self.data.settings.reel_style);
            self.save();
        }

        ui.add_space(18.0);
        section(ui, "DÉVELOPPEMENT · BOUTONS « À FAIRE » / « FAIT »");
        let ts = &mut self.data.settings.todo_style;
        let mut t_changed = false;
        center_block(ui, "dev_todo", |ui| {
            egui::Grid::new("dev_todo").num_columns(2).spacing([18.0, 10.0]).show(ui, |ui| {
                ui.spacing_mut().slider_width = 220.0;
                let pct = |v: f64, _| format!("{:.0} %", v * 100.0);
                let px = |v: f64, _| format!("{v:.1} px");
                ui.label("Hauteur (part de la roulette)");
                t_changed |= ui.add(egui::Slider::new(&mut ts.height, 0.15..=0.6).custom_formatter(pct)).changed();
                ui.end_row();
                ui.label("Écart avec la roulette");
                t_changed |= ui.add(egui::Slider::new(&mut ts.gap, 0.0..=40.0).step_by(1.0).custom_formatter(px)).changed();
                ui.end_row();
                ui.label("Rayon des coins");
                t_changed |= ui.add(egui::Slider::new(&mut ts.radius, 0.0..=60.0).step_by(1.0).custom_formatter(px)).changed();
                ui.end_row();
                ui.label("Bordure « à faire »");
                t_changed |= ui.add(egui::Slider::new(&mut ts.border, 0.0..=5.0).step_by(0.25).custom_formatter(px)).changed();
                ui.end_row();
                ui.label("Bordure « fait »");
                t_changed |= ui.add(egui::Slider::new(&mut ts.done_border, 0.0..=8.0).step_by(0.25).custom_formatter(px)).changed();
                ui.end_row();
                ui.label("Taille du texte");
                t_changed |= ui.add(egui::Slider::new(&mut ts.text_size, 10.0..=32.0).step_by(0.5).custom_formatter(|v, _| format!("{v:.1} pt"))).changed();
                ui.end_row();
                ui.label("Opacité du fond");
                t_changed |= ui.add(egui::Slider::new(&mut ts.opacity, 0.0..=1.0).custom_formatter(pct)).changed();
                ui.end_row();
                t_changed |= ui.checkbox(&mut ts.use_gradient, "Dégradé vertical").changed();
                t_changed |= ui.add_enabled(ts.use_gradient, egui::Slider::new(&mut ts.gradient, -1.0..=1.0).step_by(0.05).custom_formatter(|v, _| format!("{:+.0} %", v * 100.0))).changed();
                ui.end_row();
                t_changed |= ui.checkbox(&mut ts.use_gloss, "Reflet brillant").changed();
                t_changed |= ui.add_enabled(ts.use_gloss, egui::Slider::new(&mut ts.gloss, 0.0..=1.0).custom_formatter(pct)).changed();
                ui.end_row();
                ui.label("Hauteur du reflet");
                t_changed |= ui.add_enabled(ts.use_gloss, egui::Slider::new(&mut ts.gloss_height, 0.05..=1.0).custom_formatter(pct)).changed();
                ui.end_row();
            });
        });
        ui.add_space(8.0);
        ui.label(RichText::new("Liseré lumineux").color(theme::dim()).size(12.0));
        t_changed |= rim_controls(ui, "dev_todo_rim", &mut ts.rim);
        if ui.button("Boutons par défaut").clicked() {
            *ts = theme::TodoStyle::default();
            t_changed = true;
        }
        if t_changed {
            theme::set_todo_style(self.data.settings.todo_style);
            self.save();
        }

        ui.add_space(18.0);
        section(ui, "DÉVELOPPEMENT · LISERÉ DES AUTRES PANNEAUX");
        let g = &mut self.data.settings.glass;
        let mut g_changed = rim_controls(ui, "dev_glass", g);
        if ui.button("Liseré par défaut").clicked() {
            *g = theme::GlassRim::panels();
            g_changed = true;
        }
        if g_changed {
            theme::set_glass(self.data.settings.glass);
            self.save();
        }

        ui.add_space(18.0);
        section(ui, "DÉVELOPPEMENT · FILTRE VHS");
        let v = &mut self.data.settings.vhs;
        let mut v_changed = false;
        center_block(ui, "dev_vhs", |ui| {
            egui::Grid::new("dev_vhs").num_columns(2).spacing([18.0, 10.0]).show(ui, |ui| {
                ui.spacing_mut().slider_width = 220.0;
                let pct = |v: f64, _| format!("{:.0} %", v * 100.0);
                ui.label("Activer");
                v_changed |= ui.checkbox(&mut v.enabled, "").changed();
                ui.end_row();
                for (label, val) in [
                    ("Grain", &mut v.grain),
                    ("Lignes de balayage", &mut v.scanlines),
                    ("Bande de tracking", &mut v.tracking),
                    ("Parasites", &mut v.glitches),
                ] {
                    ui.label(label);
                    v_changed |= ui.add_enabled(v.enabled, egui::Slider::new(val, 0.0..=1.0).custom_formatter(pct)).changed();
                    ui.end_row();
                }
                ui.label("Vitesse");
                v_changed |= ui
                    .add_enabled(v.enabled, egui::Slider::new(&mut v.speed, 0.0..=2.0).step_by(0.05).custom_formatter(|v, _| format!("×{v:.2}")))
                    .changed();
                ui.end_row();
            });
        });
        if ui.button("Filtre par défaut").clicked() {
            *v = crate::vhs::VhsFilter::default();
            v_changed = true;
        }
        if v_changed {
            self.save();
        }

        ui.add_space(18.0);
        section(ui, "DÉVELOPPEMENT · FILTRE BITMAP");
        let b = &mut self.data.settings.bitmap;
        let mut b_changed = false;
        center_block(ui, "dev_bitmap", |ui| {
            egui::Grid::new("dev_bitmap").num_columns(2).spacing([18.0, 10.0]).show(ui, |ui| {
                ui.spacing_mut().slider_width = 220.0;
                let on = b.enabled;
                ui.label("Activer");
                b_changed |= ui.checkbox(&mut b.enabled, "").changed();
                ui.end_row();
                ui.label("Couleurs");
                ui.add_enabled_ui(on, |ui| {
                    egui::ComboBox::from_id_salt("bitmap_mode").width(220.0).selected_text(b.mode.label()).show_ui(ui, |ui| {
                        for m in crate::bitmap::BitmapMode::ALL {
                            b_changed |= ui.selectable_value(&mut b.mode, m, m.label()).changed();
                        }
                    });
                });
                ui.end_row();
                ui.label("Niveaux");
                b_changed |= ui
                    .add_enabled(on, egui::Slider::new(&mut b.levels, 2..=16).custom_formatter(|v, _| if v <= 2.0 { "2 (1 bit)".into() } else { format!("{v:.0}") }))
                    .changed();
                ui.end_row();
                ui.label("Taille des pixels");
                b_changed |= ui.add_enabled(on, egui::Slider::new(&mut b.pixel, 1.0..=10.0).step_by(0.5).custom_formatter(|v, _| format!("{v:.1} px"))).changed();
                ui.end_row();
                let pct = |v: f64, _| format!("{:.0} %", v * 100.0);
                ui.label("Tramage");
                b_changed |= ui.add_enabled(on, egui::Slider::new(&mut b.dither, 0.0..=1.0).custom_formatter(pct)).changed();
                ui.end_row();
                ui.label("Intensité");
                b_changed |= ui.add_enabled(on, egui::Slider::new(&mut b.intensity, 0.0..=1.0).custom_formatter(pct)).changed();
                ui.end_row();
            });
        });
        if ui.button("Bitmap par défaut").clicked() {
            *b = crate::bitmap::BitmapFilter::default();
            b_changed = true;
        }
        if b_changed {
            self.save();
        }
    }

    /// En-tête flottant au-dessus de la page : logo à gauche, onglets en pilules au centre,
    /// réglages en bouton rond à droite. Fond en dégradé : la page s'estompe en passant dessous.
    fn ui_header(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let band = Rect::from_min_size(screen.min, Vec2::new(screen.width(), theme::header_h()));
        // Tout l'en-tête suit sa hauteur réglable.
        let s = band.height() / HEADER_REF;
        let mut clicked = None;
        egui::Area::new(egui::Id::new("header")).order(egui::Order::Foreground).fixed_pos(band.min).show(ctx, |ui| {
            let (_, _) = ui.allocate_exact_size(band.size(), egui::Sense::hover());
            let p = ui.painter().clone();

            // Fond : opaque en haut, puis fondu vers le bas.
            // (couleurs prises sur le fond, dégradé compris)
            let mut m = egui::Mesh::default();
            let stops = [(0.0, 1.0), (0.62, 0.94), (1.0, 0.0)];
            let cols = 16u32;
            for (t, a) in stops.iter() {
                let y = band.top() + band.height() * t;
                for i in 0..=cols {
                    let u = i as f32 / cols as f32;
                    let c = theme::bg_at(u, (y - screen.top()) / screen.height());
                    m.colored_vertex(Pos2::new(band.left() + u * band.width(), y), c.gamma_multiply(*a));
                }
            }
            for row in 0..(stops.len() as u32 - 1) {
                for i in 0..cols {
                    let a = row * (cols + 1) + i;
                    let (b, c, d) = (a + 1, a + cols + 1, a + cols + 2);
                    m.add_triangle(a, b, d);
                    m.add_triangle(a, d, c);
                }
            }
            p.add(egui::Shape::mesh(m));

            let cy = band.top() + 34.0 * s;
            let h = 34.0 * s;

            // Faux logo : une lettre dans une capsule
            let logo = Rect::from_center_size(Pos2::new(band.left() + 24.0 + 24.0 * s, cy), Vec2::new(48.0 * s, h));
            theme::panel(&p, logo, h / 2.0, theme::fg());
            p.text(logo.center() + Vec2::new(0.0, 1.0), Align2::CENTER_CENTER, "R", theme::title(21.0 * s), theme::bg());

            // Onglets en pilules, centrés
            let tabs = [(Tab::Today, "AUJOURD'HUI"), (Tab::Week, "RÉSUMÉ ET ORGANISATION")];
            let font = FontId::proportional(13.0 * s);
            let gap = 8.0 * s;
            let galleys: Vec<_> = tabs.iter().map(|(_, l)| p.layout_no_wrap(l.to_string(), font.clone(), theme::fg())).collect();
            let widths: Vec<f32> = galleys.iter().map(|g| g.size().x + 44.0 * s).collect();
            let total = widths.iter().sum::<f32>() + gap;
            let mut x = band.center().x - total / 2.0;
            let mut rects = Vec::new();
            for w in &widths {
                rects.push(Rect::from_min_size(Pos2::new(x, cy - h / 2.0), Vec2::new(*w, h)));
                x += w + gap;
            }
            let sel = tabs.iter().position(|(t, _)| *t == self.tab);
            let target = rects[sel.unwrap_or(0)];
            let px = anim::smooth(ctx, egui::Id::new("pill_x"), target.left(), 0.38);
            let pw = anim::smooth(ctx, egui::Id::new("pill_w"), target.width(), 0.38);
            let pa = anim::smooth(ctx, egui::Id::new("pill_a"), if sel.is_some() { 1.0 } else { 0.0 }, 0.3);
            let pill = Rect::from_min_size(Pos2::new(px, cy - h / 2.0), Vec2::new(pw, h));
            for (i, ((tab, _), r)) in tabs.iter().zip(&rects).enumerate() {
                let resp = ui.interact(*r, egui::Id::new(("tab_pill", i)), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                let fill = if resp.hovered() { theme::gray(34) } else { theme::surface() };
                theme::panel(&p, *r, h / 2.0, fill);
                p.rect_stroke(*r, Rounding::same(h / 2.0), Stroke::new(theme::border(1.0_f32), theme::line()));
                if resp.clicked() && *tab != self.tab {
                    clicked = Some(*tab);
                }
            }
            if pa > 0.01 {
                theme::panel(&p, pill, h / 2.0, theme::fg().gamma_multiply(pa));
            }
            for (g, r) in galleys.into_iter().zip(&rects) {
                let pos = r.center() - g.size() / 2.0;
                p.with_clip_rect(*r).galley(pos, g.clone(), theme::fg());
                let over = r.intersect(pill);
                if pa > 0.5 && over.width() > 0.0 {
                    p.with_clip_rect(over).galley_with_override_text_color(pos, g, theme::bg());
                }
            }

            // Réglages : bouton rond à droite, icône d'engrenage dessinée
            let c = Pos2::new(band.right() - 24.0 - h / 2.0, cy);
            let r_btn = Rect::from_center_size(c, Vec2::splat(h + 4.0 * s));
            let resp = ui.interact(r_btn, egui::Id::new("settings_btn"), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Réglages");
            let on = self.tab == Tab::Settings;
            let (fill, ink) = if on {
                (theme::fg(), theme::bg())
            } else if resp.hovered() {
                (theme::gray(34), theme::fg())
            } else {
                (theme::surface(), theme::fg())
            };
            theme::panel(&p, r_btn, h / 2.0 + 2.0 * s, fill);
            p.circle_stroke(c, h / 2.0 + 2.0 * s, Stroke::new(theme::border(1.0), if on { theme::fg() } else { theme::line() }));
            paint_gear(&p, c, s, ink, fill, anim::smooth(ctx, egui::Id::new("gear_rot"), if on { 1.0 } else { 0.0 }, 0.45));
            if resp.clicked() && !on {
                clicked = Some(Tab::Settings);
            }
        });
        if let Some(tab) = clicked {
            self.tab = tab;
            self.tab_changed = Some(Instant::now());
        }
    }

    fn ui_calendar_reset(&mut self, ui: &mut egui::Ui) {
        section(ui, "CALENDRIER");
        let txt = match self.data.calendar_start {
            Some(d) => format!("Calendrier et série remis à zéro le {}.", fr_date(d)),
            None => "Remet à zéro les jours accomplis / loupés du calendrier et la série en cours.".to_string(),
        };
        ui.label(RichText::new(txt).color(theme::dim()));
        ui.label(RichText::new("La base d'exercices (séances, reps, poids) n'est pas touchée.").color(theme::dim()).size(12.0));
        ui.add_space(8.0);
        let btn = egui::Button::new(RichText::new("Réinitialiser le calendrier").color(theme::todo_text()))
            .fill(theme::todo_fill())
            .stroke(Stroke::new(1.0_f32, theme::todo_stroke()));
        if ui.add(btn).clicked() {
            self.open_modal(Modal::ResetCalendar);
        }
    }

    fn open_modal(&mut self, m: Modal) {
        self.modal = Some(m);
        self.modal_opened = Instant::now();
    }

    /// Fenêtre de validation (reset du calendrier, suppression / renommage d'exercice).
    fn ui_modal(&mut self, ctx: &egui::Context, today: NaiveDate) {
        let Some(modal) = &mut self.modal else { return };
        let k = anim::progress(self.modal_opened.elapsed().as_secs_f32(), 0.25);
        if k < 1.0 {
            ctx.request_repaint();
        }
        let screen = ctx.screen_rect();
        let mut result: Option<bool> = None; // Some(true) = confirmer, Some(false) = annuler

        egui::Area::new(egui::Id::new("modal")).order(egui::Order::Tooltip).fixed_pos(screen.min).show(ctx, |ui| {
            let backdrop = ui.allocate_rect(screen, egui::Sense::click());
            ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha((190.0 * k) as u8));
            if backdrop.clicked() {
                result = Some(false);
            }
            let size = Vec2::new(420.0_f32.min(screen.width() - 32.0), 210.0);
            let card = Rect::from_center_size(screen.center() + Vec2::new(0.0, 12.0 * (1.0 - k)), size);
            theme::panel(ui.painter(), card, 12.0, theme::surface());
            ui.painter().rect_stroke(card, Rounding::same(12.0), Stroke::new(theme::border(1.0_f32), theme::gray(70)));
            let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(card.shrink(22.0)).layout(Layout::top_down(Align::Min)));
            inner.set_opacity(k);
            let ui = &mut inner;

            let (title, body, confirm, danger) = match &*modal {
                Modal::ResetCalendar => (
                    "Réinitialiser le calendrier ?".to_string(),
                    "Les jours accomplis et loupés, ainsi que la série en cours, repartent de zéro à partir d'aujourd'hui.\nLa base d'exercices (séances, reps, poids) est conservée.".to_string(),
                    "RÉINITIALISER",
                    true,
                ),
                Modal::DeleteExercise(name) => (
                    format!("Supprimer « {name} » ?"),
                    "Il ne sera plus tiré à la roulette. Ses séances passées restent dans l'historique.".to_string(),
                    "SUPPRIMER",
                    true,
                ),
                Modal::RenameExercise(name, _) => (format!("Renommer « {name} »"), String::new(), "RENOMMER", false),
            };
            ui.label(RichText::new(title).size(19.0).color(theme::fg()));
            ui.add_space(10.0);
            if let Modal::RenameExercise(_, buf) = modal {
                let r = ui.add(egui::TextEdit::singleline(buf).desired_width(f32::INFINITY).font(FontId::proportional(16.0)));
                if !r.has_focus() && !r.lost_focus() {
                    r.request_focus();
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    result = Some(true);
                }
            } else {
                ui.label(RichText::new(body).color(theme::dim()));
            }

            // Boutons
            let bar = Rect::from_min_max(Pos2::new(card.left() + 22.0, card.bottom() - 62.0), card.right_bottom() - Vec2::new(22.0, 20.0));
            let half = (bar.width() - 10.0) / 2.0;
            let cancel_r = Rect::from_min_size(bar.min, Vec2::new(half, bar.height()));
            let ok_r = Rect::from_min_size(Pos2::new(bar.left() + half + 10.0, bar.top()), Vec2::new(half, bar.height()));
            let cancel = ui.interact(cancel_r, egui::Id::new("modal_cancel"), egui::Sense::click());
            let ok = ui.interact(ok_r, egui::Id::new("modal_ok"), egui::Sense::click());
            let p = ui.painter();
            p.rect(cancel_r, Rounding::same(6.0), if cancel.hovered() { theme::gray(44) } else { theme::gray(30) }, Stroke::new(1.0_f32, theme::line()));
            p.text(cancel_r.center(), Align2::CENTER_CENTER, "ANNULER", FontId::proportional(14.0), theme::fg());
            let (fill, fg) = if danger { (theme::todo_fill(), Color32::WHITE) } else { (theme::fg(), theme::bg()) };
            let fill = if ok.hovered() { fill.gamma_multiply(1.25) } else { fill };
            p.rect(ok_r, Rounding::same(6.0), fill, Stroke::new(1.0_f32, if danger { theme::todo_stroke() } else { theme::fg() }));
            p.text(ok_r.center(), Align2::CENTER_CENTER, confirm, FontId::proportional(14.0), fg);
            if cancel.clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                result = Some(false);
            }
            if ok.clicked() {
                result = Some(true);
            }
        });

        match result {
            Some(false) => self.modal = None,
            Some(true) => {
                match self.modal.take() {
                    Some(Modal::ResetCalendar) => {
                        self.data.reset_calendar(today);
                        self.save();
                    }
                    Some(Modal::DeleteExercise(name)) => {
                        if let Some(i) = self.data.pool.iter().position(|e| *e == name) {
                            self.data.pool.remove(i);
                            if self.gallery_open.as_deref() == Some(name.as_str()) {
                                self.gallery_open = None;
                            }
                            self.save();
                        }
                    }
                    Some(Modal::RenameExercise(old, new)) => {
                        if let Some(i) = self.data.pool.iter().position(|e| *e == old) {
                            match self.data.rename_exercise(i, &new, today) {
                                Ok(()) => {
                                    if self.gallery_open.as_deref() == Some(old.as_str()) {
                                        self.gallery_open = Some(new.trim().to_string());
                                    }
                                    self.save();
                                }
                                Err(e) => {
                                    self.message = Some(e.into());
                                    self.open_modal(Modal::RenameExercise(old, new));
                                }
                            }
                        }
                    }
                    None => {}
                }
            }
            None => {}
        }
    }

    fn ui_calendar(&mut self, ui: &mut egui::Ui, today: NaiveDate) {
        let this_month = today.with_day(1).unwrap();
        let month = self.cal_month.unwrap_or(this_month);
        ui.horizontal(|ui| {
            section(ui, "CALENDRIER");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if arrow_side_button(ui, false) {
                    self.cal_month = Some(add_months(month, 1));
                }
                ui.label(RichText::new(format!("{} {}", fr_month(month), month.year()).to_uppercase()).color(theme::fg()).size(13.0));
                if arrow_side_button(ui, true) {
                    self.cal_month = Some(add_months(month, -1));
                }
                if month != this_month && ui.small_button("Aujourd'hui").clicked() {
                    self.cal_month = None;
                }
            });
        });
        ui.add_space(6.0);

        let gap = 4.0;
        let cell_w = ((ui.available_width() - 6.0 * gap) / 7.0).min(80.0);
        let cell = Vec2::new(cell_w, 40.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(cell_w * 7.0 + gap * 6.0, 18.0 + 6.0 * (cell.y + gap)), egui::Sense::hover());
        let p = ui.painter();
        for (i, name) in ["LUN", "MAR", "MER", "JEU", "VEN", "SAM", "DIM"].iter().enumerate() {
            let x = rect.left() + i as f32 * (cell_w + gap) + cell_w / 2.0;
            p.text(Pos2::new(x, rect.top() + 7.0), Align2::CENTER_CENTER, *name, FontId::proportional(11.0), theme::dim());
        }
        let selected = self.cal_selected.unwrap_or(today);
        let mut clicked = None;
        let first = week_start(month);
        for i in 0..42 {
            let d = first + Duration::days(i);
            let (row, colm) = (i / 7, i % 7);
            if row == 5 && d.month() != month.month() && colm == 0 {
                break;
            }
            let min = Pos2::new(rect.left() + colm as f32 * (cell_w + gap), rect.top() + 18.0 + row as f32 * (cell.y + gap));
            let r = Rect::from_min_size(min, cell);
            let in_month = d.month() == month.month();
            let status = self.data.day_status(d, today);
            let resp = ui.interact(r, egui::Id::new(("cal_day", d)), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                clicked = Some(d);
            }
            let p = ui.painter();
            let (fill, stroke, text) = cal_style(status);
            let fade = |c: Color32| if in_month { c } else { c.gamma_multiply(0.35) };
            p.rect(r, Rounding::same(5.0), fade(fill), Stroke::new(1.0_f32, fade(stroke)));
            if d == today {
                p.rect_stroke(r, Rounding::same(5.0), Stroke::new(2.0_f32, theme::fg()));
            }
            if selected == d {
                p.rect_stroke(r.expand(2.0), Rounding::same(7.0), Stroke::new(1.5_f32, theme::done_stroke()));
            } else if resp.hovered() {
                p.rect_stroke(r, Rounding::same(5.0), Stroke::new(1.0_f32, theme::gray(110)));
            }
            // Tirage du jour : x/3 exercices faits ; point gris = journée d'avant un reset.
            if in_month {
                if let Some(rec) = self.data.record(d) {
                    let n = rec.done.iter().filter(|x| **x).count();
                    p.text(r.left_bottom() + Vec2::new(7.0, -5.0), Align2::LEFT_BOTTOM, format!("{n}/{}", rec.exercises.len()), theme::numbers(10.0), fade(text).gamma_multiply(0.8));
                }
            }
            p.text(r.left_top() + Vec2::new(7.0, 6.0), Align2::LEFT_TOP, d.day().to_string(), theme::numbers(12.0), fade(text));
            let mark = match status {
                DayStatus::Complete => Some(Mark::Complete),
                DayStatus::Missed => Some(Mark::Missed),
                DayStatus::Rest => Some(Mark::Rest),
                _ => None,
            };
            if let (Some(m), true) = (mark, in_month) {
                paint_mark(p, r.right_bottom() - Vec2::new(10.0, 10.0), m);
            }
        }

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            for (status, label) in [
                (DayStatus::Complete, "accomplie"),
                (DayStatus::Missed, "loupée"),
                (DayStatus::Rest, "repos"),
                (DayStatus::Planned, "séance prévue"),
                (DayStatus::Off, "weekend"),
            ] {
                let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), egui::Sense::hover());
                let (fill, stroke, _) = cal_style(status);
                ui.painter().rect(r, Rounding::same(3.0), fill, Stroke::new(1.0_f32, stroke));
                ui.label(RichText::new(label).color(theme::dim()).size(12.0));
                ui.add_space(8.0);
            }
        });
        if let Some(d) = clicked {
            self.cal_selected = Some(d);
        }
    }

    /// Détail d'un jour du calendrier (ce qu'affichait l'ancien onglet Historique).
    fn ui_day_detail(&self, ui: &mut egui::Ui, d: NaiveDate, today: NaiveDate) {
        let status = self.data.day_status(d, today);
        let label = match status {
            DayStatus::Complete => "séance accomplie",
            DayStatus::Missed => "séance loupée",
            DayStatus::Rest => "repos",
            DayStatus::Off => "weekend",
            DayStatus::Today => "aujourd'hui",
            DayStatus::Planned => "séance prévue",
            DayStatus::Before => "—",
        };
        let (fill, stroke, text) = cal_style(status);
        // Fond placé sous le contenu une fois sa taille connue.
        let bg_idx = ui.painter().add(egui::Shape::Noop);
        let framed = Frame::none()
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(fr_date(d)).font(theme::title(18.0)).color(theme::fg()));
                    ui.add_space(8.0);
                    let g = ui.painter().layout_no_wrap(label.to_string(), FontId::proportional(11.5), text);
                    let (r, _) = ui.allocate_exact_size(g.size() + Vec2::new(18.0, 8.0), egui::Sense::hover());
                    ui.painter().rect(r, Rounding::same(10.0), fill, Stroke::new(1.0_f32, stroke));
                    ui.painter().galley(r.center() - g.size() / 2.0, g, text);
                });
                ui.add_space(8.0);
                // Tirage de ce jour (les journées archivées par un reset ne sont plus affichées).
                let recs: Vec<&crate::model::DayRecord> = self.data.active_days().filter(|r| r.date == d).collect();
                if recs.is_empty() {
                    ui.label(RichText::new("Aucun tirage ce jour-là.").color(theme::dim()));
                }
                for rec in recs {
                    for (i, ex) in rec.exercises.iter().enumerate() {
                        let done = rec.done.get(i).copied().unwrap_or(false);
                        ui.horizontal(|ui| {
                            let (mr, _) = ui.allocate_exact_size(Vec2::new(12.0, 18.0), egui::Sense::hover());
                            paint_mark(ui.painter(), mr.center(), if done { Mark::Complete } else { Mark::Todo });
                            ui.allocate_ui_with_layout(Vec2::new(150.0, 18.0), Layout::left_to_right(Align::Center), |ui| {
                                ui.set_min_width(150.0);
                                ui.label(RichText::new(ex).color(theme::fg()));
                            });
                            let detail = match rec.logs.get(i).and_then(|l| l.as_ref()) {
                                Some(l) => database::fmt_log(l),
                                None if done => "fait".to_string(),
                                None => "non fait".to_string(),
                            };
                            ui.label(RichText::new(detail).size(12.5).color(if done { theme::fg() } else { theme::dim() }));
                        });
                    }
                    let n = rec.done.iter().filter(|x| **x).count();
                    ui.label(
                        RichText::new(format!("{n}/{} faits · {} relance(s)", rec.exercises.len(), rec.rerolls_used))
                            .size(11.5)
                            .color(theme::dim()),
                    );
                }
            });
        let r = framed.response.rect;
        let mut shapes = theme::panel_shapes(r, 10.0, theme::surface());
        shapes.push(egui::Shape::rect_stroke(r, 10.0, Stroke::new(theme::border(1.0_f32), theme::line())));
        ui.painter().set(bg_idx, egui::Shape::Vec(shapes));
    }

    fn ui_settings(&mut self, ui: &mut egui::Ui, today: NaiveDate) {
        // Aperçu à droite : la colonne des réglages se décale pour lui laisser la place.
        let room = PREVIEW_W + 32.0;
        if self.preview_on && ui.available_width() >= 560.0 + room {
            let w = ui.available_width() - room;
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(Vec2::new(w, 0.0), Layout::top_down(Align::Center), |ui| {
                    ui.set_width(w);
                    centered_column(ui, 640.0, |ui| self.ui_settings_inner(ui, today));
                });
            });
        } else {
            centered_column(ui, 640.0, |ui| self.ui_settings_inner(ui, today));
        }
    }

    /// Aperçu en direct des réglages d'apparence : une roulette, les boutons
    /// « à faire » / « fait » et un panneau, dessinés avec les réglages en cours.
    fn ui_preview(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let top = screen.top() + theme::header_h() + 12.0;
        let x = screen.right() - PREVIEW_W - 20.0;
        if screen.width() < 560.0 + PREVIEW_W + 32.0 + 48.0 {
            return;
        }
        if !self.preview_on {
            egui::Area::new(egui::Id::new("preview_btn")).order(egui::Order::Foreground).fixed_pos(Pos2::new(screen.right() - 120.0, top)).show(ctx, |ui| {
                if ui.add_sized([100.0, 26.0], egui::Button::new("Aperçu")).clicked() {
                    self.preview_on = true;
                }
            });
            return;
        }
        let side = PREVIEW_W - 40.0;
        let ts = theme::todo_style();
        let rs = theme::reel_style();
        let btn_h = side * ts.height.clamp(0.1, 0.8);
        let h = 40.0 + side + ts.gap + btn_h + 10.0 + btn_h + 14.0 + 44.0 + 20.0;
        let frame = Rect::from_min_size(Pos2::new(x, top), Vec2::new(PREVIEW_W, h.min(screen.bottom() - top - 12.0)));
        egui::Area::new(egui::Id::new("preview")).order(egui::Order::Foreground).fixed_pos(frame.min).show(ctx, |ui| {
            let (frame, _) = ui.allocate_exact_size(frame.size(), egui::Sense::hover());
            let p = ui.painter().with_clip_rect(frame.expand(4.0));
            p.rect(frame.shrink(1.0), Rounding::same(12.0), theme::bg(), Stroke::new(1.0_f32, theme::line()));
            p.text(Pos2::new(frame.left() + 16.0, frame.top() + 20.0), Align2::LEFT_CENTER, "APERÇU", FontId::proportional(11.0), theme::dim());
            let close = Rect::from_center_size(Pos2::new(frame.right() - 40.0, frame.top() + 20.0), Vec2::new(60.0, 20.0));
            let c = ui.interact(close, ui.id().with("preview_hide"), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
            p.text(close.center(), Align2::CENTER_CENTER, "masquer", FontId::proportional(11.0), if c.hovered() { theme::fg() } else { theme::dim() });
            if c.clicked() {
                self.preview_on = false;
            }

            // Roulette arrêtée sur un exercice
            let r = Rect::from_min_size(Pos2::new(frame.left() + 20.0, frame.top() + 40.0), Vec2::splat(side));
            theme::reel_base(&p, r);
            p.text(r.center_top() + Vec2::new(0.0, 16.0), Align2::CENTER_CENTER, "01", theme::numbers(11.0), theme::dim());
            let reel_r = Rect::from_min_max(r.min + Vec2::new(5.0, 32.0), r.max - Vec2::new(5.0, 7.0));
            let rp = p.with_clip_rect(reel_r);
            let cy = reel_r.center().y;
            let radius = reel_r.height() * 0.5;
            let row_h = radius * ROW_ANGLE.sin();
            for dy in [-row_h * 0.5, row_h * 0.5] {
                rp.hline(reel_r.x_range().shrink(8.0), cy + dy, Stroke::new(1.0_f32, theme::gray(34).gamma_multiply(rs.guides.clamp(0.0, 1.0))));
            }
            let pool = &self.data.pool;
            let name = |k: i64| -> String {
                if pool.is_empty() {
                    "Exercice".into()
                } else {
                    pool[(k.rem_euclid(pool.len() as i64)) as usize].clone()
                }
            };
            for k in -3..=3i64 {
                let Some((y, depth)) = reel::place(0.0, k) else { continue };
                let color = if k == 0 { theme::fg() } else { theme::gray((24.0 + 150.0 * depth.powi(3)) as u8) };
                rp.text(Pos2::new(reel_r.center().x, cy + radius * y), Align2::CENTER_CENTER, name(k), theme::reel(20.0 * (0.5 + 0.5 * depth)), color);
            }
            theme::reel_overlay(&p, r, rs.landed_border, theme::fg());

            // Boutons « à faire » et « fait »
            let b1 = Rect::from_min_size(Pos2::new(r.left(), r.bottom() + ts.gap), Vec2::new(side, btn_h));
            paint_done(&p, b1, false, true, 0.0, 0.0, false);
            let b2 = b1.translate(Vec2::new(0.0, btn_h + 10.0));
            paint_done(&p, b2, true, true, 1.0, 0.0, false);

            // Autre panneau
            let pr = Rect::from_min_size(Pos2::new(r.left(), b2.bottom() + 14.0), Vec2::new(side, 44.0));
            theme::panel(&p, pr, 8.0, theme::surface());
            p.rect_stroke(pr, Rounding::same(8.0), Stroke::new(theme::border(1.0_f32), theme::line()));
            p.text(pr.center(), Align2::CENTER_CENTER, "Autre panneau", FontId::proportional(13.0), theme::fg());
        });
    }

    fn ui_settings_inner(&mut self, ui: &mut egui::Ui, today: NaiveDate) {
        section(ui, "RÉGLAGES");
        let mut changed = false;
        center_block(ui, "settings", |ui| {
            egui::Grid::new("settings").num_columns(2).spacing([24.0, 12.0]).show(ui, |ui| {
                let s = &mut self.data.settings;
                ui.label("Exercices par séance");
                changed |= ui.add(egui::DragValue::new(&mut s.draws_per_day).range(1..=6)).changed();
                ui.end_row();
                ui.label("Relances par jour");
                changed |= ui.add(egui::DragValue::new(&mut s.max_rerolls).range(0..=10)).changed();
                ui.end_row();
            });
        });
        if changed {
            self.save();
        }

        ui.add_space(28.0);
        self.ui_planning(ui, today);

        ui.add_space(28.0);
        self.ui_calendar_reset(ui);
        ui.add_space(28.0);
        self.ui_dev_theme(ui);
        ui.add_space(24.0);
        section(ui, "TEST");
        center_block(ui, "test_row", |ui| {
            egui::Grid::new("test_row").show(ui, |ui| {
                if ui.button("− 1 jour").clicked() {
                    self.day_offset -= 1;
                }
                if ui.button("+ 1 jour").clicked() {
                    self.day_offset += 1;
                }
                if ui.button("Date réelle").clicked() {
                    self.day_offset = 0;
                }
                ui.label(RichText::new(format!("décalage : {} j", self.day_offset)).color(theme::dim()));
            });
        });

        ui.add_space(24.0);
        ui.label(RichText::new(format!("Données : {}", Data::path().display())).color(theme::dim()).size(11.0));
    }
}



impl RouletteApp {
    fn ui_planning(&mut self, ui: &mut egui::Ui, today: NaiveDate) {
        section(ui, "PLANIFICATION");
        let mut changed = false;
        changed |= ui
            .checkbox(&mut self.data.settings.weekly_mode, RichText::new("Modèle par semaine").color(theme::fg()))
            .on_hover_text("Choisir jour par jour les séances et les repos d'une semaine type")
            .changed();
        ui.add_space(10.0);

        if self.data.settings.weekly_mode {
            ui.label(RichText::new("Clique sur un jour pour alterner séance / repos. La semaine se répète.").color(theme::dim()));
            ui.add_space(8.0);
            let gap = 6.0;
            let w = ((ui.available_width() - 6.0 * gap) / 7.0).min(96.0);
            center_block(ui, "week_tpl", |ui| {
                egui::Grid::new("week_tpl").spacing([gap, 0.0]).show(ui, |ui| {
                    for (i, name) in ["LUN", "MAR", "MER", "JEU", "VEN", "SAM", "DIM"].iter().enumerate() {
                        let on = self.data.settings.week_template[i];
                        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 78.0), egui::Sense::click());
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        let hov = resp.hovered();
                        let p = ui.painter();
                        let (fill, stroke, fg) = if on {
                            (theme::gray(if hov { 58 } else { 44 }), theme::fg(), theme::fg())
                        } else {
                            (if hov { theme::gray(30) } else { theme::surface() }, theme::line(), theme::dim())
                        };
                        p.rect(rect, Rounding::same(6.0), fill, Stroke::new(if on { 1.5_f32 } else { 1.0_f32 }, stroke));
                        p.text(rect.center_top() + Vec2::new(0.0, 14.0), Align2::CENTER_CENTER, *name, FontId::proportional(12.0), theme::dim());
                        p.text(rect.center() + Vec2::new(0.0, 4.0), Align2::CENTER_CENTER, if on { "SÉANCE" } else { "REPOS" }, FontId::proportional(13.0), fg);
                        paint_mark(p, rect.center_bottom() - Vec2::new(0.0, 14.0), if on { Mark::Drawn } else { Mark::Rest });
                        if resp.clicked() {
                            self.data.settings.week_template[i] = !on;
                            changed = true;
                        }
                    }
                });
            });
            ui.add_space(8.0);
            let n = self.data.settings.week_template.iter().filter(|x| **x).count();
            let txt = format!("{n} séance(s) · {} jour(s) de repos par semaine", 7 - n);
            ui.label(RichText::new(txt).color(if n == 0 { theme::todo_text() } else { theme::dim() }));
        } else {
            center_block(ui, "planning", |ui| {
                egui::Grid::new("planning").num_columns(2).spacing([24.0, 12.0]).show(ui, |ui| {
                    let s = &mut self.data.settings;
                    ui.label("Jours de séance d'affilée");
                    changed |= ui.add(egui::DragValue::new(&mut s.train_days).range(1..=7).suffix(" j")).changed();
                    ui.end_row();
                    ui.label("Puis jours de repos");
                    changed |= ui.add(egui::DragValue::new(&mut s.rest_days).range(0..=7).suffix(" j")).changed();
                    ui.end_row();
                });
            });
            ui.add_space(6.0);
            let s = &self.data.settings;
            let txt = if s.rest_days == 0 {
                "Aucun jour de repos dans le cycle (weekends sans séance).".to_string()
            } else {
                format!(
                    "Cycle : {} séance(s) puis {} repos (weekends sans séance), à partir du {}.",
                    s.train_days,
                    s.rest_days,
                    fr_date(self.data.anchor)
                )
            };
            ui.label(RichText::new(txt).color(theme::dim()));
        }
        if changed {
            self.save();
        }
        ui.add_space(14.0);
        ui.label(RichText::new("Aperçu des 14 prochains jours").color(theme::dim()).size(12.0));
        ui.add_space(4.0);

        // Aperçu des 14 prochains jours
        center_block(ui, "preview14", |ui| {
            egui::Grid::new("preview14").spacing([4.0, 4.0]).show(ui, |ui| {
                for i in 0..14 {
                    if i == 7 {
                        ui.end_row();
                    }
                    let d = today + Duration::days(i);
                    let kind = self.data.day_kind(d);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(40.0, 46.0), egui::Sense::hover());
                    let p = ui.painter();
                    let (fill, fg) = match kind {
                        DayKind::Training => (theme::gray(40), theme::fg()),
                        DayKind::Rest => (theme::surface(), theme::dim()),
                        DayKind::Weekend => (theme::bg(), theme::line()),
                    };
                    p.rect(rect, Rounding::same(5.0), fill, Stroke::new(if i == 0 { 1.5_f32 } else { 1.0_f32 }, if i == 0 { theme::fg() } else { theme::line() }));
                    p.text(rect.center_top() + Vec2::new(0.0, 12.0), Align2::CENTER_CENTER, &fr_weekday(d)[..2], FontId::proportional(11.0), theme::dim());
                    p.text(rect.center() + Vec2::new(0.0, 2.0), Align2::CENTER_CENTER, d.day().to_string(), theme::numbers(14.0), fg);
                    let tag = match kind {
                        DayKind::Training => "SÉANCE",
                        DayKind::Rest => "REPOS",
                        DayKind::Weekend => "—",
                    };
                    p.text(rect.center_bottom() - Vec2::new(0.0, 9.0), Align2::CENTER_CENTER, tag, FontId::proportional(8.0), fg);
                }
            });
        });
        ui.add_space(8.0);
        if !self.data.settings.weekly_mode && ui.button("Démarrer le cycle aujourd'hui").on_hover_text("Aujourd'hui devient le 1er jour de séance du cycle").clicked() {
            self.data.anchor = today;
            self.save();
        }
    }

}

// ---------- widgets ----------

/// Centre horizontalement un bloc (grille, rangée…) en réutilisant sa largeur
/// mesurée à l'image précédente.
fn center_block<R>(ui: &mut egui::Ui, id_salt: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let id = ui.id().with(("center_block", id_salt));
    let w: f32 = ui.data(|d| d.get_temp(id)).unwrap_or(0.0);
    let avail = ui.available_width();
    let pad = ((avail - w) / 2.0).max(0.0);
    let r = ui.horizontal_top(|ui| {
        ui.add_space(pad);
        ui.scope(add)
    });
    let measured = r.inner.response.rect.width();
    if (measured - w).abs() > 0.5 {
        ui.data_mut(|d| d.insert_temp(id, measured));
        ui.ctx().request_repaint();
    }
    r.inner.inner
}

/// Colonne de largeur max `max_w`, centrée dans la page ; son contenu est centré.
fn centered_column<R>(ui: &mut egui::Ui, max_w: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let avail = ui.available_width();
    let w = avail.min(max_w);
    let pad = ((avail - w) / 2.0).max(0.0);
    ui.horizontal_top(|ui| {
        let sp = ui.spacing().item_spacing.x;
        ui.add_space((pad - sp).max(0.0));
        ui.allocate_ui_with_layout(Vec2::new(w, 0.0), Layout::top_down(Align::Center), |ui| {
            ui.set_width(w);
            add(ui)
        })
        .inner
    })
    .inner
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).size(12.0).strong().color(theme::dim()));
    ui.add_space(8.0);
}

#[derive(Clone, Copy)]
enum Mark {
    Todo,
    Drawn,
    Complete,
    Rest,
    Missed,
}

/// Marqueurs dessinés à la main (la police par défaut n'a pas tous les glyphes).
fn paint_mark(p: &egui::Painter, c: Pos2, mark: Mark) {
    match mark {
        Mark::Todo => {
            p.circle_stroke(c, 4.5, Stroke::new(1.2_f32, theme::dim()));
        }
        Mark::Drawn => {
            p.circle_filled(c, 5.0, theme::fg());
        }
        Mark::Complete => {
            p.circle_filled(c, 5.0, theme::done_stroke());
        }
        Mark::Rest => {
            p.hline((c.x - 5.0)..=(c.x + 5.0), c.y, Stroke::new(1.5_f32, theme::dim()));
        }
        Mark::Missed => {
            let s = Stroke::new(1.5_f32, theme::dim());
            p.line_segment([c + Vec2::new(-4.0, -4.0), c + Vec2::new(4.0, 4.0)], s);
            p.line_segment([c + Vec2::new(-4.0, 4.0), c + Vec2::new(4.0, -4.0)], s);
        }
    }
}

/// Phrase du grand titre selon l'avancement du tirage et des exercices faits.
fn progress_phrase(data: &Data, today: NaiveDate) -> String {
    let Some(r) = data.record(today) else { return "Pas encore commencé ?".into() };
    let total = r.exercises.len();
    let left = total - r.done.iter().filter(|d| **d).count().min(total);
    match left {
        0 => "Ah bah enfin".into(),
        1 => "Plus qu'un, on voit le bout".into(),
        n if n == total => "Il s'agirait de s'y mettre".into(),
        n => format!("Encore {n}, qu'est-ce que tu peux être long"),
    }
}

/// Icône d'engrenage (8 dents) ; `turn` 0..1 fait tourner l'engrenage d'un huitième de tour.
fn paint_gear(p: &egui::Painter, c: Pos2, scale: f32, ink: Color32, hole: Color32, turn: f32) {
    let teeth = 8;
    let rot = turn * std::f32::consts::TAU / 8.0;
    for k in 0..teeth {
        let a = rot + k as f32 * std::f32::consts::TAU / teeth as f32;
        let (s, co) = a.sin_cos();
        let dir = Vec2::new(co, s);
        let perp = Vec2::new(-s, co);
        let (r0, r1, w) = (6.0 * scale, 9.5 * scale, 2.1 * scale);
        let pts = vec![
            c + dir * r0 - perp * w,
            c + dir * r1 - perp * (w * 0.75),
            c + dir * r1 + perp * (w * 0.75),
            c + dir * r0 + perp * w,
        ];
        p.add(egui::Shape::convex_polygon(pts, ink, Stroke::NONE));
    }
    p.circle_filled(c, 7.0 * scale, ink);
    p.circle_filled(c, 3.0 * scale, hole);
}

fn cal_style(status: DayStatus) -> (Color32, Color32, Color32) {
    match status {
        DayStatus::Complete => (theme::done_fill(), theme::done_stroke(), theme::on(theme::done_fill())),
        DayStatus::Missed => (theme::todo_fill(), theme::todo_stroke(), theme::todo_text()),
        DayStatus::Rest => (theme::surface(), theme::gray(70), theme::dim()),
        DayStatus::Today | DayStatus::Planned => (theme::gray(34), theme::gray(60), theme::fg()),
        DayStatus::Off | DayStatus::Before => (theme::bg(), theme::gray(30), theme::gray(70)),
    }
}

fn add_months(d: NaiveDate, n: i32) -> NaiveDate {
    let total = d.year() * 12 + d.month0() as i32 + n;
    NaiveDate::from_ymd_opt(total.div_euclid(12), total.rem_euclid(12) as u32 + 1, 1).unwrap()
}

/// Bouton "mois précédent / suivant" (triangle dessiné).
fn arrow_side_button(ui: &mut egui::Ui, left: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(26.0, 22.0), egui::Sense::click());
    let p = ui.painter();
    p.rect(rect, Rounding::same(4.0), if resp.hovered() { theme::gray(44) } else { theme::gray(28) }, Stroke::new(1.0_f32, theme::line()));
    let c = rect.center();
    let (w, h) = (3.5, 5.0);
    let pts = if left {
        vec![Pos2::new(c.x + w, c.y - h), Pos2::new(c.x + w, c.y + h), Pos2::new(c.x - w, c.y)]
    } else {
        vec![Pos2::new(c.x - w, c.y - h), Pos2::new(c.x - w, c.y + h), Pos2::new(c.x + w, c.y)]
    };
    p.add(egui::Shape::convex_polygon(pts, theme::fg(), Stroke::NONE));
    resp.clicked()
}

/// Grand bouton du bas, même DA que les onglets de l'en-tête : capsule, texte en capitales,
/// fond plein (comme l'onglet actif) quand il est utilisable.
fn wide_button(ui: &mut egui::Ui, width: f32, text: &str, enabled: bool) -> bool {
    let sense = if enabled { egui::Sense::click() } else { egui::Sense::hover() };
    let (slot, resp) = ui.allocate_exact_size(Vec2::new(width, 56.0), sense);
    let resp = resp.on_hover_cursor(if enabled { egui::CursorIcon::PointingHand } else { egui::CursorIcon::Default });
    paint_pill_button(ui.painter(), slot, text, enabled, resp.hovered());
    enabled && resp.clicked()
}

/// Capsule façon onglet : active = fond texte / texte fond ; inactive = fond case + liseré.
fn paint_pill_button(p: &egui::Painter, slot: Rect, text: &str, enabled: bool, hovered: bool) {
    let s = theme::header_h() / HEADER_REF;
    let h = (46.0 * s).clamp(36.0, slot.height());
    let rect = Rect::from_center_size(slot.center(), Vec2::new(slot.width(), h));
    let r = h / 2.0;
    let font = FontId::proportional((15.0 * s).clamp(12.0, 20.0));
    if enabled {
        let fill = if hovered { theme::fg().gamma_multiply(0.82) } else { theme::fg() };
        theme::panel(p, rect, r, fill);
        p.text(rect.center(), Align2::CENTER_CENTER, text, font, theme::bg());
    } else {
        theme::panel(p, rect, r, theme::surface());
        p.rect_stroke(rect, Rounding::same(r), Stroke::new(theme::border(1.0_f32), theme::line()));
        p.text(rect.center(), Align2::CENTER_CENTER, text, font, theme::dim());
    }
}

/// Bouton "fait" pleine largeur de la roue. Gris-rouge -> vert ; au passage
/// à "fait" il vibre et sa bordure s'épaissit comme une roue qui s'arrête.
/// Renvoie true si cliqué.
/// Dessin d'un bouton « À FAIRE » / « FAIT » (mix : 0 = à faire, 1 = fait).
fn paint_done(p: &egui::Painter, r: Rect, is_done: bool, active: bool, mix: f32, impact: f32, hovered: bool) {
    let ts = theme::todo_style();
    if !active {
        theme::todo_panel(p, r, theme::surface(), ts.border, theme::line());
        p.text(r.center(), Align2::CENTER_CENTER, "FAIT", FontId::proportional(ts.text_size * 15.0 / 17.0), theme::line());
        return;
    }
    let lerp = |a: Color32, b: Color32| {
        let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * mix) as u8;
        Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
    };
    let mut fill = lerp(theme::todo_fill(), theme::done_fill());
    if hovered {
        fill = fill.gamma_multiply(1.15);
    }
    let width = if is_done { ts.done_border * (1.0 + impact) } else { ts.border };
    theme::todo_panel(p, r, fill, width, lerp(theme::todo_stroke(), theme::done_stroke()));

    let text_col = lerp(theme::todo_text(), theme::on(theme::done_fill()));
    let label = if is_done { "FAIT" } else { "À FAIRE" };
    let galley = p.layout_no_wrap(label.to_string(), FontId::proportional(ts.text_size), text_col);
    let k = ts.text_size / 17.0;
    let check_w = if is_done { 22.0 * k } else { 0.0 };
    let total = galley.size().x + check_w;
    let x0 = r.center().x - total / 2.0;
    if is_done {
        // coche dessinée à la main
        let c = Pos2::new(x0 + 7.0 * k, r.center().y);
        let s = Stroke::new(2.5_f32 * k, text_col);
        p.line_segment([c + Vec2::new(-6.0, 0.0) * k, c + Vec2::new(-1.5, 5.0) * k], s);
        p.line_segment([c + Vec2::new(-1.5, 5.0) * k, c + Vec2::new(8.0, -6.0) * k], s);
    }
    p.galley(Pos2::new(x0 + check_w, r.center().y - galley.size().y / 2.0), galley, text_col);
}

fn done_button(ui: &mut egui::Ui, height: f32, is_done: bool, active: bool, locked: bool, anim: Option<(Instant, bool)>) -> bool {
    let clickable = active && !locked;
    let sense = if clickable { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), sense);
    let resp = resp.on_hover_cursor(if clickable { egui::CursorIcon::PointingHand } else { egui::CursorIcon::Default });

    let since = anim.map_or(f32::MAX, |(t0, _)| t0.elapsed().as_secs_f32());
    // Transition de couleur (0 = à faire, 1 = fait)
    let mix = {
        let x = anim::progress(since, 0.3);
        if is_done { x } else { 1.0 - x }
    };
    let mix = if anim.is_none() { if is_done { 1.0 } else { 0.0 } } else { mix };
    // Vibration + épaississement uniquement quand on coche
    let impact = match anim {
        Some((_, true)) if is_done && since < SHAKE_TIME => 1.0 - since / SHAKE_TIME,
        _ => 0.0,
    };
    let dx = if impact > 0.0 { (since * 75.0).sin() * 7.0 * impact * impact } else { 0.0 };
    let r = rect.translate(Vec2::new(dx, 0.0));

    paint_done(ui.painter(), r, is_done, active, mix, impact, clickable && resp.hovered());
    clickable && resp.clicked()
}

fn progress_bar(ui: &mut egui::Ui, frac: f32) {
    let width = ui.available_width().min(520.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 6.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, Rounding::same(3.0), theme::surface());
    let mut fill = rect;
    fill.set_width(rect.width() * frac.clamp(0.0, 1.0));
    p.rect_filled(fill, Rounding::same(3.0), theme::fg());
}

/// Applique palette + polices + style egui.
/// Réglages d'un liseré lumineux (utilisé pour les roulettes et les autres panneaux).
fn rim_controls(ui: &mut egui::Ui, id: &str, g: &mut theme::GlassRim) -> bool {
    let mut changed = false;
    center_block(ui, id, |ui| {
        egui::Grid::new(id).num_columns(2).spacing([18.0, 10.0]).show(ui, |ui| {
            ui.spacing_mut().slider_width = 220.0;
            let on = g.enabled;
            let pct = |v: f64, _| format!("{:.0} %", v * 100.0);
            ui.label("Activer");
            changed |= ui.checkbox(&mut g.enabled, "").changed();
            ui.end_row();
            ui.label("Intensité");
            changed |= ui.add_enabled(on, egui::Slider::new(&mut g.intensity, 0.0..=1.0).custom_formatter(pct)).changed();
            ui.end_row();
            ui.label("Direction de la lumière");
            changed |= ui.add_enabled(on, egui::Slider::new(&mut g.angle, 0.0..=360.0).step_by(5.0).custom_formatter(|v, _| format!("{v:.0}°"))).changed();
            ui.end_row();
            ui.label("Largeur du liseré");
            changed |= ui.add_enabled(on, egui::Slider::new(&mut g.width, 0.5..=6.0).step_by(0.25).custom_formatter(|v, _| format!("{v:.2} px"))).changed();
            ui.end_row();
            ui.label("Halo intérieur");
            changed |= ui.add_enabled(on, egui::Slider::new(&mut g.halo, 0.0..=30.0).step_by(0.5).custom_formatter(|v, _| format!("{v:.1} px"))).changed();
            ui.end_row();
            ui.label("Reflet opposé");
            changed |= ui.add_enabled(on, egui::Slider::new(&mut g.opposite, 0.0..=1.0).custom_formatter(pct)).changed();
            ui.end_row();
            ui.label("Concentration");
            changed |= ui.add_enabled(on, egui::Slider::new(&mut g.focus, 0.5..=6.0).step_by(0.1).custom_formatter(|v, _| format!("{v:.1}"))).changed();
            ui.end_row();
            ui.label("Couleur de la lumière");
            ui.add_enabled_ui(on, |ui| changed |= ui.color_edit_button_srgb(&mut g.color).changed());
            ui.end_row();
        });
    });
    changed
}

fn apply_theme(ctx: &egui::Context, data: &Data) {
    theme::set_palette(data.settings.palette);
    theme::set_background(data.settings.background);
    theme::set_metrics(data.settings.metrics);
    theme::set_glass(data.settings.glass);
    theme::set_reel_style(data.settings.reel_style);
    theme::set_todo_style(data.settings.todo_style);
    theme::apply_fonts(ctx, &data.settings.fonts);
    apply_style(ctx);
}

fn apply_style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = theme::bg();
    v.window_fill = theme::surface();
    v.extreme_bg_color = theme::gray(8);
    v.faint_bg_color = theme::surface();
    v.selection.bg_fill = theme::fg();
    v.selection.stroke = Stroke::new(1.0_f32, theme::bg());
    v.hyperlink_color = theme::fg();

    let w = &mut v.widgets;
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, theme::fg());
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, theme::line());
    for (state, bg, fg) in [
        (&mut w.inactive, theme::gray(28), theme::gray(200)),
        (&mut w.hovered, theme::gray(44), theme::fg()),
        (&mut w.active, theme::gray(64), theme::fg()),
    ] {
        state.bg_fill = bg;
        state.weak_bg_fill = bg;
        state.fg_stroke = Stroke::new(1.0_f32, fg);
        state.bg_stroke = Stroke::new(1.0_f32, theme::line());
        state.rounding = Rounding::same(5.0);
    }
    w.open = w.active;
    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(10.0, 6.0);
        s.spacing.button_padding = Vec2::new(12.0, 6.0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wizard_defaults_and_flow() {
        let mut wz = LogWizard::new(0, None);
        assert_eq!(wz.sets + 1, 3);
        assert!(wz.next().is_none()); // -> reps
        assert_eq!(wz.reps, vec![DEFAULT_REPS; 3]);
        wz.reps[1] = 7; // 8 reps
        assert!(wz.next().is_none()); // -> poids
        wz.weights[2] = 125; // 62,5 kg
        let Some(WizardResult::Done(log)) = wz.next() else { panic!("pas validé") };
        assert_eq!(log.sets.iter().map(|s| s.reps).collect::<Vec<_>>(), vec![10, 8, 10]);
        assert_eq!(log.sets[2].weight, Some(62.5));
        assert_eq!(log.sets[0].weight, Some(20.0));
    }

    #[test]
    fn title_follows_progress() {
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let mut d = Data { anchor: mon, history: vec![], ..Data::default() };
        assert_eq!(progress_phrase(&d, mon), "Pas encore commencé ?");
        d.draw(mon).unwrap();
        assert_eq!(progress_phrase(&d, mon), "Il s'agirait de s'y mettre");
        d.set_done(mon, 0, true);
        assert_eq!(progress_phrase(&d, mon), "Encore 2, qu'est-ce que tu peux être long");
        d.set_done(mon, 1, true);
        assert_eq!(progress_phrase(&d, mon), "Plus qu'un, on voit le bout");
        d.set_done(mon, 2, true);
        assert_eq!(progress_phrase(&d, mon), "Ah bah enfin");
    }

    #[test]
    fn wizard_back_and_bodyweight() {
        let last = ExerciseLog { sets: vec![SetLog { reps: 15, weight: None }, SetLog { reps: 12, weight: None }], bodyweight: true };
        let mut wz = LogWizard::new(1, Some(&last));
        assert_eq!(wz.sets + 1, 2);
        assert!(matches!(wz.back(), Some(WizardResult::Cancel)));
        wz.next();
        wz.next();
        assert_eq!(wz.step, LogStep::Weight);
        wz.back();
        assert_eq!(wz.step, LogStep::Reps);
        wz.next();
        let Some(WizardResult::Done(log)) = wz.next() else { panic!() };
        assert!(log.bodyweight && log.sets.iter().all(|s| s.weight.is_none()));
        assert_eq!(log.sets[0].reps, 15);
        assert_eq!(fmt_weight(125), "62,5");
    }
}
