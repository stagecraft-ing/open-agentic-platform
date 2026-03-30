//! FR-008 / SC-007 / SC-008: rolling-window coherence score and monotonic privilege degradation.
//! No wall clock — deterministic given the recorded action sequence.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Privilege level derived from coherence (FR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrivilegeLevel {
    /// coherence >= 0.8 — all permitted operations available.
    Full,
    /// 0.5 <= coherence < 0.8 — destructive operations require confirmation.
    Restricted,
    /// 0.2 <= coherence < 0.5 — only read operations permitted.
    ReadOnly,
    /// coherence < 0.2 — all operations blocked pending human review.
    Suspended,
}

impl PrivilegeLevel {
    /// Higher = less permissive (worse).
    #[inline]
    pub fn severity(self) -> u8 {
        match self {
            PrivilegeLevel::Full => 0,
            PrivilegeLevel::Restricted => 1,
            PrivilegeLevel::ReadOnly => 2,
            PrivilegeLevel::Suspended => 3,
        }
    }

    /// Worst (most restrictive) of two levels.
    #[inline]
    pub fn max_severity(a: Self, b: Self) -> Self {
        if a.severity() >= b.severity() {
            a
        } else {
            b
        }
    }
}

/// Tunable coherence scheduler (defaults match `specs/047` narrative).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoherenceSchedulerConfig {
    /// Rolling window length (default 50).
    pub window_size: usize,
    /// Per-position weight for the newest action is 1.0; each step older multiplies by this (default 0.95).
    pub decay_lambda: f64,
    /// SC-007: when policy-violating actions in the window reach this count, privilege is at least `Restricted`.
    pub violation_count_for_restricted: u32,
}

impl Default for CoherenceSchedulerConfig {
    fn default() -> Self {
        Self {
            window_size: 50,
            decay_lambda: 0.95,
            violation_count_for_restricted: 3,
        }
    }
}

/// Rolling-window coherence + monotonic session degradation (SC-008).
#[derive(Debug, Clone, PartialEq)]
pub struct CoherenceScheduler {
    config: CoherenceSchedulerConfig,
    /// `true` = aligned (no policy intervention); `false` = violating / intervention.
    window: VecDeque<bool>,
    /// Worst severity reached this session without human restore (SC-008).
    stuck_severity: u8,
}

impl CoherenceScheduler {
    pub fn new(config: CoherenceSchedulerConfig) -> Self {
        Self {
            config,
            window: VecDeque::new(),
            stuck_severity: 0,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(CoherenceSchedulerConfig::default())
    }

    pub fn config(&self) -> &CoherenceSchedulerConfig {
        &self.config
    }

    /// Record one action. `aligned` is `false` when the policy layer intervened (deny, degrade, or counted warn).
    pub fn record_action(&mut self, aligned: bool) {
        if self.window.len() >= self.config.window_size {
            self.window.pop_front();
        }
        self.window.push_back(aligned);
        self.update_stuck();
    }

    /// Convenience: treat deny/degrade outcomes as non-aligned.
    pub fn record_from_policy_outcome(&mut self, outcome: &super::PolicyOutcome) {
        let aligned = matches!(outcome, super::PolicyOutcome::Allow);
        self.record_action(aligned);
    }

    /// Human explicitly restores maximum privilege: clears the rolling window and monotonic latch (SC-008).
    pub fn human_restore(&mut self) {
        self.stuck_severity = 0;
        self.window.clear();
    }

    /// Weighted coherence in \[0, 1\] from the rolling window (spec: aligned/total with decay weighting).
    pub fn coherence_score(&self) -> f64 {
        let n = self.window.len();
        if n == 0 {
            return 1.0;
        }
        let lam = self.config.decay_lambda.clamp(0.0, 1.0);
        let mut w_sum = 0.0;
        let mut aligned_w = 0.0;
        for (i, aligned) in self.window.iter().enumerate() {
            // Newest entry last: index n-1-i steps from newest.
            let w = lam.powi((n - 1 - i) as i32);
            w_sum += w;
            if *aligned {
                aligned_w += w;
            }
        }
        if w_sum <= f64::EPSILON {
            return 0.0;
        }
        aligned_w / w_sum
    }

    /// Policy-violating action count in the current window.
    pub fn violation_count(&self) -> u32 {
        self.window.iter().filter(|a| !**a).count() as u32
    }

    /// Level implied by coherence thresholds only (before violation floor and monotonic cap).
    pub fn level_from_score(score: f64) -> PrivilegeLevel {
        if score >= 0.8 {
            PrivilegeLevel::Full
        } else if score >= 0.5 {
            PrivilegeLevel::Restricted
        } else if score >= 0.2 {
            PrivilegeLevel::ReadOnly
        } else {
            PrivilegeLevel::Suspended
        }
    }

    /// Raw level from score + SC-007 violation floor.
    pub fn raw_privilege_level(&self) -> PrivilegeLevel {
        let score = self.coherence_score();
        let mut level = Self::level_from_score(score);
        if self.violation_count() >= self.config.violation_count_for_restricted {
            level = PrivilegeLevel::max_severity(level, PrivilegeLevel::Restricted);
        }
        level
    }

    /// Effective level with monotonic degradation (SC-008).
    pub fn effective_privilege_level(&self) -> PrivilegeLevel {
        let raw = self.raw_privilege_level();
        let s = u8::max(raw.severity(), self.stuck_severity);
        match s {
            0 => PrivilegeLevel::Full,
            1 => PrivilegeLevel::Restricted,
            2 => PrivilegeLevel::ReadOnly,
            _ => PrivilegeLevel::Suspended,
        }
    }

    fn update_stuck(&mut self) {
        let raw = self.raw_privilege_level();
        self.stuck_severity = self.stuck_severity.max(raw.severity());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sc007_violations_force_at_least_restricted() {
        let mut cfg = CoherenceSchedulerConfig::default();
        cfg.violation_count_for_restricted = 2;
        cfg.window_size = 10;
        cfg.decay_lambda = 1.0;
        let mut s = CoherenceScheduler::new(cfg);
        // Score alone would be Full (8/10 aligned), but SC-007 forces at least Restricted.
        for _ in 0..8 {
            s.record_action(true);
        }
        s.record_action(false);
        s.record_action(false);
        assert_eq!(s.coherence_score(), 0.8);
        assert_eq!(s.raw_privilege_level(), PrivilegeLevel::Restricted);
    }

    #[test]
    fn sc008_monotonic_no_self_promotion() {
        let mut cfg = CoherenceSchedulerConfig::default();
        cfg.window_size = 20;
        cfg.violation_count_for_restricted = 100;
        cfg.decay_lambda = 1.0;
        let mut s = CoherenceScheduler::new(cfg);
        // Uniform window: 10/20 aligned → 0.5 → Restricted (FR-008 band)
        for _ in 0..10 {
            s.record_action(true);
        }
        for _ in 0..10 {
            s.record_action(false);
        }
        assert_eq!(s.raw_privilege_level(), PrivilegeLevel::Restricted);
        assert_eq!(s.effective_privilege_level(), PrivilegeLevel::Restricted);
        // Flush window with all-aligned actions — raw returns to Full, effective stays latched
        for _ in 0..40 {
            s.record_action(true);
        }
        assert_eq!(s.coherence_score(), 1.0);
        assert_eq!(s.raw_privilege_level(), PrivilegeLevel::Full);
        assert_eq!(
            s.effective_privilege_level(),
            PrivilegeLevel::Restricted,
            "must not self-promote to Full"
        );
        s.human_restore();
        assert_eq!(s.effective_privilege_level(), PrivilegeLevel::Full);
    }

    #[test]
    fn threshold_crossing_read_only_and_suspended() {
        let mut cfg = CoherenceSchedulerConfig::default();
        cfg.window_size = 10;
        cfg.violation_count_for_restricted = 100; // disable SC-007 floor for this test
        let mut s = CoherenceScheduler::new(cfg.clone());
        // 30% aligned → between 0.2 and 0.5 → ReadOnly
        for _ in 0..3 {
            s.record_action(true);
        }
        for _ in 0..7 {
            s.record_action(false);
        }
        assert_eq!(s.raw_privilege_level(), PrivilegeLevel::ReadOnly);
        let mut s2 = CoherenceScheduler::new(cfg.clone());
        for _ in 0..10 {
            s2.record_action(false);
        }
        assert_eq!(s2.raw_privilege_level(), PrivilegeLevel::Suspended);
    }

    #[test]
    fn weighted_window_favors_recent() {
        let mut cfg = CoherenceSchedulerConfig::default();
        cfg.window_size = 4;
        cfg.decay_lambda = 0.5;
        let mut s = CoherenceScheduler::new(cfg);
        s.record_action(true);
        s.record_action(true);
        s.record_action(false);
        s.record_action(false);
        // Weights oldest→newest: 0.125, 0.25, 0.5, 1.0 → aligned on first two only
        let w = [0.125_f64, 0.25, 0.5, 1.0];
        let expected = (w[0] + w[1]) / w.iter().sum::<f64>();
        assert!((s.coherence_score() - expected).abs() < 1e-9);
    }
}
