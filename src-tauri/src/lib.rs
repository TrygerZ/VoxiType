//! VoxiType — free & open-source voice-to-text.
//!
//! Library root. Wires modules into the Tauri application and exposes the
//! managed [`AppStateInner`] shared by all commands.

pub mod active_window;
pub mod audio;
pub mod commands;
pub mod crypto;
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
    pub master_key: [u8; 32],
    /// Keeps the file-log writer thread alive; flushed on drop.
    pub _log_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    pub stt_engine: std::sync::Mutex<SttEngineCache>,
}

impl AppStateInner {
    fn new(
        app_data_dir: PathBuf,
        log_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    ) -> error::Result<Self> {
        let db_path = app_data_dir.join("data").join("voxitype.db");
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

        Ok(Self {
            db,
            pipeline: PipelineOrchestrator::new(),
            app_data_dir,
            master_key,
            _log_guard: log_guard,
            stt_engine: std::sync::Mutex::new(None),
        })
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle();
            let app_data_dir = handle
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));

            // Initialize logging (stderr + rotating file) once the log dir is known.
            let log_guard = logging::init(&app_data_dir.join("logs"));

            // Initialize shared state (DB + pipeline).
            let state = AppStateInner::new(app_data_dir, log_guard)
                .map_err(|e| format!("Failed to init app state: {e}"))?;

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
