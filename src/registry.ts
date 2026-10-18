import type { Command, Extension, RegisteredCommand } from "./extension-api";

export async function loadCommands(extensions: Extension[]): Promise<RegisteredCommand[]> {
  const perExtension = await Promise.all(
    extensions.map(async (extension) => {
      let dynamic: Command[] = [];
      try {
        dynamic = (await extension.items?.()) ?? [];
      } catch (error) {
        console.error(`extension ${extension.id} failed to load items`, error);
      }
      return [...(extension.commands ?? []), ...dynamic].map((command) => ({
        ...command,
        icon: command.icon ?? extension.icon,
        key: `${extension.id}/${command.id}`,
        extensionId: extension.id,
      }));
    }),
  );
  return perExtension.flat();
}
