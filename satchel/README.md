# Satchel

A Windows desktop dashboard that works as a second brain for school. Drop in anything (files,
screenshots, webcam scans of worksheets) and get a useful result card: solved and explained
answers, summaries, flashcards, extracted tables, deadlines. Every item is filed into a library
organized by subject, which you can chat with.

Built with Tauri 2, a Rust backend, React + TypeScript + Vite + Tailwind, SQLite, and the Claude API.

## Status

| Milestone | State |
|---|---|
| 1. Skeleton: app shell, SQLite + migrations, settings, API key in Credential Manager, Claude test call | **Done** |
| 2. Ingest + OCR | Next |
| 3. Classify + result cards | |
| 4. Library: subjects, to-dos, flashcards, merge/rename | |
| 5. Chat with library (embeddings + sqlite-vec) | |
| 6. Webcam capture | |
| 7. Polish + installer | |

## Setup (Windows 10/11)

1. **Microsoft C++ Build Tools.** Install [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
   and tick the **Desktop development with C++** workload. Rust needs its linker and the Windows SDK.
2. **WebView2 Runtime.** Already installed on Windows 11 and up-to-date Windows 10. If not, get the
   [Evergreen Bootstrapper](https://developer.microsoft.com/microsoft-edge/webview2/).
3. **Rust** (1.85 or newer) via [rustup](https://rustup.rs). Use the MSVC toolchain:
   ```powershell
   rustup default stable-msvc
   ```
4. **Node.js** 20 or newer ([nodejs.org](https://nodejs.org), the LTS release is fine).

Then, from this folder:

```powershell
npm install
npm run tauri dev
```

The first build compiles all the Rust dependencies and takes a few minutes. Later builds are fast.
Frontend changes hot-reload; Rust changes rebuild and restart the app automatically.

### First run

1. Create an API key in the [Claude Console](https://console.anthropic.com/settings/keys).
2. In Satchel, open **Settings**, paste the key and click **Save key**.
3. Click **Test connection**. You should see a one-line greeting and the cost of the call (a fraction of a cent).

## Where things are stored

| What | Where |
|---|---|
| Library (SQLite database + copies of your files) | `%LOCALAPPDATA%\app.satchel.desktop\` by default. Change it in Settings → Library folder. |
| Chosen library location | `%APPDATA%\app.satchel.desktop\location.json` |
| Claude API key | Windows Credential Manager → Windows Credentials → Generic → `anthropic-api-key.Satchel` |

The API key is never written to the database, a config file, the repo, or the logs.
Moving the library copies it (database via `VACUUM INTO`, plus the files folder) to an empty
folder, then restarts. The old folder is kept as a backup for you to delete.

## Development

```powershell
npm run tauri dev        # run the app
npm run typecheck        # TypeScript
cd src-tauri
cargo test               # Rust unit tests (DB, migrations, Claude client against a local mock server)
cargo clippy --all-targets
```

Logs go to the terminal. Set `RUST_LOG=debug` for more detail.

To build an installer (MSI + NSIS `.exe`, in `src-tauri\target\release\bundle\`):

```powershell
npm run tauri build
```

On Linux the app also runs (for development only; it needs the `webkit2gtk-4.1` dev packages).
There the API key is held in memory and forgotten on exit.

## Project structure

```
satchel/
├─ src/                     React frontend
│  ├─ lib/                  api.ts (typed command calls), types.ts, hooks.ts, format.ts
│  ├─ components/           Sidebar, Logo, ui.tsx (Button, Notice, Section…)
│  ├─ pages/                HomePage, SettingsPage
│  └─ styles.css            Tailwind 4 theme tokens
└─ src-tauri/               Rust backend
   ├─ migrations/           numbered SQL migrations (never edit a shipped one)
   ├─ capabilities/         window permissions: core:default only
   └─ src/
      ├─ ai/                Claude client: retries, timeouts, structured JSON, usage/cost, model table
      ├─ db/                connection, migrations runner, settings, usage accounting
      ├─ commands/          Tauri commands (the only API the frontend can call)
      ├─ secrets.rs         API key ↔ Windows Credential Manager
      ├─ paths.rs           data folder resolution and moving
      └─ (ingest/, ocr/, search/ arrive in milestones 2 and 5)
```

## How Claude calls work

All calls go through `src-tauri/src/ai/mod.rs`:

- **Retries.** Retries on 408/409/429/5xx/529 and network errors, with exponential backoff and
  jitter. It honors `retry-after` and `x-should-retry`. It never retries auth or bad-request errors.
- **Timeouts.** Each call has its own timeout (default 120 s).
- **Structured output.** Anything the app parses is requested as JSON constrained by a JSON schema
  (`output_config.format`). The reply is deserialized into a Rust type and checked by a
  `Validate` impl. If it fails, Claude gets one chance to correct it; after that the call fails
  with a clear error instead of storing bad data.
- **Refusals.** For models that support it, a safety-classifier refusal is re-run server-side on a
  fallback model (`fallbacks: "default"`). Remaining refusals surface as a readable error.
- **Usage.** Every billed call is recorded (tokens + estimated cost from list prices) and shown in
  Settings → Usage, broken down by task.

The default model is `claude-opus-5-5`. Any model id can be entered in Settings. Known models get
the right feature flags and prices; unknown ones are counted in tokens only.

## Design decisions and deviations from the original spec

These were flagged up front so nothing is silently different from what you asked for:

1. **Windows OCR has no confidence score.** `Windows.Media.Ocr` returns text and word boxes, but
   no confidence. Milestone 2 computes a 0–1 *quality score* instead (dictionary-like word ratio,
   character mix, text density per image area) and stores it in `ocr_confidence`. Low scores
   trigger the Claude vision fallback, as planned.
2. **Worksheets should be solved from the image, not from OCR text.** Windows OCR is weak on
   handwriting and mangles math (fractions, exponents, layout). Since solving a worksheet calls
   Claude anyway, milestone 3 sends the image itself to Claude vision for worksheet solving. OCR
   text is still used for search, classification, and embeddings, where it's good enough.
3. **PDF page rendering uses `Windows.Data.Pdf`**, which is built into Windows, instead of
   bundling pdfium. Text-layer extraction stays in Rust.
4. **TLS uses Windows' native stack (SChannel)**, not rustls. The rustls default (aws-lc) needs
   CMake and NASM to build on Windows, and SChannel respects school or proxy certificates
   installed in Windows.
5. **Embeddings (milestone 5):** fastembed's default models are English-only. If your notes aren't
   in English, `multilingual-e5-small` is the better choice. ONNX Runtime will be linked statically
   so an older `onnxruntime.dll` in `System32` can't be picked up by mistake.
6. **Long generations will stream (milestone 3).** Solving a long worksheet can take more than a
   minute, so those calls use streaming to avoid timeouts and to show live progress.
7. **Classification uses the same model at low effort** rather than silently switching to a cheaper
   model. Milestone 3 can add an optional "fast model" setting if you want to trade quality for cost.
