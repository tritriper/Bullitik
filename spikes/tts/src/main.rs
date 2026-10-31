//! Spike 0.2 — synthèse vocale seule.
//!
//! Un texte d'intervieweur est lu à voix haute par Piper, voix par voix, avec
//! le temps de synthèse mesuré. Sert aussi à choisir la voix à l'écoute.
//!
//!   cargo run --release -p spike-tts                       # toutes les voix
//!   cargo run --release -p spike-tts -- --voice siwis --text "Bonsoir."
//!   cargo run --release -p spike-tts -- --no-play --out-dir /tmp/voix

use anyhow::{Context, Result, bail};
use bullitik_audio::Clip;
use clap::Parser;
use piper_rs::Piper;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DEFAULT_TEXT: &str = "Bonsoir. Le pouvoir d'achat des Français a reculé cette année. \
Qui en est responsable, selon vous ? Et surtout, que comptez-vous faire concrètement ?";

/// Une voix candidate : fichier du modèle et locuteur éventuel.
struct Voice {
    name: &'static str,
    file: &'static str,
    speaker: Option<&'static str>,
    license: &'static str,
}

const VOICES: &[Voice] = &[
    Voice {
        name: "siwis",
        file: "fr_FR-siwis-medium.onnx",
        speaker: None,
        license: "CC-BY 4.0",
    },
    Voice {
        name: "upmc-jessica",
        file: "fr_FR-upmc-medium.onnx",
        speaker: Some("jessica"),
        license: "CC-BY-SA 4.0",
    },
    Voice {
        name: "upmc-pierre",
        file: "fr_FR-upmc-medium.onnx",
        speaker: Some("pierre"),
        license: "CC-BY-SA 4.0",
    },
    Voice {
        name: "tom",
        file: "fr_FR-tom-medium.onnx",
        speaker: None,
        license: "AGPL-3.0",
    },
];

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
    locate_espeak_data()?;
    let voices: Vec<&Voice> = match &args.voice {
        None => VOICES.iter().collect(),
        Some(name) => match VOICES.iter().find(|v| v.name == name) {
            Some(v) => vec![v],
            None => bail!(
                "voix inconnue : {name} (connues : {})",
                VOICES.iter().map(|v| v.name).collect::<Vec<_>>().join(", ")
            ),
        },
    };
    if let Some(dir) = &args.out_dir {
        std::fs::create_dir_all(dir)?;
    }
    let sentences = split_sentences(&args.text);

    println!("== Spike 0.2 — synthèse vocale ==");
    println!("Calcul      : CPU (Piper, onnxruntime)");
    println!(
        "Texte       : {} phrase(s), {} caractères",
        sentences.len(),
        args.text.chars().count()
    );
    let mut results = Vec::new();
    for voice in voices {
        println!("\n-- Voix {} ({}) --", voice.name, voice.license);
        let r = run_voice(voice, &sentences, &args)?;
        results.push((voice.name, r));
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
            r.total.as_secs_f64() / r.audio.as_secs_f64()
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
    let model = args.models.join(voice.file);
    let config = args.models.join(format!("{}.json", voice.file));
    let t = Instant::now();
    let mut piper = Piper::new(&model, &config)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| {
            format!(
                "chargement de {} (lancer `cargo xtask fetch` ?)",
                model.display()
            )
        })?;
    let load = t.elapsed();
    let speaker = match voice.speaker {
        None => None,
        Some(name) => Some(
            *piper
                .voices()
                .and_then(|v| v.get(name))
                .with_context(|| format!("locuteur {name} absent du modèle"))?,
        ),
    };
    println!("Chargement  : {}", ms(load));

    // Échauffement : la première inférence alloue et optimise le graphe ONNX.
    let t = Instant::now();
    synthesize(&mut piper, "Bonjour.", speaker)?;
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
            let c = synthesize(&mut piper, sentence, speaker)?;
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

fn synthesize(piper: &mut Piper, text: &str, speaker: Option<i64>) -> Result<Clip> {
    let (samples, rate) = piper
        .create(text, false, speaker, None, None, None)
        .map_err(|e| anyhow::anyhow!("synthèse : {e}"))?;
    Ok(Clip { samples, rate })
}

/// Découpe en phrases, ponctuation finale comprise.
fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        current.push(c);
        if matches!(c, '.' | '?' | '!' | '…') {
            let s = current.trim();
            if !s.is_empty() {
                sentences.push(s.to_string());
            }
            current.clear();
        }
    }
    if !current.trim().is_empty() {
        sentences.push(current.trim().to_string());
    }
    sentences
}

/// espeak-ng cherche ses données (`espeak-ng-data/`) dans la variable
/// PIPER_ESPEAKNG_DATA_DIRECTORY, le dossier courant, puis celui de
/// l'exécutable. En développement, elles sont seulement dans le dossier de
/// build d'espeak-rs-sys : on y pointe la variable.
fn locate_espeak_data() -> Result<()> {
    const VAR: &str = "PIPER_ESPEAKNG_DATA_DIRECTORY";
    let has_data = |dir: &Path| dir.join("espeak-ng-data").is_dir();
    let exe_dir = std::env::current_exe()?.parent().map(Path::to_path_buf);
    if std::env::var_os(VAR).is_some()
        || has_data(&std::env::current_dir()?)
        || exe_dir.as_deref().is_some_and(has_data)
    {
        return Ok(());
    }
    let build = Path::new(env!("BULLITIK_PROFILE_DIR")).join("build");
    let newest = std::fs::read_dir(&build)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("espeak-rs-sys-")
        })
        .map(|e| e.path().join("out").join("share"))
        .filter(|p| has_data(p))
        .max_by_key(|p| {
            p.join("espeak-ng-data")
                .metadata()
                .and_then(|m| m.modified())
                .ok()
        });
    match newest {
        Some(dir) => {
            // SAFETY : appelé au démarrage, avant tout autre thread.
            unsafe_set_env(VAR, &dir);
            Ok(())
        }
        None => bail!(
            "données d'espeak-ng introuvables : placer `espeak-ng-data/` à côté de l'exécutable"
        ),
    }
}

#[allow(unsafe_code)]
fn unsafe_set_env(key: &str, value: &Path) {
    // Édition 2024 : set_var est unsafe, car non synchronisé entre threads.
    // Ici, aucun autre thread n'existe encore.
    unsafe { std::env::set_var(key, value) }
}

fn median(values: &mut [Duration]) -> Duration {
    values.sort();
    values[values.len() / 2]
}

fn ms(d: Duration) -> String {
    format!("{} ms", d.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Aller-retour sur CPU : Piper (voix siwis) lit une phrase, Whisper small
    /// la retranscrit. Lancé par la CI avec `--ignored`, une fois l'ensemble
    /// `ci` téléchargé.
    #[test]
    #[ignore = "demande les modèles de l'ensemble ci"]
    fn aller_retour_piper_whisper() {
        use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let models = root.join("models/files");
        locate_espeak_data().unwrap();
        let mut piper = Piper::new(
            &models.join("fr_FR-siwis-medium.onnx"),
            &models.join("fr_FR-siwis-medium.onnx.json"),
        )
        .unwrap();
        // Bruit de Piper à zéro : synthèse déterministe, test stable.
        let (samples, rate) = piper
            .create(
                "Qui est responsable de la baisse du pouvoir d'achat ?",
                false,
                None,
                None,
                Some(0.0),
                Some(0.0),
            )
            .unwrap();
        let clip = Clip { samples, rate };
        assert!(
            clip.duration() > Duration::from_secs(1),
            "{:?}",
            clip.duration()
        );

        let params = WhisperContextParameters {
            use_gpu: false,
            ..Default::default()
        };
        let model = models.join("ggml-small-q5_1.bin");
        let ctx = WhisperContext::new_with_params(model.to_str().unwrap(), params).unwrap();
        let mut state = ctx.create_state().unwrap();
        let mut full = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        full.set_language(Some("fr"));
        full.set_print_progress(false);
        state.full(full, &clip.resampled(16_000).samples).unwrap();
        let text: String = state
            .as_iter()
            .map(|s| s.to_str_lossy().unwrap().into_owned())
            .collect::<String>()
            .to_lowercase();
        for word in ["responsable", "baisse", "pouvoir"] {
            assert!(text.contains(word), "« {word} » absent de « {text} »");
        }
    }

    #[test]
    fn decoupe_en_phrases() {
        assert_eq!(
            split_sentences("Bonsoir. Qui est responsable ? Personne… Fin"),
            vec!["Bonsoir.", "Qui est responsable ?", "Personne…", "Fin"]
        );
    }
}
