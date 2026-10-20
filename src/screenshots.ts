export interface Screenshot {
  path: string;
  name: string;
  modified: number;
  size: number;
  video: boolean;
}

export function screenshotSections(shots: Screenshot[], now = new Date()): { title: string; items: Screenshot[] }[] {
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const day = 86_400_000;
  const title = (time: number) => {
    if (time >= today) return "Today";
    if (time >= today - day) return "Yesterday";
    if (time >= today - 6 * day) return "Previous 7 Days";
    return new Date(time).toLocaleDateString("en-US", { month: "long", year: "numeric" });
  };
  const sections: { title: string; items: Screenshot[] }[] = [];
  for (const shot of [...shots].sort((a, b) => b.modified - a.modified)) {
    const name = title(shot.modified);
    const last = sections.at(-1);
    if (last?.title === name) last.items.push(shot);
    else sections.push({ title: name, items: [shot] });
  }
  return sections;
}
