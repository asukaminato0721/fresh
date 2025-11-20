/// <reference path="../types/fresh.d.ts" />

/**
 * Clangd helper plugin
 *
 * Provides two commands:
 *  - Switch Source/Header (uses clangd/textDocument/switchSourceHeader)
 *  - Open project .clangd configuration file
 */

const languageMap: Record<string, string> = {
  c: "cpp",
  h: "cpp",
  hp: "cpp",
  hpp: "cpp",
  hxx: "cpp",
  hh: "cpp",
  cpp: "cpp",
  cxx: "cpp",
  cc: "cpp",
  objc: "cpp",
  mm: "cpp",
};

function detectLanguage(path: string): string | null {
  const segments = path.split(".");
  if (segments.length === 1) {
    return null;
  }
  const ext = segments[segments.length - 1].toLowerCase();
  return languageMap[ext] ?? null;
}

function pathToFileUri(path: string): string {
  let normalized = path.replace(/\\/g, "/");
  if (!normalized.startsWith("/")) {
    normalized = "/" + normalized;
  }
  return "file://" + encodeURI(normalized);
}

function fileUriToPath(uri: string): string {
  if (!uri.startsWith("file://")) {
    return uri;
  }
  let path = decodeURI(uri.substring("file://".length));
  if (path.startsWith("/") && path.length > 2 && path[2] === ":") {
    path = path.substring(1);
  }
  return path;
}

globalThis.clangdSwitchSourceHeader = async function(): Promise<void> {
  const bufferId = editor.getActiveBufferId();
  const path = editor.getBufferPath(bufferId);
  if (!path) {
    editor.setStatus("Clangd: there is no active file to switch");
    return;
  }

  const language = detectLanguage(path);
  if (!language) {
    editor.setStatus("Clangd: unsupported file type for switch header");
    return;
  }

  const uri = pathToFileUri(path);
  try {
    const result = await editor.sendLspRequest(language, "textDocument/switchSourceHeader", {
      textDocument: { uri },
    });
    if (typeof result === "string" && result.length > 0) {
      const targetPath = fileUriToPath(result);
      editor.openFile(targetPath, 0, 0);
      editor.setStatus("Clangd: opened corresponding file");
      return;
    }
    editor.setStatus("Clangd: no matching header/source found");
  } catch (err) {
    editor.setStatus(`Clangd switch source/header failed: ${err}`);
    editor.debug(`clangdSwitchSourceHeader error: ${err}`);
  }
};

globalThis.clangdOpenProjectConfig = function(): void {
  const bufferId = editor.getActiveBufferId();
  const targets = new Set<string>();
  const bufferPath = editor.getBufferPath(bufferId);
  if (bufferPath) {
    const dir = editor.pathDirname(bufferPath);
    targets.add(dir);
  }
  const cwd = editor.getCwd();
  if (cwd) {
    targets.add(cwd);
  }

  let opened = false;
  for (const dir of Array.from(targets)) {
    const configPath = editor.pathJoin(dir, ".clangd");
    if (editor.fileExists(configPath)) {
      editor.openFile(configPath, 0, 0);
      editor.setStatus("Opened .clangd configuration");
      opened = true;
      break;
    }
  }

  if (!opened) {
    editor.setStatus("Could not find .clangd configuration in workspace");
  }
};

editor.registerCommand(
  "Clangd: Switch Source/Header",
  "Jump to header/source pair using clangd",
  "clangdSwitchSourceHeader",
  "normal"
);

editor.registerCommand(
  "Clangd: Open Project Config",
  "Open the nearest .clangd file",
  "clangdOpenProjectConfig",
  "normal"
);

editor.setStatus("Clangd support plugin loaded (switch header + config commands)");
