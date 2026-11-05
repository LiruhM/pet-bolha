# Pet Bolha 🫧

> **Um pet de bolha offline, local-first, que vive no seu desktop e celular.**  
> Zero nuvem, zero telemetria, zero conta. Seu pet, seus dispositivos, seus dados.

---

## 🎯 Filosofia

- **Offline-first**: Funciona 100% sem internet. Dados nunca saem dos seus dispositivos.
- **Mínimo de bateria**: Overlay Windows click-through + render event-driven (0% CPU quando dormindo).
- **Privacidade real**: Não lê conteúdo de janelas, só nome do app ativo. Memória em JSONL legível.
- **Cross-platform**: Windows (overlay nativo) + Android (service overlay) — mesmo core Rust via FFI/JNI.
- **Personalidade emergente**: State machine + rede neural LIF (470 neurônios) opcional para "alma".

---

## 🏗️ Arquitetura

```
┌─────────────────────────────────────────────────────────────┐
│                      PET-CORE (Rust)                        │
│  • State Machine (necessidades, humor, ações)               │
│  • LIF Neural Network (470 neurônios, opcional)             │
│  • Memory Store (JSONL append-only)                         │
│  • Speech Engine (500+ templates PT-BR, sem LLM)            │
│  • Personalities (Bryan, Jorge, Balanced)                   │
│  • FFI Layer (C API para Windows/Android)                   │
└──────────────────────────┬──────────────────────────────────┘
                           │ FFI / JNI
           ┌───────────────┼───────────────┐
           ▼               ▼               ▼
    ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
    │ PET-WINDOWS │ │ PET-ANDROID │ │  PET-HEADLESS│
    │  (Overlay)  │ │ (Service)   │ │   (Testes)   │
    │  tiny-skia  │ │  Canvas     │ │   (CLI)      │
    │  Win32 API  │ │  Kotlin     │ │              │
    └─────────────┘ └─────────────┘ └─────────────┘
```

### Windows Overlay (pet-windows)
- **WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW** — click-through, sem foco, sem Alt+Tab
- **SEM WS_EX_LAYERED** — evita custo DWM redirection surface (Win10+)
- **tiny-skia** — render CPU, sem GPU/Vulkan
- **Event-driven** — 5 FPS acordado, 1 frame/2s idle, 0% CPU dormindo
- **~2MB standalone .exe** — sem installer, sem dependências

### Android Service (pet-android, planejado)
- **TYPE_APPLICATION_OVERLAY** — bolha flutuante
- **Foreground service** prioridade mínima
- **JobScheduler** — check-ins 15-30min
- **Sensores**: acelerômetro (detecta pegar), screen on/off
- **Zero GPS, zero network, zero wake locks**

---

## 📁 Estrutura do Repo

```
pet-bolha/
├── Cargo.toml                 # Workspace
├── pet-core/                  # Core library (Rust)
│   ├── src/
│   │   ├── lib.rs            # Exports
│   │   ├── state.rs          # State machine, needs, mood
│   │   ├── neural.rs         # LIF neural network
│   │   ├── memory.rs         # JSONL memory store
│   │   ├── speech.rs         # Speech templates (500+)
│   │   ├── personality.rs    # Personalities config
│   │   └── ffi.rs            # C FFI exports
│   └── Cargo.toml
├── pet-windows/               # Windows overlay
│   ├── src/main.rs           # Overlay entry point
│   └── Cargo.toml
├── pet-android/               # Android service (TODO)
├── assets/
│   ├── sprites/              # Sprite sheets (PNG + JSON)
│   │   ├── bolha.png / _frames.json
│   │   ├── bryan.png / _frames.json
│   │   └── jorge.png / _frames.json
│   └── speech/               # Speech templates (JSON)
└── README.md
```

---

## 🚀 Build & Run (Windows)

### Pré-requisitos
- Rust 1.75+ (`rustup install stable`)
- Windows 10/11 x64
- Visual Studio Build Tools ou `rustup target add x86_64-pc-windows-msvc`

### Compilar
```bash
cd pet-bolha
cargo build --release --workspace
```

### Executar
```bash
# Copia assets para pasta do executável
cp -r assets target/release/

# Roda
cd target/release
./pet-windows.exe
```

### Hotkeys
- **Ctrl+Alt+Shift+P** — Desliga tudo (pânico)
- **Clique direito** na bolha — Carinho
- **Clique esquerdo + arrasta** — Brincar/arremessa
- **Hover** — Pet detecta carinho potencial

### Dados
- Memória: `%LOCALAPPDATA%\pet-bolha\memory.jsonl` (JSONL legível)
- Pesos neurais: `%LOCALAPPDATA%\pet-bolha\neural_weights.json`
- Config: `%LOCALAPPDATA%\pet-bolha\config.json`

---

## 🧠 Como Funciona

### State Machine
4 necessidades (0.0 a 1.0): **Fome, Sono, Tédio, Carinho**
- Decaem naturalmente (~4-6h até crítico)
- Interações do usuário repõem
- Humor deriva das necessidades (10 estados: Feliz → Dramático → Dormindo)

### Rede Neural LIF (Opcional)
- **470 neurônios** Leaky Integrate-and-Fire
- Entradas: necessidades + cursor + janela ativa + idle + hora + bateria
- Saídas: humor, ação, fala, movimento, expressão
- **STDP simplificado** — aprende com carinho/ignorado
- Roda em thread separada @ 10Hz quando acordado (<0.1ms CPU)

### Fala (Sem LLM)
- **500+ templates PT-BR** organizados por humor, contexto, hora
- Seleção: peso do humor + contexto matching + cooldown + aleatoriedade determinística
- Fallback: emoji simples se sem template elegível

### Personalidades
| ID | Nome | Estilo |
|----|------|--------|
| `bryan` | Bryan | Revoltado, protestas, greve, mede cursor |
| `jorge` | Jorge | Coach satírico, mentorias, fala 3ª pessoa |
| `balanced` | Bolha | Equilibrado, carinhoso, curioso (padrão) |

---

## 📱 Android (Planejado)

Mesmo `pet-core` via JNI/FFI:
- `pet_android/src/main/kotlin/.../PetService.kt` — Overlay service
- `pet_android/src/main/jni/...` — JNI bindings
- Build: `./gradlew assembleRelease` → `app-release.apk`

Sync local (Wi-Fi/USB, QUIC/TLS, sem nuvem) — planejado.

---

## 🛠️ Desenvolvimento

### Testes
```bash
cargo test --workspace
```

### Logs
```bash
# Windows
$env:RUST_LOG=debug; ./pet-windows.exe

# Ou via Event Viewer / OutputDebugString
```

### Adicionar Personalidade
1. Crie `Personality` em `pet-core/src/personality.rs`
2. Adicione sprite sheet em `assets/sprites/`
3. Registre em `Personality::all()`

### Adicionar Templates de Fala
Edite `pet-core/src/speech.rs` → `builtin_templates()` ou carregue JSON externo via `SpeechEngine::load_from_file()`.

---

## 📄 Licença

**MIT OR Apache-2.0** — Código livre.
**Assets (sprites, textos)** — CC-BY-4.0 ou substitua pelos seus.

---

## ⚠️ Status: EM CONSTRUÇÃO

> **Este projeto está em desenvolvimento ativo.**  
> Funcionalidades podem mudar, quebrar ou sumir entre commits.  
> Não use em produção. Use por sua conta e risco.

### Roadmap Próximo
- [ ] Android service overlay (Kotlin + JNI)
- [ ] Sync local Wi-Fi/USB (QUIC/TLS)
- [ ] TTS local (Piper) + STT local (Vosk)
- [ ] Config UI (Tauri/egui settings window)
- [ ] Mais personalidades + sprite sheets
- [ ] Executável assinado (Windows/macOS)
- [ ] Instalador .msi / .apk distribuível

---

## 🤝 Contribuindo

1. Fork
2. Branch: `feature/nome` ou `fix/nome`
3. Commit convencional (`feat:`, `fix:`, `docs:`)
4. PR com descrição clara

---

## 📞 Contato

Issues no GitHub para bugs/feature requests.  
Discussões técnicas: abra uma *Discussion*.

---

**Pet Bolha** — *Seu companheiro de tela, offline e só seu.* 🫧