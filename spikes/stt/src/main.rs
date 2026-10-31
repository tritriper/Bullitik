//! Spike 0.1 — reconnaissance vocale seule.
//!
//! Je parle, le texte s'affiche, et la latence est mesurée : du moment où
//! j'arrête de parler (Entrée) à l'affichage du texte. whisper.cpp tourne sur
//! le GPU par Vulkan (AMD ou NVIDIA), ou sur le CPU avec `--cpu`.
//!
//!   cargo run --release -p spike-stt                    # au micro
//!   cargo run --release -p spike-stt -- --wav crates/voice/testdata/phrase.wav --repeat 5

use anyhow::Result;
use bullitik_audio::Clip;
use bullitik_voice::stt::{Transcriber, WHISPER_RATE};
use clap::Parser;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(about = "Spike 0.1 : reconnaissance vocale (whisper.cpp), latence mesurée")]
struct Args {
    /// Modèle Whisper au format ggml.
    #[arg(long, default_value = "models/files/ggml-large-v3-turbo-q5_0.bin")]
    model: PathBuf,
    /// Transcrire ce fichier WAV au lieu du micro.
    #[arg(long)]
    wav: Option<PathBuf>,
    /// Avec --wav : nombre de transcriptions pour mesurer.
    #[arg(long, default_value_t = 3)]
    repeat: usize,
    /// Forcer le CPU, même si un GPU Vulkan est disponible.
    #[arg(long)]
    cpu: bool,
    /// Nombre de threads CPU (par défaut : tous les cœurs, 8 au plus).
    #[arg(long)]
    threads: Option<usize>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("== Spike 0.1 — reconnaissance vocale ==");
    let gpus = bullitik_voice::stt::gpus();
    if gpus.is_empty() {
        println!("GPU Vulkan  : aucun");
    }
    for g in &gpus {
        println!(
            "GPU Vulkan  : [{}] {} — {} libres sur {}",
            g.id,
            g.name,
            mib(g.free_bytes),
            mib(g.total_bytes)
        );
    }
    let use_gpu = !args.cpu && !gpus.is_empty();
    println!("Modèle      : {}", args.model.display());

    let free_before = gpus.first().map(|g| g.free_bytes);
    let t = Instant::now();
    let mut stt = Transcriber::load(&args.model, use_gpu, args.threads)?;
    let load = t.elapsed();
    println!(
        "Calcul      : {}",
        if use_gpu {
            format!("GPU [0] {}", gpus[0].name)
        } else {
            format!("CPU, {} threads", stt.threads())
        }
    );
    println!("Chargement  : {}", ms(load));

    // Première passe à vide : sur GPU, elle compile les pipelines Vulkan.
    let t = Instant::now();
    stt.transcribe_16k(&vec![0.0; WHISPER_RATE as usize])?;
    println!(
        "Échauffement: {} (1re inférence, hors mesures)",
        ms(t.elapsed())
    );
    if let (true, Some(before)) = (use_gpu, free_before) {
        let after = bullitik_voice::stt::gpus()[0].free_bytes;
        println!(
            "VRAM prise  : ~{} (mémoire libre avant/après chargement)",
            mib(before.saturating_sub(after))
        );
    }
    println!();

    let mut runs = Vec::new();
    match &args.wav {
        Some(path) => {
            let clip = bullitik_audio::read_wav(path)?;
            println!(
                "Fichier     : {} ({:.1} s à {} Hz)",
                path.display(),
                clip.duration().as_secs_f64(),
                clip.rate
            );
            for i in 1..=args.repeat.max(1) {
                let run = measure(&mut stt, &clip)?;
                if i == 1 {
                    println!("Texte       : {}", run.text);
                }
                println!("Passe {i}     : {}", ms(run.latency));
                runs.push(run);
            }
        }
        None => {
            println!("Micro       : {}", bullitik_audio::input_device_name()?);
            loop {
                print!("\nEntrée pour parler, q puis Entrée pour finir > ");
                std::io::stdout().flush()?;
                let mut line = String::new();
                // 0 octet lu : fin de l'entrée (Ctrl-D, Ctrl-Z, entrée redirigée).
                if std::io::stdin().read_line(&mut line)? == 0
                    || line.trim().eq_ignore_ascii_case("q")
                {
                    break;
                }
                print!("Parle… puis Entrée pour arrêter. ");
                std::io::stdout().flush()?;
                let clip = bullitik_audio::record_until_enter()?;
                if clip.samples.len() < clip.rate as usize / 4 {
                    println!("(trop court, ignoré)");
                    continue;
                }
                let run = measure(&mut stt, &clip)?;
                println!("« {} »", run.text);
                println!(
                    "{:.1} s de parole → texte en {}",
                    run.audio.as_secs_f64(),
                    ms(run.latency)
                );
                runs.push(run);
            }
        }
    }
    summary(&runs);
    Ok(())
}

struct Run {
    text: String,
    audio: Duration,
    latency: Duration,
}

/// Rééchantillonnage + transcription, chronométrés ensemble : c'est ce que
/// le joueur attend entre la fin de sa phrase et l'affichage du texte.
fn measure(stt: &mut Transcriber, clip: &Clip) -> Result<Run> {
    let t = Instant::now();
    let text = stt.transcribe(clip)?;
    Ok(Run {
        text,
        audio: clip.duration(),
        latency: t.elapsed(),
    })
}

fn summary(runs: &[Run]) {
    println!("\n== Résumé ==");
    if runs.is_empty() {
        println!("aucune mesure");
        return;
    }
    let mut latencies: Vec<Duration> = runs.iter().map(|r| r.latency).collect();
    latencies.sort();
    let audio: f64 = runs.iter().map(|r| r.audio.as_secs_f64()).sum();
    let compute: f64 = latencies.iter().map(Duration::as_secs_f64).sum();
    println!("Mesures     : {}", runs.len());
    println!(
        "Latence     : médiane {}, min {}, max {}",
        ms(latencies[latencies.len() / 2]),
        ms(latencies[0]),
        ms(latencies[latencies.len() - 1])
    );
    println!(
        "Temps réel  : {:.2} (calcul / durée de parole ; < 1 = plus vite que la parole)",
        compute / audio
    );
}

fn ms(d: Duration) -> String {
    format!("{} ms", d.as_millis())
}

fn mib(bytes: usize) -> String {
    format!("{} Mio", bytes / (1024 * 1024))
}
