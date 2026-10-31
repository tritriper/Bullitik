//! Spike 0.3 — modèle seul, texte en entrée et en sortie.
//!
//! Lance llama-server (llama.cpp, Vulkan) avec Ministral 3, puis vérifie les
//! deux rôles du jeu :
//!   - l'intervieweur pose une question sur un thème, puis relance après une
//!     esquive ;
//!   - le juge note des réponses annotées, en JSON imposé par un schéma.
//!
//! Mesure le temps de réponse de chaque appel et la mémoire prise par appareil.
//!
//!   cargo run --release -p spike-llm
//!   cargo run --release -p spike-llm -- --model models/files/Ministral-3-14B-Instruct-2512-Q4_K_M.gguf

use anyhow::Result;
use bullitik_llm::{Completion, Content, LlamaServer, Notation};
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(about = "Spike 0.3 : intervieweur et juge avec Ministral 3 (llama.cpp)")]
struct Args {
    /// Modèle GGUF.
    #[arg(
        long,
        default_value = "models/files/Ministral-3-8B-Instruct-2512-Q4_K_M.gguf"
    )]
    model: PathBuf,
    /// Dossier de llama.cpp (llama-server).
    #[arg(long, default_value = "runtime/llama.cpp")]
    llama: PathBuf,
    /// Dossier des contenus (prompts, fiche, questions).
    #[arg(long, default_value = "content")]
    content: PathBuf,
    /// Taille du contexte, en tokens.
    #[arg(long, default_value_t = 8192)]
    ctx: u32,
    /// Tout faire sur le CPU, sans GPU.
    #[arg(long)]
    cpu: bool,
    /// Nombre de questions de la banque à faire poser par l'intervieweur.
    #[arg(long, default_value_t = 2)]
    questions: usize,
    /// Journal de llama-server.
    #[arg(long, default_value = "spikes/llm/out/llama-server.log")]
    log: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let content = Content::load(&args.content)?;
    println!("== Spike 0.3 — modèle seul ==");
    println!("Modèle      : {}", args.model.display());

    let server = LlamaServer::start(&args.llama, &args.model, args.ctx, args.cpu, &args.log)?;
    println!(
        "Démarrage   : {} (chargement du modèle compris)",
        ms(server.startup)
    );
    for line in server.devices()? {
        println!("Appareil    : {line}");
    }
    print_memory(&server)?;

    // Échauffement : la première requête compile les pipelines Vulkan.
    let warm = server.chat(
        &[json!({"role": "user", "content": "Bonjour"})],
        0.0,
        8,
        None,
    )?;
    println!("Échauffement: {} (hors mesures)", ms(warm.wall));

    println!("\n-- Intervieweur --");
    let mut calls = Vec::new();
    for q in content.questions.iter().take(args.questions) {
        let messages = bullitik_llm::interviewer_question(&content, &q.theme);
        let c = server.chat(&messages, 0.7, 120, None)?;
        println!("[{}] {}", q.theme, c.text.trim());
        println!("    (formulation de référence : {})", q.question);
        report(&c);
        calls.push(("intervieweur", c));
    }
    // Mistral exige l'alternance utilisateur / assistant après le prompt système.
    let evasive = content.evasive_example();
    let messages = bullitik_llm::interviewer_followup(
        &content,
        "pouvoir d'achat",
        &evasive.question,
        &evasive.reponse,
    );
    let c = server.chat(&messages, 0.7, 120, None)?;
    println!("[relance] {}", c.text.trim());
    report(&c);
    calls.push(("relance", c));

    println!("\n-- Juge --");
    let format = bullitik_llm::judge_format(&content);
    let mut agreements = 0;
    let mut valid_json = 0;
    for ex in &content.examples {
        let messages = bullitik_llm::judge_messages(&content, &ex.question, &ex.reponse);
        let c = server.chat(&messages, 0.1, 600, Some(&format))?;
        // Un JSON invalide (réponse tronquée par max_tokens…) est compté, pas fatal :
        // les mesures déjà prises ne sont pas perdues.
        let notation: Notation = match serde_json::from_str(&c.text) {
            Ok(n) => n,
            Err(e) => {
                println!(
                    "[{}] JSON INVALIDE ({e}, fin : {}) : {}",
                    ex.nom, c.finish_reason, c.text
                );
                report(&c);
                calls.push(("juge", c));
                continue;
            }
        };
        valid_json += 1;
        let ok = ex.attendu.matches(&notation);
        agreements += usize::from(ok);
        println!(
            "[{}] {} — a répondu : {}, pertinence {}, discrétion {}, technique « {} », score {}",
            ex.nom,
            if ok { "conforme" } else { "NON CONFORME" },
            notation.a_repondu,
            notation.pertinence,
            notation.discretion,
            notation.technique,
            notation.score()
        );
        println!("    {}", notation.explication);
        report(&c);
        calls.push(("juge", c));
    }

    println!("\n== Résumé ==");
    for role in ["intervieweur", "relance", "juge"] {
        let mut walls: Vec<Duration> = calls
            .iter()
            .filter(|(r, _)| *r == role)
            .map(|(_, c)| c.wall)
            .collect();
        if walls.is_empty() {
            continue;
        }
        walls.sort();
        let speeds: Vec<f64> = calls
            .iter()
            .filter(|(r, _)| *r == role)
            .map(|(_, c)| c.gen_tps)
            .collect();
        println!(
            "{role:<13}: médiane {}, max {}, génération {:.1} tokens/s",
            ms(walls[walls.len() / 2]),
            ms(walls[walls.len() - 1]),
            speeds.iter().sum::<f64>() / speeds.len() as f64
        );
    }
    println!(
        "Juge        : JSON valide {valid_json}/{n}, conforme aux annotations {agreements}/{n}",
        n = content.examples.len()
    );
    print_memory(&server)?;
    Ok(())
}

fn print_memory(server: &LlamaServer) -> Result<()> {
    let rows = server.memory()?;
    if rows.is_empty() {
        println!("Mémoire     : tableau absent du journal de llama-server");
    }
    for m in rows {
        println!(
            "Mémoire     : {} : {} Mio (modèle, cache KV, calcul)",
            m.device, m.used_mib
        );
        println!("              brut : {}", m.raw);
    }
    Ok(())
}

fn report(c: &Completion) {
    println!(
        "    {} au total — prompt {} tokens en {}, réponse {} tokens à {:.1} tokens/s",
        ms(c.wall),
        c.prompt_tokens,
        ms(c.prompt_time),
        c.gen_tokens,
        c.gen_tps
    );
}

fn ms(d: Duration) -> String {
    format!("{} ms", d.as_millis())
}
