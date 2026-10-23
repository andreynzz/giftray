// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

function errorDetail(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Maps the Rust boundary's already-sanitized errors to actionable UI copy. */
export function importFailureMessage(error: unknown): string {
  const detail = errorDetail(error);

  if (detail.includes("biblioteca está em uso")) {
    return "A biblioteca está em uso por outra instância. Feche a outra instância e tente novamente.";
  }

  if (detail.includes("abrir o arquivo selecionado")) {
    return "Não foi possível ler o arquivo. Verifique se ainda está disponível e tente novamente.";
  }

  if (detail.includes("formato GIF, WebP ou PNG confirmado")) {
    return "Não foi possível importar. Escolha um GIF, WebP ou PNG válido de até 25 MB.";
  }

  if (detail.includes("excede o limite de 25 MB")) {
    return "Não foi possível importar. Escolha um arquivo de até 25 MB.";
  }

  return "Não foi possível concluir a importação. Tente novamente.";
}

export function copyFailureMessage(error: unknown): string {
  const detail = errorDetail(error);
  if (detail.includes("não está mais disponível") || detail.includes("não foi encontrado")) {
    return "Esse arquivo não está disponível na biblioteca. Atualize a lista e tente novamente.";
  }
  if (detail.includes("biblioteca está em uso")) {
    return "A biblioteca está em uso por outra instância. Feche a outra instância e tente novamente.";
  }
  if (detail.includes("excede o limite de 25 MB")) {
    return "Não foi possível copiar: o arquivo excede o limite de 25 MB.";
  }
  return "Não foi possível copiar o arquivo. Tente novamente.";
}

/** Maps rename failures without leaking storage or database details. */
export function renameFailureMessage(error: unknown): string {
  const detail = errorDetail(error);

  if (detail.includes("nome informado não é válido")) {
    return "O nome não é válido. Use até 120 caracteres sem caracteres de controle.";
  }
  if (detail.includes("não foi encontrado")) {
    return "Esse arquivo não está mais na biblioteca. Atualize a lista e tente novamente.";
  }
  if (detail.includes("biblioteca está em uso")) {
    return "A biblioteca está em uso por outra instância. Feche a outra instância e tente novamente.";
  }

  return "Não foi possível renomear o arquivo. Tente novamente.";
}

/** Maps deletion failures without leaking storage or database details. */
export function deleteFailureMessage(error: unknown): string {
  const detail = errorDetail(error);
  if (detail.includes("biblioteca está em uso")) {
    return "A biblioteca está em uso por outra instância. Feche a outra instância e tente novamente.";
  }
  if (detail.includes("não foi encontrado")) {
    return "Esse arquivo não está mais na biblioteca. Atualize a lista e tente novamente.";
  }
  return "Não foi possível excluir o arquivo. Tente novamente.";
}
