// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { useMemo, useState, type RefObject } from "react";
import { Button, Input, Text, makeStyles } from "@fluentui/react-components";

const samples = [
  { title: "Aprovação entusiasmada", tags: "aprovação, festa" },
  { title: "Pensando no assunto", tags: "reação, dúvida" },
  { title: "Vitória discreta", tags: "comemoração, sucesso" },
];

const useStyles = makeStyles({
  search: { width: "100%" },
  list: { display: "grid", gap: "6px", marginTop: "12px" },
  result: {
    textAlign: "left",
    justifyContent: "flex-start",
    minHeight: "52px",
  },
});

interface GifSearchProps {
  inputRef: RefObject<HTMLInputElement | null>;
}

export function GifSearch({ inputRef }: GifSearchProps) {
  const styles = useStyles();
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const filtered = useMemo(
    () =>
      samples.filter((item) =>
        `${item.title} ${item.tags}`
          .toLowerCase()
          .includes(query.toLowerCase()),
      ),
    [query],
  );

  return (
    <>
      <Input
        ref={inputRef}
        className={styles.search}
        size="large"
        placeholder="Pesquisar GIFs"
        value={query}
        onChange={(_, data) => {
          setQuery(data.value);
          setSelected(0);
        }}
        onKeyDown={(event) => {
          if (event.key === "ArrowDown") {
            event.preventDefault();
            setSelected((value) =>
              Math.min(value + 1, Math.max(filtered.length - 1, 0)),
            );
          }
          if (event.key === "ArrowUp") {
            event.preventDefault();
            setSelected((value) => Math.max(value - 1, 0));
          }
        }}
        aria-label="Pesquisar GIFs"
      />
      <div
        className={styles.list}
        role="listbox"
        aria-label="Resultados da pesquisa"
      >
        {filtered.map((item, index) => (
          <Button
            key={item.title}
            className={styles.result}
            appearance={index === selected ? "secondary" : "subtle"}
            onClick={() => setSelected(index)}
          >
            <span>
              <strong>{item.title}</strong>
              <br />
              <Text size={200}>{item.tags}</Text>
            </span>
          </Button>
        ))}
        {filtered.length === 0 && (
          <Text>Nenhum GIF de exemplo encontrado.</Text>
        )}
      </div>
    </>
  );
}
