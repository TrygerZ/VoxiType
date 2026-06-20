// Simple ID/EN translation dictionary for UI strings.

type Dict = Record<string, string>;

const id: Dict = {
  "app.name": "VoxiType",
  "nav.settings": "Pengaturan",
  "nav.history": "Riwayat",
  "nav.dictionary": "Kamus",
  "nav.about": "Tentang",
  "recording": "Merekam...",
  "processing": "Memproses...",
  "done": "Selesai ({count} kata)",
  "idle.hint": "Tekan Ctrl+Space untuk mulai merekam",
  "settings.general": "Umum",
  "settings.audio": "Audio",
  "settings.stt": "STT",
  "settings.llm": "LLM",
  "settings.modes": "Mode",
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
};

const en: Dict = {
  "app.name": "VoxiType",
  "nav.settings": "Settings",
  "nav.history": "History",
  "nav.dictionary": "Dictionary",
  "nav.about": "About",
  "recording": "Recording...",
  "processing": "Processing...",
  "done": "Done ({count} words)",
  "idle.hint": "Press Ctrl+Space to start recording",
  "settings.general": "General",
  "settings.audio": "Audio",
  "settings.stt": "STT",
  "settings.llm": "LLM",
  "settings.modes": "Modes",
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
