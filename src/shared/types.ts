export type HitKind =
  | "file"
  | "folder"
  | "app"
  | "command"
  | "calc"
  | "web"
  | "favorite"
  | "clipboard"
  | "window";

export interface SearchHit {
  id: string;
  kind: HitKind;
  title: string;
  subtitle: string;
  path?: string | null;
  icon?: string | null;
  score: number;
  badge?: string | null;
  meta?: Record<string, unknown> | null;
}

export interface SearchResponse {
  hits: SearchHit[];
  tookMs: number;
  totalMatched: number;
  query: string;
  hint?: string | null;
}

export interface Favorite {
  id: string;
  name: string;
  path: string;
  isDir: boolean;
  /** Disabled favorites are hidden from the launcher results but stay listed. */
  enabled: boolean;
  /** Built-in folders: can be disabled and re-pointed, but never removed. */
  pinned: boolean;
}

export interface WebEngine {
  id: string;
  name: string;
  keyword: string;
  url: string;
  enabled: boolean;
}

export interface CustomCommand {
  id: string;
  name: string;
  keyword: string;
  action: "url" | "path" | "shell" | "program";
  target: string;
  args: string;
  description: string;
  enabled: boolean;
}

export interface Settings {
  launchAtStartup: boolean;
  startMinimized: boolean;
  closeAfterOpen: boolean;
  clearQueryOnHide: boolean;
  rememberLastQuery: boolean;
  singleClickOpen: boolean;
  runAsAdmin: boolean;
  language: string;

  /** Update check frequency: "off" | "daily" | "weekly". */
  updateCheck: string;
  /** Also check right away on every launch. */
  startupCheck: boolean;
  /** Download mirror prefix; "" = GitHub direct. */
  updateMirror: string;

  theme: "light" | "dark" | "system";
  accent: string;
  /** "" = theme default, a scheme id, or "custom" for the per-slot values. */
  colorScheme: string;
  colorBg: string;
  colorSurface: string;
  colorSurface2: string;
  colorText: string;
  colorMuted: string;
  colorFaint: string;
  colorBorder: string;
  colorHover: string;
  colorSunken: string;
  colorSurface3: string;
  colorActive: string;
  colorKbd: string;
  opacity: number;
  blur: boolean;
  cornerRadius: number;
  fontScale: number;
  animations: boolean;
  compactMode: boolean;
  showPreview: boolean;
  previewPosition: string;

  hotkeyToggle: string;
  doubleCtrl: boolean;
  doubleCtrlMs: number;
  hotkeyQuickSwitch: string;
  quickSwitchEnabled: boolean;
  qsAutoShow: boolean;
  qsAutoJump: boolean;
  qsVisibleRows: number;
  replaceWinSearch: boolean;

  maxResults: number;
  indexedDrives: string[];
  excludeDirs: string[];
  indexHiddenFiles: boolean;
  matchPath: boolean;
  pinyinSearch: boolean;
  fuzzySearch: boolean;
  searchApps: boolean;
  searchCommands: boolean;
  searchFileContent: boolean;
  historyEnabled: boolean;
  historyLimit: number;

  clipboardEnabled: boolean;
  clipboardLimit: number;
  clipboardImages: boolean;
  calculatorEnabled: boolean;
  windowSwitcherEnabled: boolean;
  quickNotes: string;

  favorites: Favorite[];
  webEngines: WebEngine[];
  customCommands: CustomCommand[];
}

export interface BuildInfo {
  name: string;
  version: string;
  build: number;
  builtAt: string;
  arch: string;
  os: string;
  indexCount: number;
  indexRoots: string[];
}

export interface IndexStatus {
  state: "idle" | "scanning" | "ready";
  count: number;
  roots: string[];
  scanned: number;
  current: string;
  lastBuilt: number;
}

export interface FileInfo {
  name: string;
  path: string;
  dir: string;
  ext: string;
  isDir: boolean;
  size: number;
  modified: number;
  created: number;
  typeName: string;
  readonly: boolean;
  hidden: boolean;
  exists: boolean;
  itemCount?: number | null;
  thumb?: string | null;
}

export interface ClipboardItem {
  id: string;
  kind: "text" | "image";
  text: string;
  image?: string | null;
  at: number;
  pinned: boolean;
  source: string;
}

export interface WindowInfo {
  hwnd: number;
  title: string;
  class: string;
  pid: number;
  process: string;
  processPath: string;
}

export interface AppEntry {
  id: string;
  name: string;
  target: string;
  lnk: string;
  icon?: string | null;
  source: string;
}

export interface ColorSample {
  hex: string;
  r: number;
  g: number;
  b: number;
  x: number;
  y: number;
}
