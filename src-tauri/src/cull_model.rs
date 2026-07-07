// On-device "learn from my culling" model for the SD-card importer.
//
// It learns the *weighting* of a few interpretable people-photo cues from your own keep/skip
// decisions within each similar group, then applies that weighting to rank future imports.
// Deliberately simple and robust: a nearest-centroid (Rocchio-style) preference — we track
// the mean cue values of the frames you CHOSE vs the group-mates you REJECTED, and nudge the
// weights toward the difference, blended with sensible defaults so it stays stable with only
// a few examples. No training framework, just a handful of running sums persisted to disk.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

/// Feature order (all in [0,1]):
/// [technical, face_sharpness, look_mean, look_worst, expression_mean, eyes_worst]
pub const N_FEATURES: usize = 6;

/// Default cue weights (sum ~1). The worst-face "looking at camera" term carries the most,
/// matching the hand-tuned behaviour before any learning.
const DEFAULT_W: [f64; N_FEATURES] = [0.20, 0.20, 0.15, 0.25, 0.10, 0.10];

const MODEL_FILENAME: &str = "import_cull_model.json";

#[derive(Serialize, Deserialize, Clone)]
pub struct CullModel {
    pub version: u32,
    pub chosen_sum: [f64; N_FEATURES],
    pub chosen_count: u64,
    pub rejected_sum: [f64; N_FEATURES],
    pub rejected_count: u64,
}

impl Default for CullModel {
    fn default() -> Self {
        CullModel {
            version: 1,
            chosen_sum: [0.0; N_FEATURES],
            chosen_count: 0,
            rejected_sum: [0.0; N_FEATURES],
            rejected_count: 0,
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

    pub fn load(app_handle: &tauri::AppHandle) -> CullModel {
        Self::file_path(app_handle)
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<CullModel>(&s).ok())
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

    /// How many groups' worth of feedback have been recorded.
    pub fn sample_count(&self) -> u64 {
        self.chosen_count
    }

    /// Record one group's decision: the frames you kept (`chosen`) vs the group-mates you
    /// skipped (`rejected`), each a feature vector.
    pub fn record(&mut self, chosen: &[[f64; N_FEATURES]], rejected: &[[f64; N_FEATURES]]) {
        for c in chosen {
            for j in 0..N_FEATURES {
                self.chosen_sum[j] += c[j];
            }
            self.chosen_count += 1;
        }
        for r in rejected {
            for j in 0..N_FEATURES {
                self.rejected_sum[j] += r[j];
            }
            self.rejected_count += 1;
        }
    }

    /// Effective weights = defaults blended with the learned preference direction. The
    /// learned share grows with how much feedback we have (capped), so a couple of picks
    /// barely move it while sustained feedback meaningfully personalises it.
    pub fn effective_weights(&self) -> [f64; N_FEATURES] {
        if self.chosen_count == 0 || self.rejected_count == 0 {
            return DEFAULT_W;
        }
        // Difference of mean cue values: chosen − rejected.
        let mut diff = [0.0f64; N_FEATURES];
        let mut l1 = 0.0;
        for j in 0..N_FEATURES {
            diff[j] = self.chosen_sum[j] / self.chosen_count as f64
                - self.rejected_sum[j] / self.rejected_count as f64;
            l1 += diff[j].abs();
        }
        if l1 < 1e-6 {
            return DEFAULT_W;
        }
        for d in diff.iter_mut() {
            *d /= l1; // unit-L1, sign preserved
        }
        let n = self.chosen_count as f64;
        let alpha = (n / (n + 8.0)).min(0.6); // 0 → default; →0.6 with lots of feedback
        let mut w = [0.0f64; N_FEATURES];
        for j in 0..N_FEATURES {
            w[j] = (1.0 - alpha) * DEFAULT_W[j] + alpha * diff[j];
        }
        w
    }

    /// Score a photo's feature vector in [0,1] (higher = better) under the current weights.
    pub fn score(&self, features: &[f64; N_FEATURES], personalize: bool) -> f64 {
        let w = if personalize {
            self.effective_weights()
        } else {
            DEFAULT_W
        };
        let s: f64 = (0..N_FEATURES).map(|j| w[j] * features[j]).sum();
        s.clamp(0.0, 1.0)
    }
}
