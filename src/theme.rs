//! Thème de l'app : palette de couleurs et polices réglables (outils de développement,
//! dans Réglages › Développement).
//!
//! Toutes les couleurs passent par ce module. Les gris intermédiaires de l'interface
//! sont recalculés à partir de la palette via [`gray`], pour qu'un changement de teinte
//! se répercute partout de façon cohérente.

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::RwLock;

pub type Rgb = [u8; 3];

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(default)]
pub struct Palette {
    /// Fond de la page.
    pub bg: Rgb,
    /// Fond des cartes / cases.
    pub surface: Rgb,
    /// Traits et bordures.
    pub line: Rgb,
    /// Texte secondaire.
    pub dim: Rgb,
    /// Texte principal / accents clairs.
    pub fg: Rgb,
    /// "À faire" / loupé : fond, bordure, texte.
    pub todo_fill: Rgb,
    pub todo_stroke: Rgb,
    pub todo_text: Rgb,
    /// "Fait" / accompli : fond, bordure.
    pub done_fill: Rgb,
    pub done_stroke: Rgb,
}

impl Default for Palette {
    /// Palette par défaut : « Perso ».
    fn default() -> Self {
        Self::perso()
    }
}

impl Palette {
    /// Palette « Perso » (sombre, enregistrée le 20/10/2026).
    pub fn perso() -> Self {
        Self {
            bg: [12, 12, 12],
            surface: [22, 22, 22],
            line: [48, 48, 48],
            dim: [120, 120, 120],
            fg: [235, 235, 235],
            todo_fill: [16, 16, 16],
            todo_stroke: [41, 41, 41],
            todo_text: [205, 205, 205],
            done_fill: [201, 201, 201],
            done_stroke: [159, 159, 159],
        }
    }

    /// Ancienne palette « Perso » claire (19/10/2026).
    pub fn perso_clair() -> Self {
        Self {
            bg: [238, 236, 232],
            surface: [213, 213, 213],
            line: [101, 98, 94],
            dim: [120, 116, 110],
            fg: [24, 24, 24],
            todo_fill: [240, 214, 214],
            todo_stroke: [206, 150, 150],
            todo_text: [140, 64, 64],
            done_fill: [44, 150, 86],
            done_stroke: [26, 110, 60],
        }
    }

    /// Palette monochrome sombre d'origine.
    pub fn monochrome() -> Self {
        Self {
            bg: [12, 12, 12],
            surface: [22, 22, 22],
            line: [48, 48, 48],
            dim: [120, 120, 120],
            fg: [235, 235, 235],
            todo_fill: [70, 48, 50],
            todo_stroke: [112, 74, 77],
            todo_text: [196, 160, 162],
            done_fill: [34, 128, 72],
            done_stroke: [96, 214, 136],
        }
    }
}

impl Palette {
    /// Rôles éditables (nom affiché, accès).
    pub fn roles_mut(&mut self) -> [(&'static str, &mut Rgb); 10] {
        [
            ("Fond de page", &mut self.bg),
            ("Fond des cases", &mut self.surface),
            ("Bordures", &mut self.line),
            ("Texte secondaire", &mut self.dim),
            ("Texte principal", &mut self.fg),
            ("À faire · fond", &mut self.todo_fill),
            ("À faire · bordure", &mut self.todo_stroke),
            ("À faire · texte", &mut self.todo_text),
            ("Fait · fond", &mut self.done_fill),
            ("Fait · bordure", &mut self.done_stroke),
        ]
    }

    pub fn presets() -> Vec<(&'static str, Palette)> {
        let d = Palette::monochrome();
        vec![
            ("Perso", Palette::perso()),
            ("Perso clair", Palette::perso_clair()),
            ("Monochrome", d),
            (
                "Ardoise",
                Palette { bg: [14, 17, 23], surface: [24, 29, 38], line: [52, 61, 76], dim: [124, 136, 156], fg: [228, 234, 244], ..d },
            ),
            (
                "Sable",
                Palette {
                    bg: [24, 20, 16],
                    surface: [36, 30, 24],
                    line: [70, 60, 48],
                    dim: [150, 134, 112],
                    fg: [240, 228, 208],
                    todo_fill: [88, 52, 40],
                    todo_stroke: [140, 86, 64],
                    todo_text: [222, 176, 150],
                    done_fill: [96, 120, 52],
                    done_stroke: [170, 206, 104],
                },
            ),
            (
                "Nuit violette",
                Palette {
                    bg: [16, 12, 24],
                    surface: [27, 21, 40],
                    line: [60, 48, 86],
                    dim: [140, 126, 170],
                    fg: [236, 228, 252],
                    done_fill: [64, 92, 170],
                    done_stroke: [130, 166, 255],
                    ..d
                },
            ),
            (
                "Clair",
                Palette {
                    bg: [238, 236, 232],
                    surface: [252, 251, 249],
                    line: [206, 202, 196],
                    dim: [120, 116, 110],
                    fg: [24, 24, 24],
                    todo_fill: [240, 214, 214],
                    todo_stroke: [206, 150, 150],
                    todo_text: [140, 64, 64],
                    done_fill: [44, 150, 86],
                    done_stroke: [26, 110, 60],
                },
            ),
        ]
    }
}

/// Sens du dégradé de fond.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum GradientDir {
    Vertical,
    Horizontal,
    Diagonal,
    Radial,
}

impl GradientDir {
    pub const ALL: [GradientDir; 4] = [GradientDir::Vertical, GradientDir::Horizontal, GradientDir::Diagonal, GradientDir::Radial];
    pub fn label(self) -> &'static str {
        match self {
            GradientDir::Vertical => "Vertical (haut → bas)",
            GradientDir::Horizontal => "Horizontal (gauche → droite)",
            GradientDir::Diagonal => "Diagonale",
            GradientDir::Radial => "Radial (depuis le centre)",
        }
    }
}

/// Dégradé du fond de l'app : du fond de page vers `color`.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct BgGradient {
    pub enabled: bool,
    pub color: Rgb,
    pub direction: GradientDir,
    /// 0 = pas de dégradé, 1 = on atteint complètement `color`.
    pub strength: f32,
}

impl Default for BgGradient {
    fn default() -> Self {
        Self { enabled: false, color: [43, 0, 46], direction: GradientDir::Radial, strength: 0.71 }
    }
}

/// Liseré lumineux façon « liquid glass » sur le bord des panneaux et des roulettes.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct GlassRim {
    pub enabled: bool,
    /// Force de la lumière (0..1).
    pub intensity: f32,
    /// Direction d'où vient la lumière, en degrés (0 = droite, 90 = haut).
    pub angle: f32,
    /// Largeur du liseré, en points.
    pub width: f32,
    /// Halo qui se diffuse vers l'intérieur, en points.
    pub halo: f32,
    /// Reflet sur le bord opposé (0..1).
    pub opposite: f32,
    /// Concentration de la lumière (1 = large, 6 = très localisée).
    pub focus: f32,
    /// Couleur de la lumière.
    pub color: Rgb,
}

impl Default for GlassRim {
    fn default() -> Self {
        Self { enabled: true, intensity: 0.6, angle: 120.0, width: 1.5, halo: 8.0, opposite: 0.45, focus: 2.0, color: [255, 255, 255] }
    }
}

impl GlassRim {
    /// Réglage par défaut du liseré des « autres panneaux » (très léger).
    pub fn panels() -> Self {
        Self { intensity: 0.08, ..Self::default() }
    }

    /// Liseré coupé (réglages gardés).
    pub fn off() -> Self {
        Self { enabled: false, ..Self::default() }
    }
}

static GLASS: RwLock<Option<GlassRim>> = RwLock::new(None);

pub fn set_glass(g: GlassRim) {
    *GLASS.write().unwrap() = Some(g);
}

fn glass() -> GlassRim {
    GLASS.read().unwrap().unwrap_or_else(GlassRim::panels)
}

/// Points d'un rectangle arrondi (nombre fixe de points par coin) et normale sortante de chacun.
fn rounded_ring(rect: egui::Rect, r: f32) -> Vec<(egui::Pos2, egui::Vec2)> {
    const STEPS: usize = 10;
    let r = r.clamp(0.0, rect.width().min(rect.height()) / 2.0);
    // Coins dans le sens horaire (écran) : haut-droit, bas-droit, bas-gauche, haut-gauche.
    let corners = [
        (egui::pos2(rect.right() - r, rect.top() + r), -90.0_f32),
        (egui::pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (egui::pos2(rect.left() + r, rect.bottom() - r), 90.0),
        (egui::pos2(rect.left() + r, rect.top() + r), 180.0),
    ];
    let mut out = Vec::with_capacity(4 * (STEPS + 1));
    for (c, a0) in corners {
        for i in 0..=STEPS {
            let a = (a0 + 90.0 * i as f32 / STEPS as f32).to_radians();
            let n = egui::vec2(a.cos(), a.sin());
            out.push((c + n * r, n));
        }
    }
    out
}

/// Maillage du liseré lumineux, posé juste à l'intérieur de la bordure.
pub fn rim_shape(rect: egui::Rect, r: f32) -> Option<egui::Shape> {
    rim_shape_with(rect, r, &glass())
}

/// Liseré lumineux avec des réglages donnés.
pub fn rim_shape_with(rect: egui::Rect, r: f32, g: &GlassRim) -> Option<egui::Shape> {
    let g = *g;
    if !g.enabled || g.intensity <= 0.0 || rect.width() < 4.0 || rect.height() < 4.0 {
        return None;
    }
    let a = g.angle.to_radians();
    let light = egui::vec2(a.cos(), -a.sin());
    let inset0 = border(1.0) * 0.5;
    let insets = [inset0, inset0 + g.width.max(0.25), inset0 + g.width.max(0.25) + g.halo.max(0.0)];
    let ring_alpha = [1.0_f32, 0.55, 0.0];
    let rings: Vec<Vec<(egui::Pos2, egui::Vec2)>> =
        insets.iter().map(|d| rounded_ring(rect.shrink(d.min(rect.width().min(rect.height()) / 2.0 - 0.5)), (r - d).max(0.0))).collect();
    let n = rings[0].len();
    let mut mesh = egui::Mesh::default();
    for (ring, ra) in rings.iter().zip(ring_alpha) {
        for &(p, nrm) in ring {
            let d = nrm.dot(light);
            let f = d.max(0.0).powf(g.focus.max(0.3)) + g.opposite * (-d).max(0.0).powf(g.focus.max(0.3));
            let al = (g.intensity * f * ra).clamp(0.0, 1.0);
            let [cr, cg, cb] = g.color;
            let c = Color32::from_rgba_premultiplied((cr as f32 * al) as u8, (cg as f32 * al) as u8, (cb as f32 * al) as u8, (255.0 * al * 0.85) as u8);
            mesh.colored_vertex(p, c);
        }
    }
    for k in 0..2u32 {
        let (o, i) = (k * n as u32, (k + 1) * n as u32);
        for j in 0..n as u32 {
            let j2 = (j + 1) % n as u32;
            mesh.add_triangle(o + j, o + j2, i + j);
            mesh.add_triangle(i + j, o + j2, i + j2);
        }
    }
    Some(egui::Shape::mesh(mesh))
}

/// Style avancé des panneaux des roulettes (indépendant des autres panneaux).
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct ReelStyle {
    /// Liseré lumineux propre aux roulettes.
    pub rim: GlassRim,
    /// Rayon des coins, en points.
    pub radius: f32,
    /// Épaisseur de la bordure pendant le tirage, en points.
    pub border: f32,
    /// Épaisseur de la bordure une fois la roulette arrêtée.
    pub landed_border: f32,
    /// Teinte du verre et dosage de la teinte (0..1).
    pub tint: Rgb,
    pub tint_mix: f32,
    /// Opacité du fond (0..1).
    pub opacity: f32,
    /// Dégradé vertical (-1..1) : positif = haut plus clair, bas plus sombre.
    pub gradient: f32,
    /// Reflet brillant en haut (0..1) et hauteur du reflet (part de la case).
    pub gloss: f32,
    pub gloss_height: f32,
    /// Ombre de profondeur en haut et en bas, façon tambour (0..1), et sa taille.
    pub depth: f32,
    pub depth_size: f32,
    /// Opacité des lignes guides autour du nom central (0..1).
    pub guides: f32,
    /// Interrupteurs des effets (décochés = aplat uni).
    pub use_tint: bool,
    pub use_gradient: bool,
    pub use_gloss: bool,
    pub use_depth: bool,
}

impl Default for ReelStyle {
    fn default() -> Self {
        Self {
            rim: GlassRim::off(),
            radius: 0.0,
            border: 1.25,
            landed_border: 0.0,
            tint: [255, 255, 255],
            tint_mix: 0.0,
            opacity: 1.0,
            gradient: 0.2,
            gloss: 0.15,
            gloss_height: 0.3,
            depth: 0.25,
            depth_size: 0.22,
            guides: 1.0,
            use_tint: true,
            use_gradient: false,
            use_gloss: false,
            use_depth: true,
        }
    }
}

static REEL_STYLE: RwLock<Option<ReelStyle>> = RwLock::new(None);

pub fn set_reel_style(s: ReelStyle) {
    *REEL_STYLE.write().unwrap() = Some(s);
}

pub fn reel_style() -> ReelStyle {
    REEL_STYLE.read().unwrap().unwrap_or_default()
}

/// Bande horizontale d'un rectangle arrondi : maillage dont la couleur ne dépend que de la hauteur.
fn rounded_rows(rect: egui::Rect, r: f32, color_at: impl Fn(f32) -> Color32) -> egui::Shape {
    let r = r.clamp(0.0, rect.width().min(rect.height()) / 2.0);
    let mut ys: Vec<f32> = (0..=48).map(|i| rect.top() + rect.height() * i as f32 / 48.0).collect();
    for i in 0..=8 {
        let t = r * i as f32 / 8.0;
        ys.push(rect.top() + t);
        ys.push(rect.bottom() - t);
    }
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ys.dedup_by(|a, b| (*a - *b).abs() < 0.05);
    let inset = |y: f32| -> f32 {
        let dy = if y < rect.top() + r {
            rect.top() + r - y
        } else if y > rect.bottom() - r {
            y - (rect.bottom() - r)
        } else {
            return 0.0;
        };
        r - (r * r - dy.min(r) * dy.min(r)).max(0.0).sqrt()
    };
    let mut m = egui::Mesh::default();
    for &y in &ys {
        let c = color_at(((y - rect.top()) / rect.height()).clamp(0.0, 1.0));
        let d = inset(y);
        m.colored_vertex(egui::pos2(rect.left() + d, y), c);
        m.colored_vertex(egui::pos2(rect.right() - d, y), c);
    }
    for k in 0..ys.len().saturating_sub(1) as u32 {
        let (a, b) = (2 * k, 2 * k + 2);
        m.add_triangle(a, a + 1, b);
        m.add_triangle(a + 1, b + 1, b);
    }
    egui::Shape::mesh(m)
}

fn premul(c: [f32; 3], a: f32) -> Color32 {
    let a = a.clamp(0.0, 1.0);
    let f = |v: f32| (v.clamp(0.0, 1.0) * a * 255.0).round() as u8;
    Color32::from_rgba_premultiplied(f(c[0]), f(c[1]), f(c[2]), (a * 255.0).round() as u8)
}

/// Fond d'une roulette (sous les noms) : teinte, opacité, dégradé vertical.
pub fn reel_base(p: &egui::Painter, rect: egui::Rect) {
    let s = reel_style();
    let base = surface();
    let t = rgb(s.tint);
    let k = if s.use_tint { s.tint_mix.clamp(0.0, 1.0) } else { 0.0 };
    let lerp = |a: u8, b: u8| (a as f32 * (1.0 - k) + b as f32 * k) / 255.0;
    let c = [lerp(base.r(), t.r()), lerp(base.g(), t.g()), lerp(base.b(), t.b())];
    let grad = if s.use_gradient { s.gradient.clamp(-1.0, 1.0) } else { 0.0 };
    p.add(rounded_rows(rect, s.radius, |v| {
        // v = 0 en haut, 1 en bas ; +g éclaircit le haut et assombrit le bas.
        let shift = grad * (0.5 - v) * 0.35;
        let col = c.map(|x| if shift >= 0.0 { x + (1.0 - x) * shift } else { x * (1.0 + shift) });
        premul(col, s.opacity)
    }));
}

/// Dessus d'une roulette (par-dessus les noms) : ombre de profondeur, reflet, bordure, liseré.
pub fn reel_overlay(p: &egui::Painter, rect: egui::Rect, border_w: f32, border_color: Color32) {
    let s = reel_style();
    if s.use_depth && s.depth > 0.0 {
        let ds = s.depth_size.clamp(0.02, 0.5);
        p.add(rounded_rows(rect, s.radius, |v| {
            let e = (1.0 - v.min(1.0 - v) / ds).max(0.0);
            premul([0.0; 3], s.depth.clamp(0.0, 1.0) * 0.85 * e * e)
        }));
    }
    if s.use_gloss && s.gloss > 0.0 {
        let gh = s.gloss_height.clamp(0.05, 1.0);
        p.add(rounded_rows(rect, s.radius, |v| {
            let e = (1.0 - v / gh).max(0.0);
            premul([1.0; 3], s.gloss.clamp(0.0, 1.0) * 0.45 * e * e)
        }));
    }
    if border_w > 0.0 {
        p.rect_stroke(rect, egui::Rounding::same(s.radius), egui::Stroke::new(border(border_w), border_color));
    }
    p.extend(rim_shape_with(rect, s.radius, &s.rim));
}

/// Forme des boutons « À FAIRE » / « FAIT » sous les roulettes.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct TodoStyle {
    /// Hauteur, en part du côté de la roulette (0,15..0,6).
    pub height: f32,
    /// Écart avec la roulette, en points.
    pub gap: f32,
    /// Rayon des coins.
    pub radius: f32,
    /// Bordure « à faire » et « fait », en points.
    pub border: f32,
    pub done_border: f32,
    /// Taille du texte.
    pub text_size: f32,
    /// Opacité du fond (0..1).
    pub opacity: f32,
    /// Dégradé vertical (-1..1).
    pub gradient: f32,
    /// Reflet brillant (0..1) et hauteur du reflet.
    pub gloss: f32,
    pub gloss_height: f32,
    /// Liseré lumineux propre à ces boutons.
    pub rim: GlassRim,
    /// Interrupteurs des effets (décochés = aplat uni).
    pub use_gradient: bool,
    pub use_gloss: bool,
}

impl Default for TodoStyle {
    fn default() -> Self {
        Self {
            height: 1.0 / 3.0,
            gap: 8.0,
            radius: 8.0,
            border: 1.0,
            done_border: 2.5,
            text_size: 17.0,
            opacity: 1.0,
            gradient: 0.0,
            gloss: 0.0,
            gloss_height: 0.5,
            rim: GlassRim::off(),
            use_gradient: false,
            use_gloss: false,
        }
    }
}

static TODO_STYLE: RwLock<Option<TodoStyle>> = RwLock::new(None);

pub fn set_todo_style(s: TodoStyle) {
    *TODO_STYLE.write().unwrap() = Some(s);
}

pub fn todo_style() -> TodoStyle {
    TODO_STYLE.read().unwrap().unwrap_or_default()
}

/// Bouton « À FAIRE » / « FAIT » : fond en dégradé, reflet, bordure, liseré.
pub fn todo_panel(p: &egui::Painter, rect: egui::Rect, fill: Color32, border_w: f32, border_color: Color32) {
    let s = todo_style();
    let c = [fill.r() as f32 / 255.0, fill.g() as f32 / 255.0, fill.b() as f32 / 255.0];
    let grad = if s.use_gradient { s.gradient.clamp(-1.0, 1.0) } else { 0.0 };
    p.add(rounded_rows(rect, s.radius, |v| {
        let shift = grad * (0.5 - v) * 0.35;
        let col = c.map(|x| if shift >= 0.0 { x + (1.0 - x) * shift } else { x * (1.0 + shift) });
        premul(col, s.opacity)
    }));
    if s.use_gloss && s.gloss > 0.0 {
        let gh = s.gloss_height.clamp(0.05, 1.0);
        p.add(rounded_rows(rect, s.radius, |v| {
            let e = (1.0 - v / gh).max(0.0);
            premul([1.0; 3], s.gloss.clamp(0.0, 1.0) * 0.45 * e * e)
        }));
    }
    if border_w > 0.0 {
        p.rect_stroke(rect, egui::Rounding::same(s.radius), egui::Stroke::new(border(border_w), border_color));
    }
    p.extend(rim_shape_with(rect, s.radius, &s.rim));
}

/// Fond d'un panneau : aplat plein + liseré lumineux réglable.
pub fn panel_shapes(rect: egui::Rect, r: f32, tint: Color32) -> Vec<egui::Shape> {
    let mut v = vec![egui::Shape::rect_filled(rect, r, tint)];
    v.extend(rim_shape(rect, r));
    v
}

/// Panneau dessiné tout de suite.
pub fn panel(p: &egui::Painter, rect: egui::Rect, r: f32, tint: Color32) {
    p.extend(panel_shapes(rect, r, tint));
}

/// Dimensions réglables de l'interface (outils de développement).
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct Metrics {
    /// Hauteur de l'en-tête (logo, onglets, réglages), en points.
    pub header_h: f32,
    /// Épaisseur des bordures des panneaux, en points (pour un trait de base de 1).
    pub border: f32,
    /// Onglets et réglages en colonne verticale à gauche (au lieu de l'en-tête en haut).
    pub vertical_nav: bool,
    /// (Ancien réglage, ignoré : la colonne s'ajuste à la taille des boutons.)
    pub nav_width: f32
}

impl Default for Metrics {
    fn default() -> Self {
        Self { header_h: 89.0, border: 0.75, vertical_nav: false, nav_width: 250.0 }
    }
}

static METRICS: RwLock<Option<Metrics>> = RwLock::new(None);

pub fn set_metrics(m: Metrics) {
    *METRICS.write().unwrap() = Some(m);
}

fn metrics() -> Metrics {
    METRICS.read().unwrap().unwrap_or_default()
}

/// Hauteur de l'en-tête.
pub fn header_h() -> f32 {
    metrics().header_h
}

/// Navigation verticale à gauche ?
pub fn vertical_nav() -> bool {
    metrics().vertical_nav
}

/// Largeur de la colonne de navigation (0 si navigation en haut).
pub fn nav_left() -> f32 {
    let m = metrics();
    if m.vertical_nav {
        // Colonne fine : boutons de 34 pt de large (à l'échelle) + marges.
        48.0 * nav_scale() + 40.0
    } else {
        0.0
    }
}

/// Espace à laisser en haut de la page (sous l'en-tête, ou presque rien en mode vertical).
pub fn content_top() -> f32 {
    if vertical_nav() {
        18.0
    } else {
        header_h() + 10.0
    }
}

/// Échelle des boutons de navigation (suit la hauteur de l'en-tête réglée).
pub fn nav_scale() -> f32 {
    header_h() / 74.0
}

/// Épaisseur d'une bordure de panneau (`base` = épaisseur d'origine).
pub fn border(base: f32) -> f32 {
    base * metrics().border
}

static PALETTE: RwLock<Option<Palette>> = RwLock::new(None);
static BACKGROUND: RwLock<Option<BgGradient>> = RwLock::new(None);

pub fn set_background(b: BgGradient) {
    *BACKGROUND.write().unwrap() = Some(b);
}

fn background() -> BgGradient {
    BACKGROUND.read().unwrap().unwrap_or_default()
}

/// Couleur du fond au point (u, v) de la fenêtre (0..1, 0..1).
pub fn bg_at(u: f32, v: f32) -> Color32 {
    let g = background();
    if !g.enabled || g.strength <= 0.0 {
        return bg();
    }
    let t = match g.direction {
        GradientDir::Vertical => v,
        GradientDir::Horizontal => u,
        GradientDir::Diagonal => (u + v) / 2.0,
        GradientDir::Radial => 1.0 - (((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() / 0.7071).min(1.0),
    };
    mix(bg(), rgb(g.color), t.clamp(0.0, 1.0) * g.strength.clamp(0.0, 1.0))
}

/// Peint le fond (uni ou dégradé) sur `rect`.
pub fn paint_background(p: &egui::Painter, rect: egui::Rect) {
    let g = background();
    if !g.enabled || g.strength <= 0.0 {
        p.rect_filled(rect, 0.0, bg());
        return;
    }
    let n = 24u32;
    let mut m = egui::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let (u, v) = (i as f32 / n as f32, j as f32 / n as f32);
            let pos = egui::pos2(rect.left() + u * rect.width(), rect.top() + v * rect.height());
            m.colored_vertex(pos, bg_at(u, v));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            let b = a + 1;
            let c = a + n + 1;
            let d = c + 1;
            m.add_triangle(a, b, d);
            m.add_triangle(a, d, c);
        }
    }
    p.add(egui::Shape::mesh(m));
}

pub fn set_palette(p: Palette) {
    *PALETTE.write().unwrap() = Some(p);
}

fn pal() -> Palette {
    PALETTE.read().unwrap().unwrap_or_default()
}

pub fn rgb(c: Rgb) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

pub fn bg() -> Color32 {
    rgb(pal().bg)
}
pub fn surface() -> Color32 {
    rgb(pal().surface)
}
pub fn line() -> Color32 {
    rgb(pal().line)
}
pub fn dim() -> Color32 {
    rgb(pal().dim)
}
pub fn fg() -> Color32 {
    rgb(pal().fg)
}
pub fn todo_fill() -> Color32 {
    rgb(pal().todo_fill)
}
pub fn todo_stroke() -> Color32 {
    rgb(pal().todo_stroke)
}
pub fn todo_text() -> Color32 {
    rgb(pal().todo_text)
}
pub fn done_fill() -> Color32 {
    rgb(pal().done_fill)
}
pub fn done_stroke() -> Color32 {
    rgb(pal().done_stroke)
}

/// Couleur de texte lisible sur `fill` : sombre sur fond clair, blanc sinon.
pub fn on(fill: Color32) -> Color32 {
    let lum = (0.2126 * fill.r() as f32 + 0.7152 * fill.g() as f32 + 0.0722 * fill.b() as f32) / 255.0;
    if lum > 0.6 {
        Color32::from_gray(18)
    } else {
        Color32::WHITE
    }
}

/// Gris "n" de l'interface d'origine (0 = noir, 12 = fond, 235 = texte),
/// recalculé sur la palette courante : en monochrome c'est exactement le même gris.
pub fn gray(n: u8) -> Color32 {
    let p = pal();
    let (b, f) = (rgb(p.bg), rgb(p.fg));
    let n = n as f32;
    if n <= 12.0 {
        mix(Color32::BLACK, b, n / 12.0)
    } else if n <= 235.0 {
        mix(b, f, (n - 12.0) / (235.0 - 12.0))
    } else {
        mix(f, Color32::WHITE, (n - 235.0) / 20.0)
    }
}

// ---------------------------------------------------------------- polices

/// Choix de police par catégorie de texte (noms affichés de [`available_fonts`]).
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(default)]
pub struct FontChoice {
    /// Grands titres, message de fin, "BRAVO".
    pub title: String,
    /// Texte courant, boutons, onglets.
    pub body: String,
    /// Chiffres mis en avant (roues de nombres, stats, calendrier).
    pub numbers: String,
    /// Noms d'exercices sur les roulettes.
    pub reel: String,
}

pub const DEFAULT_FONT: &str = "Ubuntu Light (défaut)";

impl Default for FontChoice {
    fn default() -> Self {
        // Polices Windows ; si absentes, retour automatique à la police intégrée.
        Self {
            title: "MADE TOMMY Black_PERSONAL USE (perso)".into(),
            body: "MADE TOMMY Light_PERSONAL USE (perso)".into(),
            numbers: "MADE TOMMY Regular_PERSONAL USE (perso)".into(),
            reel: "MADE TOMMY ExtraBold_PERSONAL USE (perso)".into(),
        }
    }
}

impl FontChoice {
    pub fn categories_mut(&mut self) -> [(&'static str, &'static str, &mut String); 4] {
        [
            ("Titres", "JOUR DE SPORT", &mut self.title),
            ("Texte courant", "Combien de séries effectuées ?", &mut self.body),
            ("Chiffres", "12  82,5  45", &mut self.numbers),
            ("Roulettes", "Mountain climbers", &mut self.reel),
        ]
    }
}

pub const FAMILY_TITLE: &str = "titre";
pub const FAMILY_NUMBERS: &str = "chiffres";
pub const FAMILY_REEL: &str = "roulette";

pub fn title(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FAMILY_TITLE.into()))
}
pub fn numbers(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FAMILY_NUMBERS.into()))
}
pub fn reel(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FAMILY_REEL.into()))
}

/// Polices proposées : celles intégrées à egui + celles trouvées sur le système.
/// Dossier principal des polices perso (créé à la demande) : %APPDATA%\\RouletteSport\\polices.
pub fn user_font_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("RouletteSport").join("polices")
}

/// Tous les dossiers « polices » lus : celui des données, le dossier courant,
/// et à côté de l'exécutable (ou du projet quand on lance depuis target\\release).
pub fn custom_font_dirs() -> Vec<PathBuf> {
    let mut dirs_v = vec![user_font_dir(), PathBuf::from("polices")];
    if let Ok(exe) = std::env::current_exe() {
        for anc in exe.ancestors().skip(1).take(3) {
            dirs_v.push(anc.join("polices"));
        }
    }
    let mut seen = Vec::new();
    dirs_v.retain(|d| {
        let c = d.canonicalize().unwrap_or_else(|_| d.clone());
        if seen.contains(&c) {
            false
        } else {
            seen.push(c);
            true
        }
    });
    dirs_v
}

pub fn available_fonts() -> Vec<(String, Option<PathBuf>)> {
    let mut out = vec![(DEFAULT_FONT.to_string(), None), ("Hack (mono)".to_string(), None)];
    let windows = std::env::var("WINDIR").map(|w| PathBuf::from(w).join("Fonts")).unwrap_or_else(|_| PathBuf::from("C:\\Windows\\Fonts"));
    let candidates: &[(&str, &str)] = &[
        ("Segoe UI", "segoeui.ttf"),
        ("Segoe UI Light", "segoeuil.ttf"),
        ("Segoe UI Semibold", "seguisb.ttf"),
        ("Segoe UI Bold", "segoeuib.ttf"),
        ("Segoe UI Black", "seguibl.ttf"),
        ("Bahnschrift", "bahnschrift.ttf"),
        ("Arial", "arial.ttf"),
        ("Arial Bold", "arialbd.ttf"),
        ("Arial Black", "ariblk.ttf"),
        ("Calibri", "calibri.ttf"),
        ("Calibri Bold", "calibrib.ttf"),
        ("Candara", "candara.ttf"),
        ("Corbel", "corbel.ttf"),
        ("Franklin Gothic", "framd.ttf"),
        ("Impact", "impact.ttf"),
        ("Trebuchet MS", "trebuc.ttf"),
        ("Verdana", "verdana.ttf"),
        ("Tahoma", "tahoma.ttf"),
        ("Georgia", "georgia.ttf"),
        ("Cambria", "cambria.ttc"),
        ("Consolas", "consola.ttf"),
        ("Cascadia Code", "CascadiaCode.ttf"),
        ("Courier New", "cour.ttf"),
    ];
    for (name, file) in candidates {
        let p = windows.join(file);
        if p.exists() {
            out.push((name.to_string(), Some(p)));
        }
    }
    // Polices perso : fichiers .ttf / .otf / .ttc déposés dans un dossier « polices ».
    for dir in custom_font_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        files.sort();
        for p in files {
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            if !matches!(ext.as_str(), "ttf" | "otf" | "ttc") {
                continue;
            }
            let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else { continue };
            let name = format!("{stem} (perso)");
            if !out.iter().any(|(n, _)| *n == name) {
                out.push((name, Some(p)));
            }
        }
    }
    // Hors Windows (développement / tests)
    let linux: &[(&str, &str)] = &[
        ("DejaVu Sans", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
        ("DejaVu Sans Bold", "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"),
        ("DejaVu Sans Condensed Bold", "/usr/share/fonts/truetype/dejavu/DejaVuSansCondensed-Bold.ttf"),
        ("DejaVu Serif", "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf"),
        ("DejaVu Sans Mono", "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"),
        ("Carlito", "/usr/share/fonts/truetype/crosextra/Carlito-Regular.ttf"),
        ("Carlito Bold", "/usr/share/fonts/truetype/crosextra/Carlito-Bold.ttf"),
    ];
    for (name, path) in linux {
        let p = PathBuf::from(path);
        if p.exists() {
            out.push((name.to_string(), Some(p)));
        }
    }
    out
}

/// Clé interne de la police dans egui pour un nom affiché.
fn font_key(name: &str, defs: &mut FontDefinitions, fonts: &[(String, Option<PathBuf>)]) -> Option<String> {
    match name {
        DEFAULT_FONT => Some("Ubuntu-Light".into()),
        "Hack (mono)" => Some("Hack".into()),
        _ => {
            let (_, path) = fonts.iter().find(|(n, _)| n == name)?;
            let key = format!("sys:{name}");
            if !defs.font_data.contains_key(&key) {
                let bytes = std::fs::read(path.as_ref()?).ok()?;
                defs.font_data.insert(key.clone(), FontData::from_owned(bytes));
            }
            Some(key)
        }
    }
}

/// Construit les familles de polices et les applique à egui.
pub fn apply_fonts(ctx: &egui::Context, choice: &FontChoice) {
    let mut defs = FontDefinitions::default();
    let fonts = available_fonts();
    // Polices de secours d'egui (emoji, symboles) gardées en fin de chaque famille.
    let fallback: Vec<String> = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mut family = |name: &str| -> Vec<String> {
        let mut v = Vec::new();
        if let Some(k) = font_key(name, &mut defs, &fonts) {
            v.push(k);
        }
        for f in &fallback {
            if !v.contains(f) {
                v.push(f.clone());
            }
        }
        v
    };
    let body = family(&choice.body);
    let title = family(&choice.title);
    let numbers = family(&choice.numbers);
    let reel = family(&choice.reel);
    defs.families.insert(FontFamily::Proportional, body);
    defs.families.insert(FontFamily::Name(FAMILY_TITLE.into()), title);
    defs.families.insert(FontFamily::Name(FAMILY_NUMBERS.into()), numbers);
    defs.families.insert(FontFamily::Name(FAMILY_REEL.into()), reel);
    ctx.set_fonts(defs);
}

/// Texte "épais" : la police dessinée plusieurs fois avec un léger décalage.
pub fn bold_text(p: &egui::Painter, pos: egui::Pos2, align: egui::Align2, text: &str, font: FontId, color: Color32, weight: f32) {
    let offsets = [(-1.0, 0.0), (1.0, 0.0), (0.0, -0.6), (0.0, 0.6), (0.0, 0.0)];
    for (dx, dy) in offsets {
        p.text(pos + egui::vec2(dx * weight, dy * weight), align, text, font.clone(), color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gray_ramp_matches_original_in_monochrome() {
        set_palette(Palette::monochrome());
        for n in [12u8, 22, 30, 48, 120, 235] {
            let g = gray(n);
            assert!((g.r() as i32 - n as i32).abs() <= 1, "gray({n}) = {g:?}");
        }
        assert_eq!(bg(), Color32::from_gray(12));
    }

    #[test]
    fn background_gradient() {
        set_palette(Palette::monochrome());
        set_background(BgGradient { enabled: true, color: [200, 0, 0], direction: GradientDir::Vertical, strength: 1.0 });
        assert_eq!(bg_at(0.3, 0.0), bg());
        assert_eq!(bg_at(0.3, 1.0), Color32::from_rgb(200, 0, 0));
        set_background(BgGradient { direction: GradientDir::Radial, ..background() });
        assert_eq!(bg_at(0.5, 0.5), Color32::from_rgb(200, 0, 0));
        set_background(BgGradient::default());
        assert_eq!(bg_at(0.5, 0.5), bg());
    }

    #[test]
    fn presets_are_distinct() {
        let p = Palette::presets();
        assert!(p.len() >= 4);
        assert!(p.windows(2).all(|w| w[0].1 != w[1].1));
    }
}
