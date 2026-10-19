import { invoke } from "@tauri-apps/api/core";
import { marked, type Token, type Tokens } from "marked";
import { useMemo, type ReactNode } from "react";
import { resolveImage } from "./FlowKeyIcon";
import type { InstalledExtension } from "./runtime";

function openLink(href: string) {
  if (/^https?:\/\/[^/]/i.test(href)) invoke("open_url", { url: href }).catch((e) => console.error("open_url failed", e));
}

function inline(tokens: Token[] | undefined, extension: InstalledExtension): ReactNode[] {
  return (tokens ?? []).map((token, i) => {
    switch (token.type) {
      case "strong":
        return <strong key={i}>{inline((token as Tokens.Strong).tokens, extension)}</strong>;
      case "em":
        return <em key={i}>{inline((token as Tokens.Em).tokens, extension)}</em>;
      case "del":
        return <del key={i}>{inline((token as Tokens.Del).tokens, extension)}</del>;
      case "codespan":
        return (
          <code key={i} className="rounded-[4px] bg-control px-1 font-mono text-12">
            {(token as Tokens.Codespan).text}
          </code>
        );
      case "br":
        return <br key={i} />;
      case "link": {
        const link = token as Tokens.Link;
        return (
          <a
            key={i}
            href={link.href}
            title={link.title ?? undefined}
            onClick={(e) => {
              e.preventDefault();
              openLink(link.href);
            }}
            className="text-accent underline-offset-2 hover:underline"
          >
            {inline(link.tokens, extension)}
          </a>
        );
      }
      case "image": {
        const image = token as Tokens.Image;
        const src = resolveImage(image.href, extension);
        return src ? <img key={i} src={src} alt={image.text} title={image.title ?? undefined} className="inline-block max-h-[260px] max-w-full rounded-[6px] object-contain" /> : <span key={i}>{image.text}</span>;
      }
      case "text": {
        const text = token as Tokens.Text;
        return text.tokens ? <span key={i}>{inline(text.tokens, extension)}</span> : <span key={i}>{text.text.replace(/&amp;/g, "&").replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&#39;/g, "'")}</span>;
      }
      case "escape":
        return <span key={i}>{(token as Tokens.Escape).text}</span>;
      default:
        return <span key={i}>{"text" in token ? String(token.text) : token.raw}</span>;
    }
  });
}

const HEADINGS = ["text-[22px]", "text-[19px]", "text-[16px]", "text-[15px]", "text-[14px]", "text-13"];

function blocks(tokens: Token[], extension: InstalledExtension): ReactNode[] {
  return tokens.map((token, i) => {
    switch (token.type) {
      case "heading": {
        const heading = token as Tokens.Heading;
        const Tag = `h${Math.min(6, heading.depth)}` as "h1";
        return (
          <Tag key={i} className={`font-semibold text-fg ${HEADINGS[heading.depth - 1] ?? "text-13"}`}>
            {inline(heading.tokens, extension)}
          </Tag>
        );
      }
      case "paragraph":
        return (
          <p key={i} className="leading-[1.55]">
            {inline((token as Tokens.Paragraph).tokens, extension)}
          </p>
        );
      case "code":
        return (
          <pre key={i} className="overflow-x-auto rounded-[6px] bg-control p-3 font-mono text-12 leading-[1.5]">
            <code>{(token as Tokens.Code).text}</code>
          </pre>
        );
      case "blockquote":
        return (
          <blockquote key={i} className="flex flex-col gap-2 border-l-2 border-separator pl-3 text-fg-60">
            {blocks((token as Tokens.Blockquote).tokens, extension)}
          </blockquote>
        );
      case "list": {
        const list = token as Tokens.List;
        const Tag = list.ordered ? "ol" : "ul";
        return (
          <Tag key={i} start={list.ordered && list.start !== "" ? Number(list.start) : undefined} className={`flex flex-col gap-1 pl-5 ${list.ordered ? "list-decimal" : "list-disc"}`}>
            {list.items.map((item, j) => (
              <li key={j}>
                {item.task && <input type="checkbox" checked={!!item.checked} readOnly disabled className="mr-1.5 align-middle" />}
                {item.tokens.some((t) => t.type !== "text") ? blocks(item.tokens, extension) : inline(item.tokens, extension)}
              </li>
            ))}
          </Tag>
        );
      }
      case "table": {
        const table = token as Tokens.Table;
        return (
          <table key={i} className="w-full border-collapse text-12">
            <thead>
              <tr>
                {table.header.map((cell, j) => (
                  <th key={j} className="border-b border-separator px-2 py-1 text-left font-semibold">
                    {inline(cell.tokens, extension)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {table.rows.map((row, j) => (
                <tr key={j}>
                  {row.map((cell, k) => (
                    <td key={k} className="border-b border-separator px-2 py-1">
                      {inline(cell.tokens, extension)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        );
      }
      case "hr":
        return <hr key={i} className="border-separator" />;
      case "space":
        return null;
      case "html":
        return (
          <p key={i} className="whitespace-pre-wrap">
            {(token as Tokens.HTML).text}
          </p>
        );
      default:
        return (
          <p key={i} className="whitespace-pre-wrap">
            {"tokens" in token && Array.isArray(token.tokens) ? inline(token.tokens as Token[], extension) : token.raw}
          </p>
        );
    }
  });
}

export function Markdown({ source, extension }: { source: string; extension: InstalledExtension }) {
  const tokens = useMemo(() => marked.lexer(source, { gfm: true, breaks: false }), [source]);
  return <div className="flex flex-col gap-3 text-13 text-fg select-text">{blocks(tokens, extension)}</div>;
}
