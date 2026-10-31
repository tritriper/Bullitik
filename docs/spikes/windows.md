# Spikes de Bullitik sous Windows

Ce dossier vient de l'artefact `bullitik-spikes-windows` de la CI GitHub
(onglet *Actions*, une exécution, en bas de la page). Il sert à mesurer les
spikes sur un vrai PC Windows, avec sa carte graphique AMD ou NVIDIA.

## Une fois : télécharger les modèles

Dans un terminal PowerShell ouvert dans ce dossier :

```powershell
.\xtask.exe fetch
```

Environ 6,2 Go arrivent dans `models\files\` ; chaque fichier est vérifié par
SHA-256. Relancer la commande ne retélécharge rien.

## Lancer un spike

| Spike | Commande |
| --- | --- |
| 0.1 Reconnaissance vocale, au micro | `.\spike-stt.exe` |
| 0.1 Sur un fichier, 5 mesures | `.\spike-stt.exe --wav essai.wav --repeat 5` |
| 0.1 Forcer le CPU, pour comparer | `.\spike-stt.exe --cpu` |
| 0.2 Synthèse vocale, les 4 voix l'une après l'autre | `.\spike-tts.exe` |
| 0.2 Une voix, un texte | `.\spike-tts.exe --voice upmc-pierre --text "Bonsoir."` |

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
