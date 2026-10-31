<#
.SYNOPSIS
    Lance les spikes de Bullitik en mode mesure sur un PC Windows et écrit un
    rapport Markdown à recopier dans docs/spikes/.

.DESCRIPTION
    À lancer depuis le dossier de l'artefact « bullitik-spikes-windows » de la
    CI (celui qui contient spike-stt.exe, spike-tts.exe, spike-llm.exe et
    xtask.exe) :

        powershell -ExecutionPolicy Bypass -File .\mesurer.ps1

    Le script :
      1. relève la machine (cartes graphiques et pilotes, processeur, mémoire) ;
      2. télécharge les modèles s'ils manquent (xtask fetch, vérifiés par SHA-256) ;
      3. lance chaque spike sur des entrées fixes, sans micro ni écoute :
         reconnaissance vocale sur testdata\phrase.wav, synthèse des 4 voix,
         intervieweur et juge, chaîne complète (le WAV tient lieu de réponse) ;
      4. écrit tout dans mesures-<machine>-<date>.md.

.PARAMETER Avec14B
    Mesure aussi Ministral 3 14B (8,2 Go de plus à télécharger).

.PARAMETER Cpu
    Ajoute les mêmes mesures sur CPU, pour comparer (long : plusieurs minutes).
#>
[CmdletBinding()]
param(
    [switch]$Avec14B,
    [switch]$Cpu
)

$ErrorActionPreference = 'Continue'
# Les spikes écrivent en UTF-8 : sans cela, les accents sont abîmés.
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8
Set-Location -LiteralPath $PSScriptRoot

$date = Get-Date -Format 'yyyy-MM-dd-HHmm'
$rapport = Join-Path $PSScriptRoot "mesures-$env:COMPUTERNAME-$date.md"
$lignes = New-Object System.Collections.Generic.List[string]

function Ajouter([string]$texte) {
    $lignes.Add($texte)
    Write-Host $texte
}

function Lancer([string]$titre, [string]$exe, [string[]]$arguments) {
    Ajouter ''
    Ajouter "## $titre"
    Ajouter ''
    Ajouter ('`' + "$exe $($arguments -join ' ')" + '`')
    Ajouter ''
    Ajouter '```text'
    $debut = Get-Date
    # 2>&1 : les erreurs des spikes vont aussi dans le rapport.
    & ".\$exe" @arguments 2>&1 | ForEach-Object { Ajouter "$_" }
    $code = $LASTEXITCODE
    Ajouter '```'
    $duree = [math]::Round(((Get-Date) - $debut).TotalSeconds, 1)
    Ajouter "Code de sortie : $code ; durée totale : $duree s"
}

# --- 1. La machine --------------------------------------------------------------
Ajouter "# Mesures des spikes Bullitik — $env:COMPUTERNAME, $(Get-Date -Format 'yyyy-MM-dd HH:mm')"
Ajouter ''
Ajouter '## Machine'
Ajouter ''
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$ram = [math]::Round($os.TotalVisibleMemorySize / 1MB, 1)
Ajouter "- Système : $($os.Caption) $($os.Version)"
Ajouter "- Processeur : $($cpu.Name.Trim()) ($($cpu.NumberOfCores) cœurs, $($cpu.NumberOfLogicalProcessors) threads)"
Ajouter "- Mémoire vive : $ram Go"
foreach ($gpu in Get-CimInstance Win32_VideoController) {
    # AdapterRAM plafonne à 4 Go dans WMI : la VRAM réelle est donnée par les spikes.
    $datePilote = if ($gpu.DriverDate) { $gpu.DriverDate.ToString('yyyy-MM-dd') } else { 'date inconnue' }
    Ajouter "- Carte graphique : $($gpu.Name), pilote $($gpu.DriverVersion) ($datePilote)"
}

# --- 2. Les modèles -------------------------------------------------------------
$ensemble = @('fetch')
if ($Avec14B) { $ensemble += @('--with', 'ministral-3-14b') }
Lancer 'Téléchargement et vérification des modèles' 'xtask.exe' $ensemble

# --- 3. Les spikes --------------------------------------------------------------
$phrase = 'testdata\phrase.wav'
Lancer 'Spike 0.1 — reconnaissance vocale, GPU, Whisper turbo' 'spike-stt.exe' @('--wav', $phrase, '--repeat', '5')
Lancer 'Spike 0.1 — reconnaissance vocale, GPU, Whisper small' 'spike-stt.exe' @('--wav', $phrase, '--repeat', '5', '--model', 'models\files\ggml-small-q5_1.bin')
Lancer 'Spike 0.2 — synthèse vocale, 4 voix, CPU' 'spike-tts.exe' @('--no-play', '--repeat', '3')
Lancer 'Spike 0.3 — intervieweur et juge, Ministral 3 8B, GPU' 'spike-llm.exe' @('--questions', '4')
Lancer 'Spike 0.4 — chaîne complète, réponse lue dans le WAV de test, GPU' 'spike-chaine.exe' @('--reponse-wav', $phrase, '--no-play')
if ($Avec14B) {
    Lancer 'Spike 0.3 — intervieweur et juge, Ministral 3 14B, GPU' 'spike-llm.exe' @('--questions', '4', '--model', 'models\files\Ministral-3-14B-Instruct-2512-Q4_K_M.gguf')
}
if ($Cpu) {
    Lancer 'Spike 0.1 — reconnaissance vocale, CPU, Whisper turbo' 'spike-stt.exe' @('--wav', $phrase, '--repeat', '2', '--cpu')
    Lancer 'Spike 0.3 — intervieweur et juge, CPU' 'spike-llm.exe' @('--cpu')
}

$lignes | Out-File -LiteralPath $rapport -Encoding utf8
Write-Host ''
Write-Host "Rapport écrit dans $rapport"
