# Mettre en place l'environnement de développement

Tout se passe dans le devcontainer : la toolchain Rust, les dépendances C de
whisper.cpp, Vulkan et les modèles y sont installés à l'identique pour tout le
monde. La CI Linux tourne dans la même image.

## Windows + WSL2

### Prérequis, une fois

- Windows 11, ou Windows 10 22H2 avec WSLg.
- WSL2 à jour (`wsl --update` dans PowerShell) et une distribution Ubuntu.
- [Docker Desktop](https://docs.docker.com/desktop/setup/install/windows-install/)
  avec le moteur WSL2 et l'intégration activée pour cette distribution
  (*Settings → Resources → WSL integration*).
- VS Code avec les extensions *WSL* et *Dev Containers*.
- Environ 30 Go libres : l'image, 6,2 Go de modèles et le dossier de build.

### Ouvrir le projet

1. Dans un terminal Ubuntu (WSL), cloner **dans le système de fichiers WSL**,
   pas sous `/mnt/c` (dix fois plus lent) :
   `git clone https://github.com/tritriper/Bullitik.git ~/Bullitik`
2. `code ~/Bullitik`, puis *Reopen in Container* et choisir
   **« Bullitik (Windows + WSL2) »**.
3. Au premier lancement, l'image se construit (environ 5 minutes), puis
   `cargo xtask fetch` télécharge et vérifie les modèles (6,2 Go) dans le volume
   Docker `bullitik-models`. Ce volume survit à la reconstruction du conteneur.

### Vérifier

```sh
cargo test                                   # les tests passent
parecord --channels=1 /tmp/essai.wav         # parler, puis Ctrl+C
paplay /tmp/essai.wav                        # on s'entend dans les haut-parleurs
```

Si `parecord` ne trouve pas de serveur, vérifier que WSLg fonctionne hors du
conteneur (`ls /mnt/wslg/PulseServer` dans la distribution Ubuntu).

## Le GPU et les mesures

Le conteneur n'utilise pas le GPU. Sous WSL2, Vulkan n'atteint la carte qu'à
travers la couche Mesa « dozen », absente des paquets Ubuntu 24.04 et encore
incomplète. Dans le conteneur, Vulkan tourne donc sur le processeur (lavapipe) :
assez pour compiler, tester et vérifier qu'un spike fonctionne, mais pas pour
mesurer.

**Les latences et la mémoire GPU se mesurent sur Windows natif**, avec les
exécutables Windows que la CI publie pour chaque spike. C'est aussi là que le
jeu tournera. Brancher le GPU dans le conteneur reste possible plus tard
(backlog #18) si le besoin s'en fait sentir.

## Autres hôtes

Le devcontainer générique (**« Bullitik »**, `.devcontainer/devcontainer.json`)
marche sur n'importe quel hôte Docker (Linux natif, Codespaces), sans son ni
affichage.

## Commandes du projet

| Commande | Effet |
| --- | --- |
| `cargo xtask fetch` | Télécharge et vérifie l'ensemble `dev` (tous les spikes) |
| `cargo xtask fetch --set ci` | Le minimum des tests de la CI (250 Mo) |
| `cargo xtask fetch --with ministral-3-14b` | Ajoute un modèle optionnel |
| `cargo xtask fetch --dry-run` | Liste ce qui serait téléchargé, sans rien télécharger |
| `cargo xtask check-manifest` | Vérifie `models/manifest.toml` |
| `cargo xtask changelog check` | Vérifie le format du CHANGELOG |
| `cargo xtask changelog extract 0.1.0` | Affiche une version, pour le texte d'une release |

## Mettre à jour une version épinglée

| Quoi | Où |
| --- | --- |
| Toolchain Rust | `rust-toolchain.toml` |
| Paquets apt de l'image | `UBUNTU_SNAPSHOT` dans `.devcontainer/Dockerfile` |
| rustup | `RUSTUP_VERSION` et `RUSTUP_SHA256` dans le même fichier |
| Image de base | le digest de `FROM` dans le même fichier |
| Modèle ou llama.cpp | URL, taille et SHA-256 dans `models/manifest.toml` |
