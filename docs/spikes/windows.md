# Spikes de Bullitik sous Windows

Ce dossier vient de l'artefact `bullitik-spikes-windows` de la CI GitHub
(onglet *Actions*, une exécution, en bas de la page). Il sert à mesurer les
spikes sur un vrai PC Windows, avec sa carte graphique AMD ou NVIDIA.

## Récupérer le dossier

Sur GitHub, onglet *Actions*, dernière exécution réussie de la CI sur `dev`,
artefact **bullitik-spikes-windows** en bas de la page (il faut être connecté).
En ligne de commande, avec [GitHub CLI](https://cli.github.com/) :

```powershell
gh run download --repo tritriper/Bullitik --branch dev --name bullitik-spikes-windows --dir bullitik-spikes
```

## Mesurer en une commande

```powershell
cd bullitik-spikes
powershell -ExecutionPolicy Bypass -File .\mesurer.ps1
```

Le script relève la machine (cartes graphiques, pilotes, processeur, mémoire),
télécharge les modèles s'il le faut, lance les trois spikes sur des entrées
fixes (sans micro ni écoute), et écrit tout dans `mesures-<PC>-<date>.md`.
Options : `-Avec14B` mesure aussi Ministral 3 14B, `-Cpu` ajoute les mesures
sur processeur pour comparer. Ce rapport suffit pour remplir les rapports des
spikes ; les commandes ci-dessous servent à essayer à la main, au micro.

## Une fois : télécharger les modèles

Dans un terminal PowerShell ouvert dans ce dossier :

```powershell
.\xtask.exe fetch
```

Environ 6,2 Go arrivent dans `models\files\`, et llama.cpp dans
`runtime\llama.cpp\` ; chaque fichier est vérifié par SHA-256. Relancer la
commande ne retélécharge rien.

## Lancer un spike

| Spike | Commande |
| --- | --- |
| 0.1 Reconnaissance vocale, au micro | `.\spike-stt.exe` |
| 0.1 Sur un fichier, 5 mesures | `.\spike-stt.exe --wav essai.wav --repeat 5` |
| 0.1 Forcer le CPU, pour comparer | `.\spike-stt.exe --cpu` |
| 0.2 Synthèse vocale, les 4 voix l'une après l'autre | `.\spike-tts.exe` |
| 0.2 Une voix, un texte | `.\spike-tts.exe --voice upmc-pierre --text "Bonsoir."` |
| 0.3 Intervieweur et juge, Ministral 3 8B | `.\spike-llm.exe` |
| 0.3 Le même sur CPU, pour comparer | `.\spike-llm.exe --cpu` |
| 0.3 Avec le 14B (après `.\xtask.exe fetch --with ministral-3-14b`) | `.\spike-llm.exe --model models\files\Ministral-3-14B-Instruct-2512-Q4_K_M.gguf` |

Chaque spike affiche la carte graphique utilisée, ses mesures, puis un résumé
à recopier dans le rapport du spike (`docs/spikes/` dans le dépôt).

## En cas de problème

- **« VCRUNTIME140.dll introuvable »** : installer le
  [Visual C++ Redistributable](https://learn.microsoft.com/cpp/windows/latest-supported-vc-redist)
  x64 de Microsoft.
- **« GPU Vulkan : aucun »** : mettre à jour le pilote de la carte graphique
  (AMD Adrenalin ou NVIDIA Game Ready).
- **Pas de micro** : vérifier le micro par défaut dans *Paramètres → Système →
  Son → Entrée*.
