//! Spike 0.2 — synthèse vocale seule.
//!
//! Un texte d'intervieweur est lu à voix haute par Piper, voix par voix, avec
//! le temps de synthèse mesuré. Sert aussi à choisir la voix à l'écoute.
//!
//!   cargo run --release -p spike-tts                       # toutes les voix
//!   cargo run --release -p spike-tts -- --voice siwis --text "Bonsoir."
//!   cargo run --release -p spike-tts -- --no-play --out-dir /tmp/voix

use anyhow::{Result, ensure};
use bullitik_audio::Clip;
use bullitik_voice::tts::{self, Speaker, VOICES, Voice};
use clap::Parser;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const DEFAULT_TEXT: &str = "Bonsoir. Le pouvoir d'achat des Français a reculé cette année. \
Qui en est responsable, selon vous ? Et surtout, que comptez-vous faire concrètement ?";

#[derive(Parser)]
#[command(about = "Spike 0.2 : synthèse vocale (Piper), latence mesurée, choix de la voix")]
struct Args {
    /// Voix : siwis, upmc-jessica, upmc-pierre, tom. Par défaut : toutes, l'une après l'autre.
    #[arg(long)]
    voice: Option<String>,
    /// Texte à lire.
    #[arg(long, default_value = DEFAULT_TEXT)]
    text: String,
    /// Dossier des modèles Piper.
    #[arg(long, default_value = "models/files")]
    models: PathBuf,
    /// Enregistrer chaque lecture en WAV dans ce dossier.
    #[arg(long)]
    out_dir: Option<PathBuf>,
    /// Ne pas jouer le son (mesure seule).
    #[arg(long)]
    no_play: bool,
    /// Nombre de synthèses par voix pour mesurer.
    #[arg(long, default_value_t = 3)]
    repeat: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    tts::prepare_espeak()?;
    let voices: Vec<Voice> = match &args.voice {
        None => VOICES.to_vec(),
        Some(name) => vec![tts::voice(name)?],
    };
    if let Some(dir) = &args.out_dir {
        std::fs::create_dir_all(dir)?;
    }
    let sentences = tts::split_sentences(&args.text);
    ensure!(!sentences.is_empty(), "texte vide : rien à lire");

    println!("== Spike 0.2 — synthèse vocale ==");
    println!("Calcul      : CPU (Piper, onnxruntime)");
    println!(
        "Texte       : {} phrase(s), {} caractères",
        sentences.len(),
        args.text.chars().count()
    );
    let mut results = Vec::new();
    for voice in &voices {
        println!("\n-- Voix {} ({}) --", voice.name, voice.license);
        results.push((voice.name, run_voice(voice, &sentences, &args)?));
    }
    println!("\n== Résumé ==");
    println!(
        "{:<14} {:>11} {:>14} {:>13} {:>9} {:>10}",
        "voix", "chargement", "1re phrase", "texte entier", "durée", "temps réel"
    );
    for (name, r) in &results {
        println!(
            "{:<14} {:>11} {:>14} {:>13} {:>8.1}s {:>10.2}",
            name,
            ms(r.load),
            ms(r.first_sentence),
            ms(r.total),
            r.audio.as_secs_f64(),
            r.total.as_secs_f64() / r.audio.as_secs_f64().max(f64::EPSILON)
        );
    }
    println!(
        "\n1re phrase = délai avant le son si l'on joue phrase par phrase ; valeurs médianes."
    );
    Ok(())
}

struct VoiceResult {
    load: Duration,
    first_sentence: Duration,
    total: Duration,
    audio: Duration,
}

fn run_voice(voice: &Voice, sentences: &[String], args: &Args) -> Result<VoiceResult> {
    let t = Instant::now();
    let mut speaker = Speaker::load(&args.models, voice)?;
    let load = t.elapsed();
    println!("Chargement  : {}", ms(load));

    // Échauffement : la première inférence alloue et optimise le graphe ONNX.
    let t = Instant::now();
    speaker.say("Bonjour.")?;
    println!("Échauffement: {} (hors mesures)", ms(t.elapsed()));

    let mut firsts = Vec::new();
    let mut totals = Vec::new();
    let mut clip = Clip {
        samples: Vec::new(),
        rate: 0,
    };
    for _ in 0..args.repeat.max(1) {
        let mut samples = Vec::new();
        let start = Instant::now();
        for (i, sentence) in sentences.iter().enumerate() {
            let c = speaker.say(sentence)?;
            if i == 0 {
                firsts.push(start.elapsed());
            }
            clip.rate = c.rate;
            samples.extend(c.samples);
            // Courte pause entre les phrases.
            samples.extend(std::iter::repeat_n(0.0, c.rate as usize / 8));
        }
        totals.push(start.elapsed());
        clip.samples = samples;
    }
    let result = VoiceResult {
        load,
        first_sentence: median(&mut firsts),
        total: median(&mut totals),
        audio: clip.duration(),
    };
    println!(
        "Synthèse    : 1re phrase {}, texte entier {} pour {:.1} s de parole",
        ms(result.first_sentence),
        ms(result.total),
        result.audio.as_secs_f64()
    );
    if let Some(dir) = &args.out_dir {
        let path = dir.join(format!("{}.wav", voice.name));
        bullitik_audio::write_wav(&path, &clip)?;
        println!("Fichier     : {}", path.display());
    }
    if !args.no_play {
        println!("Lecture…");
        bullitik_audio::play(&clip)?;
    }
    Ok(result)
}

fn median(values: &mut [Duration]) -> Duration {
    values.sort();
    values[values.len() / 2]
}

fn ms(d: Duration) -> String {
    format!("{} ms", d.as_millis())
}
