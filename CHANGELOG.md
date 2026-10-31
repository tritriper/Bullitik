# Changelog

Tous les changements notables de Bullitik, version par version. Le format suit
l'esprit de [Keep a Changelog](https://keepachangelog.com/fr/) ; la **version
fait foi via le tag git** (`v*`). Le backlog vit dans
[`docs/dev/backlog.md`](docs/dev/backlog.md) et les items livrés dans
[`docs/dev/backlog-archive.md`](docs/dev/backlog-archive.md) ; les `#N`
ci-dessous y renvoient.

## [Non publié]

### Ajouté
- Organisation du dépôt (#2) : git flow `main` / `dev` / branches de travail, backlog numéroté, CHANGELOG, modèles de pull request et d'item de backlog, règles des agents, licence GPL-3.0-or-later.

### CI / build
- Socle Rust (#3) : workspace Cargo, toolchain 1.99.1 épinglée, `cargo xtask fetch` qui télécharge les modèles et llama.cpp de `models/manifest.toml` et vérifie leur SHA-256, `cargo xtask changelog check|extract`.
- Devcontainer reproductible (#3) : Ubuntu 24.04 épinglée par digest, paquets apt figés par un snapshot daté, son et affichage de WSLg dans la variante Windows + WSL2, modèles dans un volume Docker.
- CI GitHub Actions (#4) : à chaque push, format, lint et tests dans l'image du devcontainer sous Linux, et sur un runner Windows.
