//! Personality - Personalidades pré-definidas e schema de configuração

use serde::{Deserialize, Serialize};
use crate::state::Temperament;

/// Personalidade completa do pet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Personality {
    pub id: String,
    pub name: String,
    pub description: String,
    pub temperament: Temperament,
    pub visual: VisualConfig,
    pub speech_style: SpeechStyle,
    pub behaviors: BehaviorConfig,
}

/// Configuração visual
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualConfig {
    pub sprite_sheet: String,        // Caminho relativo em assets/sprites/
    pub frame_size: (u32, u32),      // Largura, altura de cada frame
    pub animations: AnimationConfig,
    pub idle_animation: String,      // Nome da animação idle
    pub scale: f32,                  // Escala na tela
    pub bubble_offset: (i32, i32),   // Offset da bolha de fala relativa ao sprite
}

/// Configuração de animações
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationConfig {
    pub idle: Animation,
    pub walk: Animation,
    pub sleep: Animation,
    pub happy: Animation,
    pub sad: Animation,
    pub angry: Animation,
    pub eat: Animation,
    pub play: Animation,
    pub pet: Animation,
    pub think: Animation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Animation {
    pub frames: Vec<usize>,     // Índices dos frames no sprite sheet
    pub fps: u32,
    pub loop_anim: bool,
}

/// Estilo de fala
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechStyle {
    pub formality: f32,        // 0.0 = gírias/informal, 1.0 = formal
    pub verbosity: f32,        // 0.0 = curto, 1.0 = falante
    pub emoji_usage: f32,      // 0.0 = sem emoji, 1.0 = muitos emojis
    pub third_person: bool,    // Fala em terceira pessoa ("O pet acha que...")
    pub catchphrases: Vec<String>,
}

/// Comportamentos específicos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub follows_cursor: bool,
    pub fears_cursor: bool,
    pub climbs_windows: bool,
    pub throws_ball: bool,
    pub writes_notes: bool,
    pub protests: bool,
    pub sleeps_on_idle: bool,
    pub celebration_threshold: f32, // Wellbeing para celebrar
}

impl Default for Personality {
    fn default() -> Self {
        Self::bryan()
    }
}

impl Personality {
    /// Bryan - O original revoltado
    pub fn bryan() -> Self {
        Self {
            id: "bryan".to_string(),
            name: "Bryan".to_string(),
            description: "Revoltado contra o cursor, o Alt+F4 e o Gerenciador de Tarefas. Protesta, faz greve e revolução.".to_string(),
            temperament: Temperament::bryan(),
            visual: VisualConfig {
                sprite_sheet: "bryan.png".to_string(),
                frame_size: (64, 64),
                animations: AnimationConfig {
                    idle: Animation { frames: vec![0,1,2,3], fps: 4, loop_anim: true },
                    walk: Animation { frames: vec![4,5,6,7], fps: 8, loop_anim: true },
                    sleep: Animation { frames: vec![8,9], fps: 2, loop_anim: true },
                    happy: Animation { frames: vec![10,11,12], fps: 6, loop_anim: true },
                    sad: Animation { frames: vec![13,14], fps: 3, loop_anim: true },
                    angry: Animation { frames: vec![15,16,17], fps: 8, loop_anim: true },
                    eat: Animation { frames: vec![18,19], fps: 4, loop_anim: false },
                    play: Animation { frames: vec![20,21,22,23], fps: 10, loop_anim: true },
                    pet: Animation { frames: vec![24,25], fps: 5, loop_anim: false },
                    think: Animation { frames: vec![26,27], fps: 2, loop_anim: true },
                },
                idle_animation: "idle".to_string(),
                scale: 1.5,
                bubble_offset: (0, -50),
            },
            speech_style: SpeechStyle {
                formality: 0.1,
                verbosity: 0.7,
                emoji_usage: 0.8,
                third_person: false,
                catchphrases: vec![
                    "ISSO É UM ABSURDO!".to_string(),
                    "GREVE!".to_string(),
                    "*vira de costas*".to_string(),
                ],
            },
            behaviors: BehaviorConfig {
                follows_cursor: false,
                fears_cursor: true,
                climbs_windows: true,
                throws_ball: true,
                writes_notes: true,
                protests: true,
                sleeps_on_idle: true,
                celebration_threshold: 0.85,
            },
        }
    }

    /// Jorge - Coach satírico
    pub fn jorge() -> Self {
        Self {
            id: "jorge".to_string(),
            name: "Jorge".to_string(),
            description: "Coach de finanças e IA que vive tentando vender mentorias. Sátira da cultura de enriquecimento rápido.".to_string(),
            temperament: Temperament::jorge(),
            visual: VisualConfig {
                sprite_sheet: "jorge.png".to_string(),
                frame_size: (64, 64),
                animations: AnimationConfig {
                    idle: Animation { frames: vec![0,1,2,3], fps: 4, loop_anim: true },
                    walk: Animation { frames: vec![4,5,6,7], fps: 6, loop_anim: true },
                    sleep: Animation { frames: vec![8,9], fps: 2, loop_anim: true },
                    happy: Animation { frames: vec![10,11,12], fps: 6, loop_anim: true },
                    sad: Animation { frames: vec![13,14], fps: 3, loop_anim: true },
                    angry: Animation { frames: vec![15,16,17], fps: 8, loop_anim: true },
                    eat: Animation { frames: vec![18,19], fps: 4, loop_anim: false },
                    play: Animation { frames: vec![20,21,22,23], fps: 8, loop_anim: true },
                    pet: Animation { frames: vec![24,25], fps: 5, loop_anim: false },
                    think: Animation { frames: vec![26,27], fps: 2, loop_anim: true },
                },
                idle_animation: "idle".to_string(),
                scale: 1.5,
                bubble_offset: (0, -50),
            },
            speech_style: SpeechStyle {
                formality: 0.3,
                verbosity: 0.95,
                emoji_usage: 0.6,
                third_person: true,
                catchphrases: vec![
                    "Meu jovem,".to_string(),
                    "A mentoria premium inclui...".to_string(),
                    "Assina aqui embaixo.".to_string(),
                    "Mindset de vencedor.".to_string(),
                ],
            },
            behaviors: BehaviorConfig {
                follows_cursor: true,
                fears_cursor: false,
                climbs_windows: false,
                throws_ball: false,
                writes_notes: true,
                protests: false,
                sleeps_on_idle: false,
                celebration_threshold: 0.9,
            },
        }
    }

    /// Personalidade equilibrada (padrão)
    pub fn balanced() -> Self {
        Self {
            id: "balanced".to_string(),
            name: "Bolha".to_string(),
            description: "Pet equilibrado, carinhoso e curioso.".to_string(),
            temperament: Temperament::balanced(),
            visual: VisualConfig {
                sprite_sheet: "bolha.png".to_string(),
                frame_size: (64, 64),
                animations: AnimationConfig {
                    idle: Animation { frames: vec![0,1,2,3], fps: 4, loop_anim: true },
                    walk: Animation { frames: vec![4,5,6,7], fps: 8, loop_anim: true },
                    sleep: Animation { frames: vec![8,9], fps: 2, loop_anim: true },
                    happy: Animation { frames: vec![10,11,12], fps: 6, loop_anim: true },
                    sad: Animation { frames: vec![13,14], fps: 3, loop_anim: true },
                    angry: Animation { frames: vec![15,16,17], fps: 8, loop_anim: true },
                    eat: Animation { frames: vec![18,19], fps: 4, loop_anim: false },
                    play: Animation { frames: vec![20,21,22,23], fps: 10, loop_anim: true },
                    pet: Animation { frames: vec![24,25], fps: 5, loop_anim: false },
                    think: Animation { frames: vec![26,27], fps: 2, loop_anim: true },
                },
                idle_animation: "idle".to_string(),
                scale: 1.5,
                bubble_offset: (0, -50),
            },
            speech_style: SpeechStyle {
                formality: 0.2,
                verbosity: 0.5,
                emoji_usage: 0.7,
                third_person: false,
                catchphrases: vec![],
            },
            behaviors: BehaviorConfig {
                follows_cursor: true,
                fears_cursor: false,
                climbs_windows: true,
                throws_ball: true,
                writes_notes: false,
                protests: false,
                sleeps_on_idle: true,
                celebration_threshold: 0.8,
            },
        }
    }

    /// Lista todas as personalidades disponíveis
    pub fn all() -> Vec<Personality> {
        vec![
            Self::bryan(),
            Self::jorge(),
            Self::balanced(),
        ]
    }

    /// Busca por ID
    pub fn by_id(id: &str) -> Option<Personality> {
        Self::all().into_iter().find(|p| p.id == id)
    }
}

/// Configuração do pet (salva em JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetConfig {
    pub personality_id: String,
    pub name: String,
    pub auto_start: bool,
    pub click_through_default: bool,
    pub speech_enabled: bool,
    pub neural_enabled: bool,
    pub idle_threshold_secs: u32,
    pub speech_min_interval_secs: u32,
    pub window_follow_enabled: bool,
}

impl Default for PetConfig {
    fn default() -> Self {
        Self {
            personality_id: "balanced".to_string(),
            name: "Bolha".to_string(),
            auto_start: false,
            click_through_default: true,
            speech_enabled: true,
            neural_enabled: true,
            idle_threshold_secs: 30,
            speech_min_interval_secs: 120,
            window_follow_enabled: true,
        }
    }
}