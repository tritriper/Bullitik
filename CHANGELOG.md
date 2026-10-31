# Changelog

Tous les changements notables de Bullitik, version par version. Le format suit
l'esprit de [Keep a Changelog](https://keepachangelog.com/fr/) ; la **version
fait foi via le tag git** (`v*`). Le backlog vit dans
[`docs/dev/backlog.md`](docs/dev/backlog.md) et les items livrés dans
[`docs/dev/backlog-archive.md`](docs/dev/backlog-archive.md) ; les `#N`
ci-dessous y renvoient.

## [Non publié]

### Ajouté
- Spike 0.3, modèle seul (#8) : `spike-llm` lance llama-server (Vulkan) avec Ministral 3 8B, fait poser des questions et une relance par l'intervieweur et noter des réponses annotées par le juge en JSON imposé par un schéma, et mesure temps de réponse et mémoire par appareil ; rapport dans `docs/spikes/0.3-llm.md`.
- Contenus provisoires du spike 0.3 dans `content/` : prompts de l'intervieweur et du juge, fiche des techniques d'esquive, schéma JSON de la notation, questions, réponses annotées ; leur cohérence est testée.
- Spike 0.2, synthèse vocale seule (#7) : `spike-tts` lit une question d'intervieweur avec Piper sur CPU, phrase par phrase, pour 4 voix candidates (siwis, upmc-jessica, upmc-pierre, tom), et mesure chargement, délai avant le premier son et temps réel ; rapport dans `docs/spikes/0.2-tts.md`.
- Spike 0.1, reconnaissance vocale seule (#6) : `spike-stt` transcrit le micro (Entrée pour parler, Entrée pour arrêter) ou un WAV avec whisper.cpp en Vulkan, et mesure latence, temps réel et VRAM ; rapport dans `docs/spikes/0.1-stt.md`.
- Organisation du dépôt (#2) : git flow `main` / `dev` / branches de travail, backlog numéroté, CHANGELOG, modèles de pull request et d'item de backlog, règles des agents, licence GPL-3.0-or-later.

### Corrigé
- spike-llm : les requêtes vers llama-server (127.0.0.1) ne passent plus par un proxy défini dans l'environnement ; sinon le spike attendait 10 min puis échouait.
- spike-llm : llama-server est tué avec le spike même en cas d'arrêt brutal (Job Object sous Windows, PR_SET_PDEATHSIG sous Linux), au lieu de garder des Go de VRAM.
- spike-llm : un JSON invalide ou tronqué du juge est compté (« JSON valide n/4 ») au lieu d'arrêter le spike ; explication bornée à 400 caractères, `finish_reason` lu.
- spike-llm : la relance utilise le premier exemple annoté comme esquive, plus le deuxième du fichier ; variables inconnues refusées dans les deux prompts.
- xtask fetch : téléchargement repris là où il s'est arrêté (requête `Range`), 10 essais, délai par essai ; un serveur figé ne bloque plus sans fin.
- xtask fetch : un runtime dont l'exécutable a disparu est réextrait ; noms de fichiers du manifeste limités à une liste blanche sûre sous Windows.
- spike-stt : la fin de l'entrée (Ctrl-D, Ctrl-Z) termine la boucle au lieu de tourner sans fin.
- spike-tts : un texte vide est refusé proprement au lieu de paniquer ; audio : WAV à 0 Hz refusé, fin du son protégée par un silence de garde.
- xtask : lancé dans un dossier qui contient `models/manifest.toml`, il travaille dans ce dossier, même si le dépôt où il a été compilé existe sur la machine (sur le runner Windows, les modèles partaient dans le dépôt au lieu du dossier livré).
- spike-stt : sur un PC sans pilote Vulkan, le programme avortait au démarrage, même avec `--cpu` (exception C++ de ggml) ; les cartes sont désormais listées par le registre de backends de ggml, qui renvoie une liste vide.

### CI / build
- Socle Rust (#3) : workspace Cargo, toolchain 1.99.1 épinglée, `cargo xtask fetch` qui télécharge les modèles et llama.cpp de `models/manifest.toml` et vérifie leur SHA-256, `cargo xtask changelog check|extract`.
- Devcontainer reproductible (#3) : Ubuntu 24.04 épinglée par digest, paquets apt figés par un snapshot daté, son et affichage de WSLg dans la variante Windows + WSL2, modèles dans un volume Docker.
- CI GitHub Actions (#4) : à chaque push, format, lint et tests dans l'image du devcontainer sous Linux, et sur un runner Windows.
- CI : test de transcription d'une phrase française (Whisper small sur CPU) et artefact `bullitik-spikes-windows` (exécutables des spikes, `xtask.exe`, manifeste) pour mesurer sur un PC Windows ; SDK Vulkan 1.4.363.0 épinglé par SHA-256.
- `cargo xtask --root` : `xtask.exe` livré hors du dépôt travaille dans le dossier courant.
- CI : test aller-retour sur CPU (Piper lit une phrase, Whisper small la retranscrit) ; données d'espeak-ng copiées dans l'artefact Windows.
- Kit de mesure Windows : `mesurer.ps1` (dans l'artefact) relève la machine et lance les trois spikes sur des entrées fixes, rapport Markdown ; la CI vérifie sa syntaxe et lance le dossier livré sur un runner Windows nu (CPU, petits modèles).
- CD (#5) : `release.yml` publie à chaque tag `v*` une release GitHub avec les dossiers portables des spikes en zip (Windows et Linux), texte tiré du CHANGELOG ; `cargo xtask package` assemble ce dossier, pour la CI comme pour la CD ; SDK Vulkan dans une action partagée.
- CI : délais maximum par job, code de sortie de l'installeur du SDK Vulkan vérifié, `ca-certificates` de l'image figé avant le passage au snapshot.
