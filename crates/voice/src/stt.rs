//! Reconnaissance vocale : whisper.cpp par whisper-rs, sur GPU (Vulkan, AMD
//! comme NVIDIA) ou sur CPU.

use anyhow::{Context, Result};
use bullitik_audio::Clip;
use std::path::Path;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

/// Fréquence attendue par Whisper.
pub const WHISPER_RATE: u32 = 16_000;

/// Une carte graphique vue par Vulkan (ggml).
#[derive(Debug, Clone)]
pub struct Gpu {
    pub id: i32,
    pub name: String,
    pub free_bytes: usize,
    pub total_bytes: usize,
}

/// Les cartes graphiques que whisper.cpp peut utiliser.
///
/// Passe par le registre de backends de ggml, qui rattrape les erreurs
/// d'initialisation de Vulkan : sur une machine sans pilote Vulkan, on obtient
/// une liste vide. (`whisper_rs::vulkan::list_devices`, lui, laisse passer une
/// exception C++, qui fait avorter le programme.)
pub fn gpus() -> Vec<Gpu> {
    use whisper_rs_sys as sys;
    let mut gpus = Vec::new();
    // SAFETY : API C de ggml ; les pointeurs de périphérique viennent du
    // registre et restent valides toute la vie du programme ; la description
    // est une chaîne C terminée par zéro, possédée par ggml.
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
                id: gpus.len() as i32,
                name,
                free_bytes: free,
                total_bytes: total,
            });
        }
    }
    gpus
}

/// Un modèle Whisper chargé, prêt à transcrire en français.
pub struct Transcriber {
    // L'état garde le contexte en vie ; le champ documente la dépendance.
    _ctx: WhisperContext,
    state: WhisperState,
    threads: usize,
}

impl Transcriber {
    /// Charge le modèle, sur le GPU si `use_gpu` et qu'il y en a un.
    pub fn load(model: &Path, use_gpu: bool, threads: Option<usize>) -> Result<Self> {
        whisper_rs::install_logging_hooks();
        let params = WhisperContextParameters {
            use_gpu,
            ..Default::default()
        };
        let path = model.to_str().context("chemin de modèle non UTF-8")?;
        let ctx = WhisperContext::new_with_params(path, params)
            .with_context(|| format!("chargement de {path} (lancer `cargo xtask fetch` ?)"))?;
        let state = ctx.create_state().context("création de l'état Whisper")?;
        let threads = threads
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get().min(8)));
        Ok(Self {
            _ctx: ctx,
            state,
            threads,
        })
    }

    pub fn threads(&self) -> usize {
        self.threads
    }

    /// Transcrit un extrait, à n'importe quelle fréquence.
    pub fn transcribe(&mut self, clip: &Clip) -> Result<String> {
        self.transcribe_16k(&clip.resampled(WHISPER_RATE).samples)
    }

    /// Transcrit des échantillons mono à 16 kHz.
    pub fn transcribe_16k(&mut self, pcm: &[f32]) -> Result<String> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("fr"));
        params.set_n_threads(i32::try_from(self.threads)?);
        params.set_no_context(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        self.state.full(params, pcm).context("transcription")?;
        let mut text = String::new();
        for segment in self.state.as_iter() {
            text.push_str(&segment.to_str_lossy()?);
        }
        Ok(text.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Transcrit un WAV généré par Piper (voir testdata/README.md).
    /// Lancé par la CI avec `--ignored`, une fois les modèles `ci` téléchargés.
    #[test]
    #[ignore = "demande les modèles de l'ensemble ci"]
    fn transcrit_une_phrase_francaise() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let model = root.join("../../models/files/ggml-small-q5_1.bin");
        let mut t = Transcriber::load(&model, false, Some(4)).unwrap();
        let clip = bullitik_audio::read_wav(&root.join("testdata/phrase.wav")).unwrap();
        let text = t.transcribe(&clip).unwrap().to_lowercase();
        for w in ["pouvoir", "achat"] {
            assert!(text.contains(w), "« {w} » absent de « {text} »");
        }
    }
}
