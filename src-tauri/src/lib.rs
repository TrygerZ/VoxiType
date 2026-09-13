//! VoxiType — free & open-source voice-to-text.
//!
//! Library root. Wires modules into the Tauri application and exposes the
//! managed [`AppStateInner`] shared by all commands.

pub mod active_window;
pub mod audio;
pub mod commands;
pub mod crypto;
pub mod data_dir;
pub mod error;
pub mod events;
pub mod hotkey;
pub mod injection;
pub mod llm;
pub mod logging;
pub mod overlay;
pub mod pipeline;
pub mod sound;
pub mod storage;
pub mod stt;
pub mod tray;
pub mod updater;
pub mod util;

use std::collections::HashMap;
use std::path::PathBuf;

use tauri::{Manager, Runtime};

use hotkey::HotkeyConfig;
use pipeline::PipelineOrchestrator;
use storage::{Database, HistoryRepository, SettingsManager};

/// Cached STT engine: (engine kind, cache key, engine instance).
type SttEngineCache = Option<(
    crate::stt::SttEngineKind,
    String,
    std::sync::Arc<dyn crate::stt::SttEngine>,
)>;

/// Shared application state stored in Tauri's managed state.
pub struct AppStateInner {
    pub db: Database,
    pub pipeline: PipelineOrchestrator,
    pub app_data_dir: PathBuf,
    pub default_app_data_dir: PathBuf,
    pub master_key: [u8; 32],
    /// Keeps the file-log writer thread alive; flushed on drop.
    pub _log_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    pub stt_engine: std::sync::Mutex<SttEngineCache>,
    /// Parent directory of the most recent native file-picker result, per
    /// picker kind (e.g. "whisper_binary"). Paths for those settings written
    /// back over IPC must canonicalize under these directories, so a
    /// compromised webview cannot invent attacker-controlled paths.
    pub last_picker_dirs: std::sync::Mutex<HashMap<String, PathBuf>>,
}

impl AppStateInner {
    fn new(
        app_data_dir: PathBuf,
        default_app_data_dir: PathBuf,
        log_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    ) -> error::Result<Self> {
        let db_path = app_data_dir.join("data").join("voxitype.db");
        tracing::info!("Opening database at '{}'", db_path.display());
        let db = Database::open(&db_path)?;
        let master_key = crypto::get_master_key(&app_data_dir)?;

        // Prune old history based on retention days setting
        let settings = SettingsManager::new(&db);
        let retention_days = settings
            .get::<u32>("history_retention_days")
            .ok()
            .flatten()
            .unwrap_or(90);
        let history_repo = HistoryRepository::new(&db);
        if let Err(e) = history_repo.prune_old_history(retention_days) {
            tracing::warn!("Failed to prune old history: {e}");
        }

        migrate_legacy_api_key(&db, &master_key)?;

        Ok(Self {
            db,
            pipeline: PipelineOrchestrator::new(),
            app_data_dir,
            default_app_data_dir,
            master_key,
            _log_guard: log_guard,
            stt_engine: std::sync::Mutex::new(None),
            last_picker_dirs: std::sync::Mutex::new(HashMap::new()),
        })
    }
}

/// Upgrade-on-startup: re-encrypt a legacy plaintext `groq_api_key` in place.
///
/// Idempotent — no-ops when the key is absent, empty, or already carries the
/// `enc:v1:` prefix. Fails closed with an explicit error if re-encryption or
/// persistence fails so unencrypted keys are never retained at rest.
fn migrate_legacy_api_key(db: &Database, master_key: &[u8; 32]) -> error::Result<()> {
    const API_KEY_SETTING: &str = "groq_api_key";
    let settings = SettingsManager::new(db);

    let stored = settings.get::<String>(API_KEY_SETTING)?;
    let Some(stored) = stored else {
        return Ok(()); // Key never set — nothing to migrate.
    };

    if let Some(encrypted) = crypto::migrate_plaintext_key(&stored, master_key)? {
        settings.set(API_KEY_SETTING, &encrypted)?;
        tracing::info!("Migrated legacy plaintext {API_KEY_SETTING} to encrypted storage");
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle();
            let default_app_data_dir = handle
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data directory: {e}"))?;

            // Initialize logging (stderr + rotating file) once the log dir is known.
            let log_guard = logging::init(&default_app_data_dir.join("logs"));
            let mut fallback_occurred = false;
            let (resolved_data_dir, resolve_fallback) =
                data_dir::resolve_app_data_dir_checked(default_app_data_dir.clone());
            if resolve_fallback {
                fallback_occurred = true;
            }
            let migration_source =
                data_dir::migration_source(&default_app_data_dir, &resolved_data_dir);
            let app_data_dir =
                data_dir::migrate_data_if_needed(&migration_source, &resolved_data_dir);
            if app_data_dir != resolved_data_dir {
                fallback_occurred = true;
            }

            // Initialize shared state (DB + pipeline).
            let state = match AppStateInner::new(
                app_data_dir.clone(),
                default_app_data_dir.clone(),
                log_guard,
            ) {
                Ok(state) => state,
                Err(error) if app_data_dir != default_app_data_dir => {
                    fallback_occurred = true;
                    let message = data_dir::fallback_error_message(
                        &app_data_dir,
                        &default_app_data_dir,
                        &error,
                    );
                    tracing::error!("{message}");
                    data_dir::record_error(&default_app_data_dir, &message);
                    let _ = std::fs::remove_file(
                        default_app_data_dir.join(data_dir::DATA_DIR_MARKER_FILE),
                    );
                    let fallback_guard = logging::init(&default_app_data_dir.join("logs"));
                    AppStateInner::new(
                        default_app_data_dir.clone(),
                        default_app_data_dir,
                        fallback_guard,
                    )
                    .map_err(|fallback| format!("Failed to init default app state: {fallback}"))?
                }
                Err(error) => return Err(format!("Failed to init app state: {error}").into()),
            };

            let db_path = state.app_data_dir.join("data").join("voxitype.db");
            tracing::info!(
                "Storage paths initialized: default_app_data_dir='{}', resolved_data_dir='{}', app_data_dir='{}', db_path='{}'",
                state.default_app_data_dir.display(),
                resolved_data_dir.display(),
                state.app_data_dir.display(),
                db_path.display()
            );
            data_dir::finish_startup(
                &state.default_app_data_dir,
                &state.app_data_dir,
                fallback_occurred,
            );

            // Load hotkey config from settings (fallback to default).
            let hotkey_cfg = SettingsManager::new(&state.db)
                .get::<HotkeyConfig>("hotkey")
                .ok()
                .flatten()
                .unwrap_or_default();

            app.manage(state);

            apply_window_icon(handle);

            // System tray.
            if let Err(e) = tray::setup(handle) {
                tracing::warn!("Tray setup failed: {e}");
            }

            // Global hotkey.
            if let Err(e) = hotkey::register(handle, &hotkey_cfg) {
                tracing::warn!("Hotkey registration failed: {e}");
            }

            // Floating widget: remember its position across drags and apply the
            // saved enabled/disabled state on launch.
            overlay::setup_persistence(handle);
            overlay::apply_enabled(handle, overlay::is_enabled(handle));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::get_settings,
            commands::update_setting,
            commands::set_floating_widget_enabled,
            commands::reveal_floating_widget,
            commands::get_history,
            commands::search_history,
            commands::delete_history,
            commands::clear_history,
            commands::pin_history,
            commands::re_inject,
            commands::export_history,
            commands::get_dictionary,
            commands::add_dictionary_word,
            commands::set_dictionary_active,
            commands::delete_dictionary_word,
            commands::export_dictionary,
            commands::import_dictionary,
            commands::get_snippets,
            commands::add_snippet,
            commands::delete_snippet,
            commands::get_per_app_modes,
            commands::set_per_app_mode,
            commands::delete_per_app_mode,
            commands::get_active_app,
            commands::set_hotkey,
            commands::get_microphones,
            commands::get_app_info,
            commands::check_updates,
            commands::open_url,
            commands::pick_setup_file,
            commands::set_whisper_cpp_paths,
            commands::pick_data_directory,
            commands::set_data_directory,
            commands::get_data_directory,
            commands::restart_app,
            commands::test_groq_api,
            commands::test_whisper_cpp,
            commands::get_usage_stats,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn apply_window_icon<R: Runtime>(app: &tauri::AppHandle<R>) {
    let icon = match tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png")) {
        Ok(icon) => icon,
        Err(e) => {
            tracing::warn!("Window icon load failed: {e}");
            return;
        }
    };

    for window in app.webview_windows().values() {
        if let Err(e) = window.set_icon(icon.clone()) {
            tracing::warn!("Window icon apply failed for {}: {e}", window.label());
        }
    }
}
