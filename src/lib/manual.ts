import { Marked } from "marked";

/** Identifiant d'ancre, comme GitHub : minuscules, ponctuation retirée, espaces → tirets (les liens du sommaire du manuel en dépendent). */
export const slug = (text: string) =>
  text.toLowerCase().replace(/<[^>]+>/g, "").replace(/[^\p{L}\p{N}\s_-]/gu, "").trim().replace(/\s/g, "-");

export interface Section { id: string; level: number; title: string; text: string }

/** Transforme le manuel Markdown en HTML (titres avec ancres) et en liste de sections (sommaire, recherche). Aucun HTML brut n'est interprété. */
export function renderManual(md: string): { html: string; sections: Section[] } {
  const sections: Section[] = [];
  const marked = new Marked({ gfm: true });
  marked.use({
    renderer: {
      heading(text: string, depth: number) {
        const plain = text.replace(/<[^>]+>/g, "").replace(/&#39;/g, "'").replace(/&quot;/g, '"').replace(/&amp;/g, "&");
        const id = slug(plain);
        sections.push({ id, level: depth, title: plain, text: "" });
        return `<h${depth} id="${id}">${text}</h${depth}>`;
      },
      html() { return ""; },          // pas de HTML brut dans le manuel
      link(href: string, _title: string | null | undefined, inner: string) {
        // Ancres internes : navigation dans la page ; liens vers d'autres fichiers du dépôt : simple texte.
        return href.startsWith("#") ? `<a href="${href}">${inner}</a>` : `<span class="manual-ext">${inner}</span>`;
      },
    },
  });
  const html = marked.parse(md) as string;
  // Texte de chaque section (pour la recherche) : on découpe le Markdown sur les titres.
  const parts = md.split(/\n(?=#{1,3} )/);
  for (const p of parts) {
    const first = p.split("\n", 1)[0].replace(/^#+\s*/, "");
    const s = sections.find((x) => x.title === first.replace(/[*`]/g, ""));
    if (s && !s.text) s.text = p.replace(/[#*`|>-]/g, " ").replace(/\s+/g, " ").toLowerCase();
  }
  return { html, sections };
}
