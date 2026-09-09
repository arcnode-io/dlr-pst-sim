//! Maps a live DLR line-rating (amps) to a transformer tap position.
//!
//! Band edges are derived from the actual IEEE 738 output range dlr-
//! operating-envelope's sim sensors produce over one sawtooth-sweep cycle
//! for the DRAKE_ACSR_795 conductor (~0-2860A, quartiles ~1125/1671/2168A),
//! rounded to readable numbers -- not arbitrary guesses.

#[cfg(test)]
#[path = "tap_control_test.rs"]
mod tap_control_test;

/// Discrete transformer tap position. Tap1 = highest secondary voltage
/// boost (most ampacity headroom used), Tap4 = lowest voltage (most
/// conservative).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TapPosition {
    /// Highest secondary voltage boost.
    Tap1,
    /// Second-highest voltage boost.
    Tap2,
    /// Second-most conservative (lower) voltage.
    Tap3,
    /// Lowest secondary voltage -- most conservative.
    Tap4,
}

impl TapPosition {
    /// EnumSample wire value per ems/topic_structure_adr.md §6.
    pub fn as_str(self) -> &'static str {
        match self {
            TapPosition::Tap1 => "TAP_1",
            TapPosition::Tap2 => "TAP_2",
            TapPosition::Tap3 => "TAP_3",
            TapPosition::Tap4 => "TAP_4",
        }
    }

    /// 1-indexed step number: Tap1 = 1 (highest voltage) .. Tap4 = 4 (lowest).
    fn step(self) -> u8 {
        match self {
            TapPosition::Tap1 => 1,
            TapPosition::Tap2 => 2,
            TapPosition::Tap3 => 3,
            TapPosition::Tap4 => 4,
        }
    }

    /// Inverse of `step`. Only ever called with values this module produced
    /// (always 1..=4 by construction), so anything else falls back to the
    /// conservative tap rather than panicking.
    fn from_step(step: u8) -> Self {
        match step {
            1 => TapPosition::Tap1,
            2 => TapPosition::Tap2,
            3 => TapPosition::Tap3,
            _ => TapPosition::Tap4,
        }
    }
}

/// Rating (amps) at/above which the target tap is Tap1.
const RATING_BAND_1_A: f64 = 2000.0;
/// Rating (amps) at/above which the target tap is Tap2 (below `RATING_BAND_1_A`).
const RATING_BAND_2_A: f64 = 1400.0;
/// Rating (amps) at/above which the target tap is Tap3 (below `RATING_BAND_2_A`).
const RATING_BAND_3_A: f64 = 800.0;

/// Pick the tap position implied by a rating, ignoring the current
/// position -- `TapController::on_rating` steps toward it gradually.
fn target_for_rating(rating_a: f64) -> TapPosition {
    if rating_a >= RATING_BAND_1_A {
        TapPosition::Tap1
    } else if rating_a >= RATING_BAND_2_A {
        TapPosition::Tap2
    } else if rating_a >= RATING_BAND_3_A {
        TapPosition::Tap3
    } else {
        TapPosition::Tap4
    }
}

/// Stateful tap controller. Starts at the most conservative tap and steps
/// at most one position per `on_rating` call toward the tap the latest
/// rating implies -- gradual adjustment prevents voltage spikes.
pub struct TapController {
    /// The tap position after the most recent `on_rating` step.
    current: TapPosition,
}

impl TapController {
    /// New controller, parked at the most conservative tap until the first
    /// rating arrives.
    pub const fn new() -> Self {
        Self {
            current: TapPosition::Tap4,
        }
    }

    /// Current tap position.
    pub fn current(&self) -> TapPosition {
        self.current
    }

    /// Feed a new line-rating reading (amps); steps at most one tap toward
    /// the position it implies. Returns the tap position after stepping.
    pub fn on_rating(&mut self, rating_a: f64) -> TapPosition {
        let target_step = target_for_rating(rating_a).step();
        let cur_step = self.current.step();

        let next_step = if target_step > cur_step {
            cur_step + 1
        } else if target_step < cur_step {
            cur_step - 1
        } else {
            cur_step
        };

        self.current = TapPosition::from_step(next_step);
        self.current
    }
}

impl Default for TapController {
    fn default() -> Self {
        Self::new()
    }
}
