use dz_core::db::Database;
use dz_core::settings::{self, Density, PerfMode, Settings};

#[test]
fn defaults_are_returned_when_nothing_is_stored() {
    let db = Database::open_in_memory().unwrap();
    let s = settings::load(&db).unwrap();
    assert_eq!(s, Settings::default());
    assert_eq!(s.performance.mode, PerfMode::Balanced);
}

#[test]
fn saved_settings_persist_across_reopen() {
    let dir = std::env::temp_dir().join(format!("dz-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("dizero.db");
    {
        let db = Database::open(&path).unwrap().db;
        let mut s = Settings::default();
        s.performance.mode = PerfMode::Eco;
        s.appearance.density = Density::Compact;
        s.profile.display_name = "Ada".into();
        settings::save(&db, s).unwrap();
    }
    let db = Database::open(&path).unwrap().db;
    let s = settings::load(&db).unwrap();
    assert_eq!(s.performance.mode, PerfMode::Eco);
    assert_eq!(s.appearance.density, Density::Compact);
    assert_eq!(s.profile.display_name, "Ada");
}

#[test]
fn saving_normalizes_out_of_range_values() {
    let db = Database::open_in_memory().unwrap();
    let mut s = Settings::default();
    s.appearance.font_scale = 9.0;
    s.appearance.accent = Some("javascript:alert(1)".into());
    s.appearance.sidebar_width = 5;
    s.profile.display_name = "   ".into();
    let stored = settings::save(&db, s).unwrap();
    assert_eq!(stored.appearance.font_scale, 1.3);
    assert_eq!(stored.appearance.accent, None);
    assert_eq!(stored.appearance.sidebar_width, 200);
    assert_eq!(stored.profile.display_name, "You");
}

#[test]
fn valid_accent_colors_are_kept() {
    let db = Database::open_in_memory().unwrap();
    let mut s = Settings::default();
    s.appearance.accent = Some("#A48DFF".into());
    let stored = settings::save(&db, s).unwrap();
    assert_eq!(stored.appearance.accent.as_deref(), Some("#a48dff"));
}

#[test]
fn unknown_or_partial_stored_json_falls_back_to_defaults_per_field() {
    let db = Database::open_in_memory().unwrap();
    db.with(|c| {
        c.execute(
            "INSERT INTO app_settings(key, value, updated_at) VALUES ('settings', ?1, 0)",
            [r#"{"performance":{"mode":"performance"},"futureField":true}"#],
        )?;
        Ok(())
    })
    .unwrap();
    let s = settings::load(&db).unwrap();
    assert_eq!(s.performance.mode, PerfMode::Performance);
    assert_eq!(s.appearance, Settings::default().appearance);
}

#[test]
fn export_then_import_round_trips() {
    let mut s = Settings::default();
    s.appearance.high_contrast = true;
    let json = settings::export_json(&s).unwrap();
    let back = settings::import_json(&json).unwrap();
    assert_eq!(back, s);
}

#[test]
fn import_rejects_non_settings_json() {
    assert!(settings::import_json("not json").is_err());
    assert!(settings::import_json("[1,2,3]").is_err());
}

#[test]
fn hardware_acceleration_defaults_on_and_persists_off() {
    assert!(Settings::default().performance.hardware_acceleration);
    let db = Database::open_in_memory().unwrap();
    let mut s = Settings::default();
    s.performance.hardware_acceleration = false;
    settings::save(&db, s).unwrap();
    assert!(!settings::load(&db).unwrap().performance.hardware_acceleration);
}

#[test]
fn browser_args_reflect_hardware_acceleration() {
    let on = settings::webview_browser_args(true);
    let off = settings::webview_browser_args(false);
    // wry's defaults must be preserved when we supply our own arguments.
    for a in [&on, &off] {
        assert!(a.contains("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection"));
        assert!(a.contains("NetworkServiceInProcess2"));
    }
    assert!(!on.contains("--disable-gpu"));
    assert!(off.contains("--disable-gpu"));
}

#[test]
fn perf_profiles_scale_with_mode() {
    let eco = PerfMode::Eco.profile();
    let bal = PerfMode::Balanced.profile();
    let perf = PerfMode::Performance.profile();
    assert!(eco.message_window < bal.message_window);
    assert!(bal.message_window < perf.message_window);
    assert!(!eco.animations);
    assert!(eco.plugin_idle_unload_secs.is_some());
    assert!(perf.plugin_idle_unload_secs.is_none());
    assert!(perf.preload_adjacent && !eco.preload_adjacent);
}
