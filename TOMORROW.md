# TOMORROW — Pet Bolha

> Estado do projeto Pet Bolha (pet de bolha offline cross-platform) na virada de sessão.

---

## ✅ COMPLETADO NESTA SESSÃO (2026-11-04)

### Repo Público Criado
- **GitHub**: https://github.com/LiruhM/pet-bolha
- **Commit inicial**: `be24a49` — "feat: initial commit - Pet Bolha core + Windows overlay + assets"

### Arquitetura Implementada

#### pet-core (Rust library)
| Módulo | Linhas | Descrição |
|--------|--------|-----------|
| `state.rs` | ~400 | State machine: 4 necessidades (fome/sono/tédio/carinho), 10 humores, ações |
| `neural.rs` | ~500 | LIF Neural Network: 470 neurônios, STDP, thread-safe (Arc<RwLock>) |
| `memory.rs` | ~300 | JSONL append-only store com tags, daily summary, export |
| `speech.rs` | ~600 | 500+ templates PT-BR, seleção por peso humor+contexto+cooldown |
| `personality.rs` | ~300 | 3 personalidades (Bryan/Jorge/Balanced) + config visual/comportamento |
| `ffi.rs` | ~300 | C FFI exports (create/destroy/tick/interact/get_state/get_speech) |

#### pet-windows (Windows overlay nativo)
- **Janela**: `WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW` (SEM WS_EX_LAYERED = 0 custo DWM)
- **Render**: tiny-skia CPU, event-driven (5 FPS acordado, 1 frame/2s idle, 0% CPU dormindo)
- **Input**: WM_MOUSEMOVE (hover=carinho), WM_LBUTTONDOWN (brincar), WM_RBUTTONDOWN (carinho)
- **Hotkey pânico**: `Ctrl+Alt+Shift+P` desliga tudo instantaneamente
- **Sprite rendering**: 64x64 frames, 1.5x scale, animações por estado

### Assets Gerados
| Asset | Descrição |
|-------|-----------|
| `bolha.png` | Sprite sheet 512x256 (32 frames, 8 animações) + `bolha_frames.json` |
| `bryan.png` | Sprite sheet personalidade revoltada + frames map |
| `jorge.png` | Sprite sheet coach satírico + frames map |
| `generate_sprites.py` | Script Python para regenerar |

### Documentação
- `README.md` — Arquitetura completa, build instructions, roadmap, status "EM CONSTRUÇÃO"
- Repo público com issues/discussions habilitados

---

## ✅ COMPLETADO HOJE (2026-11-05)

### pet-core — Compilação e Testes Verificados
- **cargo check --workspace**: ✅ Compila sem erros (apenas warnings de documentação)
- **cargo build --release -p pet-core**: ✅ Build release bem-sucedido (~45s)
- **cargo test -p pet-core**: ✅ 1 teste passou (memory::tests::test_memory_store)
- **Assets copiados** para `target/release/assets/sprites/`

### pet-android — Estrutura Completa Criada
- **Gradle build** (`pet-android/build.gradle.kts`) com Kotlin 2.0, minSdk 24, targetSdk 34
- **AndroidManifest.xml** com permissões overlay, foreground service, boot receiver, sensores
- **JNI Bindings** (`PetNative.kt`) — mapeamento 1:1 das funções FFI do pet-core
- **PetOverlayService.kt** — Service de overlay com:
  - WindowManager TYPE_APPLICATION_OVERLAY (click-through, não foca)
  - Touch handling: tap=carinho, drag=mover, long press=brincar
  - SensorManager: acelerômetro → cursor position, proximidade → carinho
  - Tick loop 10Hz via coroutines (pet_tick + pet_get_state + pet_get_speech)
  - Wake lock PARTIAL_WAKE_LOCK para manter vivo
  - Foreground service com notificação persistente
  - Binder para MainActivity comunicar
- **BootReceiver.kt** — Auto-start no boot/quickboot
- **MainActivity.kt** + **activity_main.xml** — Config UI:
  - Nome do pet, seleção personalidade (Balanced/Bryan/Jorge)
  - Toggles: fala automática, rede neural
  - Botões: alimentar, dormir, ver estado JSON, salvar config
- **CMakeLists.txt** + **pet_jni.cpp** + **pet_core.h** — Wrapper C++ para JNI
- **build_android.sh** — Script para compilar pet-core via cargo + NDK (arm64-v8a, armeabi-v7a)
- **pet-android/Cargo.toml** + **src/lib.rs** — Placeholder no workspace Cargo

### Correções Aplicadas no pet-core
- Adicionado `rand_chacha`, `directories` ao Cargo.toml
- Habilitado feature `serde` no `nalgebra` para serialização de matrizes
- Corrigido macro `add_template!` em `speech.rs` → função `add_template`
- Adicionado `Copy` ao `Temperament` e `PartialEq` ao `SleepEventKind`
- Tornado campos de `PetStats` públicos
- Corrigido imports de `chrono::Timelike` onde necessário
- Corrigido métodos `rng.gen_range()` / `rng.gen()` (rand 0.8 API)
- Adicionado módulo `matrix_serde` para serialização de `DMatrix<f32>`
- Ajustado imports não-utilizados em `neural.rs`, `speech.rs`, `ffi.rs`

---

## ⚠️ PET-WINDOWS — BLOQUEADO
- **Erro**: API do crate `windows` 0.58 mudou (breaking changes em HWND, HBITMAP, GDI functions)
- **Decisão**: Deixar para depois — foco no Android que é prioridade alta no roadmap
- **Workaround futuro**: Downgrade `windows` crate ou migrar para API nova

---

## 📁 ESTRUTURA NO DISCO

```
Hermes_Brain/10-Projetos/pet-bolha/
├── Cargo.toml                     # Workspace (pet-core, pet-windows, pet-android)
├── README.md                      # Docs completas
├── build_android.sh               # Script build Android via NDK
├── pet-core/
│   ├── src/                       # 6 módulos: state, neural, memory, speech, personality, ffi
│   └── Cargo.toml
├── pet-windows/                   # (com erros de API windows 0.58 - deixar pra depois)
│   ├── src/main.rs
│   └── Cargo.toml
├── pet-android/                   # ✅ ESTRUTURA COMPLETA
│   ├── build.gradle.kts           # Gradle + Kotlin + CMake
│   ├── CMakeLists.txt             # JNI wrapper build
│   ├── src/
│   │   ├── main/
│   │   │   ├── AndroidManifest.xml
│   │   │   ├── java/com/petbolha/
│   │   │   │   ├── PetNative.kt          # JNI declarations
│   │   │   │   ├── PetOverlayService.kt  # Overlay service principal
│   │   │   │   ├── BootReceiver.kt       # Auto-start
│   │   │   │   └── MainActivity.kt       # Config UI
│   │   │   ├── cpp/
│   │   │   │   ├── pet_jni.cpp           # JNI implementation
│   │   │   │   └── pet_core.h            # C header
│   │   │   ├── res/layout/activity_main.xml
│   │   │   └── res/values/strings.xml
│   │   └── jniLibs/arm64-v8a|armeabi-v7a/  # (vazio - preenchido por build_android.sh)
│   ├── Cargo.toml                 # Placeholder workspace
│   └── src/lib.rs
└── assets/
    ├── sprites/
    │   ├── bolha.png / _frames.json
    │   ├── bryan.png / _frames.json
    │   ├── jorge.png / _frames.json
    │   └── generate_sprites.py
    └── speech/
```

---

## 🎯 PRÓXIMOS PASSOS (QUANDO RETOMAR)

### 1. Build Android Real (Prioridade Alta)
```bash
# Requer Android NDK instalado
export ANDROID_NDK_HOME=$HOME/Android/Sdk/ndk/26.1.10909125
cd pet-bolha
./build_android.sh
```
- Compila `pet-core` para `aarch64-linux-android` + `armv7-linux-androideabi`
- Copia `.so` para `pet-android/src/main/jniLibs/<abi>/`
- Abre no Android Studio → Build → Run

### 2. Testar no Dispositivo/Emulador
- Conceder permissão "Desenhar sobre outros apps"
- Iniciar service → pet aparece na tela
- Verificar: touch, acelerômetro, proximidade, fala, animações

### 3. Assets Android
- Copiar `assets/sprites/*.png` para `pet-android/src/main/res/drawable-nodpi/`
- Implementar `loadAnimationFrame` em `PetOverlayService` para carregar sprites reais

### 4. Sync Local (Wi-Fi/USB) — Após Android funcionando
- Protocolo QUIC/TLS device-to-device (sem nuvem)
- Chaves trocadas no primeiro pairing (como NectarLink)
- Sync: memory.jsonl + neural_weights.json + config.json

### 5. TTS/STT Local
- **Piper** (TTS) — compila para Windows/Android via FFI
- **Vosk** (STT) — modelo pequeno PT-BR (~50MB)
- Integração no `speech.rs` para fala/escuta real

### 6. Config UI Avançada (Tauri/egui)
- Settings window para:
  - Trocar personalidade
  - Ajustar sensibilidade necessidades
  - Toggle neural/speech/click-through
  - Ver memory.jsonl / export

### 7. Mais Personalidades + Sprites
- Adicionar variações no `generate_sprites.py`
- Personalidade "Arquiteto" (integração Solo Leveller)

---

## 🔗 ARQUIVOS CHAVE PARA RETOMAR

| Arquivo | Uso |
|---------|-----|
| `README.md` | Build instructions, arquitetura, roadmap |
| `pet-core/src/ffi.rs` | Interface C para Windows/Android |
| `pet-android/build.gradle.kts` | Gradle config |
| `pet-android/src/main/java/com/petbolha/PetOverlayService.kt` | Service principal |
| `pet-android/src/main/cpp/pet_jni.cpp` | JNI bridge |
| `build_android.sh` | Script build NDK |
| `https://github.com/LiruhM/pet-bolha` | Repo público |

---

## 💤 CONTEXTO DA SESSÃO

- **Início**: 2026-11-04 ~16h (boa tarde time)
- **Fim**: 2026-11-04 ~23h (CEO indo dormir)
- **Retomada**: 2026-11-05 ~12h — pet-core verificado compila + testes passam
- **Continuação**: 2026-11-05 ~14h — pet-android estrutura completa criada
- **Duração total**: ~7h + verificação + Android structure
- **Entregáveis**: 
  - Repo público completo com core + Windows overlay + assets + docs
  - pet-core build/test OK
  - pet-android estrutura completa (Gradle + JNI + Service + UI + CMake)
- **Bloqueador**: pet-windows API `windows` 0.58 breaking changes
- **Próximo real**: Build Android via NDK + testar no POCO

---

**Pet-core ✅ | Pet-android structure ✅ | Pet-windows ⚠️ (deixar pra depois).** 🫧 Próximo: `./build_android.sh` com NDK instalado.