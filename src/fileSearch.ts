export interface FileHit {
  path: string;
  name: string;
  dir: boolean;
}

export interface FileResults {
  home: string;
  firstIndex: boolean;
  building: boolean;
  files: FileHit[];
}

export interface FileInfo {
  typeName: string;
  dir: boolean;
  size: number | null;
  created: number | null;
  modified: number | null;
  accessed: number | null;
}

export function parentFolder(path: string): string {
  const at = path.lastIndexOf("\\");
  return at > 0 ? path.slice(0, at) : path;
}

export function shortLocation(path: string, home: string): string {
  const parent = parentFolder(path);
  if (!home) return parent;
  if (parent.toLowerCase() === home.toLowerCase()) return "~";
  return parent.toLowerCase().startsWith(`${home.toLowerCase()}\\`) ? `~${parent.slice(home.length)}` : parent;
}
