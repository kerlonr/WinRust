# Compila a barra deste repositorio, roda os testes e deixa delphos\topbar.exe pronto para o instalar.ps1. Nao precisa de
# administrador. Precisa, uma vez: winget install Rustlang.Rustup BrechtSanders.WinLibs.POSIX.UCRT
#                                  rustup default stable-x86_64-pc-windows-gnu
$ErrorActionPreference = 'Stop'
$w = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Directory -Filter 'BrechtSanders.WinLibs*' | Select-Object -First 1
if (-not $w) { throw 'falta o WinLibs (veja o comeco deste arquivo)' }
$env:PATH = "$env:USERPROFILE\.cargo\bin;$($w.FullName)\mingw64\bin;$env:PATH"
Push-Location (Split-Path $PSScriptRoot)
try {
    cargo build --release -p topbar; if ($LASTEXITCODE) { throw 'nao compilou' }
    cargo test --release -q -p topbar -p winrust-common; if ($LASTEXITCODE) { throw 'teste falhou' }
    Copy-Item target\release\topbar.exe "$PSScriptRoot\topbar.exe" -Force
    'pronto: agora rode delphos\instalar.ps1 como administrador'
} finally { Pop-Location }
