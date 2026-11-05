//! LIF Neural Network - Leaky Integrate-and-Fire (470 neurônios)
//!
//! Portado do Pestinha para Rust puro. Rede neural pulsada que roda
//! em background para dar "personalidade emergente" ao pet.
//!
//! ## Arquitetura
//! - **Canais de entrada**: Sensores (fome, sono, tédio, carinho, cursor, janelas, tempo)
//! - **Interneurônios**: 400 neurônios ocultos (LIF)
//! - **Motores de saída**: 70 neurônios (humor, ações, fala, movimento)
//!
//! ## Performance
//! - ~470 neurônios @ 10Hz = ~5000 updates/sec = <0.1ms CPU
//! - Roda em thread separada, só acorda quando pet está "acordado"

use nalgebra::{DMatrix, DVector};
use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use crate::state::{Needs, Mood, Temperament};

// Helper para serialização de DMatrix
mod matrix_serde {
    use nalgebra::{DMatrix, Dyn, VecStorage};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(matrix: &DMatrix<f32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let rows = matrix.nrows();
        let cols = matrix.ncols();
        let data: Vec<f32> = matrix.iter().cloned().collect();
        (rows, cols, data).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DMatrix<f32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (rows, cols, data): (usize, usize, Vec<f32>) = Deserialize::deserialize(deserializer)?;
        Ok(DMatrix::from_vec(rows, cols, data))
    }
}

/// Configuração da rede neural
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub input_channels: usize,      // Sensores de entrada
    pub hidden_neurons: usize,      // Interneurônios
    pub output_motors: usize,       // Motores de saída
    pub tau_membrane: f32,          // Constante de tempo membrana (ms)
    pub tau_synapse: f32,           // Constante de tempo sinapse (ms)
    pub threshold: f32,             // Limiar de disparo
    pub reset_potential: f32,       // Potencial de reset
    pub refractory_ms: f32,         // Período refratário (ms)
    pub learning_rate: f32,         // Taxa de aprendizado (STDP simplificado)
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            input_channels: 12,     // 4 needs + cursor_x + cursor_y + active_window + idle_time + time_of_day + battery + screen_state
            hidden_neurons: 400,
            output_motors: 70,      // mood(8) + action(10) + speech(12) + movement(20) + expression(20)
            tau_membrane: 20.0,     // 20ms membrana
            tau_synapse: 5.0,       // 5ms sinapse
            threshold: 1.0,
            reset_potential: 0.0,
            refractory_ms: 2.0,
            learning_rate: 0.01,
        }
    }
}

/// Estado interno de um neurônio LIF
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct NeuronState {
    potential: f32,        // Potencial de membrana atual
    last_spike: f32,       // Tempo do último spike (ms)
    refractory_until: f32, // Até quando está refratário
}

/// Rede Neural LIF
pub struct LifNetwork {
    config: NetworkConfig,
    
    // Pesos (inicializados com seed determinístico por personalidade)
    w_input_hidden: DMatrix<f32>,   // [hidden x input]
    w_hidden_hidden: DMatrix<f32>,  // [hidden x hidden] - recorrente esparsa
    w_hidden_output: DMatrix<f32>,  // [output x hidden]
    
    // Estado dos neurônios
    hidden_state: Vec<NeuronState>,
    output_state: Vec<NeuronState>,
    
    // Traces para STDP
    pre_trace: DVector<f32>,   // [hidden]
    post_trace: DVector<f32>,  // [hidden]
    
    // Tempo simulado (ms)
    time_ms: f32,
    dt_ms: f32,
    
    // Seed para reprodutibilidade
    rng: ChaCha8Rng,
}

impl LifNetwork {
    /// Cria nova rede com seed baseado na personalidade
    pub fn new(config: NetworkConfig, personality_seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(personality_seed);
        
        // Inicialização Xavier/Glorot
        let w_input_hidden = Self::xavier_matrix(config.hidden_neurons, config.input_channels, &mut rng);
        let w_hidden_hidden = Self::sparse_recurrent(config.hidden_neurons, 0.05, &mut rng); // 5% conectividade
        let w_hidden_output = Self::xavier_matrix(config.output_motors, config.hidden_neurons, &mut rng);
        
        Self {
            config: config.clone(),
            w_input_hidden,
            w_hidden_hidden,
            w_hidden_output,
            hidden_state: vec![NeuronState { potential: 0.0, last_spike: -1000.0, refractory_until: 0.0 }; config.hidden_neurons],
            output_state: vec![NeuronState { potential: 0.0, last_spike: -1000.0, refractory_until: 0.0 }; config.output_motors],
            pre_trace: DVector::zeros(config.hidden_neurons),
            post_trace: DVector::zeros(config.hidden_neurons),
            time_ms: 0.0,
            dt_ms: 1.0, // 1ms step = 1000Hz sim, mas rodamos a 10Hz (100ms steps)
            rng,
        }
    }

    /// Matriz Xavier/Glorot
    fn xavier_matrix(rows: usize, cols: usize, rng: &mut ChaCha8Rng) -> DMatrix<f32> {
        let scale = (2.0 / (rows + cols) as f32).sqrt();
        DMatrix::from_fn(rows, cols, |_, _| {
            rng.gen_range(-scale..scale)
        })
    }

    /// Matriz recorrente esparsa
    fn sparse_recurrent(n: usize, density: f32, rng: &mut ChaCha8Rng) -> DMatrix<f32> {
        let mut mat = DMatrix::zeros(n, n);
        let scale = (1.0 / n as f32).sqrt();
        for i in 0..n {
            for j in 0..n {
                if i != j && rng.gen::<f32>() < density {
                    mat[(i, j)] = rng.gen_range(-scale..scale);
                }
            }
        }
        mat
    }

    /// Codifica sensores em correntes de entrada
    pub fn encode_input(&self, needs: &Needs, sensors: &SensorInput) -> DVector<f32> {
        let mut input = DVector::zeros(self.config.input_channels);
        
        // Necessidades (invertidas: baixa necessidade = alta corrente)
        input[0] = 1.0 - needs.hunger;
        input[1] = 1.0 - needs.sleep;
        input[2] = 1.0 - needs.boredom;
        input[3] = 1.0 - needs.affection;
        
        // Cursor posição normalizada (0.0 a 1.0)
        input[4] = sensors.cursor_x.clamp(0.0, 1.0);
        input[5] = sensors.cursor_y.clamp(0.0, 1.0);
        
        // Janela ativa (hash simples do nome)
        input[6] = sensors.active_window_hash;
        
        // Tempo idle normalizado (0.0 a 1.0, >5min = 1.0)
        input[7] = (sensors.idle_seconds / 300.0).clamp(0.0, 1.0);
        
        // Hora do dia (0.0 a 1.0)
        input[8] = sensors.time_of_day;
        
        // Bateria (0.0 a 1.0)
        input[9] = sensors.battery_level;
        
        // Estado da tela (0=off, 1=on)
        input[10] = if sensors.screen_on { 1.0 } else { 0.0 };
        
        // Interação recente (0.0 a 1.0)
        input[11] = sensors.recent_interaction;
        
        input
    }

    /// Step da simulação (dt_ms milissegundos)
    pub fn step(&mut self, input_current: &DVector<f32>, dt_ms: f32) -> NetworkOutput {
        self.time_ms += dt_ms;
        self.dt_ms = dt_ms;
        
        // 1. Input -> Hidden
        let input_syn = &self.w_input_hidden * input_current; // [hidden]
        
        // 2. Hidden recorrente
        let hidden_spikes = self.hidden_state.iter()
            .map(|n| if n.potential >= self.config.threshold { 1.0 } else { 0.0 })
            .collect::<Vec<f32>>();
        let hidden_spikes_vec = DVector::from_vec(hidden_spikes);
        let rec_syn = &self.w_hidden_hidden * &hidden_spikes_vec;
        
        // 3. Atualiza neurônios hidden (LIF)
        for i in 0..self.config.hidden_neurons {
            let neuron = &mut self.hidden_state[i];
            let total_current = input_syn[i] + rec_syn[i];
            
            // Decaimento da membrana
            let decay = (-dt_ms / self.config.tau_membrane).exp();
            neuron.potential = neuron.potential * decay + total_current * (1.0 - decay);
            
            // Refratário
            if self.time_ms < neuron.refractory_until {
                continue;
            }
            
            // Spike?
            if neuron.potential >= self.config.threshold {
                neuron.last_spike = self.time_ms;
                neuron.refractory_until = self.time_ms + self.config.refractory_ms;
                neuron.potential = self.config.reset_potential;
                
                // STDP: pre-synaptic trace
                self.pre_trace[i] = 1.0;
            }
            
            // Decaimento traces
            let trace_decay = (-dt_ms / self.config.tau_synapse).exp();
            self.pre_trace[i] *= trace_decay;
            self.post_trace[i] *= trace_decay;
        }
        
        // 4. Hidden -> Output
        let output_syn = &self.w_hidden_output * &hidden_spikes_vec;
        
        // 5. Atualiza neurônios output
        let mut output_spikes = Vec::with_capacity(self.config.output_motors);
        for i in 0..self.config.output_motors {
            let neuron = &mut self.output_state[i];
            let current = output_syn[i];
            
            let decay = (-dt_ms / self.config.tau_membrane).exp();
            neuron.potential = neuron.potential * decay + current * (1.0 - decay);
            
            if self.time_ms < neuron.refractory_until {
                output_spikes.push(false);
                continue;
            }
            
            if neuron.potential >= self.config.threshold {
                neuron.last_spike = self.time_ms;
                neuron.refractory_until = self.time_ms + self.config.refractory_ms;
                neuron.potential = self.config.reset_potential;
                output_spikes.push(true);
                
                // STDP: post-synaptic trace para hidden conectados
                for h in 0..self.config.hidden_neurons {
                    if self.w_hidden_output[(i, h)] > 0.0 {
                        self.post_trace[h] = 1.0;
                    }
                }
            } else {
                output_spikes.push(false);
            }
        }
        
        // 6. STDP simplificado (apenas quando há spikes)
        if hidden_spikes_vec.iter().any(|&s| s > 0.0) || output_spikes.iter().any(|&s| s) {
            self.apply_stdp(&hidden_spikes_vec, &output_spikes);
        }
        
        // 7. Decodifica saída
        self.decode_output(&hidden_spikes_vec, &output_spikes)
    }

    /// STDP simplificado: reforça conexões pre->post, enfraquece post->pre
    fn apply_stdp(&mut self, pre_spikes: &DVector<f32>, post_spikes: &[bool]) {
        let lr = self.config.learning_rate;
        
        // Hidden -> Output
        for o in 0..self.config.output_motors {
            if post_spikes[o] {
                for h in 0..self.config.hidden_neurons {
                    if pre_spikes[h] > 0.0 {
                        // Pre before post: LTP
                        self.w_hidden_output[(o, h)] += lr * 0.1;
                    } else if self.pre_trace[h] > 0.1 {
                        // Pre trace but no spike: LTD
                        self.w_hidden_output[(o, h)] -= lr * 0.01;
                    }
                }
            }
        }
        
        // Clamp weights
        for mut w in self.w_hidden_output.iter_mut() {
            *w = w.clamp(-2.0, 2.0);
        }
    }

    /// Decodifica spikes de saída em ações/emoções
    fn decode_output(&self, hidden_spikes: &DVector<f32>, output_spikes: &[bool]) -> NetworkOutput {
        // Taxa de disparo por motor (últimos spikes)
        let mut rates = vec![0.0; self.config.output_motors];
        for (i, &spiked) in output_spikes.iter().enumerate() {
            if spiked {
                rates[i] = 1.0;
            }
        }
        
        // Mapeia para saídas semânticas (baseado em índices fixos)
        NetworkOutput {
            mood_activation: rates[0..8].to_vec(),
            action_activation: rates[8..18].to_vec(),
            speech_activation: rates[18..30].to_vec(),
            movement_activation: rates[30..50].to_vec(),
            expression_activation: rates[50..70].to_vec(),
            hidden_activity: hidden_spikes.sum() / self.config.hidden_neurons as f32,
        }
    }

    /// Salva pesos (para persistência)
    pub fn save_weights(&self) -> NetworkWeights {
        NetworkWeights {
            w_input_hidden: self.w_input_hidden.clone(),
            w_hidden_hidden: self.w_hidden_hidden.clone(),
            w_hidden_output: self.w_hidden_output.clone(),
            config: self.config.clone(),
        }
    }

    /// Carrega pesos
    pub fn load_weights(&mut self, weights: NetworkWeights) {
        self.w_input_hidden = weights.w_input_hidden;
        self.w_hidden_hidden = weights.w_hidden_hidden;
        self.w_hidden_output = weights.w_hidden_output;
        self.config = weights.config;
    }
}

/// Entrada de sensores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorInput {
    pub cursor_x: f32,           // 0.0 a 1.0
    pub cursor_y: f32,           // 0.0 a 1.0
    pub active_window_hash: f32, // Hash do nome da janela ativa
    pub idle_seconds: f32,       // Segundos sem input
    pub time_of_day: f32,        // 0.0 a 1.0
    pub battery_level: f32,      // 0.0 a 1.0
    pub screen_on: bool,
    pub recent_interaction: f32, // 0.0 a 1.0 (tempo desde última interação)
}

impl Default for SensorInput {
    fn default() -> Self {
        Self {
            cursor_x: 0.5,
            cursor_y: 0.5,
            active_window_hash: 0.0,
            idle_seconds: 0.0,
            time_of_day: 0.5,
            battery_level: 1.0,
            screen_on: true,
            recent_interaction: 0.0,
        }
    }
}

/// Saída da rede
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkOutput {
    pub mood_activation: Vec<f32>,        // 8 humores
    pub action_activation: Vec<f32>,      // 10 ações
    pub speech_activation: Vec<f32>,      // 12 tipos de fala
    pub movement_activation: Vec<f32>,    // 20 movimento
    pub expression_activation: Vec<f32>,  // 20 expressões
    pub hidden_activity: f32,             // Atividade média hidden
}

/// Pesos serializáveis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkWeights {
    #[serde(with = "matrix_serde")]
    pub w_input_hidden: DMatrix<f32>,
    #[serde(with = "matrix_serde")]
    pub w_hidden_hidden: DMatrix<f32>,
    #[serde(with = "matrix_serde")]
    pub w_hidden_output: DMatrix<f32>,
    pub config: NetworkConfig,
}

/// Wrapper thread-safe para uso no overlay
pub struct NeuralEngine {
    network: Arc<RwLock<LifNetwork>>,
    last_step: Arc<Mutex<Instant>>,
    dt_accumulator: Arc<Mutex<f32>>,
}

impl NeuralEngine {
    pub fn new(config: NetworkConfig, personality_seed: u64) -> Self {
        Self {
            network: Arc::new(RwLock::new(LifNetwork::new(config, personality_seed))),
            last_step: Arc::new(Mutex::new(Instant::now())),
            dt_accumulator: Arc::new(Mutex::new(0.0)),
        }
    }

    /// Step assíncrono (chamado periodicamente quando acordado)
    pub fn tick(&self, sensors: SensorInput, needs: &Needs) -> NetworkOutput {
        let mut last = self.last_step.lock().unwrap();
        let dt = last.elapsed().as_secs_f32() * 1000.0; // ms
        *last = Instant::now();
        
        let mut acc = self.dt_accumulator.lock().unwrap();
        *acc += dt;
        
        // Step a cada 100ms (10Hz) - acumula dt
        if *acc < 100.0 {
            // Retorna última saída conhecida ou default
            return NetworkOutput::default();
        }
        
        let step_dt = *acc;
        *acc = 0.0;
        drop(acc);
        drop(last);
        
        let input = {
            let net = self.network.read();
            net.encode_input(needs, &sensors)
        };
        
        let mut net = self.network.write();
        net.step(&input, step_dt)
    }

    /// Salva pesos para disco
    pub fn save(&self) -> NetworkWeights {
        self.network.read().save_weights()
    }

    /// Carrega pesos do disco
    pub fn load(&self, weights: NetworkWeights) {
        self.network.write().load_weights(weights);
    }
}

impl Default for NetworkOutput {
    fn default() -> Self {
        Self {
            mood_activation: vec![0.0; 8],
            action_activation: vec![0.0; 10],
            speech_activation: vec![0.0; 12],
            movement_activation: vec![0.0; 20],
            expression_activation: vec![0.0; 20],
            hidden_activity: 0.0,
        }
    }
}