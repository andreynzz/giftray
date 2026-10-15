# Integração contínua

O workflow [CI](../.github/workflows/ci.yml) protege as branches de integração do aplicativo desktop. Ele executa em `windows-latest` quando há push para `develop` ou `main`, ou pull request direcionado a uma dessas branches. Execuções antigas da mesma branch ou pull request são canceladas quando uma nova execução é iniciada.

## Verificações obrigatórias

O job **Frontend checks (Windows)** instala Node 24.16.0 e pnpm 11.9.0, restaura o cache do store do pnpm a partir de `pnpm-lock.yaml`, instala dependências com lockfile congelado e executa:

- `pnpm check`
- `pnpm test`
- `pnpm build`

O job **Rust checks (Windows)** instala Rust 1.98.0, restaura o cache de Cargo do workspace `apps/desktop/src-tauri` e executa nesse diretório:

- `cargo fmt --check`
- `cargo test`
- `cargo check`

Os jobs possuem apenas `contents: read`. O workflow não usa secrets, não publica artefatos e não cria releases ou deploys.

## Reprodução local

Na raiz do repositório, instale as dependências e execute as verificações de frontend:

```powershell
pnpm install --frozen-lockfile
pnpm check
pnpm test
pnpm build
```

No Windows, se a política de execução bloquear `pnpm.ps1`, use `pnpm.cmd` nos mesmos comandos. Para Rust:

```powershell
Set-Location apps/desktop/src-tauri
cargo fmt --check
cargo test
cargo check
```

## Investigação de falhas

Consulte primeiro o log da etapa que falhou na aba **Actions** do pull request ou do commit. Reproduza o comando correspondente localmente com as mesmas versões de Node, pnpm e Rust. O cache acelera a instalação, mas não é uma entrada confiável: execute novamente sem cache ou reexecute o workflow se uma falha envolver restauração de dependências.

O runner Windows compila o código nativo, mas não comprova interação real com clipboard, CF_HDROP, aplicativos de destino, WebView2 ou as edições específicas do Windows. Esses cenários continuam exigindo smoke tests manuais.
