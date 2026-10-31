# Contenus du jeu

Tout ce qui se modifie sans toucher au code : prompts, fiche du juge,
questions. Le code lit ces fichiers tels quels.

| Fichier | Rôle |
| --- | --- |
| `prompts/intervieweur.md` | Prompt système de l'intervieweur |
| `prompts/juge.md` | Prompt système du juge ; `{{fiche}}` y est remplacé par la fiche |
| `juge/fiche-techniques.md` | Fiche courte des techniques d'esquive, pour le juge |
| `juge/schema-notation.json` | Schéma JSON imposé à la réponse du juge |
| `juge/exemples.toml` | Réponses annotées pour vérifier le juge |
| `questions/questions.toml` | Banque de questions de l'intervieweur |

**Statut** : version provisoire du spike 0.3, pour vérifier que le modèle tient
les deux rôles. Le lot 1 (#10 à #14) les réécrit à partir de l'état de l'art.

Dans le prompt du juge, `{{fiche}}` est remplacé par la fiche des techniques
avant l'envoi au modèle ; aucune autre variable n'est reconnue. La relance de
l'intervieweur utilise le premier exemple annoté comme esquive
(`a_repondu = false`).
