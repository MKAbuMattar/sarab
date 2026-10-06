use super::*;
use std::io::Write;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("sarab-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn make_zip(path: &Path, entries: &[(&str, &str)]) {
    let mut z = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for (name, body) in entries {
        z.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(body.as_bytes()).unwrap();
    }
    z.finish().unwrap();
}

#[test]
fn zip_slip_rejected() {
    let d = tmp("slip");
    let lib = d.join("lib");
    let z = d.join("evil.zip");
    make_zip(
        &z,
        &[
            ("sarab.json", r#"{"type":"web","file":"index.html"}"#),
            ("../evil.txt", "x"),
        ],
    );
    let err = import_zip(&lib, &z, MAX_UNPACKED).unwrap_err();
    assert!(err.contains("unsafe path"), "{err}");
    assert!(!d.join("evil.txt").exists());
    assert!(scan(&lib).is_empty());
}

#[test]
fn zip_size_cap() {
    let d = tmp("cap");
    let z = d.join("big.zip");
    make_zip(
        &z,
        &[
            ("sarab.json", r#"{"type":"web"}"#),
            ("a.bin", &"x".repeat(5000)),
        ],
    );
    assert!(import_zip(&d.join("lib"), &z, 1000)
        .unwrap_err()
        .contains("limit"));
}

#[test]
fn package_zip_round_trip() {
    let d = tmp("round");
    let lib = d.join("lib");
    let z = d.join("Rain.zip");
    let info = r#"{"title":"Rain","description":"d","author":"a","type":"web","file":"index.html","tags":["x"],"version":3}"#;
    make_zip(
        &z,
        &[
            ("sarab.json", info),
            ("index.html", "<p>hi</p>"),
            ("js/app.js", "1"),
            (
                "properties.json",
                r#"{"speed":{"type":"slider","value":1,"min":0,"max":5,"step":0.1,"text":"Speed"}}"#,
            ),
        ],
    );
    let w = import_zip(&lib, &z, MAX_UNPACKED).unwrap();
    assert_eq!(w.info.title.as_deref(), Some("Rain"));
    assert_eq!(w.info.r#type, Kind::Web);
    assert_eq!(w.info.version, 3);
    assert!(w.has_props);
    assert!(w.dir.join("js/app.js").is_file());
    let back = serde_json::to_value(&w.info).unwrap();
    assert_eq!(back["type"], "web");
    let scanned = scan(&lib);
    assert_eq!(scanned.len(), 1);
    assert_eq!(scanned[0].info, w.info);
    assert!(serde_json::from_str::<Manifest>(r#"{"type":"unity"}"#).is_err());
    let other = d.join("other.zip");
    make_zip(&other, &[("index.html", "x")]);
    assert!(import_zip(&lib, &other, MAX_UNPACKED)
        .unwrap_err()
        .contains("sarab.json"));
}

#[test]
fn property_merge_and_coerce() {
    let d = tmp("props");
    fs::write(d.join(INFO), r#"{"type":"web","file":"index.html"}"#).unwrap();
    fs::write(d.join(PROPS), r#"{"zeta":{"type":"slider","value":1,"min":0,"max":5,"step":1,"text":"Z"},"alpha":{"type":"checkbox","value":false,"text":"A"},"pick":{"type":"dropdown","value":0,"items":["a","b"],"text":"P"},"go":{"type":"button","value":"Go","text":"Go"}}"#).unwrap();
    let w = read(&d).unwrap();
    let saved = d.join("saved.json");
    let p = props(&w, &saved);
    assert_eq!(
        p.keys().collect::<Vec<_>>(),
        vec!["zeta", "alpha", "pick", "go"]
    );
    let v = coerce(&p["zeta"], &Value::String("9".into())).unwrap();
    assert_eq!(v, serde_json::json!(5.0));
    save_prop(&saved, &p["zeta"], "zeta", &v).unwrap();
    save_prop(&saved, &p["go"], "go", &Value::Bool(true)).unwrap();
    let p2 = props(&w, &saved);
    assert_eq!(p2["zeta"]["value"], serde_json::json!(5.0));
    assert_eq!(p2["go"]["value"], "Go");
    assert!(coerce(&p["alpha"], &Value::String("yes".into())).is_err());
    assert!(coerce(&p["pick"], &Value::String("2".into())).is_err());
    assert_eq!(
        coerce(&p["pick"], &Value::String("1".into())).unwrap(),
        serde_json::json!(1)
    );
}

#[test]
fn needs_convert_then_converts_once() {
    assert!(
        needs_convert("C:/clips/Rain.AVI") && needs_convert("a.wmv") && needs_convert("a.mpeg")
    );
    assert!(
        !needs_convert("a.mp4")
            && !needs_convert("a.webm")
            && !needs_convert("https://x.com/a.avi")
    );
    let d = tmp("convert");
    let avi = d.join("Rain.avi");
    let made = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=65x49:rate=5",
            "-t",
            "1",
            "-c:v",
            "mpeg4",
        ])
        .arg(&avi)
        .status();
    if !made.is_ok_and(|s| s.success()) {
        eprintln!("ffmpeg not installed: conversion not tested");
        return;
    }
    let lib = d.join("lib");
    let mp4 = convert(&lib, &avi).unwrap();
    assert_eq!(
        mp4.file_name().unwrap(),
        "Rain.mp4",
        "keeps the name, so the title reads well"
    );
    assert!(fs::metadata(&mp4).unwrap().len() > 0);
    let when = fs::metadata(&mp4).unwrap().modified().unwrap();
    assert_eq!(convert(&lib, &avi).unwrap(), mp4);
    assert_eq!(
        fs::metadata(&mp4).unwrap().modified().unwrap(),
        when,
        "second add reuses it"
    );
    assert!(
        scan(&lib).is_empty(),
        "the converted folder is not a wallpaper"
    );
    assert!(convert(&lib, &d.join("missing.avi")).is_err());
}

#[test]
fn add_wallpaper_engine_folder() {
    let d = tmp("we-add");
    let src = d.join("431960").join("2911");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("project.json"),
        r#"{"title":"Rain","type":"web","file":"index.html","general":{"properties":{"speed":{"type":"slider","value":2,"min":0,"max":5,"text":"Speed"}}}}"#,
    )
    .unwrap();
    fs::write(src.join("index.html"), "<p>rain</p>").unwrap();
    let lib = d.join("lib");
    let w = add(&lib, &[], src.join("project.json").to_str().unwrap()).unwrap();
    assert_eq!(w.info.title.as_deref(), Some("Rain"));
    assert_eq!(w.info.r#type, Kind::Web);
    assert!(w.has_props && w.dir.join("index.html").is_file());
    assert!(
        !src.join(INFO).exists(),
        "the original folder is left as it was"
    );
    let again = add(&lib, &scan(&lib), src.to_str().unwrap()).unwrap();
    assert_eq!(again.id, w.id, "same folder is imported once");
    assert!(
        add(&lib, &[], d.to_str().unwrap()).is_err(),
        "a folder with neither file"
    );
}

#[test]
fn add_file_and_url() {
    let d = tmp("add");
    let lib = d.join("lib");
    let f = d.join("clip.mp4");
    fs::write(&f, "x").unwrap();
    let w = add(&lib, &[], f.to_str().unwrap()).unwrap();
    assert_eq!(w.info.r#type, Kind::Video);
    assert!(w.info.external);
    let again = add(&lib, &scan(&lib), f.to_str().unwrap()).unwrap();
    assert_eq!(again.id, w.id, "same file is not added twice");
    let u = add(&lib, &[], "https://www.shadertoy.com/view/abc").unwrap();
    assert_eq!(u.info.r#type, Kind::Url);
    assert!(matches!(u.target(), Some(Target::Url(_))));
    assert!(add(&lib, &[], "C:/x.docx").is_err());
    let src = d.join("Rain");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join(INFO), r#"{"type":"web","file":"index.html"}"#).unwrap();
    let a = add(&lib, &scan(&lib), src.to_str().unwrap()).unwrap();
    let b = add(&lib, &scan(&lib), src.to_str().unwrap()).unwrap();
    assert_eq!(a.id, b.id, "same folder is copied once");
    assert_eq!(
        scan(&lib)
            .iter()
            .filter(|w| w.id.starts_with("Rain-"))
            .count(),
        1
    );
}

#[test]
fn move_library_moves_every_package() {
    let from = tmp("move-from");
    let to = tmp("move-to").join("Library");
    for name in ["one", "two"] {
        fs::create_dir_all(from.join(name)).unwrap();
        fs::write(
            from.join(name).join(INFO),
            r#"{"type":"web","file":"index.html"}"#,
        )
        .unwrap();
        fs::write(from.join(name).join("index.html"), name).unwrap();
    }
    fs::write(from.join("not-a-package.txt"), "stays").unwrap();
    assert_eq!(move_library(&from, &to).unwrap(), 2);
    assert_eq!(scan(&to).len(), 2);
    assert!(scan(&from).is_empty());
    assert_eq!(
        fs::read_to_string(to.join("two").join("index.html")).unwrap(),
        "two"
    );
    assert!(
        from.join("not-a-package.txt").exists(),
        "only packages move"
    );

    assert!(move_library(&to, &to.join("inner")).is_err());
    fs::create_dir_all(from.join("one")).unwrap();
    fs::write(
        from.join("one").join(INFO),
        r#"{"type":"web","file":"index.html"}"#,
    )
    .unwrap();
    assert!(move_library(&from, &to).is_err());
    assert!(from.join("one").join(INFO).exists());
}

#[test]
fn export_round_trip() {
    let lib = tmp("export-lib");
    let out = tmp("export-out");
    let pkg = lib.join("scene");
    fs::create_dir_all(pkg.join("img")).unwrap();
    fs::write(pkg.join(INFO), r#"{"title":"Scene: night/day","type":"web","file":"index.html","category":"city","tags":["night"]}"#).unwrap();
    fs::write(pkg.join("index.html"), "<p>hi</p>").unwrap();
    fs::write(pkg.join("img").join("a.png"), "png").unwrap();
    fs::write(pkg.join(PROPS), "{}").unwrap();
    let zip = export_zip(&read(&pkg).unwrap(), &out).unwrap();
    assert!(zip
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("Scene_ night_day"));
    let back = import_zip(&out, &zip, MAX_UNPACKED).unwrap();
    assert_eq!(back.info.title.as_deref(), Some("Scene: night/day"));
    assert_eq!(back.info.category.as_deref(), Some("city"));
    assert!(back.has_props);
    assert_eq!(
        fs::read_to_string(back.dir.join("img").join("a.png")).unwrap(),
        "png"
    );

    let video = out.join("clip.mp4");
    fs::write(&video, "video bytes").unwrap();
    let ext = add(&lib, &[], video.to_str().unwrap()).unwrap();
    assert!(ext.info.external);
    let zip = export_zip(&ext, &out).unwrap();
    let other = tmp("export-other");
    let back = import_zip(&other, &zip, MAX_UNPACKED).unwrap();
    assert!(!back.info.external);
    assert_eq!(back.info.file.as_deref(), Some("clip.mp4"));
    assert_eq!(
        fs::read_to_string(back.dir.join("clip.mp4")).unwrap(),
        "video bytes"
    );
}

#[test]
fn too_new_packages_are_flagged() {
    assert!(newer("0.0.10", "0.0.9"));
    assert!(newer("v1.0.0", "0.9.9"));
    assert!(!newer("0.0.5", "0.0.5"));
    assert!(!newer("0.0.4", "0.0.5"));
    let d = tmp("too-new");
    fs::write(
        d.join(INFO),
        r#"{"type":"web","file":"index.html","app_version":"99.0.0"}"#,
    )
    .unwrap();
    assert!(read(&d).unwrap().too_new);
    fs::write(
        d.join(INFO),
        r#"{"type":"web","file":"index.html","app_version":"0.0.1"}"#,
    )
    .unwrap();
    assert!(!read(&d).unwrap().too_new);
    fs::write(d.join(INFO), r#"{"type":"web","file":"index.html"}"#).unwrap();
    assert!(!read(&d).unwrap().too_new, "no version: assume it fits");
}

#[test]
fn number_and_password_fields() {
    let num = serde_json::json!({"type": "number", "value": 3, "min": 1, "max": 10});
    assert_eq!(
        coerce(&num, &Value::String(" 7 ".into())).unwrap(),
        serde_json::json!(7.0)
    );
    assert_eq!(
        coerce(&num, &Value::String("50".into())).unwrap(),
        serde_json::json!(10.0),
        "clamped"
    );
    assert!(coerce(&num, &Value::String("seven".into())).is_err());
    assert!(coerce(&num, &Value::String("NaN".into())).is_err());
    let pw = serde_json::json!({"type": "password", "value": ""});
    assert_eq!(
        coerce(&pw, &Value::String("s3cret !".into())).unwrap(),
        Value::String("s3cret !".into())
    );
}

#[test]
fn reset_props_restores_defaults() {
    let d = tmp("reset");
    fs::write(d.join(INFO), r#"{"type":"web","file":"index.html"}"#).unwrap();
    fs::write(
        d.join(PROPS),
        r#"{"speed":{"type":"slider","value":1,"min":0,"max":5,"step":1}}"#,
    )
    .unwrap();
    let w = read(&d).unwrap();
    let saved = d.join("saved.json");
    save_prop(
        &saved,
        &props(&w, &saved)["speed"],
        "speed",
        &serde_json::json!(4.0),
    )
    .unwrap();
    assert_eq!(props(&w, &saved)["speed"]["value"], serde_json::json!(4.0));
    reset_props(&saved).unwrap();
    assert_eq!(props(&w, &saved)["speed"]["value"], serde_json::json!(1));
    reset_props(&saved).unwrap();
}

#[test]
fn edit_info_validates_and_saves() {
    let d = tmp("edit");
    fs::write(
        d.join(INFO),
        r#"{"title":"Old","type":"video","file":"a.mp4","version":2}"#,
    )
    .unwrap();
    let edit = |title: &str, category: Option<&str>, tags: &[&str]| Edit {
        title: title.into(),
        description: "  Waves at dusk  ".into(),
        author: "".into(),
        category: category.map(String::from),
        tags: tags.iter().map(|t| t.to_string()).collect(),
    };
    let info = edit_info(
        &d,
        edit("  Sea  ", Some("nature"), &["sea", " Sea ", "dusk", ""]),
    )
    .unwrap();
    assert_eq!(info.title.as_deref(), Some("Sea"));
    assert_eq!(info.description.as_deref(), Some("Waves at dusk"));
    assert_eq!(info.author, None);
    assert_eq!(info.category.as_deref(), Some("nature"));
    assert_eq!(
        info.tags,
        vec!["sea", "dusk"],
        "trimmed, empty dropped, duplicates folded"
    );
    assert_eq!(info.version, 3);
    assert_eq!(read(&d).unwrap().info, info, "written to sarab.json");

    assert!(
        edit_info(&d, edit("   ", None, &[])).is_err(),
        "empty title"
    );
    assert!(
        edit_info(&d, edit(&"x".repeat(101), None, &[])).is_err(),
        "title too long"
    );
    assert!(
        edit_info(&d, edit("Sea", Some("cats"), &[])).is_err(),
        "unknown category"
    );
    assert!(
        edit_info(&d, edit("Sea", None, &["a", "b", "c", "d", "e", "f"])).is_err(),
        "six tags"
    );
    assert!(
        edit_info(&d, edit("Sea", None, &[&"t".repeat(21)])).is_err(),
        "long tag"
    );
    assert_eq!(
        read(&d).unwrap().info.version,
        3,
        "a refused edit writes nothing"
    );
    assert!(
        edit_info(&d, edit("سراب", Some(""), &[])).is_ok(),
        "Arabic title, no category"
    );
}

#[test]
fn details_report_folder_facts() {
    let d = tmp("details");
    fs::write(
        d.join(INFO),
        r#"{"title":"T","type":"web","file":"index.html"}"#,
    )
    .unwrap();
    fs::write(d.join("index.html"), "0123456789").unwrap();
    fs::create_dir_all(d.join("img")).unwrap();
    fs::write(d.join("img").join("a.png"), "12345").unwrap();
    fs::write(d.join(PROPS), "{}").unwrap();
    let w = read(&d).unwrap();
    let info_len = fs::metadata(d.join(INFO)).unwrap().len();
    let det = details(&w);
    assert_eq!(det.files, 4);
    assert_eq!(det.size, 10 + 5 + 2 + info_len);
    assert!(det.has_props);
    assert!(det.modified > 0 && det.created > 0);
    assert!(det.source.ends_with("index.html"));

    let url = tmp("details-url");
    fs::write(
        url.join(INFO),
        r#"{"title":"U","type":"url","file":"https://example.com/"}"#,
    )
    .unwrap();
    assert_eq!(details(&read(&url).unwrap()).source, "https://example.com/");
}
