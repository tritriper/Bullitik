# Backlog

Tout ce qui reste à faire, sur une page. Le lot dit où l'item s'inscrit dans le
[plan de développement](https://claude.ai/artifact/ThsKQ1DZEeVz51H37nb7nH),
l'état dit où il en est. Ce qui est livré part dans
l'[archive](backlog-archive.md) avec son numéro, et dans le
[CHANGELOG](../../CHANGELOG.md).

**États** : 🧭 décision à prendre · ✅ prêt · 🚧 en cours · ⏳ en attente d'un
autre item · 🧪 construit, reste à lancer.

**Au 31 octobre 2026** : 2 décisions · 1 en cours · 3 à lancer · 3 prêts · 8 en attente.

## Lot 0 — Spikes de faisabilité

| ID | Item | État | Fait quand |
| --- | --- | --- | --- |
| #2 | Organisation du dépôt : git flow, backlog, CHANGELOG, modèles de PR et d'issue, licence | 🧭 décision | Fusionné dans `dev` (PR #1). Reste : protéger `main` et `dev` (réglage GitHub, voir `contributing.md`). |
| #5 | CD : livrable portable Windows et Linux à chaque tag `v*` | 🚧 en cours (PR #7) : reste un premier tag pour l'essayer | Un tag produit une release GitHub avec le zip et le texte du CHANGELOG. |
| #6 | Spike 0.1 — reconnaissance vocale seule, latence mesurée | 🧪 à lancer : fusionné dans `dev`, reste la mesure sur PC AMD et NVIDIA (`mesurer.ps1`) | Je parle, le texte s'affiche ; rapport de latence dans `docs/spikes/`. |
| #7 | Spike 0.2 — synthèse vocale seule, latence mesurée, choix de la voix | 🧪 à lancer : fusionné dans `dev`, reste la mesure sur PC AMD et NVIDIA (`mesurer.ps1`) | Un texte est lu à voix haute ; rapport de latence et voix retenue. |
| #8 | Spike 0.3 — modèle seul : intervieweur et juge en JSON, temps et VRAM mesurés | 🧪 à lancer : fusionné dans `dev`, reste la mesure sur PC AMD et NVIDIA (`mesurer.ps1`) | Les deux rôles répondent ; rapport de temps de réponse et de mémoire GPU. |
| #9 | Spike 0.4 — chaîne complète voix → texte → modèle → voix (optionnel) | ✅ prêt | Une question complète passe de bout en bout ; latence totale mesurée. |
| #19 | CI : épingler les actions GitHub par SHA de commit, mises à jour par Dependabot | ✅ prêt | Aucune action référencée par un tag modifiable ; Dependabot ouvre les PR de montée de version. |
| #20 | Robustesse des outils : verrou contre deux `xtask fetch` simultanés ; spike-llm vérifie que le serveur qui répond est bien le sien (`/props`) | ✅ prêt | Deux `fetch` en parallèle ne corrompent rien ; un port volé est détecté. |
| #18 | GPU dans le devcontainer : Vulkan via la couche Mesa « dozen » de WSL2 | ⏳ après le lot 0, si besoin | Un spike lancé dans le conteneur utilise la carte AMD ou NVIDIA de l'hôte. |

## Lot 1 — Recherche et fiche du juge

| ID | Item | État | Fait quand |
| --- | --- | --- | --- |
| #10 | État de l'art : Bull, Bavelas, Rogers et Norton, évasion en interview politique | ⏳ lot 0 | Synthèse sourcée dans `docs/`. |
| #11 | Typologie des techniques d'esquive retenue, avec définition et exemple | ⏳ #10 | Fiche dans `content/juge/`. |
| #12 | Règles du juge : prompt système, critères, schéma JSON de sortie | ⏳ #11 | Prompt et schéma dans `content/`, validés par un test. |
| #13 | Jeu d'exemples annotés pour calibrer le juge | ⏳ #12 | Le juge retrouve la note attendue sur la majorité des exemples. |
| #14 | Prompt minimal de l'intervieweur, avec relances | ⏳ #8 | Prompt dans `content/prompts/`. |

## Lot 2 et lot 3

| ID | Item | État | Fait quand |
| --- | --- | --- | --- |
| #15 | Prototype de test avec les amis : boucle vocale, journalisation, questionnaire, consentement | ⏳ lot 1 | Une partie complète se joue sur la machine de test. |
| #16 | Game design co-conçu à partir des retours | ⏳ #15 | Univers, score et progression décrits dans le document de conception. |
| #17 | Nom du jeu | 🧭 décision | Le nom est choisi ; le dépôt et le code sont renommés si besoin. |
