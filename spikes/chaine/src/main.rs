//! Spike 0.4 — chaîne complète sur une question : voix → texte → modèle → voix.
//!
//!  1. l'intervieweur (LLM) formule une question, lue par la synthèse vocale ;
//!  2. le joueur répond au micro (ou un WAV tient lieu de réponse) ;
//!  3. la réponse est transcrite (Whisper), l'intervieweur relance (LLM) et
//!     la relance est lue : c'est la **latence de dialogue**, ce que le joueur
//!     attend entre la fin de sa phrase et la voix de l'intervieweur ;
//!  4. le juge note la réponse (LLM, JSON), après la relance.
//!
//!   cargo run --release -p spike-chaine                         # au micro
//!   cargo run --release -p spike-chaine -- --reponse-wav crates/voice/testdata/phrase.wav --no-play

use anyhow::{Context, Result, ensure};
use bullitik_audio::Clip;
use bullitik_llm::{Content, LlamaServer, Notation};
use bullitik_voice::stt::{self, Transcriber, WHISPER_RATE};
use bullitik_voice::tts::{self, Speaker};
use clap::Parser;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(about = "Spike 0.4 : chaîne complète voix → texte → modèle → voix")]
struct Args {
    /// Thème de la question.
    #[arg(long, default_value = "pouvoir d'achat")]
    theme: String,
    /// Voix de l'intervieweur.
    #[arg(long, default_value = "upmc-pierre")]
    voice: String,
    /// Réponse du joueur lue dans ce WAV au lieu du micro.
    #[arg(long)]
    reponse_wav: Option<PathBuf>,
    /// Ne pas jouer les voix (mesure seule).
    #[arg(long)]
    no_play: bool,
    /// Tout faire sur le CPU.
    #[arg(long)]
    cpu: bool,
    /// Modèle Whisper.
    #[arg(long, default_value = "models/files/ggml-large-v3-turbo-q5_0.bin")]
    stt_model: PathBuf,
    /// Modèle de langue (GGUF).
    #[arg(
        long,
        default_value = "models/files/Ministral-3-8B-Instruct-2512-Q4_K_M.gguf"
    )]
    model: PathBuf,
    /// Dossier des modèles (voix Piper).
    #[arg(long, default_value = "models/files")]
    models: PathBuf,
    /// Dossier de llama.cpp.
    #[arg(long, default_value = "runtime/llama.cpp")]
    llama: PathBuf,
    /// Dossier des contenus.
    #[arg(long, default_value = "content")]
    content: PathBuf,
    /// Journal de llama-server.
    #[arg(long, default_value = "spikes/chaine/out/llama-server.log")]
    log: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    // Avant tout thread : prepare_espeak modifie l'environnement.
    tts::prepare_espeak()?;
    let content = Content::load(&args.content)?;
    let voice = tts::voice(&args.voice)?;

    println!("== Spike 0.4 — chaîne complète ==");
    let gpus = stt::gpus();
    let use_gpu = !args.cpu && !gpus.is_empty();
    match gpus.first() {
        Some(g) if use_gpu => println!(
            "GPU         : {} — {} Mio libres",
            g.name,
            g.free_bytes >> 20
        ),
        _ => println!("GPU         : aucun, tout sur CPU"),
    }
    let free_before = gpus.first().map(|g| g.free_bytes);

    // --- Chargements -------------------------------------------------------------
    let t = Instant::now();
    let mut stt = Transcriber::load(&args.stt_model, use_gpu, None)?;
    let load_stt = t.elapsed();
    let t = Instant::now();
    let mut speaker = Speaker::load(&args.models, &voice)?;
    let load_tts = t.elapsed();
    let server = LlamaServer::start(&args.llama, &args.model, 8192, args.cpu, &args.log)?;
    println!(
        "Chargement  : Whisper {}, voix {}, llama-server {}",
        ms(load_stt),
        ms(load_tts),
        ms(server.startup)
    );

    // Échauffements, hors mesures : pipelines Vulkan, graphe ONNX, cache du prompt.
    let t = Instant::now();
    stt.transcribe_16k(&vec![0.0; WHISPER_RATE as usize])?;
    speaker.say("Bonjour.")?;
    server.chat(
        &bullitik_llm::interviewer_question(&content, &args.theme),
        0.7,
        8,
        None,
    )?;
    server.chat(
        &bullitik_llm::judge_messages(&content, "Bonjour ?", "Bonjour."),
        0.1,
        8,
        None,
    )?;
    println!("Échauffement: {} (hors mesures)", ms(t.elapsed()));
    if let (true, Some(before)) = (use_gpu, free_before) {
        let after = stt::gpus()[0].free_bytes;
        println!(
            "VRAM prise  : ~{} Mio (Whisper + llama-server, mémoire libre avant/après)",
            before.saturating_sub(after) >> 20
        );
    }

    // --- 1. Question --------------------------------------------------------------
    println!("\n-- Question --");
    let t = Instant::now();
    let q = server.chat(
        &bullitik_llm::interviewer_question(&content, &args.theme),
        0.7,
        120,
        None,
    )?;
    let question = q.text.trim().to_string();
    let t_question = t.elapsed();
    let (clip, t_first) = speak(&mut speaker, &question)?;
    println!("Intervieweur: « {question} »");
    println!(
        "              texte en {}, 1re phrase synthétisée en {}",
        ms(t_question),
        ms(t_first)
    );
    if !args.no_play {
        bullitik_audio::play(&clip)?;
    }

    // --- 2. Réponse du joueur -------------------------------------------------------
    let answer_clip = match &args.reponse_wav {
        Some(path) => {
            let c = bullitik_audio::read_wav(path)?;
            println!(
                "\nRéponse     : {} ({:.1} s)",
                path.display(),
                c.duration().as_secs_f64()
            );
            c
        }
        None => {
            print!("\nÀ toi : Entrée pour parler, Entrée quand tu as fini. ");
            std::io::stdout().flush()?;
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            print!("Parle… ");
            std::io::stdout().flush()?;
            bullitik_audio::record_until_enter()?
        }
    };
    ensure!(
        answer_clip.duration() > Duration::from_millis(250),
        "réponse trop courte"
    );

    // --- 3. Latence de dialogue : transcription, relance, première phrase --------------
    let end_of_answer = Instant::now();
    let t = Instant::now();
    let answer = stt.transcribe(&answer_clip)?;
    let t_stt = t.elapsed();
    println!("Transcrit   : « {answer} » en {}", ms(t_stt));
    let t = Instant::now();
    let followup = server.chat(
        &bullitik_llm::interviewer_followup(&content, &args.theme, &question, &answer),
        0.7,
        120,
        None,
    )?;
    let t_followup = t.elapsed();
    let followup_text = followup.text.trim().to_string();
    let (followup_clip, t_followup_tts) = speak(&mut speaker, &followup_text)?;
    let dialogue_latency = end_of_answer.elapsed();
    println!("Relance     : « {followup_text} »");
    println!(
        "              texte en {}, 1re phrase synthétisée en {}",
        ms(t_followup),
        ms(t_followup_tts)
    );
    if !args.no_play {
        bullitik_audio::play(&followup_clip)?;
    }

    // --- 4. Juge -----------------------------------------------------------------------
    let t = Instant::now();
    let format = bullitik_llm::judge_format(&content);
    let j = server.chat(
        &bullitik_llm::judge_messages(&content, &question, &answer),
        0.1,
        600,
        Some(&format),
    )?;
    let t_judge = t.elapsed();
    match serde_notation(&j.text) {
        Ok(n) => {
            println!(
                "\nJuge        : a répondu : {}, pertinence {}, discrétion {}, technique « {} », score {}",
                n.a_repondu,
                n.pertinence,
                n.discretion,
                n.technique,
                n.score()
            );
            println!("              {}", n.explication);
        }
        Err(e) => println!(
            "\nJuge        : JSON INVALIDE ({e}, fin : {}) : {}",
            j.finish_reason, j.text
        ),
    }
    println!("              notation en {}", ms(t_judge));

    // --- Résumé --------------------------------------------------------------------------
    println!("\n== Résumé ==");
    println!(
        "Question     : {} (texte) + {} (1re phrase de voix)",
        ms(t_question),
        ms(t_first)
    );
    println!(
        "Transcription: {} pour {:.1} s de parole",
        ms(t_stt),
        answer_clip.duration().as_secs_f64()
    );
    println!(
        "Relance      : {} (texte) + {} (1re phrase de voix)",
        ms(t_followup),
        ms(t_followup_tts)
    );
    println!(
        "LATENCE DE DIALOGUE : {} (fin de la réponse → début de la voix de l'intervieweur)",
        ms(dialogue_latency)
    );
    println!(
        "Juge         : {} (après la relance, en parallèle de la voix dans le jeu)",
        ms(t_judge)
    );
    for m in server.memory()? {
        println!(
            "Mémoire      : {} : {} Mio (llama-server)",
            m.device, m.used_mib
        );
    }
    Ok(())
}

/// Synthétise un texte phrase par phrase. Renvoie l'extrait complet et le temps
/// de la première phrase : dans le jeu, la voix démarre dès qu'elle est prête.
fn speak(speaker: &mut Speaker, text: &str) -> Result<(Clip, Duration)> {
    let sentences = tts::split_sentences(text);
    ensure!(!sentences.is_empty(), "texte vide à lire");
    let start = Instant::now();
    let mut first = Duration::ZERO;
    let mut clip = Clip {
        samples: Vec::new(),
        rate: 0,
    };
    for (i, s) in sentences.iter().enumerate() {
        let c = speaker
            .say(s)
            .with_context(|| format!("synthèse de « {s} »"))?;
        if i == 0 {
            first = start.elapsed();
        }
        clip.rate = c.rate;
        clip.samples.extend(c.samples);
        clip.samples
            .extend(std::iter::repeat_n(0.0, c.rate as usize / 8));
    }
    Ok((clip, first))
}

fn serde_notation(text: &str) -> Result<Notation> {
    Ok(bullitik_llm::content::parse_notation(text)?)
}

fn ms(d: Duration) -> String {
    format!("{} ms", d.as_millis())
}
