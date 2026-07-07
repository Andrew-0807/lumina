mod display;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

use tauri::{AppHandle, Manager, State, Emitter};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};

// DisplaySettings mapping from Conceptual Model
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DisplaySettings {
    pub resolution: Option<display::Resolution>,
    pub vibrance: Option<i32>, // 0 - 100%
    pub gamma: Option<f32>,     // 0.5 - 3.0
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppProfile {
    pub executable_name: String,
    pub friendly_name: String,
    pub is_enabled: bool,
    pub settings: HashMap<String, DisplaySettings>, // display_id -> settings
    pub hotkey: Option<String>,
    pub is_default: Option<bool>,
}

// Config file wrapping
#[derive(Serialize, Deserialize, Clone, Debug)]
struct AppConfig {
    pub profiles: Vec<AppProfile>,
    pub is_daemon_enabled: bool,
    pub reset_hotkey: String,
    pub daemon_hotkey: String,
    pub default_profile_name: Option<String>,
    #[serde(default)]
    pub stealth_detection: bool,
}

#[derive(Clone, Debug)]
pub struct DefaultSettings {
    pub resolution: display::Resolution,
    pub vibrance: i32,
    pub gamma: f32,
}

pub struct AppState {
    pub config_path: PathBuf,
    pub profiles: Vec<AppProfile>,
    pub is_daemon_enabled: bool,
    pub reset_hotkey: String,
    pub daemon_hotkey: String,
    pub default_profile_name: Option<String>,
    pub system_defaults: HashMap<String, DefaultSettings>,
    pub active_profile: Option<String>, // Executable name
    pub active_is_manual: bool, // true if pinned by hotkey/reset; daemon must not auto-revert it
    pub stealth_detection: bool, // resolve foreground exe via snapshot, no OpenProcess on the game
    pub hotkeys_thread_id: Option<u32>,
}

// Thread-safe state wrapper
pub struct SharedState(pub Arc<Mutex<AppState>>);

// Helper to get config path
fn get_config_path(app: &AppHandle) -> PathBuf {
    // Resolve app config directory in Tauri v2
    let mut path = app.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
    let _ = std::fs::create_dir_all(&path);
    path.push("config.json");
    path
}

// Load config from file
fn load_config(path: &PathBuf) -> (Vec<AppProfile>, bool, String, String, Option<String>, bool) {
    if path.exists() {
        if let Ok(mut file) = File::open(path) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&contents) {
                    let profiles = config.get("profiles")
                        .and_then(|v| serde_json::from_value::<Vec<AppProfile>>(v.clone()).ok())
                        .unwrap_or_default();
                    let is_daemon_enabled = config.get("is_daemon_enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    let reset_hotkey = config.get("reset_hotkey")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Ctrl+Alt+R")
                        .to_string();
                    let daemon_hotkey = config.get("daemon_hotkey")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Ctrl+Alt+D")
                        .to_string();
                    let default_profile_name = config.get("default_profile_name")
                        .and_then(|v| serde_json::from_value::<Option<String>>(v.clone()).ok())
                        .flatten();
                    let stealth_detection = config.get("stealth_detection")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    return (profiles, is_daemon_enabled, reset_hotkey, daemon_hotkey, default_profile_name, stealth_detection);
                }
            }
        }
    }
    (Vec::new(), true, "Ctrl+Alt+R".to_string(), "Ctrl+Alt+D".to_string(), None, false)
}

// Save config to file
fn save_config(
    path: &PathBuf,
    profiles: &Vec<AppProfile>,
    is_daemon_enabled: bool,
    reset_hotkey: &str,
    daemon_hotkey: &str,
    default_profile_name: Option<String>,
    stealth_detection: bool,
) {
    let config = AppConfig {
        profiles: profiles.clone(),
        is_daemon_enabled,
        reset_hotkey: reset_hotkey.to_string(),
        daemon_hotkey: daemon_hotkey.to_string(),
        default_profile_name,
        stealth_detection,
    };
    if let Ok(contents) = serde_json::to_string_pretty(&config) {
        if let Ok(mut file) = File::create(path) {
            let _ = file.write_all(contents.as_bytes());
        }
    }
}

// ponytail: Helper to parse shortcut strings into Win32 modifiers and virtual key codes natively.
fn parse_hotkey(hotkey_str: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = hotkey_str.split('+').map(|s| s.trim()).collect();
    let mut modifiers = 0u32;
    let mut vk = 0u32;

    for part in parts {
        let part_lower = part.to_lowercase();
        match part_lower.as_str() {
            "ctrl" | "control" => modifiers |= 0x0002, // MOD_CONTROL
            "alt" => modifiers |= 0x0001, // MOD_ALT
            "shift" => modifiers |= 0x0004, // MOD_SHIFT
            "win" | "super" => modifiers |= 0x0008, // MOD_WIN
            other => {
                if other.len() == 1 {
                    let c = other.chars().next().unwrap();
                    if c.is_ascii_alphabetic() {
                        vk = c.to_ascii_uppercase() as u32;
                    } else if c.is_ascii_digit() {
                        vk = c as u32;
                    }
                } else if other.starts_with('f') {
                    if let Ok(num) = other[1..].parse::<u32>() {
                        if num >= 1 && num <= 12 {
                            vk = 0x6F + num; // VK_F1 is 0x70
                        }
                    }
                }
            }
        }
    }

    if vk != 0 {
        Some((modifiers, vk))
    } else {
        None
    }
}

// Tauri commands
#[tauri::command]
fn get_displays() -> Vec<display::DisplayInfo> {
    display::get_connected_displays()
}

#[tauri::command]
fn apply_vibrance(display_id: String, vibrance_percent: i32) -> bool {
    display::apply_vibrance(&display_id, vibrance_percent)
}

#[tauri::command]
fn apply_gamma(display_id: String, gamma: f32) -> bool {
    display::apply_gamma(&display_id, gamma)
}

#[tauri::command]
fn apply_resolution(display_id: String, width: u32, height: u32, refresh_rate: u32) -> bool {
    display::apply_resolution(&display_id, width, height, refresh_rate)
}

#[tauri::command]
fn get_profiles(state: State<'_, SharedState>) -> Vec<AppProfile> {
    let s = state.0.lock().unwrap();
    s.profiles.clone()
}

#[derive(Serialize, Deserialize)]
pub struct GlobalSettings {
    pub reset_hotkey: String,
    pub daemon_hotkey: String,
    #[serde(default)]
    pub stealth_detection: bool,
}

#[tauri::command]
fn get_global_settings(state: State<'_, SharedState>) -> GlobalSettings {
    let s = state.0.lock().unwrap();
    GlobalSettings {
        reset_hotkey: s.reset_hotkey.clone(),
        daemon_hotkey: s.daemon_hotkey.clone(),
        stealth_detection: s.stealth_detection,
    }
}

#[tauri::command]
fn save_global_settings(state: State<'_, SharedState>, settings: GlobalSettings) -> bool {
    let mut s = state.0.lock().unwrap();
    s.reset_hotkey = settings.reset_hotkey;
    s.daemon_hotkey = settings.daemon_hotkey;
    s.stealth_detection = settings.stealth_detection;

    let profiles_clone = s.profiles.clone();
    let daemon_enabled = s.is_daemon_enabled;
    let reset_hotkey = s.reset_hotkey.clone();
    let daemon_hotkey = s.daemon_hotkey.clone();
    let default_profile_name = s.default_profile_name.clone();
    save_config(&s.config_path, &profiles_clone, daemon_enabled, &reset_hotkey, &daemon_hotkey, default_profile_name, s.stealth_detection);
    
    // Post thread message to reload hotkeys
    if let Some(tid) = s.hotkeys_thread_id {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(tid, 0x0400 + 1, 0, 0);
        }
    }
    true
}

#[tauri::command]
fn save_profiles(state: State<'_, SharedState>, profiles: Vec<AppProfile>) -> bool {
    let mut s = state.0.lock().unwrap();
    
    s.default_profile_name = profiles.iter()
        .find(|p| p.is_default.unwrap_or(false))
        .map(|p| p.friendly_name.clone());
        
    s.profiles = profiles;
    
    let profiles_clone = s.profiles.clone();
    let daemon_enabled = s.is_daemon_enabled;
    let reset_hotkey = s.reset_hotkey.clone();
    let daemon_hotkey = s.daemon_hotkey.clone();
    let default_profile_name = s.default_profile_name.clone();
    save_config(&s.config_path, &profiles_clone, daemon_enabled, &reset_hotkey, &daemon_hotkey, default_profile_name, s.stealth_detection);
    
    // Post thread message to reload hotkeys
    if let Some(tid) = s.hotkeys_thread_id {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(tid, 0x0400 + 1, 0, 0);
        }
    }
    true
}

#[tauri::command]
fn is_daemon_active(state: State<'_, SharedState>) -> bool {
    let s = state.0.lock().unwrap();
    s.is_daemon_enabled
}

#[tauri::command]
fn set_daemon_active(state: State<'_, SharedState>, active: bool) -> bool {
    let mut s = state.0.lock().unwrap();
    s.is_daemon_enabled = active;
    
    let profiles_clone = s.profiles.clone();
    let reset_hotkey = s.reset_hotkey.clone();
    let daemon_hotkey = s.daemon_hotkey.clone();
    let default_profile_name = s.default_profile_name.clone();
    save_config(&s.config_path, &profiles_clone, s.is_daemon_enabled, &reset_hotkey, &daemon_hotkey, default_profile_name, s.stealth_detection);
    true
}

fn apply_profile_settings_internal(
    system_defaults: &mut HashMap<String, DefaultSettings>,
    display_id: &str,
    settings: &DisplaySettings,
) -> bool {
    if !system_defaults.contains_key(display_id) {
        let displays = display::get_connected_displays();
        if let Some(d) = displays.iter().find(|x| x.id == display_id) {
            // current_vibrance is a raw NVAPI DVC level, but apply_vibrance() expects a
            // 0-100 percent. Convert here so reset round-trips instead of collapsing to ~8%.
            let range = (d.max_vibrance - d.min_vibrance).max(1);
            let vibrance_pct = ((d.current_vibrance - d.min_vibrance) * 100) / range;
            system_defaults.insert(display_id.to_string(), DefaultSettings {
                resolution: d.current_resolution.clone(),
                vibrance: vibrance_pct,
                gamma: 1.0,
            });
        }
    }

    let mut success = true;
    let mut log = format!("apply display_id={display_id}");
    if let Some(r) = &settings.resolution {
        let ok = display::apply_resolution(display_id, r.width, r.height, r.refresh_rate);
        log.push_str(&format!(" res={}x{}@{}->{}", r.width, r.height, r.refresh_rate, ok));
        success &= ok;
    }
    if let Some(v) = settings.vibrance {
        let ok = display::apply_vibrance(display_id, v);
        log.push_str(&format!(" vibrance={v}->{ok}"));
        success &= ok;
    }
    if let Some(g) = settings.gamma {
        let ok = display::apply_gamma(display_id, g);
        log.push_str(&format!(" gamma={g}->{ok}"));
        success &= ok;
    }
    debug_log(&log);
    success
}

// ponytail: temp-file debug log so the packaged app (no console) can be diagnosed.
// Remove once the per-monitor apply issue is resolved.
fn debug_log(msg: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("lumina-debug.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{msg}");
    }
}

// Apply every display in a profile with a short settle gap between monitors.
// Firing display driver calls (mode-set / gamma / NVAPI vibrance) back-to-back
// on one thread makes the driver silently drop the change for the second
// monitor; the manual apply path avoids this only because its per-display async
// IPC round-trips space the calls out. This reproduces that spacing.
fn apply_profile_all_displays(
    source: &str,
    system_defaults: &mut HashMap<String, DefaultSettings>,
    settings: &HashMap<String, DisplaySettings>,
) -> bool {
    let keys: Vec<&String> = settings.keys().collect();
    debug_log(&format!("apply_all source={source} keys={keys:?}"));

    let mut success = true;
    for (i, (display_id, s)) in settings.iter().enumerate() {
        if i > 0 {
            std::thread::sleep(std::time::Duration::from_millis(80));
        }
        let ok = apply_profile_settings_internal(system_defaults, display_id, s);
        if !ok {
            println!("[Apply] Display {display_id} failed; retrying once.");
            std::thread::sleep(std::time::Duration::from_millis(120));
            success &= apply_profile_settings_internal(system_defaults, display_id, s);
        }
    }
    success
}

#[tauri::command]
fn trigger_manual_apply(state: State<'_, SharedState>, display_id: String, settings: DisplaySettings) -> bool {
    let mut s = state.0.lock().unwrap();
    apply_profile_settings_internal(&mut s.system_defaults, &display_id, &settings)
}

// Reset target: re-apply the user's designated default profile if one exists,
// otherwise fall back to the captured Windows defaults.
fn restore_baseline(s: &mut AppState, app_handle: &AppHandle) -> bool {
    let default_p = s.profiles.iter()
        .find(|p| p.is_default.unwrap_or(false))
        .cloned();

    if let Some(dp) = default_p {
        let success = apply_profile_all_displays("restore_baseline_default", &mut s.system_defaults, &dp.settings);
        s.active_profile = Some(dp.friendly_name.clone());
        s.active_is_manual = true;
        let _ = app_handle.emit("profile-changed", Some(dp.friendly_name.clone()));
        success
    } else {
        let mut success = true;
        for (id, defaults) in s.system_defaults.iter() {
            success &= display::apply_resolution(id, defaults.resolution.width, defaults.resolution.height, defaults.resolution.refresh_rate);
            success &= display::apply_vibrance(id, defaults.vibrance);
            success &= display::apply_gamma(id, defaults.gamma);
        }
        s.system_defaults.clear();
        s.active_profile = None;
        s.active_is_manual = false;
        let _ = app_handle.emit("profile-changed", None::<String>);
        success
    }
}

#[tauri::command]
fn trigger_reset(app: AppHandle, state: State<'_, SharedState>) -> bool {
    let mut s = state.0.lock().unwrap();
    restore_baseline(&mut s, &app)
}

#[tauri::command]
fn get_active_profile(state: State<'_, SharedState>) -> Option<String> {
    let s = state.0.lock().unwrap();
    s.active_profile.clone()
}

// Background thread loop for game detection
fn spawn_daemon(state: Arc<Mutex<AppState>>, app_handle: AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));

            let (is_enabled, profiles, active_profile, active_is_manual, stealth) = {
                let s = state.lock().unwrap();
                (s.is_daemon_enabled, s.profiles.clone(), s.active_profile.clone(), s.active_is_manual, s.stealth_detection)
            };

            if !is_enabled {
                // Daemon disabled: revert only an auto-applied profile. Manually pinned
                // profiles (hotkey/reset, active_is_manual) must hold, otherwise the loop
                // wipes a hotkey-applied profile within a second of it being set.
                if active_profile.is_some() && !active_is_manual {
                    let mut s = state.lock().unwrap();
                    for (id, defaults) in s.system_defaults.iter() {
                        display::apply_resolution(id, defaults.resolution.width, defaults.resolution.height, defaults.resolution.refresh_rate);
                        display::apply_vibrance(id, defaults.vibrance);
                        display::apply_gamma(id, defaults.gamma);
                    }
                    s.system_defaults.clear();
                    s.active_profile = None;
                    s.active_is_manual = false;
                    let _ = app_handle.emit("profile-changed", None::<String>);
                }
                continue;
            }

            let foreground_proc = display::get_foreground_process_name(stealth);
            if foreground_proc.is_empty() {
                continue;
            }

            // Find matching enabled profile
            let matched_profile = profiles.iter().find(|p| {
                p.is_enabled && p.executable_name.eq_ignore_ascii_case(&foreground_proc)
            });

            match matched_profile {
                Some(profile) => {
                    let should_apply = match active_profile {
                        Some(ref name) => !name.eq_ignore_ascii_case(&profile.executable_name),
                        None => true,
                    };

                    if should_apply {
                        println!("[Daemon] Applying profile: {}", profile.friendly_name);
                        let mut s = state.lock().unwrap();

                        apply_profile_all_displays("daemon", &mut s.system_defaults, &profile.settings);

                        s.active_profile = Some(profile.executable_name.clone());
                        s.active_is_manual = false;
                        let _ = app_handle.emit("profile-changed", Some(profile.friendly_name.clone()));
                    }
                }
                None => {
                    // Revert to defaults if a profile was auto-applied but no longer matched.
                    // Manually pinned profiles (hotkey/reset) must hold and not be reverted here.
                    if active_profile.is_some() && !active_is_manual {
                        println!("[Daemon] Restoring default display settings...");
                        let mut s = state.lock().unwrap();
                        for (id, defaults) in s.system_defaults.iter() {
                            display::apply_resolution(id, defaults.resolution.width, defaults.resolution.height, defaults.resolution.refresh_rate);
                            display::apply_vibrance(id, defaults.vibrance);
                            display::apply_gamma(id, defaults.gamma);
                        }
                        s.system_defaults.clear();
                        s.active_profile = None;
                        s.active_is_manual = false;
                        let _ = app_handle.emit("profile-changed", None::<String>);
                    }
                }
            }
        }
    });
}

// ponytail: using native Win32 message loop thread to intercept global hotkeys dynamically, avoiding additional plugin dependencies.
fn spawn_hotkeys_listener(state: Arc<Mutex<AppState>>, app_handle: AppHandle) {
    std::thread::spawn(move || {
        use windows_sys::Win32::System::Threading::GetCurrentThreadId;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetMessageW, TranslateMessage, DispatchMessageW, MSG, WM_HOTKEY
        };

        let tid = unsafe { GetCurrentThreadId() };
        {
            let mut s = state.lock().unwrap();
            s.hotkeys_thread_id = Some(tid);
        }

        const WM_RELOAD_HOTKEYS: u32 = 0x0400 + 1;

        unsafe {
            let mut registered_ids = Vec::new();

            // Function to perform hotkey registration
            let register_all = |registered: &mut Vec<i32>| {
                // First unregister existing
                for id in registered.iter() {
                    UnregisterHotKey(0, *id);
                }
                registered.clear();

                let (reset_str, daemon_str, profiles_data) = {
                    let s = state.lock().unwrap();
                    (s.reset_hotkey.clone(), s.daemon_hotkey.clone(), s.profiles.clone())
                };

                // Register Global Reset -> ID 1
                if let Some((mods, vk)) = parse_hotkey(&reset_str) {
                    if RegisterHotKey(0, 1, mods, vk) != 0 {
                        registered.push(1);
                    } else {
                        let _ = app_handle.emit("hotkey-failed", format!("Reset ({reset_str})"));
                    }
                }

                // Register Global Daemon -> ID 2
                if let Some((mods, vk)) = parse_hotkey(&daemon_str) {
                    if RegisterHotKey(0, 2, mods, vk) != 0 {
                        registered.push(2);
                    } else {
                        let _ = app_handle.emit("hotkey-failed", format!("Daemon ({daemon_str})"));
                    }
                }

                // Register Profile Hotkeys -> ID 10 + idx
                for (idx, p) in profiles_data.iter().enumerate() {
                    if p.is_enabled {
                        if let Some(ref hotkey_str) = p.hotkey {
                            if let Some((mods, vk)) = parse_hotkey(hotkey_str) {
                                let id = 10 + idx as i32;
                                if RegisterHotKey(0, id, mods, vk) != 0 {
                                    registered.push(id);
                                } else {
                                    let _ = app_handle.emit("hotkey-failed", format!("{} ({hotkey_str})", p.friendly_name));
                                }
                            }
                        }
                    }
                }
                println!("[Hotkeys] Registered {} global hotkeys dynamically.", registered.len());
            };

            // Run initial registration
            register_all(&mut registered_ids);

            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) != 0 {
                if msg.message == WM_RELOAD_HOTKEYS {
                    println!("[Hotkeys] Reloading dynamic shortcuts...");
                    register_all(&mut registered_ids);
                } else if msg.message == WM_HOTKEY {
                    let id = msg.wParam as i32;
                    if id == 1 {
                        println!("[Hotkeys] Global Reset hotkey pressed.");
                        let mut s = state.lock().unwrap();
                        restore_baseline(&mut s, &app_handle);
                        let _ = app_handle.emit("displays-reset", ());
                    } else if id == 2 {
                        let active = {
                            let mut s = state.lock().unwrap();
                            s.is_daemon_enabled = !s.is_daemon_enabled;
                            let profiles_clone = s.profiles.clone();
                            let reset_hotkey = s.reset_hotkey.clone();
                            let daemon_hotkey = s.daemon_hotkey.clone();
                            let default_profile_name = s.default_profile_name.clone();
                            save_config(&s.config_path, &profiles_clone, s.is_daemon_enabled, &reset_hotkey, &daemon_hotkey, default_profile_name, s.stealth_detection);
                            s.is_daemon_enabled
                        };
                        println!("[Hotkeys] Global Daemon toggle pressed: {}", active);
                        let _ = app_handle.emit("daemon-changed", active);
                    } else if id >= 10 {
                        let idx = (id - 10) as usize;
                        
                        let (target_profile, is_already_active, default_profile) = {
                            let s = state.lock().unwrap();
                            if idx < s.profiles.len() {
                                let p = &s.profiles[idx];
                                let is_active = s.active_profile.as_ref() == Some(&p.friendly_name);
                                
                                let default_p = s.profiles.iter()
                                    .find(|dp| dp.is_default.unwrap_or(false))
                                    .cloned();

                                (Some(p.clone()), is_active, default_p)
                            } else {
                                (None, false, None)
                            }
                        };

                        if let Some(p) = target_profile {
                            if is_already_active {
                                if let Some(dp) = default_profile {
                                    println!("[Hotkeys] Reverting to designated DEFAULT profile: {}...", dp.friendly_name);
                                    let mut s = state.lock().unwrap();
                                    apply_profile_all_displays("hotkey_revert_default", &mut s.system_defaults, &dp.settings);
                                    s.active_profile = Some(dp.friendly_name.clone());
                                    s.active_is_manual = true;
                                    let _ = app_handle.emit("profile-changed", Some(dp.friendly_name.clone()));
                                } else {
                                    println!("[Hotkeys] Reverting to Windows defaults...");
                                    let mut s = state.lock().unwrap();
                                    for (disp_id, defaults) in s.system_defaults.iter() {
                                        display::apply_resolution(disp_id, defaults.resolution.width, defaults.resolution.height, defaults.resolution.refresh_rate);
                                        display::apply_vibrance(disp_id, defaults.vibrance);
                                        display::apply_gamma(disp_id, defaults.gamma);
                                    }
                                    s.system_defaults.clear();
                                    s.active_profile = None;
                                    s.active_is_manual = false;
                                    let _ = app_handle.emit("profile-changed", None::<String>);
                                    let _ = app_handle.emit("displays-reset", ());
                                }
                            } else {
                                println!("[Hotkeys] Applying profile: {}...", p.friendly_name);
                                let mut s = state.lock().unwrap();
                                apply_profile_all_displays("hotkey_apply", &mut s.system_defaults, &p.settings);
                                s.active_profile = Some(p.friendly_name.clone());
                                s.active_is_manual = true;
                                let _ = app_handle.emit("profile-changed", Some(p.friendly_name.clone()));
                            }
                        }
                    }
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });
}

#[tauri::command]
fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// Manual update check: reports back so the UI can show the result, unlike the
// silent startup check.
#[tauri::command]
async fn check_updates(app: AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await.map_err(|e| e.to_string())? {
        Some(update) => {
            update
                .download_and_install(|_, _| {}, || {})
                .await
                .map_err(|e| e.to_string())?;
            // Relaunch into the freshly installed version. Without this the old exe
            // keeps running (it lives in the tray and never exits on window close),
            // so the update never actually takes effect. restart() diverges.
            app.restart();
        }
        None => Ok(String::new()), // empty = already up to date
    }
}

async fn check_for_update(app: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_updater::UpdaterExt;
    if let Some(update) = app.updater()?.check().await? {
        update.download_and_install(|_chunk, _total| {}, || {}).await?;
        // Relaunch so the installed version loads. The app minimizes to the tray and
        // never exits on window close, so without an explicit restart the installer
        // can't replace the running exe and the update silently never applies.
        app.restart();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let app_handle = app.handle();

            // Check GitHub releases for a newer version and self-install on startup.
            let updater_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = check_for_update(updater_handle).await {
                    eprintln!("update check failed: {e}");
                }
            });

            let config_path = get_config_path(app_handle);
            let (profiles, is_daemon_enabled, reset_hotkey, daemon_hotkey, default_profile_name, stealth_detection) = load_config(&config_path);

            let state = Arc::new(Mutex::new(AppState {
                config_path,
                profiles,
                is_daemon_enabled,
                reset_hotkey,
                daemon_hotkey,
                default_profile_name,
                system_defaults: HashMap::new(),
                active_profile: None,
                active_is_manual: false,
                stealth_detection,
                hotkeys_thread_id: None,
            }));

            // Spawn the scanner daemon thread
            spawn_daemon(state.clone(), app_handle.clone());

            // Spawn the hotkeys listener thread
            spawn_hotkeys_listener(state.clone(), app_handle.clone());

            // Register state wrapper
            app.manage(SharedState(state));

            // Create Tray Menu in Tauri v2
            let show_i = MenuItem::with_id(app, "show", "Open Lumina", true, None::<&str>)?;
            let reset_i = MenuItem::with_id(app, "reset", "Reset Displays", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &reset_i, &quit_i])?;

            let app_handle_clone = app_handle.clone();

            // Build Tray Icon
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone()) // Use app default window icon
                .tooltip("Lumina Display Controller")
                .menu(&menu)
                .show_menu_on_left_click(false) // right-click => menu; left-click => open window
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "reset" => {
                            let state: State<'_, SharedState> = app.state();
                            let mut s = state.0.lock().unwrap();
                            restore_baseline(&mut s, app);
                        }
                        "quit" => {
                            let state: State<'_, SharedState> = app.state();
                            {
                                let mut s = state.0.lock().unwrap();
                                restore_baseline(&mut s, app);
                            }
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(move |_, event| {
                    // Only left-click opens the window. Right-click must fall through so
                    // Windows can show the context menu without focus being stolen.
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event {
                        if let Some(window) = app_handle_clone.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_displays,
            apply_vibrance,
            apply_gamma,
            apply_resolution,
            get_profiles,
            save_profiles,
            is_daemon_active,
            set_daemon_active,
            trigger_manual_apply,
            trigger_reset,
            get_active_profile,
            get_global_settings,
            save_global_settings,
            get_app_version,
            check_updates
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
