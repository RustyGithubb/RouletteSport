//! Données et règles métier : tirage, relances, repos, reset hebdomadaire.

use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Nombre d'exercices tirés par séance.
    pub draws_per_day: usize,
    /// Nombre de relances autorisées par jour.
    pub max_rerolls: u32,
    /// Cycle : N jours de séance...
    pub train_days: u32,
    /// ...puis M jours de repos (0 = pas de repos).
    pub rest_days: u32,
    /// Samedi et dimanche sans séance en mode cycle.
    pub weekend_off: bool,
    /// Mode "modèle par semaine" : les jours de séance sont fixés par jour de la semaine.
    pub weekly_mode: bool,
    /// Lundi..dimanche : true = séance, false = repos.
    pub week_template: [bool; 7],
    /// Outils de développement : palette de couleurs et polices.
    pub palette: crate::theme::Palette,
    pub fonts: crate::theme::FontChoice,
    pub background: crate::theme::BgGradient,
    pub metrics: crate::theme::Metrics,
    /// Filtre VHS animé sur toute la page.
    pub vhs: crate::vhs::VhsFilter,
    /// Filtre bitmap (gros pixels, tramage, palette réduite).
    pub bitmap: crate::bitmap::BitmapFilter,
    /// Liseré lumineux « liquid glass » des panneaux et roulettes.
    pub glass: crate::theme::GlassRim,
    /// Style avancé des panneaux des roulettes.
    pub reel_style: crate::theme::ReelStyle,
    /// Forme des boutons « à faire » / « fait ».
    pub todo_style: crate::theme::TodoStyle,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            draws_per_day: 3,
            max_rerolls: 10,
            train_days: 3,
            rest_days: 2,
            weekend_off: false,
            weekly_mode: true,
            week_template: [true, true, true, true, true, false, false],
            palette: Default::default(),
            fonts: Default::default(),
            background: Default::default(),
            metrics: Default::default(),
            vhs: Default::default(),
            bitmap: Default::default(),
            glass: crate::theme::GlassRim::panels(),
            reel_style: Default::default(),
            todo_style: Default::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DayRecord {
    pub date: NaiveDate,
    pub exercises: Vec<String>,
    #[serde(default)]
    pub done: Vec<bool>,
    #[serde(default)]
    pub rerolls_used: u32,
    /// Détail saisi pour chaque exercice validé (séries, reps, poids).
    #[serde(default)]
    pub logs: Vec<Option<ExerciseLog>>,
    /// Journée archivée par un reset du calendrier : ignorée par le calendrier,
    /// la série et la semaine en cours, mais conservée pour la base d'exercices.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub archived: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct SetLog {
    pub reps: u32,
    /// Charge en kg ; None si poids du corps.
    pub weight: Option<f32>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ExerciseLog {
    pub sets: Vec<SetLog>,
    #[serde(default)]
    pub bodyweight: bool,
}

/// Stats calculées d'un exercice (lecture seule, galerie).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExerciseStats {
    pub reps_max: Option<u32>,
    pub weight_max: Option<f32>,
    pub bodyweight_only: bool,
    pub total_reps: u32,
    pub total_sets: u32,
    pub sessions: u32,
    pub last: Option<(NaiveDate, ExerciseLog)>,
    /// Séances les plus récentes d'abord.
    pub recent: Vec<(NaiveDate, ExerciseLog)>,
    /// Moyenne de répétitions par série (toutes séances).
    pub avg_reps: Option<f32>,
    /// Moyenne de reps par série de chaque séance, de la plus ancienne à la plus récente (12 dernières).
    pub avg_history: Vec<(NaiveDate, f32)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DayStatus {
    Complete,
    Missed,
    Rest,
    Off,
    Today,
    Planned,
    Before,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DayKind {
    Training,
    Rest,
    Weekend,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Data {
    pub settings: Settings,
    /// Liste des exercices à faire tomber chaque semaine.
    pub pool: Vec<String>,
    /// Point de départ du cycle de repos.
    pub anchor: NaiveDate,
    pub history: Vec<DayRecord>,
    /// Colonnes de la base d'exercices.
    pub columns: Vec<Column>,
    /// Valeurs de la base : exercice -> (colonne -> valeur).
    pub stats: BTreeMap<String, BTreeMap<String, String>>,
    /// Début du calendrier après un reset : les jours d'avant sont neutres
    /// (ni accomplis ni loupés) et ne comptent plus dans la série.
    /// Les séances restent enregistrées pour la base d'exercices.
    pub calendar_start: Option<NaiveDate>,
    /// Purge unique des anciennes journées « avant le reset » (v0.16).
    #[serde(default)]
    pub archives_purged: bool,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum ColumnKind {
    Number,
    Text,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Column {
    pub name: String,
    pub kind: ColumnKind,
}

pub fn default_columns() -> Vec<Column> {
    vec![
        Column { name: "Poids max (kg)".into(), kind: ColumnKind::Number },
        Column { name: "Reps max".into(), kind: ColumnKind::Number },
        Column { name: "Notes".into(), kind: ColumnKind::Text },
    ]
}

/// Lit un nombre saisi à la française ("82,5") ou non. Vide/invalide -> None.
pub fn parse_number(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".").replace(' ', "");
    if t.is_empty() {
        return None;
    }
    t.parse().ok()
}

impl Default for Data {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            pool: default_pool(),
            anchor: Local::now().date_naive(),
            history: Vec::new(),
            columns: default_columns(),
            stats: BTreeMap::new(),
            calendar_start: None,
            archives_purged: true,
        }
    }
}

pub fn default_pool() -> Vec<String> {
    [
        "Pompes", "Squats", "Fentes", "Gainage", "Burpees", "Crunchs",
        "Mountain climbers", "Dips", "Jumping jacks", "Superman",
        "Pont fessier", "Chaise",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

pub fn is_weekend(d: NaiveDate) -> bool {
    matches!(d.weekday(), Weekday::Sat | Weekday::Sun)
}

pub fn week_start(d: NaiveDate) -> NaiveDate {
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

impl Data {
    pub fn path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("RouletteSport")
            .join("data.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        let mut data = match std::fs::read_to_string(&path) {
            Ok(txt) => serde_json::from_str(&txt).unwrap_or_else(|_| {
                // Fichier illisible : on le met de côté plutôt que de l'écraser.
                let _ = std::fs::rename(&path, path.with_extension("json.bak"));
                Self::default()
            }),
            Err(_) => Self::default(),
        };
        if !data.archives_purged {
            // Sauvegarde avant d'effacer les anciennes journées archivées.
            let _ = std::fs::copy(&path, path.with_file_name("data.avant-purge.json"));
            data.purge_archives();
            let _ = data.save();
        }
        data
    }

    /// Efface définitivement les journées archivées par des resets du calendrier.
    pub fn purge_archives(&mut self) {
        self.history.retain(|r| !r.archived);
        self.archives_purged = true;
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(tmp, path)
    }

    fn is_off(&self, d: NaiveDate) -> bool {
        self.settings.weekend_off && is_weekend(d)
    }

    pub fn day_kind(&self, d: NaiveDate) -> DayKind {
        if self.settings.weekly_mode {
            let wd = d.weekday().num_days_from_monday() as usize;
            return if self.settings.week_template[wd] { DayKind::Training } else { DayKind::Rest };
        }
        if self.is_off(d) {
            return DayKind::Weekend;
        }
        let train = self.settings.train_days.max(1) as i64;
        let rest = self.settings.rest_days as i64;
        if rest == 0 || d < self.anchor {
            return DayKind::Training;
        }
        // Position dans le cycle, en ne comptant que les jours "planifiables".
        let mut n = 0;
        let mut day = self.anchor;
        while day < d {
            if !self.is_off(day) {
                n += 1;
            }
            day += Duration::days(1);
        }
        if n % (train + rest) < train {
            DayKind::Training
        } else {
            DayKind::Rest
        }
    }

    /// Statut d'un jour pour le calendrier.
    /// Remet le calendrier, la série et la semaine en cours à zéro.
    /// Toutes les journées existantes sont archivées (y compris aujourd'hui et
    /// d'éventuels jours simulés dans le futur) ; leurs séances restent dans
    /// la base d'exercices.
    pub fn reset_calendar(&mut self, today: NaiveDate) {
        for r in &mut self.history {
            r.archived = true;
        }
        self.calendar_start = Some(today);
    }

    /// Journées prises en compte par le calendrier (non archivées).
    pub fn active_days(&self) -> impl Iterator<Item = &DayRecord> {
        self.history.iter().filter(|r| !r.archived)
    }

    pub fn day_status(&self, d: NaiveDate, today: NaiveDate) -> DayStatus {
        let mut start = self.active_days().map(|r| r.date).min().map_or(self.anchor, |f| f.min(self.anchor));
        if let Some(cs) = self.calendar_start {
            if d < cs {
                return match self.day_kind(d) {
                    DayKind::Weekend => DayStatus::Off,
                    _ => DayStatus::Before,
                };
            }
            start = start.max(cs);
        }
        if self.is_complete(d) {
            return DayStatus::Complete;
        }
        match self.day_kind(d) {
            DayKind::Weekend => DayStatus::Off,
            DayKind::Rest => DayStatus::Rest,
            DayKind::Training if d < start => DayStatus::Before,
            DayKind::Training if d < today => DayStatus::Missed,
            DayKind::Training if d == today => DayStatus::Today,
            DayKind::Training => DayStatus::Planned,
        }
    }

    pub fn record(&self, d: NaiveDate) -> Option<&DayRecord> {
        self.active_days().find(|r| r.date == d)
    }

    fn record_mut(&mut self, d: NaiveDate) -> Option<&mut DayRecord> {
        self.history.iter_mut().find(|r| r.date == d && !r.archived)
    }

    /// Exercices déjà tirés cette semaine (hors `except` si fourni).
    pub fn drawn_this_week(&self, d: NaiveDate, except: Option<NaiveDate>) -> Vec<String> {
        let ws = week_start(d);
        self.active_days()
            .filter(|r| week_start(r.date) == ws && Some(r.date) != except)
            .flat_map(|r| r.exercises.iter().cloned())
            .collect()
    }

    /// Exercices de la liste pas encore tombés cette semaine.
    pub fn remaining(&self, d: NaiveDate) -> Vec<String> {
        let drawn = self.drawn_this_week(d, None);
        self.pool.iter().filter(|e| !drawn.contains(e)).cloned().collect()
    }

    /// Jours de sport restants avant le weekend sans tirage encore effectué.
    pub fn open_training_days(&self, d: NaiveDate) -> usize {
        let mut n = 0;
        let mut day = d;
        let end = week_start(d) + Duration::days(7);
        while day < end {
            if self.day_kind(day) == DayKind::Training && self.record(day).is_none() {
                n += 1;
            }
            day += Duration::days(1);
        }
        n
    }

    /// Tirage : priorité aux exos pas encore tombés cette semaine,
    /// puis ceux à éviter (tirage précédent du jour), puis le reste.
    fn pick(&self, d: NaiveDate, avoid: &[String]) -> Vec<String> {
        let mut rng = rand::thread_rng();
        let prior = self.drawn_this_week(d, Some(d));
        let (mut fresh, mut avoided, mut done) = (Vec::new(), Vec::new(), Vec::new());
        for e in &self.pool {
            if prior.contains(e) {
                done.push(e.clone());
            } else if avoid.contains(e) {
                avoided.push(e.clone());
            } else {
                fresh.push(e.clone());
            }
        }
        fresh.shuffle(&mut rng);
        avoided.shuffle(&mut rng);
        done.shuffle(&mut rng);
        fresh
            .into_iter()
            .chain(avoided)
            .chain(done)
            .take(self.settings.draws_per_day)
            .collect()
    }

    pub fn draw(&mut self, d: NaiveDate) -> Result<(), &'static str> {
        if self.day_kind(d) != DayKind::Training {
            return Err("Pas de séance aujourd'hui.");
        }
        if self.record(d).is_some() {
            return Err("Déjà tiré aujourd'hui.");
        }
        if self.pool.is_empty() {
            return Err("La liste d'exercices est vide.");
        }
        let exercises = self.pick(d, &[]);
        let done = vec![false; exercises.len()];
        self.history.push(DayRecord { date: d, exercises, done, rerolls_used: 0, logs: Vec::new(), archived: false });
        self.history.sort_by_key(|r| r.date);
        Ok(())
    }

    pub fn reroll(&mut self, d: NaiveDate) -> Result<(), &'static str> {
        let max = self.settings.max_rerolls;
        let current = match self.record(d) {
            None => return Err("Rien à relancer."),
            Some(r) if r.rerolls_used >= max => return Err("Plus de relance disponible."),
            Some(r) => r.exercises.clone(),
        };
        let exercises = self.pick(d, &current);
        let r = self.record_mut(d).unwrap();
        r.done = vec![false; exercises.len()];
        r.logs.clear();
        r.exercises = exercises;
        r.rerolls_used += 1;
        Ok(())
    }

    /// Ajoute un exercice (ignore vide et doublon). Renvoie true si ajouté.
    pub fn add_exercise(&mut self, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() || self.pool.iter().any(|e| e.eq_ignore_ascii_case(name)) {
            return false;
        }
        self.pool.push(name.to_string());
        true
    }

    /// Renomme un exercice, y compris dans les tirages de la semaine en cours
    /// pour que la progression hebdo reste juste.
    pub fn rename_exercise(&mut self, index: usize, new_name: &str, today: NaiveDate) -> Result<(), &'static str> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err("Nom vide.");
        }
        let old = self.pool.get(index).cloned().ok_or("Exercice introuvable.")?;
        if old == new_name {
            return Ok(());
        }
        if self.pool.iter().enumerate().any(|(i, e)| i != index && e.eq_ignore_ascii_case(new_name)) {
            return Err("Cet exercice existe déjà.");
        }
        let ws = week_start(today);
        for r in self.history.iter_mut().filter(|r| week_start(r.date) == ws) {
            for e in r.exercises.iter_mut().filter(|e| **e == old) {
                *e = new_name.to_string();
            }
        }
        if let Some(v) = self.stats.remove(&old) {
            self.stats.insert(new_name.to_string(), v);
        }
        self.pool[index] = new_name.to_string();
        Ok(())
    }

    // ---------- base d'exercices ----------

    pub fn stat(&self, exercise: &str, column: &str) -> &str {
        self.stats.get(exercise).and_then(|m| m.get(column)).map_or("", |s| s.as_str())
    }

    #[allow(dead_code)] // ancienne vue tableau, conservée pour les données existantes
    pub fn set_stat(&mut self, exercise: &str, column: &str, value: String) {
        let row = self.stats.entry(exercise.to_string()).or_default();
        if value.trim().is_empty() {
            row.remove(column);
        } else {
            row.insert(column.to_string(), value);
        }
    }

    #[allow(dead_code)] // ancienne vue tableau, conservée pour les données existantes
    pub fn add_column(&mut self, name: &str, kind: ColumnKind) -> Result<(), &'static str> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Nom de colonne vide.");
        }
        if self.columns.iter().any(|c| c.name.eq_ignore_ascii_case(name)) {
            return Err("Cette colonne existe déjà.");
        }
        self.columns.push(Column { name: name.to_string(), kind });
        Ok(())
    }

    #[allow(dead_code)] // ancienne vue tableau, conservée pour les données existantes
    pub fn rename_column(&mut self, index: usize, new_name: &str) -> Result<(), &'static str> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err("Nom de colonne vide.");
        }
        if self.columns.iter().enumerate().any(|(i, c)| i != index && c.name.eq_ignore_ascii_case(new_name)) {
            return Err("Cette colonne existe déjà.");
        }
        let old = self.columns.get(index).ok_or("Colonne introuvable.")?.name.clone();
        for row in self.stats.values_mut() {
            if let Some(v) = row.remove(&old) {
                row.insert(new_name.to_string(), v);
            }
        }
        self.columns[index].name = new_name.to_string();
        Ok(())
    }

    #[allow(dead_code)] // ancienne vue tableau, conservée pour les données existantes
    pub fn remove_column(&mut self, index: usize) {
        if index < self.columns.len() {
            let name = self.columns.remove(index).name;
            for row in self.stats.values_mut() {
                row.remove(&name);
            }
        }
    }

    /// Dernière date où l'exercice est tombé à la roulette.
    #[allow(dead_code)] // ancienne vue tableau, conservée pour les données existantes
    pub fn last_drawn(&self, exercise: &str) -> Option<NaiveDate> {
        self.history.iter().filter(|r| r.exercises.iter().any(|e| e == exercise)).map(|r| r.date).max()
    }

    /// Nombre de fois où l'exercice a été fait (coché "fait").
    #[allow(dead_code)] // ancienne vue tableau, conservée pour les données existantes
    pub fn times_done(&self, exercise: &str) -> usize {
        self.history
            .iter()
            .map(|r| r.exercises.iter().zip(&r.done).filter(|(e, d)| *e == exercise && **d).count())
            .sum()
    }

    /// Séance du jour `d` tirée et entièrement cochée.
    pub fn is_complete(&self, d: NaiveDate) -> bool {
        self.record(d)
            .is_some_and(|r| !r.exercises.is_empty() && r.done.len() >= r.exercises.len() && r.done.iter().all(|x| *x))
    }

    /// Nombre de séances complétées d'affilée jusqu'à `today`.
    /// Repos et weekends ne cassent pas la série ; aujourd'hui pas encore fini non plus.
    pub fn streak(&self, today: NaiveDate) -> u32 {
        let Some(mut first) = self.active_days().map(|r| r.date).min() else { return 0 };
        if let Some(cs) = self.calendar_start {
            first = first.max(cs);
        }
        let mut n = 0;
        let mut d = today;
        while d >= first {
            if self.day_kind(d) == DayKind::Training {
                if self.is_complete(d) {
                    n += 1;
                } else if d != today {
                    break;
                }
            }
            d -= Duration::days(1);
        }
        n
    }

    pub fn set_done(&mut self, d: NaiveDate, i: usize, value: bool) {
        if let Some(r) = self.record_mut(d) {
            r.done.resize(r.exercises.len(), false);
            r.logs.resize(r.exercises.len(), None);
            if let Some(v) = r.done.get_mut(i) {
                *v = value;
            }
            if !value {
                r.logs[i] = None;
            }
        }
    }

    /// Valide l'exercice `i` du jour `d` avec le détail des séries.
    pub fn log_exercise(&mut self, d: NaiveDate, i: usize, log: ExerciseLog) {
        self.set_done(d, i, true);
        if let Some(r) = self.record_mut(d) {
            if i < r.logs.len() {
                r.logs[i] = Some(log);
            }
        }
    }

    /// Dernier détail saisi pour un exercice (pour pré-remplir).
    pub fn last_log(&self, exercise: &str) -> Option<&ExerciseLog> {
        self.history.iter().rev().find_map(|r| {
            r.exercises.iter().zip(&r.logs).find(|(e, l)| *e == exercise && l.is_some()).and_then(|(_, l)| l.as_ref())
        })
    }

    /// Stats calculées à partir des séances enregistrées.
    pub fn exercise_stats(&self, exercise: &str) -> ExerciseStats {
        let mut st = ExerciseStats::default();
        let mut any_weight = false;
        let mut any_bw = false;
        let mut logs: Vec<(NaiveDate, ExerciseLog)> = self
            .history
            .iter()
            .flat_map(|r| {
                r.exercises
                    .iter()
                    .zip(&r.logs)
                    .filter(|(e, l)| *e == exercise && l.is_some())
                    .map(move |(_, l)| (r.date, l.clone().unwrap()))
            })
            .collect();
        logs.sort_by_key(|(d, _)| std::cmp::Reverse(*d));
        for (_, l) in &logs {
            st.sessions += 1;
            for s in &l.sets {
                st.total_sets += 1;
                st.total_reps += s.reps;
                st.reps_max = Some(st.reps_max.map_or(s.reps, |m| m.max(s.reps)));
                match s.weight {
                    Some(w) => {
                        any_weight = true;
                        st.weight_max = Some(st.weight_max.map_or(w, |m| m.max(w)));
                    }
                    None => any_bw = true,
                }
            }
        }
        st.bodyweight_only = any_bw && !any_weight;
        st.avg_reps = (st.total_sets > 0).then(|| st.total_reps as f32 / st.total_sets as f32);
        st.avg_history = logs
            .iter()
            .filter(|(_, l)| !l.sets.is_empty())
            .take(12)
            .map(|(d, l)| (*d, l.sets.iter().map(|s| s.reps).sum::<u32>() as f32 / l.sets.len() as f32))
            .collect();
        st.avg_history.reverse();
        st.last = logs.first().cloned();
        st.recent = logs.into_iter().take(5).collect();
        st
    }
}

pub fn fr_weekday(d: NaiveDate) -> &'static str {
    ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"]
        [d.weekday().num_days_from_monday() as usize]
}

pub fn fr_month(d: NaiveDate) -> &'static str {
    [
        "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août",
        "septembre", "octobre", "novembre", "décembre",
    ][d.month0() as usize]
}

pub fn fr_date(d: NaiveDate) -> String {
    format!("{} {} {}", fr_weekday(d), d.day(), fr_month(d))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_at(anchor: NaiveDate) -> Data {
        // Règles fixes pour les tests (indépendantes des réglages par défaut).
        let settings = Settings {
            max_rerolls: 2,
            rest_days: 1,
            weekend_off: true,
            weekly_mode: false,
            week_template: [true, true, true, false, true, false, false],
            ..Settings::default()
        };
        Data { anchor, history: vec![], settings, ..Data::default() }
    }

    #[test]
    fn rest_every_fourth_weekday() {
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let d = data_at(mon);
        let kinds: Vec<_> = (0..9).map(|i| d.day_kind(mon + Duration::days(i))).collect();
        use DayKind::*;
        // lun mar mer jeu ven sam dim lun mar
        assert_eq!(kinds, vec![Training, Training, Training, Rest, Training, Weekend, Weekend, Training, Training]);
        assert_eq!(d.day_kind(mon + Duration::days(9)), Rest); // mercredi suivant
    }

    #[test]
    fn week_covers_pool_without_repeats() {
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let mut d = data_at(mon);
        for i in 0..5 {
            let _ = d.draw(mon + Duration::days(i));
        }
        // 4 jours de sport x 3 = 12 = taille de la liste : tout doit tomber une fois.
        let mut drawn = d.drawn_this_week(mon, None);
        drawn.sort();
        let mut pool = d.pool.clone();
        pool.sort();
        assert_eq!(drawn, pool);
        // La semaine suivante repart de zéro.
        assert_eq!(d.remaining(mon + Duration::days(7)).len(), 12);
    }

    #[test]
    fn rename_updates_current_week() {
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let mut d = data_at(mon);
        d.draw(mon).unwrap();
        let first = d.record(mon).unwrap().exercises[0].clone();
        let idx = d.pool.iter().position(|e| *e == first).unwrap();
        d.rename_exercise(idx, "Tractions", mon).unwrap();
        assert!(d.record(mon).unwrap().exercises.contains(&"Tractions".to_string()));
        assert!(!d.remaining(mon).contains(&"Tractions".to_string()));
        let other = d.pool[(idx + 1) % d.pool.len()].to_lowercase();
        assert!(d.rename_exercise(idx, &other, mon).is_err());
        assert!(!d.add_exercise(&format!("  {other} ")));
    }

    #[test]
    fn streak_skips_rest_and_weekend_but_breaks_on_missed_day() {
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let mut d = data_at(mon);
        let complete = |d: &mut Data, day: NaiveDate| {
            d.draw(day).unwrap();
            for i in 0..3 {
                d.set_done(day, i, true);
            }
        };
        // lun, mar, mer complets ; jeu = repos ; ven complet
        for i in [0, 1, 2, 4] {
            complete(&mut d, mon + Duration::days(i));
        }
        let fri = mon + Duration::days(4);
        assert_eq!(d.streak(fri), 4);
        // weekend : la série tient ; lundi suivant pas encore fait : toujours 4
        let next_mon = mon + Duration::days(7);
        assert_eq!(d.streak(next_mon), 4);
        complete(&mut d, next_mon);
        assert_eq!(d.streak(next_mon), 5);
        // mardi suivant manqué -> mercredi la série repart à 1
        let wed = mon + Duration::days(9);
        assert_eq!(d.day_kind(wed), DayKind::Rest);
        let thu = mon + Duration::days(10);
        complete(&mut d, thu);
        assert_eq!(d.streak(thu), 1);
        // à moitié coché ne compte pas
        let fri2 = mon + Duration::days(11);
        d.draw(fri2).unwrap();
        d.set_done(fri2, 0, true);
        assert!(!d.is_complete(fri2));
    }

    #[test]
    fn custom_cycle_and_weekend_option() {
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let mut d = data_at(mon);
        d.settings.train_days = 2;
        d.settings.rest_days = 2;
        use DayKind::*;
        let kinds: Vec<_> = (0..8).map(|i| d.day_kind(mon + Duration::days(i))).collect();
        // lun mar = séance, mer jeu = repos, ven = séance, weekend, lun = séance
        assert_eq!(kinds, vec![Training, Training, Rest, Rest, Training, Weekend, Weekend, Training]);
        d.settings.weekend_off = false;
        let kinds: Vec<_> = (0..8).map(|i| d.day_kind(mon + Duration::days(i))).collect();
        assert_eq!(kinds, vec![Training, Training, Rest, Rest, Training, Training, Rest, Rest]);
        d.settings.rest_days = 0;
        assert_eq!(d.day_kind(mon + Duration::days(2)), Training);
    }

    #[test]
    fn calendar_statuses() {
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let mut d = data_at(mon);
        d.draw(mon).unwrap();
        for i in 0..3 {
            d.set_done(mon, i, true);
        }
        let wed = mon + Duration::days(2);
        assert_eq!(d.day_status(mon, wed), DayStatus::Complete);
        assert_eq!(d.day_status(mon + Duration::days(1), wed), DayStatus::Missed);
        assert_eq!(d.day_status(wed, wed), DayStatus::Today);
        assert_eq!(d.day_status(mon + Duration::days(3), wed), DayStatus::Rest);
        assert_eq!(d.day_status(mon + Duration::days(4), wed), DayStatus::Planned);
        assert_eq!(d.day_status(mon + Duration::days(5), wed), DayStatus::Off);
        assert_eq!(d.day_status(mon - Duration::days(3), wed), DayStatus::Before);
    }

    #[test]
    fn weekly_template_mode() {
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let mut d = data_at(mon);
        d.settings.weekly_mode = true;
        d.settings.week_template = [true, false, true, false, true, true, false];
        use DayKind::*;
        let kinds: Vec<_> = (0..7).map(|i| d.day_kind(mon + Duration::days(i))).collect();
        assert_eq!(kinds, vec![Training, Rest, Training, Rest, Training, Training, Rest]);
        // Samedi est un jour de séance : on peut tirer.
        assert!(d.draw(mon + Duration::days(5)).is_ok());
        assert!(d.draw(mon + Duration::days(6)).is_err());
    }

    #[test]
    fn exercise_database() {
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let mut d = data_at(mon);
        d.set_stat("Squats", "Poids max (kg)", "82,5".into());
        d.set_stat("Squats", "Notes", "genoux ok".into());
        assert_eq!(parse_number(d.stat("Squats", "Poids max (kg)")), Some(82.5));
        assert_eq!(parse_number("abc"), None);
        // renommer l'exercice garde ses stats
        let idx = d.pool.iter().position(|e| e == "Squats").unwrap();
        d.rename_exercise(idx, "Squats barre", mon).unwrap();
        assert_eq!(d.stat("Squats barre", "Notes"), "genoux ok");
        // renommer / supprimer une colonne
        d.rename_column(0, "1RM (kg)").unwrap();
        assert_eq!(d.stat("Squats barre", "1RM (kg)"), "82,5");
        assert!(d.add_column("notes", ColumnKind::Text).is_err());
        d.add_column("Temps (s)", ColumnKind::Number).unwrap();
        let notes = d.columns.iter().position(|c| c.name == "Notes").unwrap();
        d.remove_column(notes);
        assert_eq!(d.stat("Squats barre", "Notes"), "");
        // vider une valeur la retire
        d.set_stat("Squats barre", "1RM (kg)", " ".into());
        assert_eq!(d.stat("Squats barre", "1RM (kg)"), "");
    }

    #[test]
    fn logging_sets_and_stats() {
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let mut d = data_at(mon);
        d.pool = vec!["Squats".into(), "Pompes".into(), "Dips".into()];
        d.draw(mon).unwrap();
        let i = d.record(mon).unwrap().exercises.iter().position(|e| e == "Squats").unwrap();
        let log = ExerciseLog {
            sets: vec![SetLog { reps: 10, weight: Some(60.0) }, SetLog { reps: 8, weight: Some(70.0) }],
            bodyweight: false,
        };
        d.log_exercise(mon, i, log.clone());
        assert!(d.record(mon).unwrap().done[i]);
        let st = d.exercise_stats("Squats");
        assert_eq!(st.reps_max, Some(10));
        assert_eq!(st.weight_max, Some(70.0));
        assert_eq!(st.total_reps, 18);
        assert_eq!(st.total_sets, 2);
        assert_eq!(st.sessions, 1);
        assert_eq!(st.avg_reps, Some(9.0));
        assert_eq!(st.avg_history.len(), 1);
        assert_eq!(st.avg_history[0].1, 9.0);
        assert_eq!(d.last_log("Squats"), Some(&log));
        // poids du corps
        let j = d.record(mon).unwrap().exercises.iter().position(|e| e == "Pompes").unwrap();
        d.log_exercise(mon, j, ExerciseLog { sets: vec![SetLog { reps: 20, weight: None }], bodyweight: true });
        assert!(d.exercise_stats("Pompes").bodyweight_only);
        // décocher efface le détail
        d.set_done(mon, i, false);
        assert_eq!(d.exercise_stats("Squats").sessions, 0);
    }

    #[test]
    fn calendar_reset_keeps_exercise_data() {
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let mut d = data_at(mon);
        let log = ExerciseLog { sets: vec![SetLog { reps: 10, weight: Some(50.0) }], bodyweight: false };
        for i in [0, 1] {
            let day = mon + Duration::days(i);
            d.draw(day).unwrap();
            for k in 0..3 {
                d.log_exercise(day, k, log.clone());
            }
        }
        let tue = mon + Duration::days(1);
        let ex = d.record(mon).unwrap().exercises[0].clone();
        let before = d.exercise_stats(&ex);
        assert_eq!(d.streak(tue), 2);

        // Reset fait le lundi : même les journées "futures" (date simulée) sont effacées du calendrier.
        d.reset_calendar(mon);
        assert_eq!(d.day_status(mon, mon), DayStatus::Today);
        assert_eq!(d.day_status(tue, mon), DayStatus::Planned);
        assert_eq!(d.day_status(mon - Duration::days(1), mon), DayStatus::Off); // dimanche
        assert_eq!(d.day_status(mon - Duration::days(3), mon), DayStatus::Before); // vendredi
        assert_eq!(d.streak(tue), 0);
        assert!(d.drawn_this_week(mon, None).is_empty());
        // on peut retirer aujourd'hui
        assert!(d.draw(mon).is_ok());
        // la base d'exercices n'est pas touchée
        assert_eq!(d.exercise_stats(&ex), before);
        assert_eq!(d.history.len(), 3);
    }

    #[test]
    fn purge_removes_archived_days_only() {
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let mut d = data_at(mon);
        d.draw(mon).unwrap();
        d.reset_calendar(mon);
        d.draw(mon).unwrap();
        assert_eq!(d.history.len(), 2);
        d.purge_archives();
        assert_eq!(d.history.len(), 1);
        assert!(!d.history[0].archived && d.archives_purged);
        // un ancien fichier sans le champ doit déclencher la purge
        let old: Data = serde_json::from_str("{}").unwrap();
        assert!(!old.archives_purged);
    }

    #[test]
    fn rerolls_are_limited_and_change_draw() {
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let mut d = data_at(mon);
        d.draw(mon).unwrap();
        let first = d.record(mon).unwrap().exercises.clone();
        d.reroll(mon).unwrap();
        let second = d.record(mon).unwrap().exercises.clone();
        assert!(first.iter().all(|e| !second.contains(e)));
        d.reroll(mon).unwrap();
        assert!(d.reroll(mon).is_err());
        assert!(d.draw(mon).is_err());
    }
}
