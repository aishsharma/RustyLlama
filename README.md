# RustyLlama
Rust-based client for Ollama

Build a native Windows desktop application in Rust that serves as a full-featured
local LLM chat client, initially targeting Ollama as the backend, but architected
to support additional LLM providers (OpenAI, Anthropic, Mistral, LM Studio, etc.)
via a provider trait/abstraction layer.

The UI framework should be one of: egui (via eframe), Tauri (Rust backend + web
frontend), or Slint — choose whichever best supports the feature set below with
native Windows feel and GPU-accelerated rendering.

---

## PHASE 1 — Core Chat (MVP)

### Provider Abstraction Layer
- Define a `LLMProvider` trait with methods: list_models(), chat_stream(),
  cancel(), health_check()
- Implement `OllamaProvider` as the first concrete backend (REST API at
  localhost:11434)
- Store provider configs (base URL, API key if needed, timeout) in a per-provider
  config struct
- All provider implementations must support async streaming via SSE or chunked HTTP

### Chat Interface
- Persistent sidebar listing all conversation sessions (title, timestamp, model used)
- Main chat panel with scrollable message history
- User messages and assistant messages visually differentiated (bubble or panel style)
- Markdown rendering in assistant messages: bold, italic, headers, bullet lists,
  numbered lists, blockquotes, inline code, fenced code blocks with syntax
  highlighting, tables
- Code blocks must have: language label, line numbers (optional toggle), one-click
  copy button
- Input area: multi-line textarea that auto-expands, Shift+Enter for newline,
  Enter to send, send button
- Streaming token display — tokens appear word-by-word as they arrive, with a
  blinking cursor indicator
- Stop/cancel generation button visible while streaming
- Token count display (prompt tokens, completion tokens, total) shown per message
  or in status bar

### Model Selection
- Dropdown or modal to select from models returned by list_models()
- Show model metadata: parameter size, quantization level, context length if
  available from Ollama model info API
- "Pull model" shortcut that opens a dialog to enter a model name and streams the
  pull progress (layers, download %, speed)
- Remember last-used model per conversation

### Conversations / Sessions
- Create new conversation (blank slate)
- Rename conversation (double-click title or context menu)
- Delete conversation with confirmation
- Conversations persisted locally (SQLite via rusqlite or sled key-value store)
- Each conversation stores: id, title, model, system prompt, message history,
  created_at, updated_at
- Auto-title: after first assistant response, optionally call the model with a
  short summarize-this prompt to generate a title (toggle in settings)

---

## PHASE 2 — Power User Features

### System Prompts & Personas
- Per-conversation system prompt editor (collapsible panel above chat)
- Global system prompt default in settings
- Saved "Personas" library: name, system prompt, optional model preference
- Apply a persona to any conversation from a dropdown

### Context & Memory Controls
- Visible context window usage bar (tokens used / max context)
- Option to set max context length override per conversation
- "Summarize & Compress" button: sends current history to model, replaces it with
  a compressed summary to free context space
- Conversation branching: at any message, right-click → "Branch from here" to
  create a fork of the conversation up to that point

### File & Image Input
- Drag-and-drop or file picker to attach files to a message
- Text files (.txt, .md, .rs, .py, etc.): read content and inject into message
  as a fenced code block or quoted text
- PDF: extract text via pdf-extract or similar crate and inject
- Images: pass as base64 to multimodal models that support vision (e.g. llava,
  bakllava) — show image thumbnail in message
- Show attached file names as chips/tags above the input area; removable

### Search & History
- Full-text search across all conversations (search bar in sidebar)
- Highlight matching messages, jump to them
- Filter conversations by model, date range, persona

---

## PHASE 3 — Advanced / Power Features

### Multi-Provider Support (extend the trait)
- Settings panel: add/remove/edit providers
- Provider types selectable: Ollama, OpenAI-compatible (any base URL + API key),
  Anthropic, LM Studio
- Per-conversation provider+model selection
- Show provider health status (green/red dot) in sidebar header

### Prompt Templates & Snippets
- Template library: saved reusable prompt templates with variable placeholders
  like {{topic}} or {{code}}
- Insert template into input area and fill variables via a small form dialog
- Keyboard shortcut (e.g. / prefix) to open quick-insert template picker

### Parameters Panel
- Collapsible side panel or popover for inference parameters:
  temperature, top_p, top_k, repeat_penalty, seed, num_predict (max tokens),
  stop sequences
- Parameters stored per-conversation, with a "reset to defaults" button
- Preset slots: save/load named parameter presets (e.g. "Creative", "Precise",
  "Code")

### Export & Import
- Export conversation as: Markdown file, plain text, JSON
- Import conversation from JSON (for backup/restore or sharing)
- Copy entire conversation to clipboard as formatted Markdown

### Keyboard Shortcuts
- New chat: Ctrl+N
- Search: Ctrl+F or Ctrl+K (command palette style)
- Cycle conversations: Ctrl+Tab / Ctrl+Shift+Tab
- Toggle sidebar: Ctrl+B
- Focus input: Escape (when not typing)
- Send message: Enter
- Newline in input: Shift+Enter

---

## PHASE 4 — Settings & Polish

### Settings Screen
- General: theme (dark/light/system), font size, font family
- Default model and provider
- Default system prompt
- Auto-title conversations toggle + model used for titling
- Data directory path (where SQLite/files are stored)
- Proxy settings (HTTP proxy for provider API calls)

### Theming
- Dark mode and light mode
- At minimum: a dark theme with a neutral dark sidebar and accent color,
  and a light theme
- Accent color picker (applied to buttons, selections, streaming cursor, etc.)

### Notifications & Status
- Status bar at bottom: current model, provider health, token counts, request
  latency (ms)
- Non-blocking toast notifications for: model pull complete, error, copy success

### Error Handling
- If Ollama is not running, show a clear banner with a "Retry connection" button
- Stream errors shown inline in the chat (not modal dialogs) with option to retry
- Log panel (toggle) showing raw request/response metadata for debugging

---

## NON-FUNCTIONAL REQUIREMENTS

- All data stored locally — no telemetry, no cloud sync, fully offline capable
- Application state (open conversation, scroll position, sidebar width) persisted
  across restarts
- Responsive to window resize; sidebar collapsible
- Windows 10/11 native, no admin rights required to install (portable .exe or
  NSIS/MSI installer)
- Async runtime: Tokio
- HTTP client: reqwest with streaming support
- Persistence: rusqlite (SQLite) with migrations via rusqlite_migration or refinery
- Markdown rendering: pulldown-cmark for parsing; render to styled widgets
- Syntax highlighting: syntect
- Target binary size: reasonable (under 50MB ideally); use release profile with LTO

---

## SUGGESTED CRATE STACK

UI framework:       eframe + egui  (or Tauri if web-renderer preferred)
Async:              tokio
HTTP:               reqwest (with stream feature)
Serialization:      serde + serde_json
Persistence:        rusqlite + rusqlite_migration
Markdown:           pulldown-cmark
Syntax highlight:   syntect
PDF text extract:   pdf-extract
Logging:            tracing + tracing-subscriber
Config:             dirs (for app data path) + toml or serde_json for config file
UUID:               uuid

---

## DELIVERY EXPECTATIONS

- The codebase should be modular: separate crates or modules for
  provider/, ui/, db/, config/, models/
- Each phase can be delivered independently; Phase 1 must be fully working
  before Phase 2 begins
- Unit tests for: provider trait mock, DB CRUD operations, markdown parsing
- A README with: build instructions, how to add a new provider, how to run

---

## BUILD INSTRUCTIONS

1. Ensure you have Rust installed (https://rustup.rs/)
2. Clone or download the repository
3. Run `cargo build --release` to build the application
4. Run `cargo run --release` to start the application

The application requires Ollama to be running on localhost:11434. Install Ollama from https://ollama.ai and pull a model, e.g., `ollama pull llama2`.

## HOW TO ADD A NEW PROVIDER

1. Implement the `LLMProvider` trait in a new module under `src/provider/`
2. Add the provider config to `src/config/mod.rs`
3. Update the UI in `src/ui/mod.rs` to allow selecting the new provider
4. Add the provider to the config loading/saving

## HOW TO RUN

After building, execute the binary. The app will create a data directory in your system's data folder (e.g., `~/.local/share/RustyLlama` on Linux) for storing conversations and config.

Start with Phase 1. Confirm the UI framework choice before writing code,
with a one-paragraph rationale.
