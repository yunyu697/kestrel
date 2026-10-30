/**
 * Quick Switch path navigation.
 *
 * Inside a file dialog the box knows where the dialog is standing, so a typed
 * path can be resolved right there and entered directly. The shortcuts are the
 * ones every shell understands (`..`, `\`, `D:`, `~`), and a half-typed name
 * is completed from the folders that actually exist.
 *
 * Kept out of the component so the rules can be tested on their own.
 */

export function trimEnd(p: string): string {
  return p.replace(/[\\/]+$/, "");
}

/**
 * A folder without a trailing separator — except a drive root, where dropping
 * the backslash would turn `C:\` into `C:` and every join below it into
 * nonsense (`C:sub`).
 */
export function normFolder(p: string): string {
  const x = trimEnd(p);
  return /^[A-Za-z]:$/.test(x) ? x + "\\" : x;
}

/** Parent folder. Stops at the drive root — there is nothing above `C:\`. */
export function upOne(p: string): string {
  const t = normFolder(p);
  if (/^[A-Za-z]:[\\/]?$/.test(t)) return normFolder(t);
  const i = Math.max(t.lastIndexOf("\\"), t.lastIndexOf("/"));
  if (i < 0) return t;
  return normFolder(t.slice(0, i));
}

export function joinPath(a: string, b: string): string {
  // Fold forward slashes into backslashes and drop separators on both ends:
  // the result is always shown to the user, and `a/b` inside `C:\Users`
  // should read as `C:\Users\a\b`.
  const y = trimEnd(b.replace(/^[\\/]+/, "")).replace(/\//g, "\\");
  const x = normFolder(a);
  if (!y) return x;
  return x.endsWith("\\") ? x + y : x + "\\" + y;
}

/** Two paths are the same place even if they differ in case or a trailing `\`. */
export function sameFolder(a: string, b: string): boolean {
  return normFolder(a).toLowerCase() === normFolder(b).toLowerCase();
}

export function driveRoot(p: string): string {
  const m = p.match(/^([A-Za-z]:)/);
  return m ? m[1].toUpperCase() + "\\" : "";
}

export function baseName(p: string): string {
  const t = trimEnd(p);
  // A drive root is its own name.
  if (/^[A-Za-z]:$/.test(t)) return t + "\\";
  const i = Math.max(t.lastIndexOf("\\"), t.lastIndexOf("/"));
  return i < 0 ? t : t.slice(i + 1);
}

/**
 * Resolve what was typed into an absolute folder, or `null` when there is not
 * enough context (no known location and no home folder).
 */
export function resolvePath(input: string, folder: string | null, home: string): string | null {
  const s = input.trim();
  // `D:` or `D:\sub` — switching drive needs no context at all, so it is
  // handled before the "do we know where we are" check below.
  const dm = s.match(/^([A-Za-z]:)(.*)$/);
  if (dm) {
    const drive = dm[1].toUpperCase() + "\\";
    return dm[2] ? joinPath(drive, dm[2]) : drive;
  }
  const base = normFolder(folder || home);
  if (!s) return base || null;
  if (!base) return null;

  if (s === "~") return normFolder(home) || base;
  if (s.startsWith("~/") || s.startsWith("~\\")) return joinPath(home || base, s.slice(2));
  if (s === "\\" || s === "/") return driveRoot(base);
  if (s.startsWith("\\") || s.startsWith("/")) {
    return joinPath(driveRoot(base) || base, s.slice(1));
  }
  if (s === ".") return base;
  if (s.startsWith("./") || s.startsWith(".\\")) return joinPath(base, s.slice(2));
  if (s.startsWith("..")) {
    let target = base;
    let rest = s;
    // `..\..` climbs two levels; the loop stops at the first real name.
    while (rest === ".." || rest.startsWith("../") || rest.startsWith("..\\")) {
      target = upOne(target);
      rest = rest.slice(2).replace(/^[\\/]/, "");
    }
    return rest ? joinPath(target, rest) : target;
  }
  return joinPath(base, s);
}

/**
 * Does the typed text read as a path rather than a search term?
 * A separator or one of the shortcut forms is what gives it away.
 */
export function isPathLike(s: string): boolean {
  const v = s.trim();
  if (!v) return false;
  if (/^[\\/~]/.test(v)) return true;
  if (/^[A-Za-z]:/.test(v)) return true;
  if (/^\.\.?($|[\\/])/.test(v)) return true;
  return /[\\/]/.test(v);
}

/** Folders whose name starts with `name`, then ones merely containing it. */
export function matchKids(kids: string[], name: string): string[] {
  const low = name.toLowerCase();
  const starts = kids.filter((k) => baseName(k).toLowerCase().startsWith(low));
  if (starts.length) return starts;
  return kids.filter((k) => baseName(k).toLowerCase().includes(low));
}
