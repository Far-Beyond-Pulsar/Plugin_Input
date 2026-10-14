//! Showing what an action does before the game runs.
//!
//! [`simulate`] feeds a made-up input (a tap, a hold, a double tap, a slow
//! push of a trigger) through the real evaluator with the action alone
//! bound to one key, and records what the action did at every step. The
//! editor draws the result as a timeline, so a hold time or a dead zone can
//! be judged by looking at it.

use std::collections::HashMap;
use std::sync::Arc;

use crate::keys::{catalog, Pad};
use crate::model::behavior::{magnitude, Value3};
use crate::model::{ActionAsset, ActionRef, MappingAsset};
use crate::runtime::raw::RawInput;
use crate::runtime::system::{ActionLookup, InputSystem, Phase};
use crate::runtime::trigger::TriggerState;

/// A made-up input to try the action with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pattern {
    /// One quick press.
    Tap,
    /// Held for a second.
    Hold,
    /// Two quick presses.
    DoubleTap,
    /// A trigger pulled slowly from rest to full.
    Ramp,
}

impl Pattern {
    pub const ALL: [Pattern; 4] = [
        Pattern::Tap,
        Pattern::Hold,
        Pattern::DoubleTap,
        Pattern::Ramp,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Pattern::Tap => "Tap",
            Pattern::Hold => "Hold",
            Pattern::DoubleTap => "Double tap",
            Pattern::Ramp => "Slow push",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Pattern::Tap => "A key pressed and released after 0.1 seconds",
            Pattern::Hold => "A key held for a second",
            Pattern::DoubleTap => "Two taps, 0.15 seconds apart",
            Pattern::Ramp => "A trigger pulled from rest to full over a second and a half",
        }
    }

    /// How long the preview runs, in seconds.
    pub fn seconds(self) -> f32 {
        match self {
            Pattern::Tap => 1.2,
            Pattern::Hold => 2.0,
            Pattern::DoubleTap => 1.5,
            Pattern::Ramp => 2.0,
        }
    }

    /// The input `t` seconds in.
    fn input(self, t: f32) -> f32 {
        let window = |from: f32, to: f32| if t >= from && t < to { 1.0 } else { 0.0 };
        match self {
            Pattern::Tap => window(0.2, 0.3),
            Pattern::Hold => window(0.2, 1.2),
            Pattern::DoubleTap => window(0.2, 0.3).max(window(0.45, 0.55)),
            Pattern::Ramp => ((t - 0.2) / 1.5).clamp(0.0, 1.0),
        }
    }
}

/// One step of the preview.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub time: f32,
    pub input: f32,
    pub value: Value3,
    pub state: TriggerState,
    pub phases: Vec<Phase>,
}

/// Seconds per step.
pub const STEP: f32 = 1.0 / 60.0;

/// What `action` does with `pattern`.
pub fn simulate(action: &ActionAsset, pattern: Pattern) -> Vec<Sample> {
    let mut action = action.clone();
    action.consume_input = false;
    let reference = ActionRef::new(action.id.clone(), "");
    let mut mapping = MappingAsset::new();
    let category = mapping.categories[0].id;
    let key = match pattern {
        Pattern::Ramp => "Gamepad.RightTrigger",
        _ => "Keyboard.Space",
    };
    mapping.add_entry(category, key, Some(reference));
    let mut lookup: HashMap<_, (Arc<ActionAsset>, String)> = HashMap::new();
    lookup.insert(
        action.id.clone(),
        (Arc::new(action.clone()), "Preview".to_string()),
    );
    let lookup: &dyn ActionLookup = &lookup;

    let mut system = InputSystem::new();
    system.add(&mapping, lookup, 0, None);
    let mut raw = RawInput::new();
    let space = catalog().index_of("Keyboard.Space").unwrap_or_default();
    let steps = (pattern.seconds() / STEP).round() as usize;
    let mut was_down = false;
    let mut samples = Vec::with_capacity(steps);
    for step in 0..steps {
        let time = step as f32 * STEP;
        let input = pattern.input(time);
        match pattern {
            Pattern::Ramp => raw.pad(Pad::RightTrigger, input),
            _ => {
                let down = input > 0.5;
                if down != was_down {
                    raw.set_button(space, down);
                    was_down = down;
                }
            }
        }
        raw.derive();
        let events = system.update(&raw, STEP, false);
        raw.end_frame();
        let (state, value) = system
            .value_of(&action.id)
            .unwrap_or((TriggerState::None, [0.0; 3]));
        samples.push(Sample {
            time,
            input,
            value,
            state,
            phases: events.iter().map(|e| e.phase).collect(),
        });
    }
    samples
}

/// What the made-up input is, in a sentence.
impl Pattern {
    pub fn story(self) -> &'static str {
        match self {
            Pattern::Tap => "You press Space at 0.2 s and let go 0.1 s later.",
            Pattern::Hold => "You press Space at 0.2 s and hold it until 1.2 s.",
            Pattern::DoubleTap => "You tap Space twice: at 0.2 s, and again at 0.45 s.",
            Pattern::Ramp => {
                "You pull the right trigger slowly from rest at 0.2 s to full at 1.7 s."
            }
        }
    }

    /// What the input lane shows.
    pub fn device(self) -> &'static str {
        match self {
            Pattern::Ramp => "Right trigger, 0 to 1",
            _ => "Space key, down or up",
        }
    }
}

/// One line of the event log: a phase, when, and how many frames it lasted.
#[derive(Clone, Debug, PartialEq)]
pub struct LogEntry {
    pub phase: Phase,
    pub from: f32,
    pub to: f32,
    /// How many frames the phase was reported in this run.
    pub frames: usize,
}

/// The events of a run, with the frames where one phase repeats (it keeps
/// firing, it keeps building) folded into one line.
pub fn log(samples: &[Sample]) -> Vec<LogEntry> {
    let mut out: Vec<LogEntry> = Vec::new();
    for s in samples {
        for &phase in &s.phases {
            let repeats = matches!(phase, Phase::Ongoing | Phase::Triggered);
            match out.last_mut() {
                Some(last) if repeats && last.phase == phase && s.time - last.to <= STEP * 1.5 => {
                    last.to = s.time;
                    last.frames += 1;
                }
                _ => out.push(LogEntry {
                    phase,
                    from: s.time,
                    to: s.time,
                    frames: 1,
                }),
            }
        }
    }
    out
}

/// What a phase means to whoever hears the event.
pub fn phase_meaning(phase: Phase) -> &'static str {
    match phase {
        Phase::Started => "the input began to act on this action",
        Phase::Ongoing => "building up, not firing yet",
        Phase::Triggered => "firing: scripts get a triggered event",
        Phase::Completed => "it stopped after firing",
        Phase::Canceled => "it stopped before it fired",
    }
}

/// The result in one sentence.
pub fn verdict(samples: &[Sample]) -> String {
    let entries = log(samples);
    let fired: Vec<_> = entries
        .iter()
        .filter(|e| e.phase == Phase::Triggered)
        .collect();
    if let Some(first) = fired.first() {
        let frames: usize = fired.iter().map(|e| e.frames).sum();
        let runs = if fired.len() == 1 {
            "once".to_string()
        } else {
            format!("{} separate times", fired.len())
        };
        format!(
            "It fires {runs}, for {frames} frames in all, starting at {:.2} s.",
            first.from
        )
    } else if entries.iter().any(|e| e.phase == Phase::Canceled) {
        "It never fires: it started to build up, then the input stopped too soon.".to_string()
    } else {
        "It never fires with this input.".to_string()
    }
}

/// The action's settings as plain sentences, in the order they apply.
pub fn describe(action: &ActionAsset) -> Vec<String> {
    use crate::model::behavior::Trigger;
    let mut lines = Vec::new();
    let triggers: Vec<String> = action
        .known_triggers()
        .map(|t| match t {
            Trigger::Down => "fires every frame the input is held".to_string(),
            Trigger::Pressed => "fires once, the moment the input goes down".to_string(),
            Trigger::Released => "fires once, when the input comes up".to_string(),
            Trigger::Hold { seconds, once } => format!(
                "fires after the input has been held {seconds:.2} s{}",
                if *once {
                    ", once"
                } else {
                    ", and keeps firing"
                }
            ),
            Trigger::HoldAndRelease { seconds } => {
                format!("fires on release, if it was held at least {seconds:.2} s")
            }
            Trigger::Tap { max_seconds } => {
                format!("fires on release, if it was held no longer than {max_seconds:.2} s")
            }
            Trigger::Pulse {
                interval,
                limit,
                on_start,
            } => format!(
                "while held, fires every {interval:.2} s{}{}",
                if *limit > 0 {
                    format!(", up to {limit} times")
                } else {
                    String::new()
                },
                if *on_start {
                    " (and at once on press)"
                } else {
                    ""
                }
            ),
            Trigger::DoubleTap { window } => {
                format!("fires on a second press within {window:.2} s of the first")
            }
            Trigger::Chorded { .. } => {
                "only while another action is firing (not tried here)".to_string()
            }
        })
        .collect();
    if triggers.is_empty() {
        lines.push("Fires every frame the input is held (it has no triggers).".to_string());
    } else {
        for (i, t) in triggers.iter().enumerate() {
            let mut t = t.clone();
            if let Some(first) = t.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            lines.push(if i == 0 {
                format!("{t}.")
            } else {
                format!("Or: {}.", t.to_lowercase())
            });
        }
    }
    for m in action.known_modifiers() {
        lines.push(format!("Reshapes the value first: {}.", m.summary()));
    }
    lines.push(format!(
        "Counts as pressed past {:.2}.",
        action.actuation_threshold
    ));
    lines
}

/// How large the value gets, for scaling the chart (at least 1).
pub fn peak(samples: &[Sample]) -> f32 {
    samples
        .iter()
        .map(|s| magnitude(s.value))
        .fold(0.0, f32::max)
        .max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::behavior::{Modifier, Tolerant, Trigger};

    fn count(samples: &[Sample], phase: Phase) -> usize {
        samples
            .iter()
            .flat_map(|s| &s.phases)
            .filter(|p| **p == phase)
            .count()
    }

    #[test]
    fn a_plain_action_fires_for_as_long_as_the_key_is_down() {
        let s = simulate(&ActionAsset::new(), Pattern::Hold);
        assert_eq!(count(&s, Phase::Started), 1);
        assert_eq!(count(&s, Phase::Completed), 1);
        let firing = s
            .iter()
            .filter(|s| s.state == TriggerState::Triggered)
            .count();
        assert!(
            (55..=65).contains(&firing),
            "about a second at 60 steps a second: {firing}"
        );
    }

    #[test]
    fn a_hold_trigger_needs_the_hold_and_a_tap_does_not_reach_it() {
        let mut a = ActionAsset::new();
        a.triggers = vec![Tolerant::Known(Trigger::Hold {
            seconds: 0.5,
            once: true,
        })];
        assert_eq!(count(&simulate(&a, Pattern::Hold), Phase::Triggered), 1);
        assert_eq!(count(&simulate(&a, Pattern::Tap), Phase::Triggered), 0);
        assert_eq!(
            count(&simulate(&a, Pattern::Tap), Phase::Canceled),
            1,
            "a tap on a hold action is a cancel"
        );
    }

    #[test]
    fn a_double_tap_trigger_fires_only_for_two_taps() {
        let mut a = ActionAsset::new();
        a.triggers = vec![Tolerant::Known(Trigger::DoubleTap { window: 0.4 })];
        assert_eq!(
            count(&simulate(&a, Pattern::DoubleTap), Phase::Triggered),
            1
        );
        assert_eq!(count(&simulate(&a, Pattern::Tap), Phase::Triggered), 0);
    }

    #[test]
    fn a_dead_zone_delays_a_slow_push() {
        let mut a = ActionAsset::new();
        a.value_type = crate::model::ValueType::Axis1D;
        a.actuation_threshold = 0.05;
        let plain = simulate(&a, Pattern::Ramp);
        a.modifiers = vec![Tolerant::Known(Modifier::DeadZone {
            lower: 0.5,
            upper: 1.0,
            radial: false,
        })];
        let dead = simulate(&a, Pattern::Ramp);
        let first = |s: &[Sample]| {
            s.iter()
                .position(|s| s.state == TriggerState::Triggered)
                .unwrap()
        };
        assert!(
            first(&dead) > first(&plain) + 20,
            "the dead zone swallows the first half of the push"
        );
        let end = dead.last().unwrap().value[0];
        assert!((end - 1.0).abs() < 0.01, "and still reaches full: {end}");
    }

    #[test]
    fn the_log_folds_repeating_phases_and_the_verdict_says_what_happened() {
        let s = simulate(&ActionAsset::new(), Pattern::Hold);
        let entries = log(&s);
        let phases: Vec<_> = entries.iter().map(|e| e.phase).collect();
        assert_eq!(phases, [Phase::Started, Phase::Triggered, Phase::Completed]);
        let firing = &entries[1];
        assert!(firing.frames > 50 && (firing.to - firing.from - 1.0).abs() < 0.05);
        assert!(verdict(&s).starts_with("It fires once"));

        let mut hold = ActionAsset::new();
        hold.triggers = vec![Tolerant::Known(Trigger::Hold {
            seconds: 0.5,
            once: true,
        })];
        let tap = simulate(&hold, Pattern::Tap);
        assert_eq!(
            verdict(&tap),
            "It never fires: it started to build up, then the input stopped too soon."
        );
        let blank = simulate(&ActionAsset::new(), Pattern::Ramp);
        assert!(verdict(&blank).contains("fires"));
    }

    #[test]
    fn an_action_is_described_in_the_order_things_apply() {
        let mut a = ActionAsset::new();
        assert_eq!(
            describe(&a)[0],
            "Fires every frame the input is held (it has no triggers)."
        );
        a.triggers = vec![
            Tolerant::Known(Trigger::Hold {
                seconds: 0.5,
                once: true,
            }),
            Tolerant::Known(Trigger::Pressed),
        ];
        a.modifiers = vec![Tolerant::Known(Modifier::Scale {
            x: 2.0,
            y: 1.0,
            z: 1.0,
        })];
        let lines = describe(&a);
        assert_eq!(
            lines[0],
            "Fires after the input has been held 0.50 s, once."
        );
        assert!(lines[1].starts_with("Or: fires once"));
        assert!(lines[2].starts_with("Reshapes the value first: Scale"));
        assert!(lines.last().unwrap().starts_with("Counts as pressed past"));
    }

    #[test]
    fn every_pattern_runs_and_reports_its_length() {
        for pattern in Pattern::ALL {
            let s = simulate(&ActionAsset::new(), pattern);
            assert_eq!(s.len(), (pattern.seconds() / STEP).round() as usize);
            assert!(s.windows(2).all(|w| w[1].time > w[0].time));
        }
    }
}
