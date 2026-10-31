import { invoke } from "@tauri-apps/api/core";
import type {
  AppEntry,
  BuildInfo,
  ClipboardItem,
  ColorSample,
  FileInfo,
  HttpProbeResult,
  IndexStatus,
  SearchResponse,
  Settings,
  WindowInfo,
  Favorite,
} from "./types";

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args);
}

export const api = {
  search: (query: string, limit?: number) =>
    call<SearchResponse>("search", { query, limit }),
  recordQuery: (query: string) => call<void>("record_query", { query }),
  historyList: () => call<{ query: string; at: number; count: number }[]>("history_list"),
  historyClear: () => call<void>("history_clear"),

  indexStatus: () => call<IndexStatus>("index_status"),
  rebuildIndex: () => call<void>("rebuild_index"),
  cancelIndex: () => call<void>("cancel_index"),
  clearIndex: () => call<void>("clear_index"),

  fileInfo: (path: string) => call<FileInfo>("file_info", { path }),
  iconsFor: (paths: string[], size?: number) =>
    call<Record<string, string>>("icons_for", { paths, size }),
  openPath: (path: string) => call<boolean>("open_path", { path }),
  openUrl: (url: string) => call<boolean>("open_url", { url }),
  revealPath: (path: string) => call<boolean>("reveal_path", { path }),
  openWith: (path: string) => call<boolean>("open_with", { path }),
  runAsAdmin: (path: string) => call<boolean>("run_as_admin", { path }),
  showProperties: (path: string) => call<boolean>("show_properties", { path }),
  openTerminal: (path: string) => call<boolean>("open_terminal", { path }),
  copyFileRefs: (paths: string[]) => call<boolean>("copy_file_refs", { paths }),
  deleteToTrash: (paths: string[]) => call<number>("delete_to_trash", { paths }),
  renamePath: (path: string, newName: string) =>
    call<string>("rename_path", { path, newName }),
  createFolder: (parent: string, name: string) =>
    call<string>("create_folder", { parent, name }),
  bumpUsage: (path: string) => call<void>("bump_usage", { path }),
  browseFolder: (path: string) => call<boolean>("browse_folder", { path }),
  runShell: (command: string, cwd?: string) =>
    call<boolean>("run_shell_command", { command, cwd }),
  spawnProgram: (program: string, args?: string, cwd?: string) =>
    call<boolean>("spawn_program", { program, args, cwd }),

  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<void>("save_settings", { settings }),
  resetSettings: () => call<Settings>("reset_settings"),
  applyHotkey: (accelerator: string) => call<boolean>("apply_hotkey", { accelerator }),
  buildInfo: () => call<BuildInfo>("build_info"),
  /** Network probe on the Rust side; `rangeBytes` asks for a partial body. */
  httpProbe: (url: string, timeoutMs?: number, rangeBytes?: number) =>
    call<HttpProbeResult>("http_probe", { url, timeoutMs, rangeBytes }),
  /** Start an installer download in the background; returns the job id. */
  updateDownload: (url: string, fileName: string) =>
    call<string>("update_download", { url, fileName }),
  updateCancel: (id: string) => call<boolean>("update_cancel", { id }),
  /** Hand the installer to a helper that waits for this process to exit. */
  updateInstall: (path: string) => call<boolean>("update_install", { path }),
  /** Show a downloaded installer in Explorer. */
  updateReveal: (path: string) => call<boolean>("update_reveal", { path }),
  /** "perMachine" (MSI) or "currentUser" (NSIS) — how this copy was installed. */
  installMode: () => call<string>("install_mode"),
  listApps: () => call<AppEntry[]>("list_apps"),
  refreshApps: () => call<AppEntry[]>("refresh_apps"),

  clipboardList: () => call<ClipboardItem[]>("clipboard_list"),
  clipboardCopy: (id: string) => call<boolean>("clipboard_copy", { id }),
  clipboardDelete: (id: string) => call<void>("clipboard_delete", { id }),
  clipboardPin: (id: string, pinned: boolean) =>
    call<void>("clipboard_pin", { id, pinned }),
  clipboardClear: () => call<void>("clipboard_clear"),
  clipboardPushText: (text: string) => call<void>("clipboard_push_text", { text }),

  favoritesAdd: (path: string, name?: string) =>
    call<Favorite[]>("favorites_add", { path, name }),
  favoritesRemove: (id: string) => call<Favorite[]>("favorites_remove", { id }),
  favoritesSetEnabled: (id: string, enabled: boolean) =>
    call<Favorite[]>("favorites_set_enabled", { id, enabled }),
  favoritesSetPath: (id: string, path: string) =>
    call<Favorite[]>("favorites_set_path", { id, path }),

  openSettingsWindow: () => call<void>("open_settings_window"),
  isElevated: () => call<boolean>("is_elevated"),
  setRunAsAdmin: (enabled: boolean) =>
    call<"restarted" | "saved" | "failed">("set_run_as_admin", { enabled }),
  hideLauncher: () => call<void>("hide_launcher_window"),
  showLauncher: () => call<void>("show_launcher_window"),
  setAlwaysOnTop: (onTop: boolean) => call<void>("set_always_on_top", { onTop }),
  quit: () => call<void>("quit_app"),

  listWindows: () => call<WindowInfo[]>("list_windows"),
  activateWindow: (hwnd: number) => call<boolean>("activate_window", { hwnd }),
  closeWindow: (hwnd: number) => call<void>("close_window", { hwnd }),
  minimizeWindow: (hwnd: number) => call<void>("minimize_window", { hwnd }),
  toggleTopmost: (hwnd: number) => call<boolean>("toggle_topmost_window", { hwnd }),
  snapWindow: (hwnd: number, side: string) => call<boolean>("snap_window", { hwnd, side }),
  pickColor: () => call<ColorSample>("pick_color"),
  runBuiltin: (id: string) => call<boolean>("run_builtin", { id }),
  quickSwitchApply: (path: string) => call<boolean>("quick_switch_apply", { path }),
  fileHashes: (path: string) =>
    call<{ md5: string; sha1: string; sha256: string; size: number }>("file_hashes", { path }),
  serviceStatus: () => call<boolean>("service_status"),
  serviceInstall: () => call<boolean>("service_install"),
  serviceUninstall: () => call<boolean>("service_uninstall"),
  clearDialogTarget: () => call<void>("clear_dialog_target"),
  hasDialogTarget: () => call<boolean>("has_dialog_target"),

  qsContext: () =>
    call<{ folder: string | null; home: string; children: string[] }>("qs_context"),
};
