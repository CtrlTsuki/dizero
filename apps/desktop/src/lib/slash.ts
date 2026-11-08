export interface SlashCommand {
  pluginId: string;
  pluginName: string;
  name: string;
  description: string;
  usage: string | null;
}

const COMMAND = /^\/([a-z][a-z0-9-]{0,31})(?:\s+([\s\S]*))?$/;

/** `/name args…` → `{ name, args }`, or null if the text is not a command. */
export function parseSlash(text: string): { name: string; args: string } | null {
  const m = COMMAND.exec(text.trim());
  if (!m || !m[1]) return null;
  return { name: m[1], args: (m[2] ?? "").trim() };
}

/** Suggestions while the user is still typing the command name. */
export function matchCommands(text: string, commands: SlashCommand[]): SlashCommand[] {
  if (!text.startsWith("/") || /\s/.test(text)) return [];
  const prefix = text.slice(1).toLowerCase();
  return commands.filter((c) => c.name.startsWith(prefix));
}
