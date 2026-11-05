//! Memory Store - Persistência local append-only (JSONL)
//!
//! Filosofia: dados legíveis pelo usuário, portáveis, sem schema fixo.
//! Cada linha é um evento completo. O usuário pode abrir, ler, apagar, editar.

use anyhow::Result;
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;
use crate::state::{Mood, PetAction, UserAction};

/// Entrada de memória (uma linha JSONL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub kind: MemoryKind,
    pub payload: serde_json::Value,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MemoryKind {
    /// Interação do usuário
    Interaction {
        action: UserAction,
        mood_before: Mood,
        mood_after: Mood,
    },
    /// Fala do pet
    Speech {
        template_id: String,
        mood: Mood,
        context: String,
    },
    /// Mudança de humor
    MoodChange {
        from: Mood,
        to: Mood,
        trigger: String,
    },
    /// Ação autônoma
    AutonomousAction {
        action: PetAction,
        reason: String,
    },
    /// Evento de sono
    SleepEvent {
        kind: SleepEventKind,
        duration_secs: Option<u64>,
    },
    /// Marco/estatística
    Milestone {
        name: String,
        value: serde_json::Value,
    },
    /// Checkpoint diário (resumo do dia)
    DailySummary {
        date: String, // YYYY-MM-DD
        stats: DailyStats,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SleepEventKind {
    FellAsleep,
    WokeUp,
    Nap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub interactions: u32,
    pub words_spoken: u32,
    pub dominant_mood: Mood,
    pub avg_wellbeing: f32,
    pub sleep_duration_secs: u64,
    pub longest_idle_secs: u64,
}

/// Store de memória com arquivo JSONL
pub struct MemoryStore {
    file: Arc<Mutex<BufWriter<File>>>,
    path: PathBuf,
    index: Arc<Mutex<HashMap<Uuid, u64>>>, // id -> line offset
    daily_summary_time: Arc<Mutex<Option<DateTime<Utc>>>>,
}

impl MemoryStore {
    /// Abre/cria store no diretório de dados do app
    pub fn open(app_name: &str) -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "pet-bolha", app_name)
            .ok_or_else(|| anyhow::anyhow!("Could not find data directory"))?;
        
        let data_dir = proj_dirs.data_dir();
        std::fs::create_dir_all(data_dir)?;
        
        let path = data_dir.join("memory.jsonl");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        
        let mut store = Self {
            file: Arc::new(Mutex::new(BufWriter::new(file))),
            path: path.clone(),
            index: Arc::new(Mutex::new(HashMap::new())),
            daily_summary_time: Arc::new(Mutex::new(None)),
        };
        
        store.rebuild_index()?;
        Ok(store)
    }

    /// Reconstrói índice lendo arquivo
    fn rebuild_index(&mut self) -> Result<()> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut index = self.index.lock();
        index.clear();
        
        let mut offset = 0u64;
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                offset += line.len() as u64 + 1;
                continue;
            }
            
            if let Ok(entry) = serde_json::from_str::<MemoryEntry>(&line) {
                index.insert(entry.id, offset);
            }
            offset += line.len() as u64 + 1; // +1 for newline
        }
        
        Ok(())
    }

    /// Adiciona entrada (append-only)
    pub fn append(&self, kind: MemoryKind, tags: Vec<String>) -> Result<Uuid> {
        let entry = MemoryEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            kind,
            payload: serde_json::Value::Null,
            tags,
        };
        
        let json = serde_json::to_string(&entry)?;
        let mut file = self.file.lock();
        file.write_all(json.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()?;
        
        // Atualiza índice
        let offset = self.current_file_size()?;
        self.index.lock().insert(entry.id, offset);
        
        Ok(entry.id)
    }

    /// Tamanho atual do arquivo
    fn current_file_size(&self) -> Result<u64> {
        let metadata = std::fs::metadata(&self.path)?;
        Ok(metadata.len())
    }

    /// Lê todas as entradas (para export/debug)
    pub fn read_all(&self) -> Result<Vec<MemoryEntry>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        
        for line in reader.lines() {
            let line = line?;
            if let Ok(entry) = serde_json::from_str(&line) {
                entries.push(entry);
            }
        }
        
        Ok(entries)
    }

    /// Lê entradas por tag
    pub fn read_by_tag(&self, tag: &str) -> Result<Vec<MemoryEntry>> {
        let all = self.read_all()?;
        Ok(all.into_iter().filter(|e| e.tags.contains(&tag.to_string())).collect())
    }

    /// Lê últimas N entradas
    pub fn read_recent(&self, n: usize) -> Result<Vec<MemoryEntry>> {
        let all = self.read_all()?;
        Ok(all.into_iter().rev().take(n).collect())
    }

    /// Gera resumo diário (chamado à meia-noite ou na inicialização)
    pub fn generate_daily_summary(&self) -> Result<Option<DailyStats>> {
        let now = Utc::now();
        let today = now.format("%Y-%m-%d").to_string();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        
        let entries = self.read_all()?;
        let today_entries: Vec<_> = entries.iter()
            .filter(|e| e.timestamp >= today_start)
            .collect();
        
        if today_entries.is_empty() {
            return Ok(None);
        }
        
        let mut interactions = 0;
        let mut words = 0;
        let mut mood_counts: HashMap<Mood, u32> = HashMap::new();
        let mut wellbeing_sum = 0.0;
        let mut wellbeing_count = 0;
        let mut sleep_duration = 0;
        let mut max_idle = 0;
        let mut last_interaction = today_start;
        
        for entry in &today_entries {
            match &entry.kind {
                MemoryKind::Interaction { .. } => interactions += 1,
                MemoryKind::Speech { .. } => words += 1, // aproximado
                MemoryKind::MoodChange { to, .. } => {
                    *mood_counts.entry(to.clone()).or_default() += 1;
                }
                MemoryKind::SleepEvent { kind, duration_secs } => {
                    if *kind == SleepEventKind::WokeUp {
                        sleep_duration += duration_secs.unwrap_or(0);
                    }
                }
                _ => {}
            }
            
            // Idle tracking via timestamps
            let idle = (entry.timestamp - last_interaction).num_seconds().max(0) as u64;
            max_idle = max_idle.max(idle);
            last_interaction = entry.timestamp;
        }
        
        let dominant_mood = mood_counts.into_iter()
            .max_by_key(|(_, v)| *v)
            .map(|(m, _)| m)
            .unwrap_or(Mood::Neutral);
        
        let stats = DailyStats {
            interactions,
            words_spoken: words,
            dominant_mood,
            avg_wellbeing: if wellbeing_count > 0 { wellbeing_sum / wellbeing_count as f32 } else { 0.5 },
            sleep_duration_secs: sleep_duration,
            longest_idle_secs: max_idle,
        };
        
        // Salva como milestone
        self.append(
            MemoryKind::Milestone {
                name: "daily_summary".to_string(),
                value: serde_json::to_value(&stats)?,
            },
            vec!["daily".to_string(), today.clone()],
        )?;
        
        Ok(Some(stats))
    }

    /// Exporta para JSON legível
    pub fn export_json(&self, output_path: &Path) -> Result<()> {
        let entries = self.read_all()?;
        let json = serde_json::to_string_pretty(&entries)?;
        std::fs::write(output_path, json)?;
        Ok(())
    }

    /// Caminho do arquivo
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_memory_store() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_memory.jsonl");
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path).unwrap();
        
        let mut store = MemoryStore {
            file: Arc::new(Mutex::new(BufWriter::new(file))),
            path,
            index: Arc::new(Mutex::new(HashMap::new())),
            daily_summary_time: Arc::new(Mutex::new(None)),
        };
        
        store.rebuild_index().unwrap();
        
        let id = store.append(
            MemoryKind::Interaction {
                action: UserAction::Pet,
                mood_before: Mood::Content,
                mood_after: Mood::Happy,
            },
            vec!["test".to_string()],
        ).unwrap();
        
        let recent = store.read_recent(1).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, id);
    }
}