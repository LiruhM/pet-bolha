//! State Machine - Comportamento do pet baseado em necessidades e humor

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

/// Necessidades básicas do pet (0.0 a 1.0, onde 1.0 = satisfeito)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Needs {
    pub hunger: f32,      // Fome - diminui com tempo, aumenta com "comida" (interação)
    pub sleep: f32,       // Sono - diminui acordado, aumenta dormindo
    pub boredom: f32,     // Tédio - diminui sozinho, aumenta com brincadeira
    pub affection: f32,   // Carinho - diminui ignorado, aumenta com hover/click
}

impl Default for Needs {
    fn default() -> Self {
        Self {
            hunger: 1.0,
            sleep: 1.0,
            boredom: 1.0,
            affection: 1.0,
        }
    }
}

impl Needs {
    /// Decaimento natural por segundo (ajustável por personalidade)
    pub fn decay(&mut self, dt_secs: f32, temperament: Temperament) {
        let decay_rate = temperament.need_decay_rate();
        self.hunger = (self.hunger - decay_rate.hunger * dt_secs).clamp(0.0, 1.0);
        self.sleep = (self.sleep - decay_rate.sleep * dt_secs).clamp(0.0, 1.0);
        self.boredom = (self.boredom - decay_rate.boredom * dt_secs).clamp(0.0, 1.0);
        self.affection = (self.affection - decay_rate.affection * dt_secs).clamp(0.0, 1.0);
    }

    /// Aplica uma ação do usuário
    pub fn apply_action(&mut self, action: UserAction) {
        match action {
            UserAction::Pet => {
                self.affection = (self.affection + 0.15).clamp(0.0, 1.0);
                self.boredom = (self.boredom + 0.05).clamp(0.0, 1.0);
            }
            UserAction::Play => {
                self.boredom = (self.boredom + 0.25).clamp(0.0, 1.0);
                self.hunger = (self.hunger - 0.05).clamp(0.0, 1.0);
                self.affection = (self.affection + 0.1).clamp(0.0, 1.0);
            }
            UserAction::Feed => {
                self.hunger = (self.hunger + 0.3).clamp(0.0, 1.0);
            }
            UserAction::Sleep => {
                self.sleep = (self.sleep + 0.4).clamp(0.0, 1.0);
            }
            UserAction::Ignore => {
                // Decaimento extra por ignorar
                self.affection = (self.affection - 0.02).clamp(0.0, 1.0);
                self.boredom = (self.boredom - 0.01).clamp(0.0, 1.0);
            }
        }
    }

    /// Retorna a necessidade mais urgente (menor valor)
    pub fn most_urgent(&self) -> Need {
        let needs = [
            (Need::Hunger, self.hunger),
            (Need::Sleep, self.sleep),
            (Need::Boredom, self.boredom),
            (Need::Affection, self.affection),
        ];
        needs.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap().0
    }

    /// Score geral de bem-estar (0.0 a 1.0)
    pub fn wellbeing(&self) -> f32 {
        (self.hunger + self.sleep + self.boredom + self.affection) / 4.0
    }
}

/// Tipos de necessidade
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Need {
    Hunger,
    Sleep,
    Boredom,
    Affection,
}

/// Ações do usuário
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum UserAction {
    Pet,      // Carinho (hover/click suave)
    Play,     // Brincar (click+drag, bola)
    Feed,     // Alimentar (item especial)
    Sleep,    // Colocar pra dormir
    Ignore,   // Usuário presente mas sem interagir
}

/// Humores possíveis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mood {
    Happy,        // Todas necessidades > 0.7
    Content,      // Bem-estar > 0.5
    Neutral,      // Bem-estar > 0.3
    Bored,        // Tédio baixo
    Hungry,       // Fome baixa
    Sleepy,       // Sono baixo
    Lonely,       // Carinho baixo
    Sad,          // Múltiplas baixas
    Dramatic,     // Crítico - protesta
    Sleeping,     // Dormindo (sono > 0.9 + inativo)
}

impl Mood {
    pub fn from_needs(needs: &Needs, is_idle: bool) -> Self {
        let wellbeing = needs.wellbeing();
        
        // Dormindo tem prioridade
        if needs.sleep > 0.9 && is_idle {
            return Mood::Sleeping;
        }

        // Necessidades críticas individuais
        if needs.affection < 0.15 { return Mood::Dramatic; }
        if needs.hunger < 0.15 { return Mood::Hungry; }
        if needs.sleep < 0.15 { return Mood::Sleepy; }
        if needs.boredom < 0.15 { return Mood::Bored; }
        if needs.affection < 0.3 { return Mood::Lonely; }
        
        // Múltiplas baixas
        let critical_count = [needs.hunger, needs.sleep, needs.boredom, needs.affection]
            .iter().filter(|&&v| v < 0.3).count();
        if critical_count >= 2 { return Mood::Sad; }

        // Bem-estar geral
        if wellbeing > 0.75 { Mood::Happy }
        else if wellbeing > 0.55 { Mood::Content }
        else if wellbeing > 0.35 { Mood::Neutral }
        else { Mood::Sad }
    }

    /// Peso para seleção de fala
    pub fn speech_weight(&self) -> f32 {
        match self {
            Mood::Happy => 1.0,
            Mood::Content => 0.9,
            Mood::Neutral => 0.7,
            Mood::Bored | Mood::Hungry | Mood::Sleepy | Mood::Lonely => 0.6,
            Mood::Sad => 0.5,
            Mood::Dramatic => 0.4,
            Mood::Sleeping => 0.1,
        }
    }
}

/// Temperamento base (personalidade)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Temperament {
    pub base_decay: NeedDecayRate,
    pub mood_sensitivity: f32,      // Quão rápido muda de humor
    pub playfulness: f32,           // Probabilidade de iniciar brincadeira
    pub chattiness: f32,            // Frequência de falas
    pub independence: f32,          // Quanto tempo fica sozinho sem ficar triste
    pub forgiveness: f32,           // Quão rápido perdoa ignorância
}

impl Default for Temperament {
    fn default() -> Self {
        Self {
            base_decay: NeedDecayRate::default(),
            mood_sensitivity: 0.5,
            playfulness: 0.5,
            chattiness: 0.5,
            independence: 0.5,
            forgiveness: 0.5,
        }
    }
}

impl Temperament {
    pub fn need_decay_rate(&self) -> NeedDecayRate {
        NeedDecayRate {
            hunger: self.base_decay.hunger * (2.0 - self.independence),
            sleep: self.base_decay.sleep,
            boredom: self.base_decay.boredom * (2.0 - self.playfulness),
            affection: self.base_decay.affection * (2.0 - self.forgiveness),
        }
    }

    /// Temperamento "Bryan" - revoltado, brincalhão, dramático
    pub fn bryan() -> Self {
        Self {
            base_decay: NeedDecayRate { hunger: 0.0001, sleep: 0.00005, boredom: 0.0002, affection: 0.0003 },
            mood_sensitivity: 0.8,
            playfulness: 0.9,
            chattiness: 0.7,
            independence: 0.3,
            forgiveness: 0.2,
        }
    }

    /// Temperamento "Jorge" - coach, falante, persistente
    pub fn jorge() -> Self {
        Self {
            base_decay: NeedDecayRate { hunger: 0.00008, sleep: 0.00008, boredom: 0.00015, affection: 0.00025 },
            mood_sensitivity: 0.6,
            playfulness: 0.4,
            chattiness: 0.95,
            independence: 0.6,
            forgiveness: 0.4,
        }
    }

    /// Temperamento padrão equilibrado
    pub fn balanced() -> Self {
        Self::default()
    }
}

/// Taxas de decaimento por segundo
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NeedDecayRate {
    pub hunger: f32,
    pub sleep: f32,
    pub boredom: f32,
    pub affection: f32,
}

impl Default for NeedDecayRate {
    fn default() -> Self {
        // Valores calibrados para ~4-6h até ficar crítico
        Self {
            hunger: 0.00008,      // ~3.5h de 1.0 a 0.0
            sleep: 0.00006,       // ~4.5h
            boredom: 0.00015,     // ~2h
            affection: 0.0002,    // ~1.5h
        }
    }
}

/// Estado completo do pet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetState {
    pub id: Uuid,
    pub name: String,
    pub personality_id: String,
    pub needs: Needs,
    pub mood: Mood,
    pub temperament: Temperament,
    pub last_update: DateTime<Utc>,
    pub last_interaction: DateTime<Utc>,
    pub is_idle: bool,
    pub idle_since: Option<DateTime<Utc>>,
    pub sleep_since: Option<DateTime<Utc>>,
    pub stats: PetStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PetStats {
    total_pets: u64,
    total_plays: u64,
    total_feeds: u64,
    total_sleeps: u64,
    words_spoken: u64,
    days_alive: u32,
}

impl PetState {
    pub fn new(name: String, temperament: Temperament) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            personality_id: "default".to_string(),
            needs: Needs::default(),
            mood: Mood::Content,
            temperament,
            last_update: now,
            last_interaction: now,
            is_idle: false,
            idle_since: None,
            sleep_since: None,
            stats: PetStats::default(),
        }
    }

    /// Atualiza estado (chamado periodicamente quando acordado)
    pub fn tick(&mut self, dt_secs: f32) {
        let now = Utc::now();
        self.last_update = now;
        
        // Decaimento natural
        self.needs.decay(dt_secs, self.temperament);
        
        // Detecta idle (sem interação por 30s)
        let idle_duration = now.signed_duration_since(self.last_interaction);
        self.is_idle = idle_duration > Duration::seconds(30);
        if self.is_idle && self.idle_since.is_none() {
            self.idle_since = Some(now);
        } else if !self.is_idle {
            self.idle_since = None;
        }

        // Sono automático se sono crítico e idle
        if self.needs.sleep < 0.1 && self.is_idle && self.sleep_since.is_none() {
            self.sleep_since = Some(now);
        } else if self.needs.sleep > 0.5 {
            self.sleep_since = None;
        }

        // Atualiza humor
        self.mood = Mood::from_needs(&self.needs, self.is_idle);
    }

    /// Aplica ação do usuário
    pub fn interact(&mut self, action: UserAction) {
        self.needs.apply_action(action);
        self.last_interaction = Utc::now();
        self.is_idle = false;
        self.idle_since = None;
        
        // Stats
        match action {
            UserAction::Pet => self.stats.total_pets += 1,
            UserAction::Play => self.stats.total_plays += 1,
            UserAction::Feed => self.stats.total_feeds += 1,
            UserAction::Sleep => self.stats.total_sleeps += 1,
            UserAction::Ignore => {}
        }
        
        self.mood = Mood::from_needs(&self.needs, self.is_idle);
    }

    /// Verifica se deve falar agora (baseado em chattiness + mood + tempo)
    pub fn should_speak(&self, min_interval_secs: f32) -> bool {
        let since_last = Utc::now().signed_duration_since(self.last_interaction);
        if since_last.num_seconds() as f32 < min_interval_secs {
            return false;
        }
        
        let chattiness = self.temperament.chattiness;
        let mood_weight = self.mood.speech_weight();
        let threshold = 0.3 * (1.0 - chattiness * mood_weight);
        
        // Aleatoriedade determinística baseada no tempo
        let seed = (Utc::now().timestamp() as u64) ^ self.id.as_u128() as u64;
        let random = (seed.wrapping_mul(1103515245).wrapping_add(12345) >> 16) as f32 / u16::MAX as f32;
        
        random < threshold
    }
}

/// Ações autônomas do pet (decididas pela state machine)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PetAction {
    Idle,           // Parado, respirando
    Wander,         // Anda pela tela
    Sleep,          // Dorme no canto
    Think,          // "Pensando" (bolha de pensamento)
    Protest,        // Protesta (cartazes)
    RequestPet,     // Pede carinho (olha pro cursor)
    RequestPlay,    // Traz bolinha
    RequestFood,    // Indica fome
    Celebrate,      // Fogos (muito feliz)
    Hide,           // Esconde (medo/tristeza)
}

impl PetAction {
    /// Decide ação baseada no estado atual
    pub fn decide(state: &PetState) -> Self {
        match state.mood {
            Mood::Sleeping => PetAction::Sleep,
            Mood::Dramatic => PetAction::Protest,
            Mood::Sad => PetAction::Hide,
            Mood::Lonely => PetAction::RequestPet,
            Mood::Hungry => PetAction::RequestFood,
            Mood::Bored => PetAction::RequestPlay,
            Mood::Happy => {
                if state.needs.wellbeing() > 0.9 {
                    PetAction::Celebrate
                } else {
                    PetAction::Wander
                }
            }
            Mood::Content => PetAction::Wander,
            Mood::Neutral => PetAction::Think,
            Mood::Sleepy => PetAction::Sleep,
        }
    }
}