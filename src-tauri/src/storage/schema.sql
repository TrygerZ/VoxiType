-- ============================================================
-- VoxiType Database Schema v1.0
-- ============================================================

-- Transcription History
CREATE TABLE IF NOT EXISTS transcriptions (
    id              TEXT PRIMARY KEY,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    text_raw        TEXT NOT NULL,
    text_formatted  TEXT NOT NULL,
    source_lang     TEXT NOT NULL DEFAULT 'id',
    target_lang     TEXT,
    mode            TEXT NOT NULL DEFAULT 'dictation',
    stt_engine      TEXT NOT NULL DEFAULT 'whisper_cpp',
    stt_confidence  REAL,
    llm_engine      TEXT,
    duration_ms     INTEGER,
    word_count      INTEGER NOT NULL DEFAULT 0,
    character_count INTEGER NOT NULL DEFAULT 0,
    is_pinned       INTEGER NOT NULL DEFAULT 0,
    tags            TEXT,
    app_context     TEXT,
    metadata        TEXT
);

CREATE INDEX IF NOT EXISTS idx_transcriptions_created_at
    ON transcriptions(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_transcriptions_mode
    ON transcriptions(mode);
CREATE INDEX IF NOT EXISTS idx_transcriptions_lang
    ON transcriptions(source_lang);

-- Full-text search for transcriptions
CREATE VIRTUAL TABLE IF NOT EXISTS transcriptions_fts USING fts5(
    text_formatted,
    content='transcriptions',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS transcriptions_ai
    AFTER INSERT ON transcriptions BEGIN
    INSERT INTO transcriptions_fts(rowid, text_formatted)
    VALUES (new.rowid, new.text_formatted);
END;

CREATE TRIGGER IF NOT EXISTS transcriptions_ad
    AFTER DELETE ON transcriptions BEGIN
    INSERT INTO transcriptions_fts(transcriptions_fts, rowid, text_formatted)
    VALUES ('delete', old.rowid, old.text_formatted);
END;

CREATE TRIGGER IF NOT EXISTS transcriptions_au
    AFTER UPDATE ON transcriptions BEGIN
    INSERT INTO transcriptions_fts(transcriptions_fts, rowid, text_formatted)
    VALUES ('delete', old.rowid, old.text_formatted);
    INSERT INTO transcriptions_fts(rowid, text_formatted)
    VALUES (new.rowid, new.text_formatted);
END;

-- ============================================================
-- Custom Dictionary
CREATE TABLE IF NOT EXISTS dictionary_entries (
    id              TEXT PRIMARY KEY,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    word            TEXT NOT NULL,
    pronunciation   TEXT,
    category        TEXT NOT NULL DEFAULT 'custom',
    replacement     TEXT,
    language        TEXT NOT NULL DEFAULT 'id',
    usage_count     INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    metadata        TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_dictionary_word
    ON dictionary_entries(word, language);
CREATE INDEX IF NOT EXISTS idx_dictionary_category
    ON dictionary_entries(category);

-- ============================================================
-- Snippets (Voice Shortcuts)
CREATE TABLE IF NOT EXISTS snippets (
    id              TEXT PRIMARY KEY,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    name            TEXT NOT NULL,
    trigger_phrase  TEXT NOT NULL UNIQUE,
    content         TEXT NOT NULL,
    category        TEXT,
    mode            TEXT,
    usage_count     INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_snippets_trigger
    ON snippets(trigger_phrase);

-- ============================================================
-- Settings (flat key-value, JSON-encoded values)
CREATE TABLE IF NOT EXISTS settings (
    key             TEXT PRIMARY KEY,
    value           TEXT NOT NULL,
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO settings (key, value) VALUES
    ('hotkey', '{"key":"Ctrl+Space","mode":"ptt"}'),
    ('stt_engine', '"whisper_cpp"'),
    ('stt_model', '"small"'),
    ('llm_engine', '"ollama"'),
    ('llm_model', '"qwen2.5:3b"'),
    ('active_mode', '"dictation"'),
    ('language', '"id"'),
    ('stt_language', '"auto"'),
    ('mic_device', '"default"'),
    ('auto_start', 'false'),
    ('auto_update', 'true'),
    ('sound_cues', 'false'),
    ('telemetry', 'false'),
    ('onboarding_completed', 'false'),
    ('theme', '"dark"'),
    ('history_retention_days', '90'),
    ('groq_api_key', '""'),
    ('translation_enabled', 'false'),
    ('translation_target', '"en"'),
    ('command_mode', 'false'),
    ('per_app_mode', 'false'),
    ('floating_widget', 'true'),
    ('floating_widget_pos', 'null');

-- ============================================================
-- Usage Stats (opt-in telemetry, anonymous)
CREATE TABLE IF NOT EXISTS usage_stats (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    date            TEXT NOT NULL,
    session_count   INTEGER NOT NULL DEFAULT 0,
    transcription_count INTEGER NOT NULL DEFAULT 0,
    total_words     INTEGER NOT NULL DEFAULT 0,
    total_duration_ms INTEGER NOT NULL DEFAULT 0,
    stt_local_count INTEGER NOT NULL DEFAULT 0,
    stt_cloud_count INTEGER NOT NULL DEFAULT 0,
    llm_local_count INTEGER NOT NULL DEFAULT 0,
    llm_cloud_count INTEGER NOT NULL DEFAULT 0,
    error_count     INTEGER NOT NULL DEFAULT 0,
    UNIQUE(date)
);

-- ============================================================
-- Modes (User-defined modes)
CREATE TABLE IF NOT EXISTS modes (
    id              TEXT PRIMARY KEY,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    name            TEXT NOT NULL UNIQUE,
    description     TEXT,
    system_prompt   TEXT NOT NULL,
    is_builtin      INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    sort_order      INTEGER NOT NULL DEFAULT 0
);

INSERT OR IGNORE INTO modes (id, name, description, system_prompt, is_builtin, sort_order) VALUES
    ('mode_dictation', 'Dictation',
     'Raw transcription with minimal filler removal',
     'You are a transcription text cleaner. Remove filler words. Fix punctuation and capitalization. DO NOT paraphrase. Output ONLY the cleaned text.',
     1, 1),
    ('mode_message', 'Message',
     'Casual format for chat',
     'Format text as a casual chat message. Short sentences, natural flow. Use sentence case. May add emoji if expressive. Output ONLY the formatted text.',
     1, 2),
    ('mode_email', 'Email',
     'Formal format for email with proper structure',
     'Format text as a formal email. Use structure: greeting, body, sign-off. Proper capitalization. Professional tone. Output ONLY the formatted text.',
     1, 3);

-- ============================================================
-- Per-App Mode Mapping
CREATE TABLE IF NOT EXISTS per_app_modes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    app_process_name TEXT NOT NULL UNIQUE,
    app_display_name TEXT,
    mode_id         TEXT NOT NULL,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
