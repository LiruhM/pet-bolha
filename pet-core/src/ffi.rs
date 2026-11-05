//! FFI Layer - Interface C para uso pelo overlay Windows
//!
//! Expõe funções simples que o pet-windows pode chamar via FFI.
//! O core roda em Rust, o overlay chama estas funções.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_float, c_int, c_uint, c_void};
use std::ptr;
use crate::state::{PetState, Needs, Mood, UserAction, PetAction, Temperament};
use crate::neural::{NeuralEngine, SensorInput, NetworkConfig};
use crate::memory::MemoryStore;
use crate::speech::SpeechEngine;
use crate::personality::{Personality, PetConfig};

// Handles opacos para objetos Rust
pub struct PetHandle {
    state: PetState,
    neural: Option<NeuralEngine>,
    memory: Option<MemoryStore>,
    speech: SpeechEngine,
    config: PetConfig,
}

/// Cria novo pet
#[no_mangle]
pub extern "C" fn pet_create(
    name: *const c_char,
    personality_id: *const c_char,
    config_json: *const c_char,
) -> *mut PetHandle {
    let name = unsafe { CStr::from_ptr(name).to_string_lossy().into_owned() };
    let personality_id = unsafe { CStr::from_ptr(personality_id).to_string_lossy().into_owned() };
    
    let config: PetConfig = if config_json.is_null() {
        PetConfig::default()
    } else {
        let json = unsafe { CStr::from_ptr(config_json).to_string_lossy().into_owned() };
        serde_json::from_str(&json).unwrap_or_default()
    };
    
    let personality = Personality::by_id(&personality_id).unwrap_or_else(Personality::balanced);
    let temperament = personality.temperament.clone();
    
    let mut state = PetState::new(name, temperament);
    state.personality_id = personality_id;
    
    let neural = if config.neural_enabled {
        Some(NeuralEngine::new(NetworkConfig::default(), state.id.as_u128() as u64))
    } else {
        None
    };
    
    let memory = MemoryStore::open("pet-bolha").ok();
    let speech = SpeechEngine::with_builtin();
    
    let handle = Box::new(PetHandle {
        state,
        neural,
        memory,
        speech,
        config,
    });
    
    Box::into_raw(handle)
}

/// Libera pet
#[no_mangle]
pub extern "C" fn pet_destroy(handle: *mut PetHandle) {
    if !handle.is_null() {
        unsafe { Box::from_raw(handle); }
    }
}

/// Tick principal (chamado periodicamente quando acordado)
#[no_mangle]
pub extern "C" fn pet_tick(handle: *mut PetHandle, dt_secs: c_float) {
    if handle.is_null() { return; }
    let handle = unsafe { &mut *handle };
    
    handle.state.tick(dt_secs);
    
    // Neural tick se habilitado
    if let (Some(neural), Some(memory)) = (&mut handle.neural, &handle.memory) {
        let sensors = SensorInput {
            cursor_x: 0.5, // Atualizado pelo overlay via pet_set_cursor
            cursor_y: 0.5,
            active_window_hash: 0.0,
            idle_seconds: handle.state.idle_since
                .map(|t| (chrono::Utc::now() - t).num_seconds() as f32)
                .unwrap_or(0.0),
            time_of_day: (chrono::Utc::now().hour() as f32) / 24.0,
            battery_level: 1.0, // Atualizado pelo overlay
            screen_on: true,
            recent_interaction: 0.0,
        };
        
        let output = neural.tick(sensors, &handle.state.needs);
        // TODO: usar output para modular comportamento
        let _ = output;
    }
    
    // Fala automática
    if handle.config.speech_enabled {
        let hour = chrono::Utc::now().hour();
        if let Some(text) = handle.speech.speak(
            handle.state.mood,
            &[],
            &handle.state.needs,
            hour,
        ) {
            // TODO: callback para overlay mostrar bolha
            let _ = text;
        }
    }
}

/// Interação do usuário
#[no_mangle]
pub extern "C" fn pet_interact(handle: *mut PetHandle, action: c_int) {
    if handle.is_null() { return; }
    let handle = unsafe { &mut *handle };
    
    let action = match action {
        0 => UserAction::Pet,
        1 => UserAction::Play,
        2 => UserAction::Feed,
        3 => UserAction::Sleep,
        4 => UserAction::Ignore,
        _ => return,
    };
    
    handle.state.interact(action);
    
    // Feedback de fala imediato
    if handle.config.speech_enabled {
        if let Some(text) = handle.speech.react_to_action(action, handle.state.mood) {
            // TODO: callback para overlay
            let _ = text;
        }
    }
    
    // Salva na memória
    if let Some(memory) = &handle.memory {
        let _ = memory.append(
            crate::memory::MemoryKind::Interaction {
                action,
                mood_before: handle.state.mood,
                mood_after: handle.state.mood,
            },
            vec!["interaction".to_string()],
        );
    }
}

/// Atualiza posição do cursor
#[no_mangle]
pub extern "C" fn pet_set_cursor(handle: *mut PetHandle, x: c_float, y: c_float) {
    if handle.is_null() { return; }
    // Armazenado para próximo neural tick
    // TODO: adicionar campo no handle
    let _ = (x, y);
}

/// Atualiza janela ativa
#[no_mangle]
pub extern "C" fn pet_set_active_window(handle: *mut PetHandle, window_name: *const c_char) {
    if handle.is_null() { return; }
    let _name = unsafe { CStr::from_ptr(window_name).to_string_lossy().into_owned() };
    // Hash simples
    // TODO: armazenar
}

/// Obtém estado atual (para overlay renderizar)
#[no_mangle]
pub extern "C" fn pet_get_state(handle: *mut PetHandle, out_json: *mut *mut c_char) -> c_int {
    if handle.is_null() || out_json.is_null() { return -1; }
    let handle = unsafe { &*handle };
    
    let json = serde_json::json!({
        "id": handle.state.id.to_string(),
        "name": handle.state.name,
        "personality": handle.state.personality_id,
        "mood": format!("{:?}", handle.state.mood),
        "needs": {
            "hunger": handle.state.needs.hunger,
            "sleep": handle.state.needs.sleep,
            "boredom": handle.state.needs.boredom,
            "affection": handle.state.needs.affection,
            "wellbeing": handle.state.needs.wellbeing(),
        },
        "is_idle": handle.state.is_idle,
        "idle_seconds": handle.state.idle_since.map(|t| (chrono::Utc::now() - t).num_seconds()).unwrap_or(0),
        "action": format!("{:?}", crate::state::PetAction::decide(&handle.state)),
        "stats": {
            "total_pets": handle.state.stats.total_pets,
            "total_plays": handle.state.stats.total_plays,
            "total_feeds": handle.state.stats.total_feeds,
            "words_spoken": handle.state.stats.words_spoken,
        }
    });
    
    let json_str = json.to_string();
    let c_str = CString::new(json_str).unwrap();
    unsafe { *out_json = c_str.into_raw(); }
    0
}

/// Libera string alocada pelo pet_get_state
#[no_mangle]
pub extern "C" fn pet_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { CString::from_raw(ptr); }
    }
}

/// Obtém próxima fala (para overlay mostrar bolha)
#[no_mangle]
pub extern "C" fn pet_get_speech(handle: *mut PetHandle, out_text: *mut *mut c_char) -> c_int {
    if handle.is_null() || out_text.is_null() { return -1; }
    let handle = unsafe { &mut *handle };
    
    let hour = chrono::Utc::now().hour();
    let text = handle.speech.speak(
        handle.state.mood,
        &[],
        &handle.state.needs,
        hour,
    );
    
    match text {
        Some(t) => {
            let c_str = CString::new(t).unwrap();
            unsafe { *out_text = c_str.into_raw(); }
            0
        }
        None => -1,
    }
}

/// Configura se overlay está em click-through
#[no_mangle]
pub extern "C" fn pet_set_click_through(handle: *mut PetHandle, enabled: c_int) {
    if handle.is_null() { return; }
    let handle = unsafe { &mut *handle };
    handle.config.click_through_default = enabled != 0;
}

/// Salva estado (pesos neurais, memória)
#[no_mangle]
pub extern "C" fn pet_save(handle: *mut PetHandle) -> c_int {
    if handle.is_null() { return -1; }
    let handle = unsafe { &mut *handle };
    
    // Salva pesos neurais
    if let Some(neural) = &handle.neural {
        let weights = neural.save();
        let json = serde_json::to_string(&weights).unwrap();
        let _ = std::fs::write(
            dirs::data_dir().unwrap().join("pet-bolha").join("neural_weights.json"),
            json,
        );
    }
    
    // Memória já é append-only auto-save
    0
}

/// Carrega estado salvo
#[no_mangle]
pub extern "C" fn pet_load(handle: *mut PetHandle) -> c_int {
    if handle.is_null() { return -1; }
    let handle = unsafe { &mut *handle };
    
    if let Some(neural) = &handle.neural {
        let path = dirs::data_dir().unwrap().join("pet-bolha").join("neural_weights.json");
        if let Ok(json) = std::fs::read_to_string(path) {
            if let Ok(weights) = serde_json::from_str(&json) {
                neural.load(weights);
            }
        }
    }
    0
}

/// Verifica se pet deve render (acordado)
#[no_mangle]
pub extern "C" fn pet_should_render(handle: *mut PetHandle) -> c_int {
    if handle.is_null() { return 0; }
    let handle = unsafe { &*handle };
    
    // Não render se dormindo profundo e idle
    if handle.state.mood == Mood::Sleeping && handle.state.is_idle {
        return 0;
    }
    
    // Render se acordado ou interagindo recentemente
    if !handle.state.is_idle {
        return 1;
    }
    
    // Render ocasional mesmo idle (respiração)
    1
}

/// Obtém animação atual baseada no humor/ação
#[no_mangle]
pub extern "C" fn pet_get_animation(handle: *mut PetHandle, out_name: *mut *mut c_char) -> c_int {
    if handle.is_null() || out_name.is_null() { return -1; }
    let handle = unsafe { &*handle };
    
    let action = crate::state::PetAction::decide(&handle.state);
    let anim = match action {
        crate::state::PetAction::Sleep => "sleep",
        crate::state::PetAction::Wander => "walk",
        crate::state::PetAction::Play => "play",
        crate::state::PetAction::Protest => "angry",
        crate::state::PetAction::RequestPet | crate::state::PetAction::Think => "think",
        crate::state::PetAction::Celebrate => "happy",
        crate::state::PetAction::Hide => "sad",
        _ => "idle",
    };
    
    let c_str = CString::new(anim).unwrap();
    unsafe { *out_name = c_str.into_raw(); }
    0
}