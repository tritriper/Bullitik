# AGENTS.md

Consignes pour les agents de code (Claude Code et autres) qui travaillent dans
ce dépôt.

## Ce qu'est le projet

Bullitik (nom de travail) est un mini jeu vocal en français. Un intervieweur
pose des questions factuelles façon interview politique ; le joueur répond à
voix haute et doit parler autour du sujet sans y répondre, le plus discrètement
possible. Un juge, un LLM local, note la réponse.

Contraintes non négociables :

- tout tourne en local, hors ligne, sans API cloud ;
- LLM : un modèle Mistral exécuté sur la carte graphique, **AMD comme NVIDIA** ;
- le LLM ne traite que du texte ; reconnaissance et synthèse vocales passent par
  des bibliothèques existantes ;
- langue du jeu : français ;
- livrable : application PC portable, sans installeur ;
- développement dans un devcontainer reproductible (hôte : Windows + WSL2) ;
- dépôt GitHub public, licence GPL-3.0-or-later.

## Documents de référence

Deux documents vivants, hors dépôt, font référence :

- le [document de conception](https://claude.ai/artifact/EEiH5bRnuimk7a6ChuzuWg) :
  concept, choix validés, architecture, juge, questions ouvertes, **journal des
  décisions** ;
- le [plan de développement](https://claude.ai/artifact/ThsKQ1DZEeVz51H37nb7nH) :
  les lots et leurs critères de réussite.

Une décision prise en cours de route s'inscrit dans le journal des décisions ;
une question ouverte tranchée est retirée de la liste ou marquée comme résolue.
Si ces documents deviennent inaccessibles, en tenir une copie à jour dans
`docs/CONCEPTION.md` et `docs/PLAN.md`.

## Manière de travailler

- **Pas à pas, avec validation.** Chaque étape d'un lot attend l'accord du
  responsable du projet avant la suivante. Aucun outil installé ni code écrit
  sur une techno qui n'a pas été validée.
- **Poser la question plutôt que supposer** quand quelque chose est flou.
- **Contenus séparés du code.** Prompts, fiche du juge et questions vivent dans
  `content/`, modifiables sans toucher au code.

## Environnement

- Tout se développe dans le devcontainer ([`docs/dev/setup.md`](docs/dev/setup.md)).
  Toolchain, image, paquets apt : tout est épinglé ; une montée de version se
  fait dans le fichier indiqué par `setup.md`, jamais à la main dans le conteneur.
- **Tout ce qui se télécharge** (modèle, binaire tiers) est déclaré dans
  `models/manifest.toml` avec son URL figée, sa taille, son SHA-256 et sa
  licence, puis récupéré par `cargo xtask fetch`. Jamais de téléchargement
  ad hoc dans un script ou dans le code. Les outils de la CI (SDK Vulkan…)
  sont épinglés dans le workflow, avec leur SHA-256. Seule exception, connue :
  la crate `ort-sys` télécharge onnxruntime pendant la compilation (version et
  empreinte épinglées par la crate elle-même) ; un build hors ligne n'est donc
  pas encore possible.
- Les mesures de performance (latence, VRAM) se font sur Windows natif, avec
  les exécutables produits par la CI, jamais dans le conteneur.

## Git flow

Le détail est dans [`docs/dev/contributing.md`](docs/dev/contributing.md).
En bref :

- **Aucun push direct sur `main` ni sur `dev`.** Tout passe par une branche de
  travail et une pull request vers `dev`.
- `main` ne reçoit que des releases (`release/x.y.z` depuis `dev`) et des
  correctifs urgents (`hotfix/<slug>`).
- Tagger et pousser un tag est la décision du responsable du projet : préparer
  la release, ne pas tagger soi-même.

## CHANGELOG et backlog, dans la même PR

- **Mettre à jour `CHANGELOG.md` dans la PR qui introduit le changement**, sous
  `## [Non publié]`, dans la sous-section qui convient : `### Ajouté`,
  `### Modifié`, `### Corrigé`, `### Limitations connues`, `### CI / build`.
  **Un changement = une ligne**, sans retour à la ligne ; les détails vont dans
  le corps du commit ou dans `docs/`.
- **Garder le backlog vrai.** Une PR qui touche un item numéroté met à jour sa
  ligne dans [`docs/dev/backlog.md`](docs/dev/backlog.md). Un item livré part
  dans [`docs/dev/backlog-archive.md`](docs/dev/backlog-archive.md) avec son
  numéro. **Un numéro n'est jamais réutilisé ni renuméroté.**
- **Une release, c'est son tag.** `## [x.y.z] — <date>` n'apparaît dans le
  CHANGELOG que dans la PR de release, taggée `vx.y.z` juste après.

## Spikes

Chaque spike du lot 0 est isolé et lançable seul. Il livre, dans la même PR :
une commande de lancement documentée, un court rapport dans `docs/spikes/`
(mesures, matériel utilisé, verdict) et une ligne de CHANGELOG.
