# Contribuer

## Git flow

```text
main       ●──────────────────────●──────   v0.1.0 (tag), protégée
            \                    / \
release/     \             ●────●   \       préparation de la version
              \           /          \
dev            ●───●─────●────────────●──   intégration, protégée
                    \   /
feat/…               ●─●                    une branche par sujet
```

| Branche | Part de | Fusionne dans | Rôle |
| --- | --- | --- | --- |
| `main` | — | — | Ce qui est publié. Chaque merge porte un tag `vx.y.z`. |
| `dev` | `main` | `release/*` | Intégration continue de tout le travail. |
| `feat/<slug>`, `fix/<slug>`, `docs/<slug>`, `spike/<slug>` | `dev` | `dev` | Une branche par sujet, par pull request. |
| `release/x.y.z` | `dev` | `main`, puis `dev` | Datation du CHANGELOG, archive du backlog, tag. |
| `hotfix/<slug>` | `main` | `main`, puis `dev` | Correctif urgent d'une version publiée. |

Les sessions cloud de Claude Code imposent leur propre nom de branche
(`ccr-…`) ; il joue le rôle d'une branche `feat/…`.

### Faire une pull request

1. Partir de `dev` à jour : `git switch dev && git pull`, puis
   `git switch -c feat/<slug>`.
2. Ajouter la ligne de `CHANGELOG.md` et mettre à jour le backlog dans la même
   branche (voir [AGENTS.md](../../AGENTS.md)).
3. Ouvrir la pull request vers `dev` en remplissant le modèle.
4. La CI doit être verte avant la fusion (dès qu'elle existe, item #4).

### Faire une release

1. `git switch -c release/x.y.z dev`.
2. Dans `CHANGELOG.md`, renommer `## [Non publié]` en `## [x.y.z] — <date>` et
   ajouter le lien de comparaison en bas du fichier.
3. Déplacer les items livrés du [backlog](backlog.md) vers
   l'[archive](backlog-archive.md).
4. Pull request vers `main`. Après fusion, le responsable du projet tagge :
   `git tag vx.y.z && git push origin vx.y.z`. La CD produit le livrable.
5. Refusionner `main` dans `dev`.

On suit [SemVer](https://semver.org/lang/fr/) : correctif, fonctionnalité,
rupture.

## Protection des branches (réglage GitHub, une fois)

Dans *Settings → Rules → Rulesets*, deux rulesets :

| Réglage | `main` | `dev` |
| --- | --- | --- |
| Pull request obligatoire avant fusion | oui | oui |
| Statut de la CI exigé (une fois la CI en place) | oui | oui |
| Interdire le force push | oui | oui |
| Interdire la suppression | oui | oui |

## Prendre un item du backlog

Le [backlog](backlog.md) fait foi, pas les issues. On n'ouvre une issue (modèle
*Item du backlog*) que pour un item qu'on prend vraiment ou qu'on propose à
quelqu'un ; on passe alors sa ligne à 🚧 avec le lien de l'issue.
