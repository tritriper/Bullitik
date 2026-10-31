# Travailler avec Claude Code sur son propre PC

Les sessions Claude Code dans le cloud (claude.ai/code) n'ont pas de GPU et
n'atteignent pas le PC de l'utilisateur. Pour tout ce qui demande le vrai
matériel (mesures GPU AMD ou NVIDIA, micro, haut-parleurs), on continue sur le
PC, de deux façons.

Sources : [Claude Code on the web](https://code.claude.com/docs/en/claude-code-on-the-web),
[Remote Control](https://code.claude.com/docs/en/remote-control),
[installation](https://code.claude.com/docs/en/setup).

## Une fois : installer Claude Code sous Windows

Dans PowerShell, sans droits administrateur :

```powershell
irm https://claude.ai/install.ps1 | iex
claude --version
claude auth login      # le même compte claude.ai que la session cloud
```

Recommandé : [Git for Windows](https://git-scm.com/download/win) (Claude Code
utilise alors Git Bash ; sinon PowerShell) et
[GitHub CLI](https://cli.github.com/) (`gh auth login`), pour récupérer les
artefacts de la CI.

Puis cloner le dépôt sur un chemin court, sous Windows (pas dans WSL : les
mesures se font sous Windows natif) :

```powershell
git clone https://github.com/tritriper/Bullitik.git C:\dev\Bullitik
cd C:\dev\Bullitik
```

## Option A : reprendre la session cloud en local (teleport)

La conversation et la branche de la session cloud passent sur le PC.

1. Dans la session cloud, sur claude.ai/code, taper `/teleport` : elle répond
   la commande exacte. Ou bien, dans le menu de la session, *Open in →
   Terminal*.
2. Dans `C:\dev\Bullitik`, avec un dépôt propre (`git status` vide), lancer la
   commande, de la forme `claude --teleport <identifiant de session>`.
   Sans identifiant, `claude --teleport` propose la liste des sessions.

Ensuite, le travail continue en local ; il ne remonte pas dans la session
cloud. Les commits poussés, eux, sont visibles partout.

## Option B : piloter le PC depuis claude.ai (Remote Control)

La session tourne sur le PC, et on la suit depuis le navigateur ou le
téléphone.

```powershell
cd C:\dev\Bullitik
claude remote-control --name "PC de mesure"
```

Le PC doit rester allumé et la commande ouverte. `claude doctor` indique si
Remote Control est disponible pour le compte.

## Ce que la session locale fait ensuite

1. Récupérer le dossier des spikes construit par la CI (voir
   [windows.md](../spikes/windows.md)) :
   `gh run download --repo tritriper/Bullitik --branch dev --name bullitik-spikes-windows --dir C:\dev\bullitik-spikes`
2. Lancer `powershell -ExecutionPolicy Bypass -File C:\dev\bullitik-spikes\mesurer.ps1`.
3. Reporter les chiffres du rapport produit dans `docs/spikes/0.1-stt.md`,
   `0.2-tts.md` et `0.3-llm.md`, sur une branche, puis ouvrir une pull request
   vers `dev`.

Les spikes n'ont pas besoin d'être compilés sur le PC : la CI le fait. Compiler
sous Windows demande Visual Studio Build Tools, le SDK Vulkan et un dossier de
build court (voir `.github/workflows/ci.yml`).
