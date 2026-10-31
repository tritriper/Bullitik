# Bullitik

Jeu vidéo vocal open source, en français (nom définitif à venir). Un
intervieweur pose des questions factuelles façon interview politique ; le
joueur répond à voix haute et doit parler autour du sujet sans y répondre, le
plus discrètement possible. Un juge, un modèle Mistral exécuté en local, note
la réponse. Tout tourne sur le PC du joueur, hors ligne.

Statut : lot 0, spikes de faisabilité. Rien de jouable pour l'instant.

## Contraintes du projet

- Tout en local et hors ligne, sans API cloud.
- LLM : un modèle Mistral sur la carte graphique, AMD comme NVIDIA.
- Le LLM ne traite que du texte ; la voix passe par des bibliothèques dédiées.
- Livrable : un dossier portable qu'on lance directement, sans installeur.
- Développement dans un devcontainer reproductible.

## Où trouver quoi

| Quoi | Où |
| --- | --- |
| Ce qui reste à faire | [`docs/dev/backlog.md`](docs/dev/backlog.md) |
| Ce qui a été livré | [`CHANGELOG.md`](CHANGELOG.md) |
| Installer l'environnement (devcontainer) | [`docs/dev/setup.md`](docs/dev/setup.md) |
| Mesurer les spikes sur un PC Windows | [`docs/spikes/windows.md`](docs/spikes/windows.md) |
| Travailler avec Claude Code sur son PC | [`docs/dev/session-locale.md`](docs/dev/session-locale.md) |
| Comment contribuer (git flow, PR) | [`docs/dev/contributing.md`](docs/dev/contributing.md) |
| Règles pour les agents de code | [`AGENTS.md`](AGENTS.md) |
| Conception et plan de développement | documents vivants hors dépôt, liés depuis [`AGENTS.md`](AGENTS.md) |

## Licence

GPL-3.0-or-later, voir [`LICENSE`](LICENSE). La synthèse vocale s'appuie sur
espeak-ng (phonétisation), sous GPL-3.0 et compilé dans le jeu : le jeu est
donc sous GPL lui aussi. Les autres composants et les crédits des voix sont
dans [`docs/licences-tierces.md`](docs/licences-tierces.md).
