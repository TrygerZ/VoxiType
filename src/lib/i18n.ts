// Simple ID/EN translation dictionary for UI strings.

type Dict = Record<string, string>;

const id: Dict = {
  "app.name": "VoxiType",
  "nav.home": "Beranda",
  "nav.settings": "Pengaturan",
  "nav.history": "Riwayat",
  "nav.dictionary": "Kamus",
  "nav.snippets": "Snippet",
  "nav.about": "Tentang",
  "recording": "Merekam...",
  "processing": "Memproses...",
  "done": "Selesai ({count} kata)",
  "idle.hint": "Tekan {shortcut} untuk mulai merekam",
  "settings.general": "Umum",
  "settings.audio": "Audio",
  "settings.stt": "STT",
  "settings.llm": "LLM",
  "settings.modes": "Mode",
  "settings.app_rules": "Aturan App",
  "settings.dictionary": "Kamus",
  "settings.shortcuts": "Pintasan",
  "settings.about": "Tentang",
  "onboarding.welcome.title": "Selamat datang di VoxiType",
  "onboarding.welcome.body":
    "Dikte ke teks untuk aplikasi apa pun. Gratis dan open source.",
  "onboarding.welcome.start": "Mulai Setup",
  "onboarding.welcome.skip": "Lewati",
  "onboarding.complete.title": "Siap!",
  "onboarding.complete.start": "Mulai Pakai",
  "common.save": "Simpan",
  "common.cancel": "Batal",
  "common.delete": "Hapus",
  "common.add": "Tambah",
  "home.greeting.morning": "Selamat Pagi",
  "home.greeting.afternoon": "Selamat Siang",
  "home.greeting.evening": "Selamat Malam",
  "home.greeting.generic": "Halo",
  "home.active_mode": "Mode Aktif",
  "home.quick_settings": "Pengaturan Cepat",
  "home.recent_transcriptions": "Transkripsi Terakhir",
  "home.no_history": "Belum ada transkripsi terbaru",
  "home.mic_tooltip": "Klik untuk mulai mendikte",
  "home.mic_recording_tooltip": "Klik untuk selesai mendikte",
  "home.time_saved": "Menghemat waktu Anda",
  "home.words": "kata",
  "home.seconds": "detik",
  "home.copied": "Tersalin!",
  "home.re_injected": "Dikirim!",
  "home.pin_tooltip": "Pin transkripsi",
  "home.unpin_tooltip": "Lepas pin",
  "home.engine": "Mesin",
  "home.shortcut_tip": "Tekan {shortcut} untuk mendikte",
};

const en: Dict = {
  "app.name": "VoxiType",
  "nav.home": "Home",
  "nav.settings": "Settings",
  "nav.history": "History",
  "nav.dictionary": "Dictionary",
  "nav.snippets": "Snippets",
  "nav.about": "About",
  "recording": "Recording...",
  "processing": "Processing...",
  "done": "Done ({count} words)",
  "idle.hint": "Press {shortcut} to start recording",
  "settings.general": "General",
  "settings.audio": "Audio",
  "settings.stt": "STT",
  "settings.llm": "LLM",
  "settings.modes": "Modes",
  "settings.app_rules": "App Rules",
  "settings.dictionary": "Dictionary",
  "settings.shortcuts": "Shortcuts",
  "settings.about": "About",
  "onboarding.welcome.title": "Welcome to VoxiType",
  "onboarding.welcome.body":
    "Dictate to text in any application. Free and open source.",
  "onboarding.welcome.start": "Start Setup",
  "onboarding.welcome.skip": "Skip",
  "onboarding.complete.title": "All set!",
  "onboarding.complete.start": "Start using",
  "common.save": "Save",
  "common.cancel": "Cancel",
  "common.delete": "Delete",
  "common.add": "Add",
  "home.greeting.morning": "Good Morning",
  "home.greeting.afternoon": "Good Afternoon",
  "home.greeting.evening": "Good Evening",
  "home.greeting.generic": "Hello",
  "home.active_mode": "Active Mode",
  "home.quick_settings": "Quick Settings",
  "home.recent_transcriptions": "Recent Transcriptions",
  "home.no_history": "No recent transcriptions yet",
  "home.mic_tooltip": "Click to start dictating",
  "home.mic_recording_tooltip": "Click to stop dictating",
  "home.time_saved": "Saved you time",
  "home.words": "words",
  "home.seconds": "seconds",
  "home.copied": "Copied!",
  "home.re_injected": "Injected!",
  "home.pin_tooltip": "Pin transcription",
  "home.unpin_tooltip": "Unpin transcription",
  "home.engine": "Engine",
  "home.shortcut_tip": "Press {shortcut} to dictate",
};

const dictionaries: Record<string, Dict> = { id, en };

let currentLang = "id";

export function setLanguage(lang: string) {
  if (dictionaries[lang]) currentLang = lang;
}

export function t(key: string, vars?: Record<string, string | number>): string {
  const dict = dictionaries[currentLang] ?? id;
  let value = dict[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      value = value.replace(`{${k}}`, String(v));
    }
  }
  return value;
}

// ponytail: minimal reactive hook — upgrade to react-i18next if >5 languages
import { useSettingsStore } from "../stores/settingsStore";
export function useT() {
  const lang = useSettingsStore((s) => s.settings.language) as string | undefined;
  if (lang && dictionaries[lang]) setLanguage(lang);
  return t;
}
