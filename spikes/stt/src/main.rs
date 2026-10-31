//! Spike 0.1 — reconnaissance vocale seule.
//!
//! Je parle, le texte s'affiche, et la latence est mesurée : du moment où
//! j'arrête de parler (Entrée) à l'affichage du texte. whisper.cpp tourne sur
//! le GPU par Vulkan (AMD ou NVIDIA), ou sur le CPU avec `--cpu`.
//!
//!   cargo run --release -p spike-stt                    # au micro
//!   cargo run --release -p spike-stt -- --wav essai.wav --repeat 5

use anyhow::{Context, Result};
use bullitik_audio::Clip;
use clap::Parser;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Fréquence attendue par Whisper.
const WHISPER_RATE: u32 = 16_000;

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
    whisper_rs::install_logging_hooks();

    println!("== Spike 0.1 — reconnaissance vocale ==");
    let gpus = gpus();
    if gpus.is_empty() {
        println!("GPU Vulkan  : aucun");
    }
    for g in &gpus {
        println!(
            "GPU Vulkan  : [{}] {} — {} libres sur {}",
            g.id,
            g.name,
            mib(g.free),
            mib(g.total)
        );
    }
    let use_gpu = !args.cpu && !gpus.is_empty();
    let threads = args
        .threads
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get().min(8)));
    println!(
        "Calcul      : {}",
        if use_gpu {
            format!("GPU [0] {}", gpus[0].name)
        } else {
            format!("CPU, {threads} threads")
        }
    );
    println!("Modèle      : {}", args.model.display());

    let free_before = gpus.first().map(|g| g.free);
    let t = Instant::now();
    let ctx_params = WhisperContextParameters {
        use_gpu,
        ..Default::default()
    };
    let model = args.model.to_str().context("chemin de modèle non UTF-8")?;
    let ctx = WhisperContext::new_with_params(model, ctx_params)
        .with_context(|| format!("chargement de {model} (lancer `cargo xtask fetch` ?)"))?;
    let mut state = ctx.create_state().context("création de l'état Whisper")?;
    let load = t.elapsed();
    println!("Chargement  : {}", ms(load));

    // Première passe à vide : sur GPU, elle compile les pipelines Vulkan.
    let t = Instant::now();
    transcribe(&mut state, &vec![0.0; WHISPER_RATE as usize], threads)?;
    println!(
        "Échauffement: {} (1re inférence, hors mesures)",
        ms(t.elapsed())
    );
    if let (true, Some(before)) = (use_gpu, free_before) {
        let after = gpus()[0].free;
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
                let run = measure(&mut state, &clip, threads)?;
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
                std::io::stdin().read_line(&mut line)?;
                if line.trim().eq_ignore_ascii_case("q") {
                    break;
                }
                print!("Parle… puis Entrée pour arrêter. ");
                std::io::stdout().flush()?;
                let clip = bullitik_audio::record_until_enter()?;
                if clip.samples.len() < clip.rate as usize / 4 {
                    println!("(trop court, ignoré)");
                    continue;
                }
                let run = measure(&mut state, &clip, threads)?;
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

struct Gpu {
    id: usize,
    name: String,
    free: usize,
    total: usize,
}

/// Les cartes graphiques vues par ggml, par son registre de backends : il
/// rattrape les erreurs d'initialisation de Vulkan (machine sans pilote) et
/// renvoie alors une liste vide. `whisper_rs::vulkan::list_devices`, lui,
/// laisse passer une exception C++ qui fait avorter le programme.
fn gpus() -> Vec<Gpu> {
    use whisper_rs_sys as sys;
    let mut gpus = Vec::new();
    // SAFETY : API C de ggml ; les périphériques viennent du registre et vivent
    // jusqu'à la fin du programme ; la description est une chaîne C de ggml.
    #[allow(unsafe_code)]
    unsafe {
        for i in 0..sys::ggml_backend_dev_count() {
            let dev = sys::ggml_backend_dev_get(i);
            let kind = sys::ggml_backend_dev_type(dev);
            if kind != sys::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_GPU
                && kind != sys::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_IGPU
            {
                continue;
            }
            let name = std::ffi::CStr::from_ptr(sys::ggml_backend_dev_description(dev))
                .to_string_lossy()
                .into_owned();
            let (mut free, mut total) = (0usize, 0usize);
            sys::ggml_backend_dev_memory(dev, &mut free, &mut total);
            gpus.push(Gpu {
                id: gpus.len(),
                name,
                free,
                total,
            });
        }
    }
    gpus
}

struct Run {
    text: String,
    audio: Duration,
    latency: Duration,
}

/// Rééchantillonnage + transcription, chronométrés ensemble : c'est ce que
/// le joueur attend entre la fin de sa phrase et l'affichage du texte.
fn measure(state: &mut whisper_rs::WhisperState, clip: &Clip, threads: usize) -> Result<Run> {
    let t = Instant::now();
    let pcm = clip.resampled(WHISPER_RATE).samples;
    let text = transcribe(state, &pcm, threads)?;
    Ok(Run {
        text,
        audio: clip.duration(),
        latency: t.elapsed(),
    })
}

fn transcribe(state: &mut whisper_rs::WhisperState, pcm: &[f32], threads: usize) -> Result<String> {
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("fr"));
    params.set_n_threads(i32::try_from(threads)?);
    params.set_no_context(true);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    state.full(params, pcm).context("transcription")?;
    let mut text = String::new();
    for segment in state.as_iter() {
        text.push_str(&segment.to_str_lossy()?);
    }
    Ok(text.trim().to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::ensure;

    /// Transcrit un WAV généré par Piper (voir docs/spikes/0.1-stt.md).
    /// Lancé par la CI avec `--ignored`, une fois les modèles `ci` téléchargés.
    #[test]
    #[ignore = "demande les modèles de l'ensemble ci"]
    fn transcrit_une_phrase_francaise() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let model = root.join("models/files/ggml-small-q5_1.bin");
        let wav = root.join("spikes/stt/testdata/phrase.wav");
        let params = WhisperContextParameters {
            use_gpu: false,
            ..Default::default()
        };
        let ctx = WhisperContext::new_with_params(model.to_str().unwrap(), params).unwrap();
        let mut state = ctx.create_state().unwrap();
        let clip = bullitik_audio::read_wav(&wav).unwrap();
        let text = transcribe(&mut state, &clip.resampled(WHISPER_RATE).samples, 4)
            .unwrap()
            .to_lowercase();
        ensure_words(&text, &["pouvoir", "achat"]).unwrap();
    }

    fn ensure_words(text: &str, words: &[&str]) -> Result<()> {
        for w in words {
            ensure!(text.contains(w), "« {w} » absent de « {text} »");
        }
        Ok(())
    }
}
