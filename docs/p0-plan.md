# P0 — Prova técnica do desktop

> **Status:** concluída, condicionada à integração aprovada da P0.4 em `develop`.  
> **Leitores:** pessoas que implementam, testam ou acompanham a viabilidade do GIFTray.

## Por que começamos pela P0

O valor do GIFTray depende de dois gestos: **salvar uma reação recebida em outro aplicativo** e **encontrá-la para enviar de volta rapidamente**. Esses gestos parecem simples, mas seus dados podem variar conforme navegador, Discord, sistema operacional e destino da colagem.

Construir contas, banco remoto e landing page antes de conhecer essas limitações aumentaria o risco de retrabalho. A P0 responde às perguntas técnicas com protótipos mínimos e evidências reais, sem custo de infraestrutura.

## Hipóteses a verificar

- É possível abrir e fechar uma busca compacta no Windows 10/11 usando um atalho global configurável, sem quebrar o foco do aplicativo anterior.
- Arrastar ou colar uma mídia fornece informação suficiente para salvá-la como URL durável ou arquivo local, ao menos em cenários relevantes.
- Há pelo menos um fluxo de cópia e colagem de GIF **animado** funcional em destinos prioritários.
- O Tauri 2 permite uma experiência aceitavelmente próxima dos controles do Windows, inclusive seguindo tema do sistema e respeitando acessibilidade.

A P0 não prova que todos os aplicativos são compatíveis; ela define o conjunto real de fluxos suportados e as alternativas quando algo falha.

## Abordagem técnica

**Candidata:** Tauri 2 + React + TypeScript + Vite, com Fluent UI React. O código nativo Rust/Windows API só será adicionado quando um experimento mostrar necessidade real. Usaremos `pnpm workspaces` para manter o aplicativo em `apps/desktop`; nenhum pacote compartilhado é obrigatório nesta etapa.

Dados e mídias de teste devem ser locais. Não haverá login, API, storage de nuvem ou dependência de KLIPY/GIPHY/Discord API.

## Etapas e evidências esperadas

| Experimento | Pergunta que responde | Evidência mínima |
|---|---|---|
| Ambiente e janela | A aplicação inicia nos dois Windows? | Build e execução registrados por versão |
| Atalho global | O usuário pode configurar e recuperar um atalho válido? | Teste de registro, conflitos e persistência |
| Diagnóstico de entrada | Que dados chegam ao arrastar/colar de cada origem? | Tipos recebidos e passos reproduzíveis, sem dados sensíveis |
| Salvamento | Conseguimos guardar URL ou arquivo com fallback correto? | Item importado e recuperável após reinício |
| Compartilhamento | A animação sobrevive à colagem nos destinos? | Registro com formato, destino e resultado observado |
| Aparência e foco | A experiência parece adequada a Windows 10/11? | Testes de tema, DPI, contraste, múltiplos monitores e foco |

A execução deve seguir essa ordem aproximada para manter o risco principal visível. Não é necessário entregar a biblioteca definitiva para conduzir os experimentos.

## Regras de segurança para os protótipos

Entradas importadas não são confiáveis. Inspecionar esquemas de URL, tipo real do arquivo, tamanho e tempo de downloads; não executar conteúdo recebido nem renderizar HTML sem tratamento. Nunca persistir clipboard ou tokens de URLs em logs de diagnóstico. O clipboard só será lido após ação explícita.

## O que deliberadamente não será feito

Biblioteca web (em repositório privado separado), autenticação, API, Cloudflare, sincronização, busca de GIFs externa, importador de favoritos do Discord, IA, anúncios e estrutura genérica de plugins.

## Critérios para avançar (Go/No-go)

A decisão de continuar para a biblioteca local deve se apoiar em: execução e atalho configurável nos dois Windows; pelo menos um fluxo completo de **captura → importação → busca → cópia → colagem animada** comprovado; fallback adequado quando só existe arquivo local; testes automatizados para validações independentes do Windows; e [matriz de evidências](p0-test-matrix.md) preenchida honestamente.

Se o Tauri não atingir requisitos essenciais, o relatório deve dizer se o problema aceita uma implementação nativa pontual ou se justifica reavaliar WinUI 3. A conclusão não deve ser presumida antecipadamente.

## Registro do desenvolvimento

Após o commit inicial da documentação na `main` e a criação de `develop`, o primeiro incremento será realizado na branch `feature/p0-desktop-spike`, derivada de `develop`. As mudanças devem ser divididas em commits convencionais por responsabilidade, com revisão antes de integrar. Essas convenções são explicadas operacionalmente em [`AGENTS.md`](../AGENTS.md); este documento registra o propósito técnico e o critério de validação.

## Encerramento da P0

**Decisão:** a prova técnica P0 está concluída, condicionada ao merge local aprovado da P0.4.  
**Etapas:** P0.1 concluída; P0.2 concluída; P0.3 concluída; P0.4 concluída, condicionada ao merge aprovado.  
**Cenários comprovadamente suportados:** importação e compartilhamento manual registrados na matriz, incluindo GIF/WebP remoto em destinos Discord e GIF remoto no Explorer. Isso não demonstra compatibilidade universal.  
**Riscos remanescentes:** testes específicos em Windows 10 22H2 e Windows 11, compatibilidade parcial do WhatsApp, falhas nativas reais de clipboard não totalmente simuláveis, disponibilidade/políticas de servidores remotos e smoke test final da retenção do cache.  
**Próximo incremento autorizado:** P1 — Biblioteca local, sem início automático.
