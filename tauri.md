Tauri v2 Migration Plan
Branch: Tauri-branch
Goal: Convert WASM-based app to native Tauri desktop app

Phase 1: Tauri Setup (30 min)
1.1 Install Tauri CLI
npm install -D @tauri-apps/cli@latest
1.2 Initialize Tauri in existing project
npx tauri init
Prompts to answer:

Question	Answer
App name	kittclouds
Window title	KittClouds
Web assets location	./dist
Dev server URL	http://localhost:5173
Dev command	npm run dev
Build command	npm run build
This creates src-tauri/ directory.

1.3 First test run
npx tauri dev
Gate: Native window opens showing your React app.

Phase 2: Migrate kittcore to Tauri Commands (2-3 hours)
2.1 Move Rust code
Current structure:

rust/kittcore/src/
├── scanner/
├── reality/
├── graphdb/
├── db/           ← WASM SQLite (REMOVE)
└── lib.rs        ← wasm_bindgen exports
New structure:

src-tauri/src/
├── lib.rs        ← #[tauri::command] exports
├── scanner/      ← (copy from kittcore)
├── reality/      ← (copy from kittcore)
├── graphdb/      ← (copy from kittcore)
└── main.rs       ← Tauri entry point
2.2 Replace wasm_bindgen with tauri::command
Before (WASM):

#[wasm_bindgen]
pub fn scan_document(text: &str, entities_json: &str) -> String { ... }
After (Tauri):

#[tauri::command]
fn scan_document(text: String, entities_json: String) -> Result<String, String> { ... }
// Register in main.rs
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![scan_document])
2.3 Swap SQLite stack
WASM	Tauri
sqlite-wasm-rs	rusqlite
OPFS VFS	Native filesystem
Async workers	Sync calls (or tokio)
# src-tauri/Cargo.toml
[dependencies]
rusqlite = { version = "0.32", features = ["bundled"] }
Gate: cargo check passes in src-tauri.

Phase 3: Frontend Integration (2 hours)
3.1 Install Tauri API package
npm install @tauri-apps/api
3.2 Replace WASM imports with 
invoke()
Before:

import init, { WasmProjector } from '@/lib/wasm/kittcore';
await init();
const projector = new WasmProjector();
const result = projector.buildTimeline(refs);
After:

import { invoke } from '@tauri-apps/api/core';
const result = await invoke<string>('build_timeline', { refs });
3.3 Create Tauri facade
// src/lib/tauri/kittcore.ts
import { invoke } from '@tauri-apps/api/core';
export async function scanDocument(text: string, entitiesJson: string) {
  return invoke<string>('scan_document', { text, entitiesJson });
}
export async function buildTimeline(refs: any[]) {
  return invoke<any>('build_timeline', { refs: JSON.stringify(refs) });
}
3.4 Update imports in existing facades
Current Location	Action
ScannerFacade.ts	Import from @/lib/tauri/kittcore
ProjectionsFacade.ts	Import from @/lib/tauri/kittcore
EmbeddingsFacade.ts	Import from @/lib/tauri/kittcore
Gate: App runs with npx tauri dev, scanner works.

Phase 4: Remove WASM artifacts
 Delete rust/kittcore/ (old WASM crate)
 Delete src/lib/wasm/kittcore/ (generated WASM)
 Remove wasm-pack from build scripts
 Remove sqlite-wasm-rs dependencies
 Clean up package.json
Phase 5: CozoDB Migration (Optional, later)
CozoDB has native Rust support. If you keep it:

cozo = { version = "0.7", features = ["storage-sqlite"] }
Native CozoDB = no more WASM shim → way simpler.

Commands Cheatsheet
Command	Purpose
npx tauri dev	Dev mode (Vite + native window)
npx tauri build	Build .exe installer
npx tauri icon	Generate app icons from PNG
Expected Benefits Post-Migration
Aspect	Before	After
SQLite	Worker + OPFS dance	Just rusqlite
Concurrency	Single-threaded	Full native threads
Startup	Load 17MB WASM	Native binary
Distribution	Browser only	.exe, .dmg, .AppImage
Performance	WASM overhead	Native Rust speed