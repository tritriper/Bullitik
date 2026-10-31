# Spikes de Bullitik sous Linux

Ce dossier vient d'une release GitHub de Bullitik. Il sert à mesurer les spikes
sur un PC Linux avec sa carte graphique AMD ou NVIDIA (pilote Vulkan à jour).

Construit sous Ubuntu 24.04 : il faut une glibc 2.39 ou plus récente, et les
bibliothèques `libvulkan1` et `libasound2` (présentes sur un poste de bureau).

## Une fois : télécharger les modèles

```sh
./xtask fetch
```

Environ 6,2 Go arrivent dans `models/files/`, et llama.cpp dans
`runtime/llama.cpp/` ; chaque fichier est vérifié par SHA-256.

## Lancer un spike

| Spike | Commande |
| --- | --- |
| 0.1 Reconnaissance vocale, au micro | `./spike-stt` |
| 0.1 Sur le fichier de test, 5 mesures | `./spike-stt --wav testdata/phrase.wav --repeat 5` |
| 0.2 Synthèse vocale, les 4 voix | `./spike-tts` |
| 0.3 Intervieweur et juge | `./spike-llm` |

Ajouter `--cpu` à `spike-stt` ou `spike-llm` pour comparer avec le processeur.
