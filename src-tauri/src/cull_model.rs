// On-device "learn from my culling" model for the SD-card / folder importer.
//
// Two things are learned from what you finally import, per similar group (burst):
//
// 1. Which cues you prefer — a nearest-centroid (Rocchio-style) preference per kind of
//    photo: the mean cue values of the frames you KEPT vs the group-mates you SKIPPED,
//    blended into the default weights as feedback accumulates. People shots, non-people
//    shots and technical-only scores (AI models unavailable) have different cues, so each
//    kind learns separately — before v2 only people shots learned anything.
// 2. Your keep policy — how many frames of a burst you keep (by the app's ranking) and how
//    good a burst's best frame has to be before you keep the burst at all. Auto-select used
//    to always keep exactly one frame per burst at a fixed grade threshold.
//
// Deliberately simple and robust: running sums with priors that reproduce the old
// behaviour until real feedback outweighs them. No training framework; persisted to disk.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::Manager;

/// The kind of photo a cue vector describes; each kind has its own cue set.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CueKind {
    /// Faces found: [technical, face_sharpness, eye_sharpness, look_mean, look_worst,
    /// expression_mean, eyes_open_worst, face_lighting]
    People,
    /// No faces: [overall_sharpness, subject_sharpness, highlights, shadows, balance, aesthetic]
    Scene,
    /// AI models unavailable: [sharpness, centre_focus, highlights, shadows, balance]
    Technical,
}

/// Default cue weights per kind — the hand-tuned behaviour before any learning. The
/// worst-face "looking at camera" term carries the most for people shots; focus at the
/// eyes and light on the faces each count as much as the blink check.
const PEOPLE_DEFAULT_W: [f64; 8] = [0.15, 0.10, 0.15, 0.12, 0.20, 0.08, 0.10, 0.10];
const SCENE_DEFAULT_W: [f64; 6] = [0.25, 0.30, 0.10, 0.08, 0.07, 0.20];
const TECHNICAL_DEFAULT_W: [f64; 5] = crate::culling::TECHNICAL_WEIGHTS;

impl CueKind {
    pub fn default_weights(self) -> &'static [f64] {
        match self {
            CueKind::People => &PEOPLE_DEFAULT_W,
            CueKind::Scene => &SCENE_DEFAULT_W,
            CueKind::Technical => &TECHNICAL_DEFAULT_W,
        }
    }
}

const MODEL_FILENAME: &str = "import_cull_model.json";

/// Running sums of the cue vectors of kept vs skipped group-mates for one kind.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(default)]
pub struct Preference {
    pub chosen_sum: Vec<f64>,
    pub chosen_count: u64,
    pub rejected_sum: Vec<f64>,
    pub rejected_count: u64,
}

impl Preference {
    fn add(sum: &mut Vec<f64>, v: &[f64]) {
        if sum.len() != v.len() {
            *sum = vec![0.0; v.len()];
        }
        for (s, x) in sum.iter_mut().zip(v) {
            *s += x;
        }
    }

    fn record(&mut self, chosen: &[&[f64]], rejected: &[&[f64]]) {
        // A changed cue set (an app update adding a cue) makes old sums meaningless:
        // start this kind over rather than mixing vectors of different shapes.
        let n = chosen.first().or(rejected.first()).map_or(0, |v| v.len());
        let stale = |sum: &Vec<f64>, count: u64| count > 0 && sum.len() != n;
        if stale(&self.chosen_sum, self.chosen_count) || stale(&self.rejected_sum, self.rejected_count) {
            *self = Preference::default();
        }
        for c in chosen {
            Self::add(&mut self.chosen_sum, c);
            self.chosen_count += 1;
        }
        for r in rejected {
            Self::add(&mut self.rejected_sum, r);
            self.rejected_count += 1;
        }
    }

    /// Defaults blended with the learned preference direction. The learned share grows with
    /// the amount of feedback (capped), so a couple of picks barely move it while sustained
    /// feedback meaningfully personalises it.
    pub fn effective_weights(&self, defaults: &[f64]) -> Vec<f64> {
        let n = defaults.len();
        if self.chosen_count == 0
            || self.rejected_count == 0
            || self.chosen_sum.len() != n
            || self.rejected_sum.len() != n
        {
            return defaults.to_vec();
        }
        // Difference of mean cue values: chosen − rejected.
        let diff: Vec<f64> = (0..n)
            .map(|j| {
                self.chosen_sum[j] / self.chosen_count as f64
                    - self.rejected_sum[j] / self.rejected_count as f64
            })
            .collect();
        let l1: f64 = diff.iter().map(|d| d.abs()).sum();
        if l1 < 1e-6 {
            return defaults.to_vec();
        }
        let c = self.chosen_count as f64;
        let alpha = (c / (c + 8.0)).min(0.6); // 0 → default; →0.6 with lots of feedback
        (0..n)
            .map(|j| (1.0 - alpha) * defaults[j] + alpha * diff[j] / l1)
            .collect()
    }
}

/// Rank buckets: the app's best frame, 2nd, 3rd, and "4th or later".
pub const RANK_BUCKETS: usize = 4;
/// Pseudo-observations behind each prior, so one import can't flip the policy.
const PRIOR_N: f64 = 2.0;
/// Grade (0–5) at which the defaults keep a burst — matches the old fixed threshold.
const DEFAULT_KEEP_GRADE: usize = 3;

/// What you keep from a burst, learned from finished imports.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(default)]
pub struct KeepPolicy {
    /// Given you kept the burst: how often the frame at each rank was kept / seen.
    pub rank_kept: [f64; RANK_BUCKETS],
    pub rank_seen: [f64; RANK_BUCKETS],
    /// By the burst's best grade (0–5): bursts you kept any frame of / bursts seen.
    pub grade_kept: [f64; 6],
    pub grade_seen: [f64; 6],
}

impl KeepPolicy {
    fn rank_prior(r: usize) -> f64 {
        if r == 0 { 1.0 } else { 0.0 }
    }
    fn grade_prior(g: usize) -> f64 {
        if g >= DEFAULT_KEEP_GRADE { 1.0 } else { 0.0 }
    }

    /// P(keep the frame at rank r | the burst is kept), never rising with rank.
    pub fn keep_rank(&self) -> [f64; RANK_BUCKETS] {
        let mut p = [0.0; RANK_BUCKETS];
        for r in 0..RANK_BUCKETS {
            let raw = (self.rank_kept[r] + Self::rank_prior(r) * PRIOR_N) / (self.rank_seen[r] + PRIOR_N);
            p[r] = if r == 0 { raw } else { raw.min(p[r - 1]) };
        }
        p
    }

    /// P(keep a burst | its best frame's grade), never falling as the grade rises.
    pub fn keep_grade(&self) -> [f64; 6] {
        let mut p = [0.0; 6];
        for g in 0..6 {
            let raw = (self.grade_kept[g] + Self::grade_prior(g) * PRIOR_N) / (self.grade_seen[g] + PRIOR_N);
            p[g] = if g == 0 { raw } else { raw.max(p[g - 1]) };
        }
        p
    }

    /// Bursts learned from.
    pub fn groups(&self) -> u64 {
        self.grade_seen.iter().sum::<f64>() as u64
    }

    /// Average frames kept per kept burst, if any were kept.
    pub fn typical_keep(&self) -> Option<f64> {
        (self.rank_seen[0] > 0.0).then(|| self.rank_kept.iter().sum::<f64>() / self.rank_seen[0])
    }
}

/// One similar group as finally imported: members best-first by the app's score, whether
/// each was kept, and the best frame's 0–5 grade as shown on the badges.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupOutcome {
    pub ranked: Vec<String>,
    pub kept: Vec<bool>,
    pub best_grade: u8,
}

/// What has been learned so far, for the import UI and the assistant's prompt.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LearningSummary {
    pub imports: u64,
    pub groups: u64,
    pub cue_samples: u64,
    /// P(keep a burst) by best grade 0–5.
    pub keep_grade: [f64; 6],
    /// P(keep frame | burst kept) by rank: best, 2nd, 3rd, 4th+.
    pub keep_rank: [f64; RANK_BUCKETS],
    pub typical_keep: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CullModel {
    pub version: u32,
    #[serde(default)]
    pub people: Preference,
    #[serde(default)]
    pub scene: Preference,
    #[serde(default)]
    pub technical: Preference,
    #[serde(default)]
    pub policy: KeepPolicy,
    #[serde(default)]
    pub imports: u64,
    // v1 kept people-shot sums at the top level; read once, migrated into `people`.
    #[serde(default, skip_serializing)]
    chosen_sum: Option<Vec<f64>>,
    #[serde(default, skip_serializing)]
    chosen_count: Option<u64>,
    #[serde(default, skip_serializing)]
    rejected_sum: Option<Vec<f64>>,
    #[serde(default, skip_serializing)]
    rejected_count: Option<u64>,
}

impl Default for CullModel {
    fn default() -> Self {
        CullModel {
            version: 2,
            people: Preference::default(),
            scene: Preference::default(),
            technical: Preference::default(),
            policy: KeepPolicy::default(),
            imports: 0,
            chosen_sum: None,
            chosen_count: None,
            rejected_sum: None,
            rejected_count: None,
        }
    }
}

impl CullModel {
    fn file_path(app_handle: &tauri::AppHandle) -> Option<PathBuf> {
        app_handle
            .path()
            .app_data_dir()
            .ok()
            .map(|d| d.join(MODEL_FILENAME))
    }

    /// Parse a stored model, migrating v1 (people-only sums) into v2.
    pub fn from_json(s: &str) -> Option<CullModel> {
        let mut m: CullModel = serde_json::from_str(s).ok()?;
        if m.version < 2 {
            // v1 people cues: [technical, face_sharp, look_mean, look_worst, expr, eyes].
            // v2 adds eye focus (seeded from face focus) and face lighting (neutral, so it
            // carries no learned direction yet) — what was learned keeps its direction.
            let widen = |sum: Option<Vec<f64>>, count: u64| -> Vec<f64> {
                match sum {
                    Some(v) if v.len() == 6 => {
                        vec![v[0], v[1], v[1], v[2], v[3], v[4], v[5], 0.5 * count as f64]
                    }
                    _ => Vec::new(),
                }
            };
            let chosen_count = m.chosen_count.take().unwrap_or(0);
            let rejected_count = m.rejected_count.take().unwrap_or(0);
            m.people = Preference {
                chosen_sum: widen(m.chosen_sum.take(), chosen_count),
                chosen_count,
                rejected_sum: widen(m.rejected_sum.take(), rejected_count),
                rejected_count,
            };
            m.version = 2;
        }
        Some(m)
    }

    pub fn load(app_handle: &tauri::AppHandle) -> CullModel {
        Self::file_path(app_handle)
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| Self::from_json(&s))
            .unwrap_or_default()
    }

    pub fn save(&self, app_handle: &tauri::AppHandle) -> Result<(), String> {
        let path = Self::file_path(app_handle).ok_or("No app data dir")?;
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    pub fn reset(app_handle: &tauri::AppHandle) -> Result<(), String> {
        CullModel::default().save(app_handle)
    }

    fn preference(&self, kind: CueKind) -> &Preference {
        match kind {
            CueKind::People => &self.people,
            CueKind::Scene => &self.scene,
            CueKind::Technical => &self.technical,
        }
    }

    fn preference_mut(&mut self, kind: CueKind) -> &mut Preference {
        match kind {
            CueKind::People => &mut self.people,
            CueKind::Scene => &mut self.scene,
            CueKind::Technical => &mut self.technical,
        }
    }

    /// Learn from one finished import. Cue preferences compare kept vs skipped frames of the
    /// SAME kind within each burst; the keep policy learns from every scored burst. Returns
    /// whether anything was learned.
    pub fn record_import(
        &mut self,
        groups: &[GroupOutcome],
        features: &HashMap<String, (CueKind, Vec<f64>)>,
    ) -> bool {
        let mut learned = false;
        for g in groups {
            if g.ranked.len() < 2 || g.ranked.len() != g.kept.len() {
                continue; // only real bursts with a decision per frame
            }
            learned = true;

            let grade = (g.best_grade as usize).min(5);
            let any_kept = g.kept.iter().any(|&k| k);
            self.policy.grade_seen[grade] += 1.0;
            if any_kept {
                self.policy.grade_kept[grade] += 1.0;
                for (r, &k) in g.kept.iter().enumerate() {
                    let b = r.min(RANK_BUCKETS - 1);
                    self.policy.rank_seen[b] += 1.0;
                    if k {
                        self.policy.rank_kept[b] += 1.0;
                    }
                }
            }

            for kind in [CueKind::People, CueKind::Scene, CueKind::Technical] {
                let mut chosen: Vec<&[f64]> = Vec::new();
                let mut rejected: Vec<&[f64]> = Vec::new();
                for (p, &k) in g.ranked.iter().zip(&g.kept) {
                    if let Some((fk, f)) = features.get(p) {
                        if *fk == kind {
                            if k { chosen.push(f) } else { rejected.push(f) }
                        }
                    }
                }
                if !chosen.is_empty() && !rejected.is_empty() {
                    self.preference_mut(kind).record(&chosen, &rejected);
                }
            }
        }
        if learned {
            self.imports += 1;
        }
        learned
    }

    /// Score a photo's cue vector in [0,1] (higher = better) under the current weights.
    pub fn score(&self, kind: CueKind, features: &[f64], personalize: bool) -> f64 {
        let defaults = kind.default_weights();
        let w = if personalize {
            self.preference(kind).effective_weights(defaults)
        } else {
            defaults.to_vec()
        };
        let s: f64 = w.iter().zip(features).map(|(w, f)| w * f).sum();
        s.clamp(0.0, 1.0)
    }

    pub fn summary(&self) -> LearningSummary {
        LearningSummary {
            imports: self.imports,
            groups: self.policy.groups(),
            cue_samples: self.people.chosen_count + self.scene.chosen_count + self.technical.chosen_count,
            keep_grade: self.policy.keep_grade(),
            keep_rank: self.policy.keep_rank(),
            typical_keep: self.policy.typical_keep(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(n: usize, kept: &[usize], grade: u8) -> GroupOutcome {
        GroupOutcome {
            ranked: (0..n).map(|i| format!("f{i}")).collect(),
            kept: (0..n).map(|i| kept.contains(&i)).collect(),
            best_grade: grade,
        }
    }

    #[test]
    fn defaults_reproduce_the_old_behaviour() {
        let m = CullModel::default();
        let rank = m.policy.keep_rank();
        assert_eq!(rank[0], 1.0);
        assert!(rank[1..].iter().all(|&p| p == 0.0));
        let grade = m.policy.keep_grade();
        assert!(grade[..3].iter().all(|&p| p < 0.5));
        assert!(grade[3..].iter().all(|&p| p >= 0.5));
    }

    #[test]
    fn keeping_two_per_burst_is_learned() {
        let mut m = CullModel::default();
        let none = HashMap::new();
        // One import of three bursts where the photographer kept the top two frames.
        let groups = vec![outcome(4, &[0, 1], 4), outcome(5, &[0, 1], 5), outcome(3, &[0, 1], 3)];
        assert!(m.record_import(&groups, &none));
        let rank = m.policy.keep_rank();
        assert!(rank[1] >= 0.5, "2nd frame should now be kept: {rank:?}");
        assert!(rank[2] < 0.5, "3rd frame still skipped: {rank:?}");
        assert_eq!(m.policy.typical_keep(), Some(2.0));
        assert_eq!(m.imports, 1);
    }

    #[test]
    fn keeping_weak_bursts_lowers_the_grade_threshold() {
        let mut m = CullModel::default();
        let none = HashMap::new();
        for _ in 0..2 {
            m.record_import(&[outcome(3, &[0], 2), outcome(3, &[0], 1)], &none);
        }
        let grade = m.policy.keep_grade();
        assert!(grade[1] >= 0.5 && grade[2] >= 0.5, "{grade:?}");
        assert!(grade[0] < 0.5, "{grade:?}");
    }

    #[test]
    fn cue_preferences_learn_per_kind_within_a_burst() {
        let mut m = CullModel::default();
        // Scene shots: the photographer keeps the well-exposed frame over a slightly
        // sharper one that came out too dark.
        let mut f = HashMap::new();
        f.insert("f0".to_string(), (CueKind::Scene, vec![0.9, 0.9, 1.0, 1.0, 0.3, 0.5]));
        f.insert("f1".to_string(), (CueKind::Scene, vec![0.7, 0.7, 1.0, 1.0, 1.0, 0.5]));
        let g = GroupOutcome { ranked: vec!["f0".into(), "f1".into()], kept: vec![false, true], best_grade: 4 };
        for _ in 0..10 {
            m.record_import(std::slice::from_ref(&g), &f);
        }
        assert_eq!(m.people.chosen_count, 0);
        assert_eq!(m.scene.chosen_count, 10);
        let before = m.score(CueKind::Scene, &f["f0"].1, false) - m.score(CueKind::Scene, &f["f1"].1, false);
        let after = m.score(CueKind::Scene, &f["f0"].1, true) - m.score(CueKind::Scene, &f["f1"].1, true);
        assert!(before > 0.0, "defaults prefer the sharper frame");
        assert!(after < 0.0, "learned weights prefer the kept frame");
    }

    #[test]
    fn mixed_kinds_in_a_burst_are_not_compared() {
        let mut m = CullModel::default();
        let mut f = HashMap::new();
        f.insert("f0".to_string(), (CueKind::People, vec![0.5; 8]));
        f.insert("f1".to_string(), (CueKind::Scene, vec![0.5; 6]));
        m.record_import(&[outcome(2, &[1], 4)], &f);
        assert_eq!(m.people.chosen_count + m.people.rejected_count, 0);
        assert_eq!(m.scene.chosen_count + m.scene.rejected_count, 0);
        assert_eq!(m.policy.groups(), 1, "the keep policy still learns");
    }

    #[test]
    fn v1_model_files_migrate_into_people() {
        let v1 = r#"{"version":1,"chosen_sum":[1,2,3,4,5,6],"chosen_count":3,"rejected_sum":[6,5,4,3,2,1],"rejected_count":4}"#;
        let m = CullModel::from_json(v1).unwrap();
        assert_eq!(m.version, 2);
        assert_eq!(m.people.chosen_count, 3);
        assert_eq!(m.people.chosen_sum, vec![1.0, 2.0, 2.0, 3.0, 4.0, 5.0, 6.0, 1.5]);
        assert_eq!(m.people.rejected_sum, vec![6.0, 5.0, 5.0, 4.0, 3.0, 2.0, 1.0, 2.0]);
        // The migrated preference is usable straight away with the 8-cue defaults.
        let w = m.people.effective_weights(CueKind::People.default_weights());
        assert_eq!(w.len(), 8);
        assert_ne!(w, CueKind::People.default_weights().to_vec());
        let saved = serde_json::to_string(&m).unwrap();
        let top: serde_json::Value = serde_json::from_str(&saved).unwrap();
        assert!(top.get("chosen_sum").is_none(), "legacy fields are not written back");
        let again = CullModel::from_json(&saved).unwrap();
        assert_eq!(again.people.chosen_count, 3);
    }
}

#[cfg(test)]
mod weight_tests {
    use super::*;

    #[test]
    fn default_weights_sum_to_one() {
        for kind in [CueKind::People, CueKind::Scene, CueKind::Technical] {
            let sum: f64 = kind.default_weights().iter().sum();
            assert!((sum - 1.0).abs() < 1e-9, "{kind:?} sums to {sum}");
        }
    }

    #[test]
    fn a_changed_cue_set_starts_that_kind_over() {
        let mut p = Preference::default();
        p.record(&[&[1.0, 1.0]], &[&[0.0, 0.0]]);
        p.record(&[&[1.0, 1.0, 1.0]], &[&[0.0, 0.0, 0.0]]);
        assert_eq!(p.chosen_count, 1);
        assert_eq!(p.chosen_sum.len(), 3);
    }
}
