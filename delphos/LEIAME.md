# WinRust Delphos

A barra (`plugins/topbar`) no visual Delphos, sobre o WinRust oficial v0.5.8.

| Arquivo | O que faz |
|---|---|
| `instalar-tudo.ps1` | PC novo: instala o WinRust oficial, se faltar, e troca a barra. `-Verificar` só baixa e confere |
| `instalar.ps1` | Troca só a barra (administrador). `-Desfazer` volta à original |
| `compilar.ps1` | Compila, testa e gera o `topbar.exe` daqui |
| `hashes.txt` | SHA-256 do que o `instalar-tudo.ps1` baixa |

- A barra pronta fica na release `delphos-barra` deste fork. Ao publicar outra, atualize `hashes.txt` no mesmo commit.
- A tarefa `WinRust-update` fica desligada: ela colocaria a barra original de volta.
- Voltar a barra: `instalar.ps1 -Desfazer`. Tirar tudo: `WinRust.exe` → **Remover tudo**.
- Prévia sem instalar: `topbar --preview bar|start|tray|quick|wifi|cal|menu|thumbs`, com `WINRUST_STYLE=delphos`.
  `TOPBAR_NET=sinal,internet,conectando` simula a rede (ex.: `-1,0,0` desconectado).
