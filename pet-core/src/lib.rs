//! Pet Bolha Core - Motor do pet de bolha offline
//!
//! ## Arquitetura
//! - **State Machine**: Necessidades (fome, sono, tédio, carinho) + humor
//! - **LIF Neural** (opcional): 470 neurônios leaky integrate-and-fire para personalidade
//! - **Memory**: JSONL append-only local, legível pelo usuário
//! - **Speech**: Templates pré-escritos PT-BR (sem LLM, sem nuvem)
//! - **Sync**: Protocolo local Wi-Fi/USB (QUIC) - implementação futura
//!
//! **Filosofia**: Zero rede, zero telemetria, zero conta. Seu pet, seu PC, seu celular.

#![warn(missing_docs)]

pub mod state;
pub mod neural;
pub mod memory;
pub mod speech;
pub mod personality;
pub mod ffi;

pub use state::{PetState, Need, Mood, PetAction};
pub use neural::LifNetwork;
pub use memory::{MemoryStore, MemoryEntry};
pub use speech::{SpeechEngine, SpeechTemplate};
pub use personality::{Personality, Temperament};