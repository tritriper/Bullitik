# Backlog

Tout ce qui reste à faire, sur une page. Le lot dit où l'item s'inscrit dans le
[plan de développement](https://claude.ai/artifact/ThsKQ1DZEeVz51H37nb7nH),
l'état dit où il en est. Ce qui est livré part dans
l'[archive](backlog-archive.md) avec son numéro, et dans le
[CHANGELOG](../../CHANGELOG.md).

**États** : 🧭 décision à prendre · ✅ prêt · 🚧 en cours · ⏳ en attente d'un
autre item · 🧪 construit, reste à lancer.

**Au 31 octobre 2026** : 2 décisions · 4 à lancer · 4 prêts · 7 en attente.

## Lot 0 — Spikes de faisabilité

| ID | Item | État | Fait quand |
| --- | --- | --- | --- |
| #5 | CD : livrable portable Windows et Linux à chaque tag `v*` | 🧪 à lancer : fusionné dans `dev` (PR #7), reste un premier tag pour l'essayer | Un tag produit une release GitHub avec le zip et le texte du CHANGELOG. |
| #6 | Spike 0.1 — reconnaissance vocale seule, latence mesurée | 🧪 à lancer : mesuré sur AMD (2 s en turbo, 1 s en small), reste une carte NVIDIA | Je parle, le texte s'affiche ; rapport de latence dans `docs/spikes/`. |
| #7 | Spike 0.2 — synthèse vocale seule, latence mesurée, choix de la voix | 🧭 décision : mesuré sur PC (moins de 70 ms avant le son), reste à choisir la voix à l'écoute | Un texte est lu à voix haute ; rapport de latence et voix retenue. |
| #8 | Spike 0.3 — modèle seul : intervieweur et juge en JSON, temps et VRAM mesurés | 🧪 à lancer : mesuré sur AMD (question 0,8 s, juge 2,4 s, 5,9 Go), reste une carte NVIDIA | Les deux rôles répondent ; rapport de temps de réponse et de mémoire GPU. |
| #9 | Spike 0.4 — chaîne complète voix → texte → modèle → voix (optionnel) | 🧪 à lancer : 3,0 s de latence de dialogue sur AMD (seuil : 3 s), reste une carte NVIDIA | Une question complète passe de bout en bout ; latence totale mesurée. |
| #19 | CI : épingler les actions GitHub par SHA de commit, mises à jour par Dependabot | ✅ prêt | Aucune action référencée par un tag modifiable ; Dependabot ouvre les PR de montée de version. |
| #20 | Robustesse des outils : verrou contre deux `xtask fetch` simultanés ; spike-llm vérifie que le serveur qui répond est bien le sien (`/props`) | ✅ prêt | Deux `fetch` en parallèle ne corrompent rien ; un port volé est détecté. |
| #21 | Latence de la reconnaissance vocale : `audio_ctx` réduit à la durée de la réponse, turbo comparé à small sur des réponses de joueurs | ✅ prêt | Latence de dialogue sous 3 s sur AMD, avec une transcription aussi juste ; rapport du spike 0.1 à jour. |
| #18 | GPU dans le devcontainer : Vulkan via la couche Mesa « dozen » de WSL2 | ⏳ après le lot 0, si besoin | Un spike lancé dans le conteneur utilise la carte AMD ou NVIDIA de l'hôte. |

## Lot 1 — Recherche et fiche du juge

| ID | Item | État | Fait quand |
| --- | --- | --- | --- |
| #22 | Méthode de recherche du lot 1 : état de l'art, documentation, expériences de validation (tests de performance du juge), critères pour garder ou écarter une technique | ✅ prêt | Méthode écrite dans `docs/recherche/`, validée par le responsable du projet. |
| #11 | Typologie des techniques d'esquive retenue, avec définition et exemple ; part de la typologie proposée par #10 | ⏳ #22 | Fiche dans `content/juge/`. |
| #12 | Règles du juge : prompt système, critères, schéma JSON de sortie | ⏳ #11 | Prompt et schéma dans `content/`, validés par un test. |
| #13 | Jeu d'exemples annotés pour calibrer le juge | ⏳ #12 | Le juge retrouve la note attendue sur la majorité des exemples. |
| #14 | Prompt minimal de l'intervieweur, avec relances | ⏳ #8 | Prompt dans `content/prompts/`. |

## Lot 2 et lot 3

| ID | Item | État | Fait quand |
| --- | --- | --- | --- |
| #15 | Prototype de test avec les amis : boucle vocale, journalisation, questionnaire, consentement | ⏳ lot 1 | Une partie complète se joue sur la machine de test. |
| #16 | Game design co-conçu à partir des retours | ⏳ #15 | Univers, score et progression décrits dans le document de conception. |
| #17 | Nom du jeu | 🧭 décision | Le nom est choisi ; le dépôt et le code sont renommés si besoin. |
