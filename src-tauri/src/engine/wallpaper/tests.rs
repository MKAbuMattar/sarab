use super::*;

#[test]
fn keep_frame_path_is_safe() {
    let p = frame_path(r"\\.\DISPLAY2");
    assert_eq!(p.file_name().unwrap(), "DISPLAY2.png");
    assert_eq!(p.parent().unwrap().file_name().unwrap(), "frames");
}

#[test]
fn screensaver_due_after_idle_unless_busy() {
    let min = 60_000;
    assert!(screensaver_due(5, 5 * min, false));
    assert!(!screensaver_due(5, 5 * min - 1, false), "too soon");
    assert!(!screensaver_due(0, 60 * min, false), "off");
    assert!(
        !screensaver_due(5, 60 * min, true),
        "a video or game keeps it away"
    );
}

#[test]
fn span_covers_all_displays() {
    let r = |left, top, right, bottom| RECT {
        left,
        top,
        right,
        bottom,
    };
    let both = span_rect(&[r(0, 0, 1920, 1080), r(-1080, 200, 0, 2120)]);
    assert_eq!(both, r(-1080, 0, 1920, 2120));
    assert_eq!(span_rect(&[r(0, 0, 10, 10)]), r(0, 0, 10, 10));
    let play = (State::Play, Reason::None);
    let covered = (State::Covered, Reason::Covered);
    let battery = (State::Frozen, Reason::Battery);
    assert_eq!(
        span_state(&[covered, play]),
        play,
        "one display still shows it"
    );
    assert_eq!(
        span_state(&[battery, covered]),
        battery,
        "all rest: the first reason"
    );
    assert_eq!(span_state(&[]), play);
}

#[test]
fn effective_volume_rules() {
    let s = |desk: bool, others: bool| Settings {
        volume: 60,
        audio_desktop_only: desk,
        audio_mute_others: others,
        ..Settings::default()
    };
    assert_eq!(
        effective_volume(&s(false, false), false, true),
        60,
        "no rule on"
    );
    assert_eq!(
        effective_volume(&s(true, false), true, false),
        60,
        "desktop shown"
    );
    assert_eq!(
        effective_volume(&s(true, false), false, false),
        0,
        "an app has focus"
    );
    assert_eq!(
        effective_volume(&s(false, true), false, true),
        0,
        "another app plays"
    );
    assert_eq!(
        effective_volume(&s(false, true), false, false),
        60,
        "silence elsewhere"
    );
    assert_eq!(effective_volume(&s(true, true), true, false), 60);
}

#[test]
fn should_unload_only_when_unseen() {
    use std::time::Duration;
    let m = |n: u64| Duration::from_secs(n * 60);
    assert!(should_unload(Reason::Covered, m(5), 5));
    assert!(should_unload(Reason::Locked, m(9), 5));
    assert!(should_unload(Reason::Remote, m(5), 5));
    assert!(!should_unload(Reason::Covered, m(4), 5), "too soon");
    assert!(!should_unload(Reason::Covered, m(60), 0), "off");
    for why in [
        Reason::Battery,
        Reason::PowerSaver,
        Reason::Focus,
        Reason::Manual,
        Reason::AppPause,
        Reason::OtherCovered,
        Reason::None,
    ] {
        assert!(!should_unload(why, m(60), 5), "{why:?}");
    }
}

#[test]
fn pick_next_order_random_category() {
    let ids: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
    assert_eq!(pick_next(&ids, Some("a"), false, 0).as_deref(), Some("b"));
    assert_eq!(
        pick_next(&ids, Some("c"), false, 0).as_deref(),
        Some("a"),
        "wraps"
    );
    assert_eq!(pick_next(&ids, None, false, 0).as_deref(), Some("a"));
    assert_eq!(
        pick_next(&ids, Some("gone"), false, 0).as_deref(),
        Some("a")
    );
    assert_eq!(pick_next(&[], None, true, 5), None);
    for cur in ["a", "b", "c"] {
        let seen: std::collections::BTreeSet<String> = (0..20)
            .filter_map(|seed| pick_next(&ids, Some(cur), true, seed))
            .collect();
        assert!(!seen.contains(cur), "{cur} repeated");
        assert_eq!(seen.len(), 2);
    }
    let one = vec!["a".to_string()];
    assert_eq!(
        pick_next(&one, Some("a"), true, 7).as_deref(),
        Some("a"),
        "only one to show"
    );
    let nature: Vec<String> = vec!["b".into()];
    assert_eq!(
        pick_next(&nature, Some("a"), false, 0).as_deref(),
        Some("b")
    );
}

#[test]
fn sync_groups_pick_shared_videos() {
    let d = |id: Option<&str>, ok: bool| (id.map(String::from), ok);
    let groups = sync_groups(
        vec![
            d(Some("a"), true),
            d(Some("b"), true),
            d(Some("a"), true),
            d(Some("a"), false),
        ]
        .into_iter(),
    );
    assert_eq!(groups, vec![vec![0, 2]]);
    assert!(sync_groups(vec![d(Some("a"), true), d(None, true)].into_iter()).is_empty());
}

#[test]
fn cycle_due_only_when_on_playing_and_seen() {
    use std::time::Duration;
    let m = |n: u64| Duration::from_secs(n * 60);
    assert!(cycle_due(15, m(15), true, false));
    assert!(cycle_due(15, m(40), true, false));
    assert!(!cycle_due(15, m(14), true, false), "too early");
    assert!(!cycle_due(0, m(999), true, false), "off");
    assert!(!cycle_due(15, m(15), false, false), "nothing playing");
    assert!(!cycle_due(15, m(15), true, true), "resting");
}

#[test]
fn youtube_links() {
    let v = |id: &str| Some((Some(id.to_string()), None));
    assert_eq!(
        youtube_ids("https://www.youtube.com/watch?v=aqz-KE-bpKQ"),
        v("aqz-KE-bpKQ")
    );
    assert_eq!(
        youtube_ids("https://youtube.com/watch?feature=share&v=aqz-KE-bpKQ&t=30"),
        v("aqz-KE-bpKQ")
    );
    assert_eq!(
        youtube_ids("https://youtu.be/aqz-KE-bpKQ?si=x"),
        v("aqz-KE-bpKQ")
    );
    assert_eq!(
        youtube_ids("https://m.youtube.com/watch?v=aqz-KE-bpKQ"),
        v("aqz-KE-bpKQ")
    );
    assert_eq!(
        youtube_ids("https://www.youtube.com/shorts/aqz-KE-bpKQ"),
        v("aqz-KE-bpKQ")
    );
    assert_eq!(
        youtube_ids("https://www.youtube.com/live/jfKfPfyJRdk?si=y"),
        v("jfKfPfyJRdk")
    );
    assert_eq!(
        youtube_ids("https://www.youtube.com/embed/aqz-KE-bpKQ"),
        v("aqz-KE-bpKQ")
    );
    assert_eq!(
        youtube_ids("https://www.youtube.com/playlist?list=PL123_ab-C"),
        Some((None, Some("PL123_ab-C".into())))
    );
    assert_eq!(
        youtube_ids("https://www.youtube.com/watch?v=aqz-KE-bpKQ&list=PL123"),
        Some((Some("aqz-KE-bpKQ".into()), Some("PL123".into())))
    );
    assert_eq!(youtube_ids("https://www.youtube.com/@blender"), None);
    assert_eq!(
        youtube_ids("https://notyoutube.com/watch?v=aqz-KE-bpKQ"),
        None
    );
    assert_eq!(youtube_ids("https://example.com/youtu.be/x"), None);
    assert_eq!(youtube_ids("https://youtu.be/abc\"><script>"), v("abc"));
    assert_eq!(
        rewrite_url("https://youtu.be/aqz-KE-bpKQ"),
        "http://tauri.localhost/youtube.html?v=aqz-KE-bpKQ"
    );
    assert_eq!(rewrite_url("https://example.com/"), "https://example.com/");
}
