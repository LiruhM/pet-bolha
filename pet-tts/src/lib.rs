//! Piper TTS integration for Pet Bolha
//! Chama o binário `piper` via CLI para síntese de fala offline PT-BR

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::{Write, Read};
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct PiperConfig {
    pub model_path: PathBuf,
    pub config_path: PathBuf,
    pub piper_bin: PathBuf,
    pub output_dir: PathBuf,
    pub speaker_id: Option<u32>,
    pub length_scale: f32,
    pub noise_scale: f32,
    pub noise_w_scale: f32,
    pub sentence_silence: f32,
}

impl Default for PiperConfig {
    fn default() -> Self {
        // Tenta achar o Piper no venv do Hermes
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let venv_base = home.join("AppData/Local/hermes/hermes-agent/venv");
        
        Self {
            model_path: venv_base.join("share/piper-voices/pt_BR-faber-medium.onnx"),
            config_path: venv_base.join("share/piper-voices/pt_BR-faber-medium.onnx.json"),
            piper_bin: venv_base.join("Scripts/piper.exe"),
            output_dir: venv_base.join("share/piper-output"),
            speaker_id: Some(0),
            length_scale: 1.0,
            noise_scale: 0.667,
            noise_w_scale: 0.8,
            sentence_silence: 0.2,
        }
    }
}

#[derive(Debug)]
pub struct TtsEngine {
    config: PiperConfig,
}

impl TtsEngine {
    pub fn new(config: PiperConfig) -> Result<Self> {
        // Cria diretório de saída
        std::fs::create_dir_all(&config.output_dir)
            .context("Failed to create TTS output directory")?;
        
        // Verifica se arquivos existem
        if !config.model_path.exists() {
            warn!("Piper model not found at {:?}", config.model_path);
        }
        if !config.config_path.exists() {
            warn!("Piper config not found at {:?}", config.config_path);
        }
        if !config.piper_bin.exists() {
            warn!("Piper binary not found at {:?}", config.piper_bin);
        }
        
        info!("TTS Engine initialized: model={:?}", config.model_path);
        Ok(Self { config })
    }
    
    /// Sintetiza texto para arquivo WAV
    pub fn synthesize_to_file(&self, text: &str, output_filename: &str) -> Result<PathBuf> {
        let output_path = self.config.output_dir.join(output_filename);
        
        let mut cmd = Command::new(&self.config.piper_bin);
        cmd.arg("-m").arg(&self.config.model_path)
            .arg("-c").arg(&self.config.config_path)
            .arg("-f").arg(&output_path);
        
        if let Some(speaker) = self.config.speaker_id {
            cmd.arg("-s").arg(speaker.to_string());
        }
        cmd.arg("--length-scale").arg(self.config.length_scale.to_string())
            .arg("--noise-scale").arg(self.config.noise_scale.to_string())
            .arg("--noise-w-scale").arg(self.config.noise_w_scale.to_string())
            .arg("--sentence-silence").arg(self.config.sentence_silence.to_string());
        
        debug!("Running Piper TTS: {:?}", cmd);
        
        // Passa texto via stdin
        let mut child = cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn Piper process")?;
        
        // Escreve texto no stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
            stdin.flush()?;
            drop(stdin); // Fecha stdin para sinalizar EOF
        }
        
        let output = child.wait().context("Piper process failed")?;
        
        if !output.success() {
            let mut stderr = String::new();
            if let Some(mut child_stderr) = child.stderr {
                child_stderr.read_to_string(&mut stderr)?;
            }
            anyhow::bail!("Piper TTS failed: {}", stderr);
        }
        
        info!("TTS synthesized: {:?} ({} bytes)", output_path, output_path.metadata()?.len());
        Ok(output_path)
    }
    
    /// Sintetiza para raw audio (streaming)
    pub fn synthesize_raw(&self, text: &str) -> Result<Vec<u8>> {
        let mut cmd = Command::new(&self.config.piper_bin);
        cmd.arg("-m").arg(&self.config.model_path)
            .arg("-c").arg(&self.config.config_path)
            .arg("--output-raw");
        
        if let Some(speaker) = self.config.speaker_id {
            cmd.arg("-s").arg(speaker.to_string());
        }
        cmd.arg("--length-scale").arg(self.config.length_scale.to_string())
            .arg("--noise-scale").arg(self.config.noise_scale.to_string())
            .arg("--noise-w-scale").arg(self.config.noise_w_scale.to_string())
            .arg("--sentence-silence").arg(self.config.sentence_silence.to_string());
        
        let mut child = cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn Piper process")?;
        
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
            stdin.flush()?;
            drop(stdin);
        }
        
        let output = child.wait().context("Piper process failed")?;
        
        if !output.success() {
            let mut stderr = String::new();
            if let Some(mut child_stderr) = child.stderr {
                child_stderr.read_to_string(&mut stderr)?;
            }
            anyhow::bail!("Piper TTS failed: {}", stderr);
        }
        
        let mut stdout = Vec::new();
        if let Some(mut child_stdout) = child.stdout {
            child_stdout.read_to_end(&mut stdout)?;
        }
        Ok(stdout)
    }
    
    pub fn check_availability(&self) -> bool {
        self.config.model_path.exists() 
            && self.config.config_path.exists() 
            && self.config.piper_bin.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tts_basic() {
        let config = PiperConfig::default();
        if !config.check_availability() {
            println!("Piper not available, skipping test");
            return;
        }
        
        let engine = TtsEngine::new(config).unwrap();
        let result = engine.synthesize_to_file("Olá, teste!", "test_tts.wav").await;
        assert!(result.is_ok());
    }
}