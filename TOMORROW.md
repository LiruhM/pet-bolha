# TOMORROW — Pet Bolha

> Estado do projeto Pet Bolha (pet de bolha offline cross-platform) na virada de sessão.

---

## ✅ COMPLETADO NESTA SESSÃO (2026-11-06)

### 1. Build Android Real — CONCLUÍDO
- **pet-core** compilado para `aarch64-linux-android` (arm64-v8a) + `armv7-linux-androideabi` (armeabi-v7a)
- `.so` copiados para `pet-android/src/main/jniLibs/`:
  - `arm64-v8a/libpet_core.so` (1.2 MB)
  - `armeabi-v7a/libpet_core.so` (879 KB)
- **Sprites** copiados para `pet-android/src/main/res/drawable-nodpi/`:
  - `bolha.png`, `bryan.png`, `jorge.png`

### 2. pet-windows + TTS — CONCLUÍDO
- Overlay Windows nativo funcional (transparente, click-through, 5 FPS)
- 32 frames extraídos do sprite sheet (8×4 grid)
- Piper TTS integrado (modelo `pt_BR-faber-medium`, offline PT-BR)
- Hotkey pânico `Ctrl+Alt+Shift+P`

### 3. pet-core — OK
- Neural LIF 470 neurônios, memory JSONL, speech 500+ templates
- 3 personalidades (Bryan/Jorge/Balanced)
- FFI C para Windows/Android

### Build Android pronto para testar no POCO:
```bash
# No Android Studio:
# 1. Open pet-bolha/pet-android
# 2. Build → Run
# 3. Conceder permissão "Desenhar sobre outros apps"
# 4. Verificar: touch, acelerômetro, proximidade, animações
```

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