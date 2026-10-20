declare module "virtual:lucide-names" {
  const names: string[];
  export default names;
}

declare module "node:fs" {
  export function readdirSync(path: string): string[];
}
