use crate::core::settings::Settings;
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum State {
    Play,
    /// Visible, animation stopped. Used when the desktop can still be seen.
    Frozen,
    /// Nobody can see it: covered by a fullscreen or maximized app, locked, or remote.
    /// Frozen in place like `Frozen`; kept separate so the UI and logs say why.
    Covered,
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Signals {
    /// Some(true) = user paused, Some(false) = user forced play, None = automatic.
    pub manual: Option<bool>,
    pub on_battery: bool,
    pub power_saver: bool,
    pub locked: bool,
    pub remote: bool,
    pub foreground_app: Option<String>,
    pub desktop_focused: bool,
    /// Per display, in the same order as the displays: is it covered by a fullscreen or maximized window?
    pub covered: Vec<bool>,
    /// Other apps (never Sarab itself) have kept the CPU over the set limit; see `CpuGate`.
    pub cpu_busy: bool,
}

/// Turns CPU readings into a steady yes or no. Busy after 3 readings at or over the limit in a
/// row; quiet again only after 5 readings at least 15 points under it. The gap and the counts
/// stop the wallpaper from flapping between playing and resting around the limit.
#[derive(Default)]
pub struct CpuGate {
    over: u8,
    under: u8,
    busy: bool,
}

impl CpuGate {
    pub fn step(&mut self, percent: f64, limit: u8) -> bool {
        if limit == 0 {
            *self = CpuGate::default();
            return false;
        }
        let limit = f64::from(limit);
        if percent >= limit {
            self.over = self.over.saturating_add(1);
            self.under = 0;
        } else if percent < limit - 15.0 {
            self.under = self.under.saturating_add(1);
            self.over = 0;
        } else {
            self.over = 0;
            self.under = 0;
        }
        if !self.busy && self.over >= 3 {
            self.busy = true;
        } else if self.busy && self.under >= 5 {
            self.busy = false;
        }
        self.busy
    }
}

/// Why a display is in its state. Chosen by the same branch that chose the state, so the
/// UI can never show a reason that is not the one holding the pause.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    None,
    Manual,
    ForcedPlay,
    Locked,
    Remote,
    AppPlay,
    AppPause,
    Covered,
    OtherCovered,
    Battery,
    PowerSaver,
    Focus,
    Busy,
}

pub fn decide(s: &Signals, r: &Settings) -> Vec<(State, Reason)> {
    let n = s.covered.len();
    let all = |st, why| vec![(st, why); n];
    match s.manual {
        Some(true) => return all(State::Frozen, Reason::Manual),
        Some(false) => return all(State::Play, Reason::ForcedPlay),
        None => {}
    }
    if s.locked {
        return all(State::Covered, Reason::Locked);
    }
    if s.remote && r.pause_remote {
        return all(State::Covered, Reason::Remote);
    }
    let app = s.foreground_app.as_deref().map(str::to_lowercase);
    let listed = |list: &[String]| {
        app.as_ref()
            .is_some_and(|a| list.iter().any(|x| x.to_lowercase() == *a))
    };
    if listed(&r.app_play) {
        return all(State::Play, Reason::AppPlay);
    }
    if listed(&r.app_pause) {
        return all(State::Frozen, Reason::AppPause);
    }
    let any_covered = s.covered.iter().any(|&c| c);
    let soft = if s.on_battery && r.pause_battery {
        Some(Reason::Battery)
    } else if s.power_saver && r.pause_power_saver {
        Some(Reason::PowerSaver)
    } else if s.cpu_busy {
        Some(Reason::Busy)
    } else if r.pause_focus && !s.desktop_focused {
        Some(Reason::Focus)
    } else {
        None
    };
    s.covered
        .iter()
        .map(|&covered| {
            if r.pause_fullscreen && covered {
                (State::Covered, Reason::Covered)
            } else if let Some(why) = soft {
                (State::Frozen, why)
            } else if r.pause_fullscreen && !r.per_display && any_covered {
                // "All displays" mode: an uncovered display freezes too.
                (State::Frozen, Reason::OtherCovered)
            } else {
                (State::Play, Reason::None)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{State::*, *};

    fn sig(covered: &[bool]) -> Signals {
        Signals {
            covered: covered.to_vec(),
            desktop_focused: true,
            ..Default::default()
        }
    }

    #[test]
    fn cpu_busy_needs_time_and_a_gap() {
        let mut g = CpuGate::default();
        let run =
            |g: &mut CpuGate, xs: &[f64]| xs.iter().map(|&x| g.step(x, 80)).collect::<Vec<_>>();
        assert_eq!(
            run(&mut g, &[95.0, 95.0]),
            [false, false],
            "two busy seconds are not enough"
        );
        assert_eq!(run(&mut g, &[95.0]), [true], "the third is");
        assert_eq!(
            run(&mut g, &[70.0, 70.0, 70.0, 70.0, 70.0, 70.0]),
            [true; 6],
            "inside the gap it stays busy"
        );
        assert_eq!(run(&mut g, &[50.0, 50.0, 50.0, 50.0]), [true; 4]);
        assert_eq!(
            run(&mut g, &[50.0]),
            [false],
            "the fifth quiet second frees it"
        );
        assert_eq!(
            run(&mut g, &[95.0, 60.0, 95.0, 95.0]),
            [false; 4],
            "a dip resets the count"
        );
        assert!(!g.step(99.0, 0), "off");
        let busy = Signals {
            covered: vec![false],
            desktop_focused: true,
            cpu_busy: true,
            ..Default::default()
        };
        assert_eq!(
            decide(&busy, &Settings::default()),
            vec![(Frozen, Reason::Busy)]
        );
    }

    #[test]
    fn decide_table() {
        let d = Settings::default();
        let per_all = Settings {
            per_display: false,
            ..Settings::default()
        };
        let focus = Settings {
            pause_focus: true,
            ..Settings::default()
        };
        let no_battery = Settings {
            pause_battery: false,
            ..Settings::default()
        };
        let no_fullscreen = Settings {
            pause_fullscreen: false,
            ..Settings::default()
        };
        let no_remote = Settings {
            pause_remote: false,
            ..Settings::default()
        };
        let rules = Settings {
            app_pause: vec!["Blender.exe".into()],
            app_play: vec!["obs64.exe".into()],
            ..Settings::default()
        };

        let cases: Vec<(&str, Signals, &Settings, Vec<State>)> = vec![
            (
                "idle desktop plays",
                sig(&[false, false]),
                &d,
                vec![Play, Play],
            ),
            (
                "covered display pauses, other plays",
                sig(&[true, false]),
                &d,
                vec![Covered, Play],
            ),
            (
                "all-displays mode freezes the uncovered one",
                sig(&[true, false]),
                &per_all,
                vec![Covered, Frozen],
            ),
            (
                "fullscreen rule off ignores cover",
                sig(&[true, false]),
                &no_fullscreen,
                vec![Play, Play],
            ),
            (
                "battery freezes",
                Signals {
                    on_battery: true,
                    ..sig(&[false])
                },
                &d,
                vec![Frozen],
            ),
            (
                "battery rule off",
                Signals {
                    on_battery: true,
                    ..sig(&[false])
                },
                &no_battery,
                vec![Play],
            ),
            (
                "power saver freezes",
                Signals {
                    power_saver: true,
                    ..sig(&[false])
                },
                &d,
                vec![Frozen],
            ),
            (
                "battery plus cover: cover wins on that display",
                Signals {
                    on_battery: true,
                    ..sig(&[true, false])
                },
                &d,
                vec![Covered, Frozen],
            ),
            (
                "lock pauses all",
                Signals {
                    locked: true,
                    ..sig(&[false, false])
                },
                &d,
                vec![Covered, Covered],
            ),
            (
                "remote pauses all",
                Signals {
                    remote: true,
                    ..sig(&[false])
                },
                &d,
                vec![Covered],
            ),
            (
                "remote rule off",
                Signals {
                    remote: true,
                    ..sig(&[false])
                },
                &no_remote,
                vec![Play],
            ),
            (
                "manual pause beats everything",
                Signals {
                    manual: Some(true),
                    locked: true,
                    ..sig(&[true, false])
                },
                &d,
                vec![Frozen, Frozen],
            ),
            (
                "manual play beats everything",
                Signals {
                    manual: Some(false),
                    on_battery: true,
                    ..sig(&[true])
                },
                &d,
                vec![Play],
            ),
            (
                "app pause rule, case-insensitive",
                Signals {
                    foreground_app: Some("blender.exe".into()),
                    ..sig(&[false])
                },
                &rules,
                vec![Frozen],
            ),
            (
                "app play rule beats cover",
                Signals {
                    foreground_app: Some("OBS64.exe".into()),
                    ..sig(&[true])
                },
                &rules,
                vec![Play],
            ),
            (
                "lock beats app play rule",
                Signals {
                    locked: true,
                    foreground_app: Some("obs64.exe".into()),
                    ..sig(&[false])
                },
                &rules,
                vec![Covered],
            ),
            (
                "focus rule: app focused freezes",
                Signals {
                    desktop_focused: false,
                    ..sig(&[false])
                },
                &focus,
                vec![Frozen],
            ),
            (
                "focus rule: desktop focused plays",
                sig(&[false]),
                &focus,
                vec![Play],
            ),
            (
                "focus rule off by default",
                Signals {
                    desktop_focused: false,
                    ..sig(&[false])
                },
                &d,
                vec![Play],
            ),
            ("no displays, no decisions", sig(&[]), &d, vec![]),
        ];
        for (name, s, r, want) in cases {
            let states: Vec<State> = decide(&s, r).into_iter().map(|(st, _)| st).collect();
            assert_eq!(states, want, "case: {name}");
        }
        // The reason comes from the branch that decided, including when rules overlap.
        let why = |s: Signals, r: &Settings| {
            decide(&s, r)
                .into_iter()
                .map(|(_, w)| w)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            why(sig(&[true, false]), &d),
            vec![Reason::Covered, Reason::None]
        );
        assert_eq!(
            why(
                Signals {
                    on_battery: true,
                    ..sig(&[true, false])
                },
                &d
            ),
            vec![Reason::Covered, Reason::Battery]
        );
        assert_eq!(
            why(sig(&[true, false]), &per_all),
            vec![Reason::Covered, Reason::OtherCovered]
        );
        assert_eq!(
            why(
                Signals {
                    locked: true,
                    remote: true,
                    ..sig(&[false])
                },
                &d
            ),
            vec![Reason::Locked]
        );
        assert_eq!(
            why(
                Signals {
                    manual: Some(true),
                    ..sig(&[false])
                },
                &d
            ),
            vec![Reason::Manual]
        );
        assert_eq!(
            why(
                Signals {
                    foreground_app: Some("blender.exe".into()),
                    ..sig(&[false])
                },
                &rules
            ),
            vec![Reason::AppPause]
        );
    }
}
