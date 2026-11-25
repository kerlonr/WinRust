# Deixa um PC novo com o WinRust no visual Delphos: instala o WinRust oficial (se faltar) e troca a barra pela Delphos.
#   powershell -ExecutionPolicy Bypass -File delphos\instalar-tudo.ps1             instala (pede administrador sozinho)
#   powershell -ExecutionPolicy Bypass -File delphos\instalar-tudo.ps1 -Verificar  so baixa e confere, nao muda nada
# Voltar a barra original: delphos\instalar.ps1 -Desfazer
param([switch]$Verificar, [string]$Log)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = 'Tls12'

$oficial = 'https://github.com/Lemxy/WinRust/releases/download/v0.5.8/WinRust.exe'
$barra = 'https://github.com/kerlonr/WinRust/releases/download/delphos-barra/topbar.exe'
$componentes = 'rust-search', 'rtaskmgr-win32', 'topbar', 'wiztree', 'photoview', 'rust-settings'
$tmp = "$env:LOCALAPPDATA\winrust-delphos"
$hashes = @{}
Get-Content "$PSScriptRoot\hashes.txt" | Where-Object { $_ -match '^\w{64} ' } | ForEach-Object { $h, $n = $_ -split '\s+', 2; $hashes[$n] = $h }

function Dizer($s) { Write-Host $s; if ($Log) { Add-Content $Log $s -Encoding utf8 } }
function Baixar($url, $nome) {
    $f = "$tmp\$nome"
    if (-not ((Test-Path $f) -and (Get-FileHash $f).Hash -eq $hashes[$nome])) { Invoke-WebRequest $url -OutFile $f -UseBasicParsing }
    if ((Get-FileHash $f).Hash -ne $hashes[$nome]) { throw "$nome nao confere com o hash do repositorio: nao sera usado" }
    Dizer "ok: $nome baixado e conferido"
    $f
}

New-Item -ItemType Directory -Force $tmp | Out-Null
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin -and -not $Verificar) {
    $l = "$tmp\instalar.log"
    Remove-Item $l -ErrorAction SilentlyContinue
    $p = Start-Process powershell -Verb RunAs -PassThru -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`" -Log `"$l`""
    $null = $p.Handle; $p.WaitForExit()
    if (Test-Path $l) { Get-Content $l -Encoding utf8 }
    exit $p.ExitCode
}

try {
    if (-not [Environment]::Is64BitOperatingSystem) { throw 'precisa do Windows 10 ou 11 de 64 bits' }
    $exeBarra = Baixar $barra 'topbar.exe'
    $tem = Test-Path 'C:\Program Files\topbar\topbar.exe'
    if (-not $tem) { $exeWin = Baixar $oficial 'WinRust.exe' }
    if ($Verificar) { Dizer "WinRust instalado: $tem. Nada foi mudado."; exit 0 }

    if (-not $tem) {
        Dizer 'instalando o WinRust (pode demorar alguns minutos)...'
        $saida = "$tmp\winrust-saida.txt"
        $p = Start-Process $exeWin -ArgumentList (@('install') + $componentes + @('--lang', 'pt', '--yes')) -PassThru -RedirectStandardOutput $saida
        $null = $p.Handle; $p.WaitForExit() # nao -Wait: ele esperaria tambem a barra que o instalador abre
        Get-Content $saida -Encoding utf8 | ForEach-Object { Dizer $_ }
        if (-not (Test-Path 'C:\Program Files\topbar\topbar.exe')) { throw "a topbar nao foi instalada (codigo $($p.ExitCode))" }
        if ($p.ExitCode) { Dizer 'aviso: algum componente falhou (veja acima); seguindo com a barra' }
    }

    # a atualizacao automatica do WinRust colocaria a barra original de volta
    $t = Get-ScheduledTask -TaskName 'WinRust-update' -ErrorAction SilentlyContinue
    if ($t) { $t | Disable-ScheduledTask | Out-Null; Dizer 'ok: atualizacao automatica do WinRust desligada' }

    Copy-Item $exeBarra "$PSScriptRoot\topbar.exe" -Force
    & "$PSScriptRoot\instalar.ps1" | ForEach-Object { Dizer $_ }

    # mesmas preferencias da barra, so onde ainda nao ha escolha feita
    $cfg = "$env:ProgramData\WinRust"
    $prefs = @{ 'lang' = 'pt'; 'side' = 'bottom'; 'bar-opacity' = '85'; 'bar-size' = '1'; 'autohide' = '0'; 'align-left' = '0' }
    foreach ($k in $prefs.Keys) { if (-not (Test-Path "$cfg\$k.txt")) { Set-Content "$cfg\$k.txt" $prefs[$k] -NoNewline -Encoding ascii } }

    Start-Sleep 3
    $ok = (Get-FileHash 'C:\Program Files\topbar\topbar.exe').Hash -eq $hashes['topbar.exe']
    $viva = [bool](Get-Process topbar -ErrorAction SilentlyContinue)
    Dizer "barra Delphos no lugar: $ok | rodando: $viva"
    if (-not ($ok -and $viva)) { exit 1 }
    Dizer 'PRONTO'
} catch {
    Dizer "ERRO: $_"
    exit 1
}
