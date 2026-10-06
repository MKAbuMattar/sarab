use super::*;

fn r(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

#[test]
fn tiled_windows_cover_a_display() {
    let work = r(0, 0, 1920, 1040);
    // One maximized window (its frame sits a few pixels outside the work area).
    assert!(covered(&work, &[r(-7, -7, 1927, 1047)]));
    // Two windows snapped left and right.
    assert!(covered(&work, &[r(0, 0, 960, 1040), r(960, 0, 1920, 1040)]));
    // Four quarters.
    let q = [
        r(0, 0, 960, 520),
        r(960, 0, 1920, 520),
        r(0, 520, 960, 1040),
        r(960, 520, 1920, 1040),
    ];
    assert!(covered(&work, &q));
    // Half the screen is still desktop.
    assert!(!covered(&work, &[r(0, 0, 960, 1040)]));
    // A window on the other display does not count.
    assert!(!covered(&work, &[r(1920, 0, 3840, 1040)]));
    // A strip of desktop left between two windows is seen.
    assert!(!covered(
        &work,
        &[r(0, 0, 800, 1040), r(1100, 0, 1920, 1040)]
    ));
    assert!(!covered(&work, &[]));
}

#[test]
fn shell_thumbnail_writes_png() {
    let d = std::env::temp_dir().join(format!("sarab-test-thumb-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    // A 64 by 32 24-bit BMP, written by hand so the test needs no fixture.
    let (w, h) = (64u32, 32u32);
    let row = (w * 3).div_ceil(4) * 4;
    let mut bmp = vec![];
    bmp.extend(b"BM");
    bmp.extend((54 + row * h).to_le_bytes());
    bmp.extend([0u8; 4]);
    bmp.extend(54u32.to_le_bytes());
    bmp.extend(40u32.to_le_bytes());
    bmp.extend(w.to_le_bytes());
    bmp.extend(h.to_le_bytes());
    bmp.extend(1u16.to_le_bytes());
    bmp.extend(24u16.to_le_bytes());
    bmp.extend([0u8; 24]);
    for y in 0..h {
        for x in 0..row {
            bmp.push(if x % 3 == 0 { 200 } else { (y * 7) as u8 });
        }
    }
    std::fs::write(d.join("pic.bmp"), &bmp).unwrap();
    let out = d.join("thumbnail.png");
    shell_thumbnail(&d.join("pic.bmp"), &out, 48).unwrap();
    let png = std::fs::read(&out).unwrap();
    assert!(png.starts_with(b"\x89PNG"), "a PNG");
    let small = d.join("small.png");
    shrink_png(&out, &small, 16).unwrap();
    let p = std::fs::read(&small).unwrap();
    // IHDR width, big-endian, at byte 16.
    assert_eq!(u32::from_be_bytes(p[16..20].try_into().unwrap()), 16);
    assert!(shell_thumbnail(&d.join("missing.mp4"), &out, 48).is_err());
}

#[test]
fn forward_only_over_desktop() {
    let r = |left, top, right, bottom| RECT {
        left,
        top,
        right,
        bottom,
    };
    let t = [(r(0, 0, 1920, 1080), 11), (r(1920, 0, 3840, 1080), 22)];
    assert_eq!(forward_target(100, 50, true, &t), Some((11, 100, 50)));
    assert_eq!(
        forward_target(2000, 70, true, &t),
        Some((22, 80, 70)),
        "second display, own coordinates"
    );
    assert_eq!(
        forward_target(100, 50, false, &t),
        None,
        "over an app window"
    );
    assert_eq!(
        forward_target(5000, 50, true, &t),
        None,
        "outside every wallpaper"
    );
    assert_eq!(
        forward_target(1920, 0, true, &t).map(|f| f.0),
        Some(22),
        "edges belong to the right"
    );
}

#[test]
fn screensaver_running_reads_the_real_state() {
    // Tests run while someone (or CI) works, so no screensaver is on screen.
    assert!(!screensaver_running());
}

#[test]
fn update_toast_has_buttons_and_escapes() {
    let x = update_toast_xml(
        "Sarab 0.0.6 is available",
        "Fixes & <more>",
        "Update now",
        "Later",
    );
    assert!(x.contains(r#"scenario="reminder""#), "stays until answered");
    assert!(x.contains(r#"launch="open""#));
    assert!(x.contains(r#"arguments="install""#) && x.contains(r#"arguments="later""#));
    assert!(x.contains("Fixes &amp; &lt;more&gt;"));
    let doc = windows::Data::Xml::Dom::XmlDocument::new().unwrap();
    doc.LoadXml(&HSTRING::from(x)).expect("well-formed XML");
}

#[test]
fn app_wallpaper_ends_with_its_job() {
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    let ping = std::path::Path::new(r"C:\Windows\System32\PING.EXE");
    let p = launch_app(ping, &["-n", "60", "127.0.0.1"]).unwrap();
    let pid = p.pid;
    let alive = || unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };
        let mut code = 0;
        let _ = GetExitCodeProcess(h, &mut code);
        let _ = CloseHandle(h);
        code == 259 // STILL_ACTIVE
    };
    assert!(alive(), "running");
    drop(p);
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(!alive(), "closing the job ends the program");
    assert!(launch_app(std::path::Path::new("C:/no/such.exe"), &[]).is_err());
}

#[test]
fn on_path_adds_once_and_removes_only_ours() {
    let dir = r"C:\Users\a\AppData\Local\Sarab";
    assert_eq!(path_with("", dir, true), dir);
    assert_eq!(
        path_with(r"%USERPROFILE%\bin;", dir, true),
        format!(r"%USERPROFILE%\bin;{dir};")
    );
    for orig in ["", r"C:\x", r"C:\x;", r"%USERPROFILE%\bin;;C:\y;"] {
        let back = path_with(&path_with(orig, dir, true), dir, false);
        assert_eq!(back, orig, "add then remove is exact");
    }
    let with = path_with(r"C:\x;;C:\y", dir, true);
    assert_eq!(with, format!(r"C:\x;;C:\y;{dir}"), "keeps empty entries");
    assert_eq!(path_with(&with, dir, true), with, "added once");
    assert_eq!(
        path_with(&with.to_uppercase(), dir, true),
        with.to_uppercase(),
        "case does not matter"
    );
    assert_eq!(
        path_with(&format!(r"{dir}\;C:\x"), dir, false),
        r"C:\x",
        "trailing slash still ours"
    );
    assert_eq!(path_with(&with, dir, false), r"C:\x;;C:\y");
    assert_eq!(
        path_with(r"C:\x;C:\Sarab2", r"C:\Sarab", false),
        r"C:\x;C:\Sarab2",
        "a longer name is not ours"
    );
    // The installer calls both, outside an update.
    let hooks = include_str!("../../../windows/hooks.nsh");
    assert!(hooks.contains("--add-to-path") && hooks.contains("--remove-from-path"));
}

#[test]
fn console_twin_only_flips_the_subsystem() {
    // This test binary is a console program (3); mark it windowed (2), then flip it back.
    let mut exe = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    let at = subsystem_at(&exe).unwrap();
    assert_eq!(u16::from_le_bytes([exe[at], exe[at + 1]]), 3);
    exe[at] = 2;
    let twin = console_twin(exe.clone()).unwrap();
    assert_eq!(u16::from_le_bytes([twin[at], twin[at + 1]]), 3);
    let changed = exe.iter().zip(&twin).filter(|(a, b)| a != b).count();
    assert_eq!(
        (changed, exe.len()),
        (1, twin.len()),
        "one byte differs, nothing else"
    );
    assert!(console_twin(b"MZ not a program".to_vec()).is_err());
    assert!(!is_sarab(std::process::id()), "this test is not sarab.exe");
}

#[test]
fn recycle_moves_to_bin() {
    // Leaves one small folder named sarab-test-recycle-<pid> in the Recycle Bin.
    let d = std::env::temp_dir().join(format!("sarab-test-recycle-{}", std::process::id()));
    std::fs::create_dir_all(d.join("inner")).unwrap();
    std::fs::write(d.join("inner").join("a.txt"), "x").unwrap();
    // remove() passes a canonicalized \\?\ path; the Delete button failed on exactly that.
    let canon = std::fs::canonicalize(&d).unwrap();
    assert!(canon.to_string_lossy().starts_with(r"\\?\"));
    recycle(&canon).unwrap();
    assert!(!d.exists());
    assert!(recycle(&d).is_err(), "nothing left to recycle");
}
