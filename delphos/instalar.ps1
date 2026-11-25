# Troca o topbar.exe do WinRust instalado pela versao Delphos. -Desfazer volta ao original.
# Precisa de PowerShell como administrador.
param([switch]$Desfazer)
$ErrorActionPreference = 'Stop'
$bar = 'C:\Program Files\topbar\topbar.exe'
$orig = 'C:\Program Files\topbar\topbar.original.exe'
$cfg = "$env:ProgramData\WinRust"
if (-not (Test-Path $orig)) { if ($Desfazer) { throw 'nada para desfazer' }; Copy-Item $bar $orig }
$src = if ($Desfazer) { $orig } else { "$PSScriptRoot\topbar.exe" }
# um exe em uso nao pode ser sobrescrito, mas pode ser renomeado
Rename-Item $bar ('topbar.exe.' + (Get-Date -Format HHmmssfff) + '.old')
Copy-Item $src $bar
if ($Desfazer) {
    if (Test-Path "$cfg\style.bak") { Move-Item "$cfg\style.bak" "$cfg\style.txt" -Force }
    if (Test-Path "$cfg\colors.bak") { Move-Item "$cfg\colors.bak" "$cfg\colors.txt" -Force }
} else {
    # guarda o tema e as cores atuais: as cores personalizadas cobririam as do tema
    if ((Test-Path "$cfg\style.txt") -and -not (Test-Path "$cfg\style.bak")) { Copy-Item "$cfg\style.txt" "$cfg\style.bak" }
    if ((Test-Path "$cfg\colors.txt") -and -not (Test-Path "$cfg\colors.bak")) { Move-Item "$cfg\colors.txt" "$cfg\colors.bak" -Force }
    Set-Content "$cfg\style.txt" 'delphos' -NoNewline -Encoding ascii
}
# Prioridade das tarefas: o padrao do Agendador (7) deixa a barra, e todo app aberto por ela, abaixo do normal
foreach ($n in 'topbar', 'rs-launcher') {
    try {
        $t = Get-ScheduledTask -TaskName $n -ErrorAction Stop
        $t.Settings.Priority = if ($Desfazer) { 7 } else { 4 }
        Set-ScheduledTask -InputObject $t -ErrorAction Stop | Out-Null
    } catch { "aviso: prioridade da tarefa $n nao mudou: $_" }
}
Stop-Process -Name topbar -Force -ErrorAction SilentlyContinue
Start-Sleep 1
schtasks /run /tn topbar | Out-Null
'pronto'
