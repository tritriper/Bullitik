# Licences tierces

Bullitik est sous licence GPL-3.0-or-later (voir `LICENSE`). Il s'appuie sur
les composants suivants, chacun sous sa propre licence.

## Compilés dans les exécutables

| Composant | Rôle | Licence |
| --- | --- | --- |
| [whisper.cpp](https://github.com/ggml-org/whisper.cpp) (via whisper-rs) | Reconnaissance vocale | MIT |
| [espeak-ng](https://github.com/espeak-ng/espeak-ng) (via espeak-rs) et ses données `espeak-ng-data/` | Phonétisation pour Piper | GPL-3.0-or-later |
| [piper-rs](https://github.com/thewh1teagle/piper-rs) | Synthèse vocale avec les voix Piper | MIT |
| [ONNX Runtime](https://github.com/microsoft/onnxruntime) (via ort) | Exécution des voix Piper | MIT |
| [cpal](https://github.com/RustAudio/cpal), hound | Micro, haut-parleurs, WAV | Apache-2.0 ; Apache-2.0 |
| Autres crates Rust (voir `Cargo.lock`) | Divers | MIT, Apache-2.0, Unlicense, BSD |

## Téléchargés par `xtask fetch` (voir `models/manifest.toml`)

| Composant | Rôle | Licence |
| --- | --- | --- |
| [llama.cpp](https://github.com/ggml-org/llama.cpp) (`llama-server`) | Moteur du modèle de langue | MIT |
| [Ministral 3 Instruct 2512](https://huggingface.co/mistralai/Ministral-3-8B-Instruct-2512) (Mistral AI) | Intervieweur et juge | Apache-2.0 |
| [Modèles Whisper](https://huggingface.co/ggerganov/whisper.cpp) (OpenAI, convertis par ggml) | Reconnaissance vocale | MIT |
| Voix Piper fr_FR-siwis-medium | Synthèse vocale | CC-BY 4.0 |
| Voix Piper fr_FR-upmc-medium | Synthèse vocale | CC-BY-SA 4.0 |
| Voix Piper fr_FR-tom-medium | Synthèse vocale | AGPL-3.0 |

## Crédits des voix

- **siwis** : entraînée sur la *SIWIS French Speech Synthesis Database*
  ([datashare.is.ed.ac.uk/handle/10283/2353](https://datashare.is.ed.ac.uk/handle/10283/2353)),
  licence CC-BY 4.0, via [rhasspy/piper-voices](https://huggingface.co/rhasspy/piper-voices).
- **upmc** : entraînée sur le corpus *UPMC Pierre* de MaryTTS
  ([github.com/marytts/upmc-pierre-data](https://github.com/marytts/upmc-pierre-data)),
  licence CC-BY-SA 4.0, via rhasspy/piper-voices.
- **tom** : voix de Tjiho
  ([git.bksp.space/Tjiho/French-tts-model-piper](https://git.bksp.space/Tjiho/French-tts-model-piper)),
  licence AGPL-3.0, via rhasspy/piper-voices.

Le texte complet des licences est disponible aux adresses ci-dessus.
