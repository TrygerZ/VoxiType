# RCA Bug 2 VoxiType 0.4.3

## 1. Struktur direktori ditemukan

Default app-data:

```text
C:\Users\Tryger\AppData\Roaming\com.voxitype.app\
├── data_dir.txt                         45 B
├── master.key                           361 B
├── data\
│   ├── voxitype.db                      4,096 B
│   ├── voxitype.db-shm                  32,768 B
│   └── voxitype.db-wal                  267,832 B
└── logs\
    ├── voxitype.log.2026-08-24          709 B
    ├── voxitype.log.2026-09-03          1,756 B
    ├── voxitype.log.2026-09-07          3,028 B
    ├── voxitype.log.2026-09-10          200 B
    └── voxitype.log.2026-09-12          780 B
```

Format log sesuai `src-tauri/src/logging.rs:35`: `logs\voxitype.log.<tanggal>`.

Custom directory dari marker:

```text
D:\0 - Voxitype\
├── data\
│   ├── voxitype.db                      4,096 B
│   ├── voxitype.db-shm                  32,768 B
│   └── voxitype.db-wal                  1,001,192 B
├── logs\
│   └── voxitype.log.2026-08-24          426 B
├── master.key                           361 B
├── VoxiType\voxitype.exe
└── Voxitype Build\...
```

## 2. Status `data_dir.txt`

Isi:

```json
{"current":"D:\\0 - Voxitype","pending":null}
```

Status: ada, JSON valid, `current` absolut, `pending` kosong. Target `D:\0 - Voxitype` ada dan dapat dibaca. Tidak ada indikasi marker invalid atau target network/removable dari metadata yang tersedia.

## 3. Bukti log per cabang

### (a) Marker invalid

Tidak ada bukti.

Pesan yang dicari: `Data directory marker ignored`. Tidak ditemukan pada lima file log default maupun log custom yang tersedia.

Marker saat ini valid. Cabang ini tidak terbukti pada startup yang terekam.

### (b) Migrasi gagal

Tidak ada bukti migrasi gagal.

Bukti kebalikan:

```text
2026-09-03T09:07:37.859321Z  INFO Data migration skipped; target database already exists
2026-09-03T09:12:49.496597Z  INFO Data migration skipped; target database already exists
2026-09-07T09:49:19.328465Z  INFO Data migration skipped; target database already exists
2026-09-07T13:25:05.804603Z  INFO Data migration skipped; target database already exists
2026-09-10T14:43:06.305574Z  INFO Data migration skipped; target database already exists
2026-09-12T03:45:44.648256Z  INFO Data migration skipped; target database already exists
2026-09-12T03:47:06.966490Z  INFO Data migration skipped; target database already exists
```

Pesan `Data directory migration skipped; using default directory` tidak ditemukan. Log membuktikan target custom sudah memiliki DB valid dan migrasi dilewati, bukan fallback ke default.

### (c) Custom DB initialization gagal

Tidak ada bukti.

Pesan `Custom data directory initialization failed; recovering with default directory` tidak ditemukan. Tidak ada error DB open, permission, atau SQLite pada log startup yang tersedia.

### (d) Frontend `getSettings()` IPC gagal

Tidak dapat dikonfirmasi dari backend log. Sesuai konteks RCA, kegagalan ini silent dan tidak menghasilkan pesan backend yang dapat dicari.

### Onboarding

Tidak ada log yang memuat `onboarding` pada file yang tersedia. Log tidak mencatat nilai `onboarding_completed`.

## 4. Perbandingan metadata DB

| Lokasi | Ukuran DB | Mtime DB | Catatan |
|---|---:|---|---|
| Default `C:\Users\Tryger\AppData\Roaming\com.voxitype.app\data\voxitype.db` | 4,096 B | 2026-08-24 19:17:03 | WAL/SHM juga bertanggal 2026-08-24 |
| Custom `D:\0 - Voxitype\data\voxitype.db` | 4,096 B | 2026-08-24 19:17:03 | WAL 1,001,192 B, SHM mtime 2026-09-12 10:47:06 |

`sqlite3` CLI tidak tersedia. Query read-only `settings.onboarding_completed` dilewati. DB tidak dibuka atau dimutasi.

Custom WAL/SHM menunjukkan aktivitas SQLite terbaru pada custom directory. Ini konsisten dengan startup memakai custom directory, tetapi tidak membuktikan nilai flag onboarding.

## 5. Verdict

**Tidak ada cabang (a), (b), atau (c) yang terbukti memicu onboarding pada periode laporan.**

Bukti startup berulang menunjukkan marker valid dan migrasi custom berhasil/terlewati karena target DB sudah ada. Cabang paling mungkin dari kandidat yang diberikan adalah **(d) frontend `getSettings()` IPC gagal silent**, tetapi ini **tidak terbukti dari log**.

Periode log yang tersedia: 2026-08-24 sampai 2026-09-12. Jika onboarding muncul di luar periode ini atau setelah log terhapus/terrotasi, pemicu tidak dapat dikonfirmasi dari artefak saat ini.

## 6. Anomali relevan

- Ollama selalu gagal koneksi pada beberapa sesi, lalu fallback ke `rule_based`:

  ```text
  2026-09-12T03:45:54.156809Z  WARN LLM 'ollama' failed ([LlmConnectionRefused] Ollama connection refused: error sending request for url (http://localhost:11434/api/generate)); falling back to 'rule_based'
  ```

  Tidak terkait langsung dengan onboarding.
- Startup 2026-09-12 mencatat pipeline error:

  ```text
  2026-09-12T03:45:46.684712Z ERROR Pipeline error: [InvalidTransition] Invalid state transition
  ```

  Tidak terkait langsung dengan pemilihan data directory, tetapi relevan untuk investigasi startup/session.
- Tidak ditemukan error permission, target removable/network, marker removal, migration failure, atau custom DB initialization failure.
- Tidak ada laporan bug terpisah dibuat selain file ini. Tidak ada kode atau data aplikasi diubah.
