# WinRust Delphos

A barra (`plugins/topbar`) no visual Delphos, sobre o WinRust oficial v0.5.8.

## Instalar num PC do zero

Precisa de Windows 10 ou 11 de 64 bits, internet e uma conta de administrador. Não precisa de Rust nem de Git.

1. Baixe o repositório: <https://github.com/kerlonr/WinRust/archive/refs/heads/main.zip>.
2. Extraia o zip, por exemplo em `C:\WinRust-main`.
3. Abra o PowerShell (normal, sem administrador) e entre na pasta:
   ```powershell
   cd C:\WinRust-main
   ```
4. Opcional: confira os downloads sem mudar nada no PC:
   ```powershell
   powershell -ExecutionPolicy Bypass -File delphos\instalar-tudo.ps1 -Verificar
   ```
5. Instale:
   ```powershell
   powershell -ExecutionPolicy Bypass -File delphos\instalar-tudo.ps1
   ```
6. Aceite o pedido de administrador (UAC). O script baixa o WinRust oficial e a barra Delphos, confere os dois pelo
   `hashes.txt`, instala o WinRust se faltar e troca a barra. A primeira instalação leva alguns minutos.
7. Espere a linha `PRONTO`. A barra Delphos aparece no lugar do taskbar.

Se aparecer `ERRO: ... nao confere com o hash`, o arquivo da release não é o deste commit: baixe o zip de novo.
Se o PC já tiver o WinRust, o passo 6 só troca a barra.

| Arquivo | O que faz |
|---|---|
| `instalar-tudo.ps1` | PC novo: instala o WinRust oficial, se faltar, e troca a barra. `-Verificar` só baixa e confere |
| `instalar.ps1` | Troca só a barra (administrador). `-Desfazer` volta à original |
| `compilar.ps1` | Compila, testa e gera o `topbar.exe` daqui |
| `hashes.txt` | SHA-256 do que o `instalar-tudo.ps1` baixa |

- A barra pronta fica na release `delphos-barra` deste fork. Ao publicar outra, atualize `hashes.txt` no mesmo commit.
- A tarefa `WinRust-update` fica desligada: ela colocaria a barra original de volta.
- Voltar a barra: `instalar.ps1 -Desfazer`. Tirar tudo: `WinRust.exe` → **Remover tudo**.
- Prévia sem instalar: `topbar --preview bar|start|tray|quick|mixer|wifi|cal|menu|thumbs`, com `WINRUST_STYLE=delphos`.
  `TOPBAR_NET=sinal,internet,conectando` simula a rede (ex.: `-1,0,0` desconectado).
