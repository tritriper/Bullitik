//! Synthèse vocale : voix Piper par piper-rs (onnxruntime, sur CPU).

use anyhow::{Context, Result, bail};
use bullitik_audio::Clip;
use piper_rs::Piper;
use std::path::Path;

/// Une voix candidate : fichier du modèle et locuteur éventuel.
#[derive(Debug, Clone, Copy)]
pub struct Voice {
    pub name: &'static str,
    pub file: &'static str,
    pub speaker: Option<&'static str>,
    pub license: &'static str,
}

/// Les voix françaises du manifeste.
pub const VOICES: &[Voice] = &[
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

/// Retrouve une voix par son nom.
pub fn voice(name: &str) -> Result<Voice> {
    match VOICES.iter().find(|v| v.name == name) {
        Some(v) => Ok(*v),
        None => bail!(
            "voix inconnue : {name} (connues : {})",
            VOICES.iter().map(|v| v.name).collect::<Vec<_>>().join(", ")
        ),
    }
}

/// Une voix chargée, prête à parler.
pub struct Speaker {
    piper: Piper,
    speaker: Option<i64>,
}

impl Speaker {
    /// Charge une voix depuis le dossier des modèles. Appeler
    /// [`prepare_espeak`] avant, au début du programme.
    pub fn load(models: &Path, voice: &Voice) -> Result<Self> {
        let model = models.join(voice.file);
        let config = models.join(format!("{}.json", voice.file));
        let piper = Piper::new(&model, &config)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| {
                format!(
                    "chargement de {} (lancer `cargo xtask fetch` ?)",
                    model.display()
                )
            })?;
        let speaker = match voice.speaker {
            None => None,
            Some(name) => Some(
                *piper
                    .voices()
                    .and_then(|v| v.get(name))
                    .with_context(|| format!("locuteur {name} absent du modèle"))?,
            ),
        };
        Ok(Self { piper, speaker })
    }

    /// Synthétise une phrase.
    pub fn say(&mut self, text: &str) -> Result<Clip> {
        self.say_with_noise(text, None)
    }

    /// Comme [`Speaker::say`], avec le bruit de Piper imposé (`Some(0.0)` :
    /// synthèse déterministe, pour les tests).
    pub fn say_with_noise(&mut self, text: &str, noise: Option<f32>) -> Result<Clip> {
        let (samples, rate) = self
            .piper
            .create(text, false, self.speaker, None, noise, noise)
            .map_err(|e| anyhow::anyhow!("synthèse : {e}"))?;
        Ok(Clip { samples, rate })
    }
}

/// Découpe en phrases, ponctuation finale comprise.
pub fn split_sentences(text: &str) -> Vec<String> {
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
///
/// À appeler au tout début de `main`, avant de lancer le moindre thread :
/// modifier une variable d'environnement n'est pas synchronisé entre threads.
pub fn prepare_espeak() -> Result<()> {
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
            set_env(VAR, &dir);
            Ok(())
        }
        None => bail!(
            "données d'espeak-ng introuvables : placer `espeak-ng-data/` à côté de l'exécutable"
        ),
    }
}

#[allow(unsafe_code)]
fn set_env(key: &str, value: &Path) {
    // SAFETY : édition 2024, set_var est unsafe car non synchronisé entre
    // threads ; prepare_espeak est documentée pour être appelée avant tout thread.
    unsafe { std::env::set_var(key, value) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    #[test]
    fn decoupe_en_phrases() {
        assert_eq!(
            split_sentences("Bonsoir. Qui est responsable ? Personne… Fin"),
            vec!["Bonsoir.", "Qui est responsable ?", "Personne…", "Fin"]
        );
    }

    #[test]
    fn voix_inconnue_refusee() {
        assert!(voice("upmc-pierre").is_ok());
        assert!(voice("gilles").is_err());
    }

    /// Aller-retour sur CPU : Piper (voix siwis) lit une phrase, Whisper small
    /// la retranscrit. Lancé par la CI avec `--ignored`.
    #[test]
    #[ignore = "demande les modèles de l'ensemble ci"]
    fn aller_retour_piper_whisper() {
        let models = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../models/files");
        prepare_espeak().unwrap();
        let mut speaker = Speaker::load(&models, &voice("siwis").unwrap()).unwrap();
        // Bruit de Piper à zéro : synthèse déterministe, test stable.
        let clip = speaker
            .say_with_noise(
                "Qui est responsable de la baisse du pouvoir d'achat ?",
                Some(0.0),
            )
            .unwrap();
        assert!(
            clip.duration() > Duration::from_secs(1),
            "{:?}",
            clip.duration()
        );
        let mut t =
            crate::stt::Transcriber::load(&models.join("ggml-small-q5_1.bin"), false, Some(4))
                .unwrap();
        let text = t.transcribe(&clip).unwrap().to_lowercase();
        for word in ["responsable", "baisse", "pouvoir"] {
            assert!(text.contains(word), "« {word} » absent de « {text} »");
        }
    }
}
