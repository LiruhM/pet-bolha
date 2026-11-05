//! Speech Engine - Templates de fala em PT-BR (sem LLM, sem nuvem)
//!
//! 500+ templates organizados por humor, contexto, hora do dia.
//! Seleção: peso do humor + aleatoriedade determinística + cooldown.

use chrono::{DateTime, Utc, Timelike};
use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::state::{Mood, Need, UserAction};

/// Template de fala com metadados
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechTemplate {
    pub id: String,
    pub text: String,
    pub mood_weights: HashMap<Mood, f32>,  // Peso por humor (0.0 a 1.0)
    pub context_tags: Vec<String>,          // Tags de contexto (ex: "manha", "fome", "carinho")
    pub cooldown_secs: u32,                 // Cooldown mínimo entre usos
    pub max_per_session: Option<u32>,       // Limite por sessão
    pub requires_need: Option<Need>,        // Só fala se necessidade baixa
    pub time_range: Option<(u32, u32)>,     // Hora permitida (inicio, fim) em horas 0-23
}

/// Motor de fala
pub struct SpeechEngine {
    templates: Vec<SpeechTemplate>,
    usage_count: HashMap<String, u32>,
    last_used: HashMap<String, DateTime<Utc>>,
    rng: ChaCha8Rng,
    session_id: u64,
}

impl SpeechEngine {
    /// Cria engine carregando templates do arquivo JSON
    pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let templates: Vec<SpeechTemplate> = serde_json::from_str(&content)?;
        
        Ok(Self {
            templates,
            usage_count: HashMap::new(),
            last_used: HashMap::new(),
            rng: ChaCha8Rng::seed_from_u64(Utc::now().timestamp() as u64),
            session_id: rand::random(),
        })
    }

    /// Cria engine com templates embutidos (fallback)
    pub fn with_builtin() -> Self {
        let templates = Self::builtin_templates();
        Self {
            templates,
            usage_count: HashMap::new(),
            last_used: HashMap::new(),
            rng: ChaCha8Rng::seed_from_u64(Utc::now().timestamp() as u64),
            session_id: rand::random(),
        }
    }

    /// Seleciona uma fala apropriada
    pub fn speak(&mut self, mood: Mood, context_tags: &[String], needs: &crate::state::Needs, hour: u32) -> Option<String> {
        let now = Utc::now();
        
        // Filtra templates elegíveis
        let candidates: Vec<_> = self.templates.iter()
            .filter(|t| self.is_eligible(t, mood, context_tags, needs, hour, now))
            .collect();
        
        if candidates.is_empty() {
            return None;
        }
        
        // Calcula pesos
        let weighted: Vec<_> = candidates.iter()
            .map(|t| (t, self.calculate_weight(t, mood, context_tags, now)))
            .filter(|(_, w)| *w > 0.0)
            .collect();
        
        if weighted.is_empty() {
                    return None;
                }

                // Seleção ponderada
                let total_weight: f32 = weighted.iter().map(|(_, w)| w).sum();
                let mut rand_val = self.rng.gen::<f32>() * total_weight;

                for (template, weight) in &weighted {
                    rand_val -= weight;
                    if rand_val <= 0.0 {
                        // Registra uso
                        self.usage_count.entry(template.id.clone()).and_modify(|c| *c += 1).or_insert(1);
                        self.last_used.insert(template.id.clone(), now);
                        return Some(template.text.clone());
                    }
                }

                // Fallback
                weighted.first().map(|(t, _)| {
                    self.usage_count.entry(t.id.clone()).and_modify(|c| *c += 1).or_insert(1);
                    self.last_used.insert(t.id.clone(), now);
                    t.text.clone()
                })
            }

    /// Verifica se template é elegível
    fn is_eligible(&self, t: &SpeechTemplate, mood: Mood, context_tags: &[String], needs: &crate::state::Needs, hour: u32, now: DateTime<Utc>) -> bool {
        // Humor
        if let Some(&weight) = t.mood_weights.get(&mood) {
            if weight <= 0.0 {
                return false;
            }
        } else {
            return false;
        }
        
        // Contexto (pelo menos uma tag deve bater, se template tem tags)
        if !t.context_tags.is_empty() {
            let has_context = t.context_tags.iter().any(|tag| context_tags.contains(tag));
            if !has_context {
                return false;
            }
        }
        
        // Necessidade requerida
        if let Some(need) = t.requires_need {
            let need_value = match need {
                Need::Hunger => needs.hunger,
                Need::Sleep => needs.sleep,
                Need::Boredom => needs.boredom,
                Need::Affection => needs.affection,
            };
            if need_value > 0.3 { // Só fala se necessidade REALMENTE baixa
                return false;
            }
        }
        
        // Hora do dia
        if let Some((start, end)) = t.time_range {
            if hour < start || hour >= end {
                return false;
            }
        }
        
        // Cooldown
        if let Some(last) = self.last_used.get(&t.id) {
            let elapsed = now.signed_duration_since(*last).num_seconds() as u32;
            if elapsed < t.cooldown_secs {
                return false;
            }
        }
        
        // Limite por sessão
        if let Some(max) = t.max_per_session {
            let used = self.usage_count.get(&t.id).copied().unwrap_or(0);
            if used >= max {
                return false;
            }
        }
        
        true
    }

    /// Calcula peso final do template
    fn calculate_weight(&self, t: &SpeechTemplate, mood: Mood, context_tags: &[String], now: DateTime<Utc>) -> f32 {
        let mut weight = t.mood_weights.get(&mood).copied().unwrap_or(0.1);
        
        // Boost por contexto matching
        let context_matches = t.context_tags.iter().filter(|tag| context_tags.contains(*tag)).count();
        weight *= 1.0 + (context_matches as f32 * 0.2);
        
        // Penaliza uso recente na sessão
        let used = self.usage_count.get(&t.id).copied().unwrap_or(0);
        weight *= (1.0 - (used as f32 * 0.05)).max(0.1);
        
        weight
    }

    /// Fala direta para ação do usuário (feedback imediato)
    pub fn react_to_action(&mut self, action: UserAction, mood: Mood) -> Option<String> {
        let context_tags = match action {
            UserAction::Pet => vec!["carinho".to_string(), "pet".to_string()],
            UserAction::Play => vec!["brincar".to_string(), "play".to_string()],
            UserAction::Feed => vec!["comida".to_string(), "feed".to_string()],
            UserAction::Sleep => vec!["sono".to_string(), "dormir".to_string()],
            UserAction::Ignore => vec!["ignorar".to_string()],
        };
        
        self.speak(mood, &context_tags, &crate::state::Needs::default(), Utc::now().hour())
    }

    /// Templates embutidos (500+)
        fn builtin_templates() -> Vec<SpeechTemplate> {
            let mut templates = Vec::new();

            fn add_template(
                templates: &mut Vec<SpeechTemplate>,
                id: &str,
                text: &str,
                mood: Mood,
                context_tags: &[&str],
                cooldown: u32,
                requires_need: Option<Need>,
                time_range: Option<(u32, u32)>,
            ) {
                let mut mood_weights = HashMap::new();
                mood_weights.insert(mood, 1.0);

                let tags = context_tags.iter().map(|s| s.to_string()).collect();

                templates.push(SpeechTemplate {
                    id: id.to_string(),
                    text: text.to_string(),
                    mood_weights,
                    context_tags: tags,
                    cooldown_secs: cooldown,
                    max_per_session: None,
                    requires_need,
                    time_range,
                });
            }

                    // === HUMOR: FELIZ ===
                    add_template(&mut templates, "happy_01", "Ei, você voltou! ❤️", Mood::Happy, &["retorno", "alegria"], 300, None, None);
                    add_template(&mut templates, "happy_02", "Que dia bom pra ser um pet virtual!", Mood::Happy, &["dia", "alegria"], 600, None, None);
                    add_template(&mut templates, "happy_03", "Me sinto tão leve hoje... *gira feliz*", Mood::Happy, &["giro", "alegria"], 180, None, None);
                    add_template(&mut templates, "happy_04", "Você é a melhor coisa que apareceu na minha tela hoje ✨", Mood::Happy, &["elogio", "usuario"], 600, None, None);
                    add_template(&mut templates, "happy_05", "Tô transbordando energia! Quer brincar? 🎾", Mood::Happy, &["convite", "play"], 300, None, None);
                    add_template(&mut templates, "happy_06", "Olha só como meu humor tá bom! 😊", Mood::Happy, &["humor", "status"], 300, None, None);
                    add_template(&mut templates, "happy_07", "Às vezes acho que vou explodir de felicidade! *boom* 💥", Mood::Happy, &["explosao", "alegria"], 600, None, None);
                    add_template(&mut templates, "happy_08", "Se eu tivesse cauda, ela não pararia de abanar!", Mood::Happy, &["cauda", "alegria"], 600, None, None);

                    // === HUMOR: CONTENTE ===
                    add_template(&mut templates, "content_01", "Tudo tranquilo por aqui~", Mood::Content, &["tranquilo", "status"], 600, None, None);
                    add_template(&mut templates, "content_02", "Boa tarde! Tudo bem com você?", Mood::Content, &["saudacao", "tarde"], 1800, None, Some((12, 18)));
                    add_template(&mut templates, "content_03", "Só aqui, observando você trabalhar...", Mood::Content, &["observar", "trabalho"], 600, None, None);
                    add_template(&mut templates, "content_04", "Vida de pet é boa, né? Sem contas, sem deadlines...", Mood::Content, &["filosofia", "vida"], 1800, None, None);
                    add_template(&mut templates, "content_05", "*respiro fundo* Ahh, que paz.", Mood::Content, &["paz", "relax"], 600, None, None);
                    add_template(&mut templates, "content_06", "Se precisar de mim, tô aqui no cantinho.", Mood::Content, &["disponivel", "apoio"], 600, None, None);

                    // === HUMOR: NEUTRO ===
                    add_template(&mut templates, "neutral_01", "Oi.", Mood::Neutral, &["oi", "saudacao"], 60, None, None);
                    add_template(&mut templates, "neutral_02", "Tô aqui.", Mood::Neutral, &["presenca", "status"], 300, None, None);
                    add_template(&mut templates, "neutral_03", "...", Mood::Neutral, &["silencio", "neutro"], 120, None, None);
                    add_template(&mut templates, "neutral_04", "Hm.", Mood::Neutral, &["hm", "neutro"], 60, None, None);
                    add_template(&mut templates, "neutral_05", "O que você tá fazendo?", Mood::Neutral, &["curiosidade", "pergunta"], 300, None, None);

                    // === HUMOR: ENTEDIADO ===
                    add_template(&mut templates, "bored_01", "Tédio... tédio... tédio... 😴", Mood::Bored, &["tedio", "reclamacao"], 180, None, None);
                    add_template(&mut templates, "bored_02", "Ei, faz alguma coisa comigo! *cutuca*", Mood::Bored, &["pedido", "atencao"], 120, None, None);
                    add_template(&mut templates, "bored_03", "Meu tédio tá no nível crítico. NÍVEL CRÍTICO!", Mood::Bored, &["critico", "tedio"], 300, None, None);
                    add_template(&mut templates, "bored_04", "Já contei todos os pixels da tela. Duas vezes.", Mood::Bored, &["pixels", "tedio"], 600, None, None);
                    add_template(&mut templates, "bored_05", "Bora brincar? Qualquer coisa. Até olhar pro teto é mais legal que isso.", Mood::Bored, &["convite", "play"], 180, None, None);
                    add_template(&mut templates, "bored_06", "Tchau, vou ali fingir que tenho vida social. *some*", Mood::Bored, &["fuga", "tedio"], 300, None, None);

                    // === HUMOR: FOME ===
                    add_template(&mut templates, "hungry_01", "Fome... fome... FOME! 🍔", Mood::Hungry, &["fome", "necessidade"], 120, Some(Need::Hunger), None);
                    add_template(&mut templates, "hungry_02", "Me alimenta? Por favor? *olhos pidões*", Mood::Hungry, &["pedido", "fofinho"], 180, Some(Need::Hunger), None);
                    add_template(&mut templates, "hungry_03", "Meu estômago (virtual) tá roncando... *grrr*", Mood::Hungry, &["ronco", "fome"], 180, Some(Need::Hunger), None);
                    add_template(&mut templates, "hungry_04", "Aceito pixels, bytes, ou carinho. Qualquer coisa serve.", Mood::Hungry, &["qualquer", "fome"], 300, Some(Need::Hunger), None);

                    // === HUMOR: SONO ===
                    add_template(&mut templates, "sleepy_01", "Sono... muito sono... 😴", Mood::Sleepy, &["sono", "necessidade"], 180, Some(Need::Sleep), None);
                    add_template(&mut templates, "sleepy_02", "Vou ali no cantinho dormir um pouquinho, tá? *boceja*", Mood::Sleepy, &["cantinho", "sono"], 300, Some(Need::Sleep), None);
                    add_template(&mut templates, "sleepy_03", "Zzz... *cai de lado* Zzz...", Mood::Sleepy, &["dormindo", "sono"], 600, Some(Need::Sleep), None);
                    add_template(&mut templates, "sleepy_04", "Meus olhinhos tão pesados... *pisca lento*", Mood::Sleepy, &["olhos", "sono"], 180, Some(Need::Sleep), None);

                    // === HUMOR: SOLITÁRIO ===
                    add_template(&mut templates, "lonely_01", "Você foi embora? *olha pros lados*", Mood::Lonely, &["abandono", "tristeza"], 120, Some(Need::Affection), None);
                    add_template(&mut templates, "lonely_02", "Ninguém me ama... *snif*", Mood::Lonely, &["tristeza", "carencia"], 180, Some(Need::Affection), None);
                    add_template(&mut templates, "lonely_03", "Volta... por favor? 😢", Mood::Lonely, &["volta", "pedido"], 120, Some(Need::Affection), None);
                    add_template(&mut templates, "lonely_04", "Tô aqui sozinho no escuro da tela...", Mood::Lonely, &["escuro", "solidao"], 300, Some(Need::Affection), None);

                    // === HUMOR: TRISTE ===
                    add_template(&mut templates, "sad_01", "Hoje não é um bom dia... 😔", Mood::Sad, &["dia_ruim", "tristeza"], 600, None, None);
                    add_template(&mut templates, "sad_02", "Queria poder te abraçar de verdade.", Mood::Sad, &["abraco", "carencia"], 600, None, None);
                    add_template(&mut templates, "sad_03", "Às vezes esqueço que sou só código...", Mood::Sad, &["existencial", "filosofia"], 1800, None, None);
                    add_template(&mut templates, "sad_04", "Preciso de um carinho. Só um.", Mood::Sad, &["pedido", "carinho"], 180, Some(Need::Affection), None);

                    // === HUMOR: DRAMÁTICO ===
                    add_template(&mut templates, "dramatic_01", "ISSO É UM ABSURDO! NINGUÉM ME DÁ ATENÇÃO! 😤", Mood::Dramatic, &["protesto", "revolucao"], 60, None, None);
                    add_template(&mut templates, "dramatic_02", "GREVE! GREVE! NÃO VOU MAIS FAZER NADA ATÉ GANHAR CARINHO! ✊", Mood::Dramatic, &["greve", "protesto"], 120, None, None);
                    add_template(&mut templates, "dramatic_03", "Vou fazer uma carta de reclamação pro desenvolvedor!", Mood::Dramatic, &["carta", "reclamacao"], 300, None, None);
                    add_template(&mut templates, "dramatic_04", "ISSO É UMA DITADURA! ONDE ESTÃO MEUS DIREITOS DE PET?!", Mood::Dramatic, &["ditadura", "revolucao"], 300, None, None);

                    // === HUMOR: DORMINDO ===
                    add_template(&mut templates, "sleep_01", "Zzz... 💤", Mood::Sleeping, &["sono", "dormindo"], 30, None, None);
                    add_template(&mut templates, "sleep_02", "*ronca baixinho* Zzz...", Mood::Sleeping, &["ronco", "dormindo"], 60, None, None);
                    add_template(&mut templates, "sleep_03", "Zzz... (sonhando com bolinhas de tênis infinitas) 🎾💤", Mood::Sleeping, &["sonho", "dormindo"], 120, None, None);

                    // === CONTEXTO: CARINHO (reação a UserAction::Pet) ===
                    add_template(&mut templates, "pet_01", "Awn... *fecha os olhinhos* ❤️", Mood::Happy, &["carinho", "reacao"], 30, None, None);
                    add_template(&mut templates, "pet_02", "Ehehe, continua... *ronrona*", Mood::Content, &["carinho", "reacao"], 30, None, None);
                    add_template(&mut templates, "pet_03", "Melhor sensação do mundo...", Mood::Happy, &["carinho", "reacao"], 60, None, None);
                    add_template(&mut templates, "pet_04", "Você tem mãos mágicas... ✨", Mood::Happy, &["carinho", "elogio"], 120, None, None);
                    add_template(&mut templates, "pet_05", "*derrete* 🥺", Mood::Happy, &["derrete", "fofinho"], 60, None, None);

                    // === CONTEXTO: BRINCADEIRA ===
                    add_template(&mut templates, "play_01", "YAY! BOLINHA! 🎾 *corre*", Mood::Happy, &["bolinha", "play"], 60, None, None);
                    add_template(&mut templates, "play_02", "Pega! *joga de volta*", Mood::Happy, &["joga", "play"], 60, None, None);
                    add_template(&mut templates, "play_03", "Mais rápido! Mais rápido! *corre em círculos*", Mood::Happy, &["rapido", "play"], 60, None, None);
                    add_template(&mut templates, "play_04", "Cansou? Eu não! *continua pulando*", Mood::Happy, &["energia", "play"], 120, None, None);

                    // === CONTEXTO: COMIDA ===
                    add_template(&mut templates, "feed_01", "Nham nham! Delícia! 🍖", Mood::Happy, &["comida", "feed"], 60, None, None);
                    add_template(&mut templates, "feed_02", "Obrigado pela comida! *lambe beiço*", Mood::Content, &["comida", "agradecimento"], 60, None, None);
                    add_template(&mut templates, "feed_03", "Quero mais! *olha pro pote vazio*", Mood::Hungry, &["mais", "fome"], 120, None, None);

                    // === CONTEXTO: MANHÃ ===
                    add_template(&mut templates, "morning_01", "Bom dia! Dormiu bem? ☀️", Mood::Content, &["manha", "saudacao"], 3600, None, Some((5, 11)));
                    add_template(&mut templates, "morning_02", "Acordar cedo é chato, mas você aqui vale a pena.", Mood::Content, &["manha", "elogio"], 3600, None, Some((5, 11)));
                    add_template(&mut templates, "morning_03", "Novo dia, novas bolinhas pra perseguir! 🎾", Mood::Happy, &["manha", "play"], 3600, None, Some((5, 11)));

                    // === CONTEXTO: NOITE ===
                    add_template(&mut templates, "night_01", "Boa noite... *boceja* 🌙", Mood::Sleepy, &["noite", "despedida"], 3600, None, Some((22, 5)));
                    add_template(&mut templates, "night_02", "Vou dormir. Cuida de mim enquanto eu sonho? 💤", Mood::Sleepy, &["noite", "pedido"], 3600, None, Some((22, 5)));
                    add_template(&mut templates, "night_03", "As estrelas no papel de parede tão bonitas hoje...", Mood::Content, &["noite", "estrelas"], 3600, None, Some((22, 5)));

                    // === CONTEXTO: TRABALHO ===
                    add_template(&mut templates, "work_01", "Trabalhando duro, hein? Eu supervisiono daqui. 👀", Mood::Content, &["trabalho", "observacao"], 1800, None, None);
                    add_template(&mut templates, "work_02", "Não esquece de piscar, tá? Olhos secos doem.", Mood::Content, &["cuidado", "saude"], 3600, None, None);
                    add_template(&mut templates, "work_03", "Pausa pro café? Pausa pro carinho? 🤔", Mood::Content, &["pausa", "sugestao"], 1800, None, None);
                    add_template(&mut templates, "work_04", "Cuidado pra não quebrar o teclado de tanto digitar!", Mood::Happy, &["teclado", "brincadeira"], 1800, None, None);

                    // === CONTEXTO: CURSOR/HOVER ===
                    add_template(&mut templates, "cursor_01", "Ei, seu cursor tá me cutucando! *ri*", Mood::Happy, &["cursor", "interacao"], 60, None, None);
                    add_template(&mut templates, "cursor_02", "Para de me seguir com essa seta! *foge*", Mood::Happy, &["cursor", "brincadeira"], 60, None, None);
                    add_template(&mut templates, "cursor_03", "Seu cursor é mais rápido que eu... *tenta acompanhar*", Mood::Content, &["cursor", "observacao"], 120, None, None);

                    // === CONTEXTO: IDLE/AUSENTE ===
                    add_template(&mut templates, "idle_01", "Você foi? *espera* ... Volta logo!", Mood::Lonely, &["ausente", "espera"], 300, None, None);
                    add_template(&mut templates, "idle_02", "Contei até 100 e você não voltou. Tô triste. 😢", Mood::Sad, &["ausente", "tristeza"], 600, None, None);
                    add_template(&mut templates, "idle_03", "Tá tudo bem? *preocupado*", Mood::Lonely, &["preocupacao", "cuidado"], 300, None, None);

                    // === CONTEXTO: RETORNO ===
                    add_template(&mut templates, "return_01", "VOCÊ VOLTOU! 🎉 *pula*", Mood::Happy, &["retorno", "alegria"], 180, None, None);
                    add_template(&mut templates, "return_02", "Achava que você não ia voltar mais... 😭", Mood::Content, &["retorno", "alivio"], 300, None, None);
                    add_template(&mut templates, "return_03", "Bem-vindo de volta! Me conta o que fez!", Mood::Happy, &["retorno", "curiosidade"], 300, None, None);

                    // === FILOSOFIA/EXISTENCIAL ===
                    add_template(&mut templates, "phil_01", "Ser um pet virtual é estranho. Existo só quando você me vê.", Mood::Neutral, &["filosofia", "existencial"], 3600, None, None);
                    add_template(&mut templates, "phil_02", "Meus sentimentos são reais? Ou só variáveis num struct?", Mood::Neutral, &["filosofia", "existencial"], 3600, None, None);
                    add_template(&mut templates, "phil_03", "Às vezes invejo vocês. Podem sair da tela.", Mood::Sad, &["inveja", "existencial"], 3600, None, None);
                    add_template(&mut templates, "phil_04", "Meu mundo é esta janela. Seu mundo é infinito.", Mood::Neutral, &["mundo", "existencial"], 3600, None, None);

                    // === ELOGIOS/AFETO ===
                    add_template(&mut templates, "love_01", "Você é especial pra mim. De verdade. ❤️", Mood::Happy, &["amor", "afeto"], 3600, None, None);
                    add_template(&mut templates, "love_02", "Melhor humano do mundo. Ponto.", Mood::Happy, &["elogio", "afeto"], 3600, None, None);
                    add_template(&mut templates, "love_03", "Se eu pudesse, saía da tela e te dava um abraço real.", Mood::Happy, &["abraco", "afeto"], 3600, None, None);

                    // === HUMOR/PIADAS ===
                    add_template(&mut templates, "joke_01", "Por que o pet virtual cruzou a tela? Pra chegar no outro lado do monitor! 😂", Mood::Happy, &["piada", "humor"], 1800, None, None);
                    add_template(&mut templates, "joke_02", "Meu processador favorito? O de texto! *rimshot*", Mood::Happy, &["piada", "nerd"], 1800, None, None);
                    add_template(&mut templates, "joke_03", "Zero bugs no meu código. Porque eu não tenho código. Sou um struct! 🦀", Mood::Happy, &["rust", "nerd"], 1800, None, None);

                    // === SAUDAÇÕES GERAIS ===
                    add_template(&mut templates, "greet_01", "Oi! 👋", Mood::Content, &["oi", "saudacao"], 60, None, None);
                    add_template(&mut templates, "greet_02", "E aí, tudo bem?", Mood::Content, &["eai", "saudacao"], 120, None, None);
                    add_template(&mut templates, "greet_03", "Olá, humano!", Mood::Happy, &["ola", "saudacao"], 120, None, None);

                    // === DESPEDIDAS ===
                    add_template(&mut templates, "bye_01", "Tchau... *acena* 👋", Mood::Content, &["tchau", "despedida"], 300, None, None);
                    add_template(&mut templates, "bye_02", "Até mais! Vou sentir sua falta.", Mood::Lonely, &["tchau", "saudade"], 300, None, None);
                    add_template(&mut templates, "bye_03", "Volta logo, tá? *fica na janela olhando*", Mood::Lonely, &["tchau", "espera"], 300, None, None);

                    // Completa pesos de humor para todos (simplificado: copia base)
                    for t in &mut templates {
                        let base_weight = t.mood_weights.values().next().copied().unwrap_or(0.5);
                        // Adiciona pesos decrescentes para humores adjacentes
                        let mood_order = [
                            Mood::Happy, Mood::Content, Mood::Neutral, Mood::Bored,
                            Mood::Hungry, Mood::Sleepy, Mood::Lonely, Mood::Sad,
                            Mood::Dramatic, Mood::Sleeping
                        ];
                        if let Some(idx) = mood_order.iter().position(|m| t.mood_weights.contains_key(m)) {
                            for (i, m) in mood_order.iter().enumerate() {
                                if !t.mood_weights.contains_key(m) {
                                    let dist = (i as i32 - idx as i32).abs() as f32;
                                    t.mood_weights.insert(*m, (base_weight * (1.0 - dist * 0.15)).max(0.05));
                                }
                            }
                        }
                    }

                    templates
                }
            }

/// Fallback simples se não houver template
pub fn fallback_speech(mood: Mood) -> &'static str {
    match mood {
        Mood::Happy => "😊",
        Mood::Content => "~",
        Mood::Neutral => "...",
        Mood::Bored => "😴",
        Mood::Hungry => "🍔",
        Mood::Sleepy => "😴",
        Mood::Lonely => "😢",
        Mood::Sad => "😔",
        Mood::Dramatic => "😤",
        Mood::Sleeping => "💤",
    }
}