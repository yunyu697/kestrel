import type { JSX } from "react";

const paths: Record<string, JSX.Element> = {
  search: (
    <>
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.5-3.5" />
    </>
  ),
  folder: <path d="M3 7.5A2.5 2.5 0 0 1 5.5 5h3.2c.5 0 1 .2 1.4.6l1.2 1.2c.3.3.7.5 1.2.5h5A2.5 2.5 0 0 1 20 9.8v6.7A2.5 2.5 0 0 1 17.5 19h-12A2.5 2.5 0 0 1 3 16.5z" />,
  file: (
    <>
      <path d="M6 3.5h7l5 5v11a1.5 1.5 0 0 1-1.5 1.5h-10A1.5 1.5 0 0 1 5 19.5v-15A1 1 0 0 1 6 3.5z" />
      <path d="M13 3.5V9h5" />
    </>
  ),
  app: (
    <>
      <rect x="3.5" y="3.5" width="7" height="7" rx="2" />
      <rect x="13.5" y="3.5" width="7" height="7" rx="2" />
      <rect x="3.5" y="13.5" width="7" height="7" rx="2" />
      <rect x="13.5" y="13.5" width="7" height="7" rx="2" />
    </>
  ),
  terminal: (
    <>
      <rect x="3" y="4" width="18" height="16" rx="2.5" />
      <path d="m7 9 3 3-3 3M13 15h4" />
    </>
  ),
  gear: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-1.8-.3 1.6 1.6 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.6 1.6 0 0 0-1-1.5 1.6 1.6 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0 .3-1.8 1.6 1.6 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.6 1.6 0 0 0 1.5-1 1.6 1.6 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 1.8.3H9a1.6 1.6 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.6 1.6 0 0 0 1 1.5 1.6 1.6 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0-.3 1.8V9a1.6 1.6 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.6 1.6 0 0 0-1.5 1z" />
    </>
  ),
  clipboard: (
    <>
      <rect x="8" y="2.5" width="8" height="4" rx="1.5" />
      <path d="M16 4.5h2A2 2 0 0 1 20 6.5v13a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-13a2 2 0 0 1 2-2h2" />
    </>
  ),
  window: (
    <>
      <rect x="3" y="4" width="18" height="16" rx="2.5" />
      <path d="M3 8.5h18" />
    </>
  ),
  calc: (
    <>
      <rect x="4" y="2.5" width="16" height="19" rx="2.5" />
      <path d="M8 7h8M8 12h2M14 12h2M8 16.5h2M14 16.5h2" />
    </>
  ),
  globe: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M3.5 12h17M12 3.5c2.4 2.4 3.6 5.3 3.6 8.5S14.4 18.1 12 20.5c-2.4-2.4-3.6-5.3-3.6-8.5S9.6 5.9 12 3.5z" />
    </>
  ),
  star: <path d="m12 3.6 2.6 5.3 5.9.9-4.3 4.1 1 5.8-5.2-2.7-5.2 2.7 1-5.8L3.5 9.8l5.9-.9z" />,
  clock: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 7.5V12l3 1.8" />
    </>
  ),
  trash: (
    <>
      <path d="M4 6.5h16M9 6.5V4.8A1.3 1.3 0 0 1 10.3 3.5h3.4A1.3 1.3 0 0 1 15 4.8v1.7" />
      <path d="M6.5 6.5 7.5 20a1.5 1.5 0 0 0 1.5 1.4h6a1.5 1.5 0 0 0 1.5-1.4l1-13.5" />
    </>
  ),
  copy: (
    <>
      <rect x="9" y="9" width="11" height="11" rx="2" />
      <path d="M15 9V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h3" />
    </>
  ),
  external: (
    <>
      <path d="M14 4h6v6M20 4l-8.5 8.5" />
      <path d="M18 14v4.5A2.5 2.5 0 0 1 15.5 21h-9A2.5 2.5 0 0 1 4 18.5v-9A2.5 2.5 0 0 1 6.5 7H11" />
    </>
  ),
  info: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 11v5.5M12 8h.01" />
    </>
  ),
  close: <path d="M6 6l12 12M18 6L6 18" />,
  minimize: <path d="M5 12h14" />,
  maximize: <rect x="5.5" y="5.5" width="13" height="13" rx="1.5" />,
  restore: (
    <>
      <rect x="4.5" y="7.5" width="11" height="11" rx="1.5" />
      <path d="M8.5 7.5v-2a1 1 0 0 1 1-1h9a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1h-2" />
    </>
  ),
  pin: <path d="M9 3.5h6l-1 5 3 3v2H7v-2l3-3zM12 13.5v7" />,
  note: (
    <>
      <path d="M5 4.5h14v10l-4.5 5H5z" />
      <path d="M14.5 19.5V15H19" />
    </>
  ),
  palette: (
    <>
      <path d="M12 3.5a8.5 8.5 0 1 0 0 17c1.4 0 2-1 2-2 0-1.4 1-2 2.4-2H18a3.5 3.5 0 0 0 3.5-3.5C21.5 7.4 17.3 3.5 12 3.5z" />
      <circle cx="8" cy="10" r="1.1" fill="currentColor" stroke="none" />
      <circle cx="12" cy="7.5" r="1.1" fill="currentColor" stroke="none" />
      <circle cx="15.8" cy="10" r="1.1" fill="currentColor" stroke="none" />
    </>
  ),
  refresh: (
    <>
      <path d="M20 11a8 8 0 1 0-2.3 6" />
      <path d="M20 5v6h-6" />
    </>
  ),
  check: <path d="m5 12.5 4.5 4.5L19 7.5" />,
  plus: <path d="M12 5v14M5 12h14" />,
  minus: <path d="M5 12h14" />,
  "chevron-right": <path d="m9 5 7 7-7 7" />,
  "chevron-down": <path d="m5 9 7 7 7-7" />,
  "arrow-up": <path d="M12 20V4M5 11l7-7 7 7" />,
  "arrow-down": <path d="M12 4v16M19 13l-7 7-7-7" />,
  "corner-down-left": <path d="M20 5v6a3 3 0 0 1-3 3H5m0 0 4-4m-4 4 4 4" />,
  hash: <path d="M4 9h16M4 15h16M10 3 8 21M16 3l-2 18" />,
  "panel-right": (
    <>
      <rect x="3" y="4.5" width="18" height="15" rx="2.5" />
      <path d="M14.5 4.5v15" />
    </>
  ),
  "panel-bottom": (
    <>
      <rect x="3" y="4.5" width="18" height="15" rx="2.5" />
      <path d="M3 14.5h18" />
    </>
  ),
  eye: (
    <>
      <path d="M2.5 12s3.6-6.5 9.5-6.5S21.5 12 21.5 12s-3.6 6.5-9.5 6.5S2.5 12 2.5 12z" />
      <circle cx="12" cy="12" r="2.6" />
    </>
  ),
  "eye-off": (
    <>
      <path d="M4 4.5 20 20" />
      <path d="M9.6 6.1A9.7 9.7 0 0 1 12 5.5c5.9 0 9.5 6.5 9.5 6.5a17 17 0 0 1-3.3 3.9" />
      <path d="M6.4 8.1A16.6 16.6 0 0 0 2.5 12s3.6 6.5 9.5 6.5c1.2 0 2.3-.3 3.3-.7" />
    </>
  ),
  layers: (
    <>
      <path d="m12 3 9 5-9 5-9-5z" />
      <path d="m3 13 9 5 9-5" />
    </>
  ),
  keyboard: (
    <>
      <rect x="2.5" y="6" width="19" height="12" rx="2.5" />
      <path d="M6 10h.01M9.5 10h.01M13 10h.01M16.5 10h.01M7 14h10" />
    </>
  ),
  sliders: (
    <>
      <path d="M4 8h10M18 8h2M4 16h4M12 16h8" />
      <circle cx="16" cy="8" r="2" />
      <circle cx="10" cy="16" r="2" />
    </>
  ),
  database: (
    <>
      <ellipse cx="12" cy="6" rx="7.5" ry="3" />
      <path d="M4.5 6v12c0 1.7 3.4 3 7.5 3s7.5-1.3 7.5-3V6" />
      <path d="M4.5 12c0 1.7 3.4 3 7.5 3s7.5-1.3 7.5-3" />
    </>
  ),
  code: <path d="m9 8-4 4 4 4M15 8l4 4-4 4" />,
  cpu: (
    <>
      <rect x="6.5" y="6.5" width="11" height="11" rx="2" />
      <path d="M10 2.5v4M14 2.5v4M10 17.5v4M14 17.5v4M2.5 10h4M2.5 14h4M17.5 10h4M17.5 14h4" />
    </>
  ),
  "hard-drive": (
    <>
      <rect x="2.5" y="13" width="19" height="7" rx="2" />
      <path d="M5.5 13 8 4.5h8L18.5 13" />
      <path d="M17.5 16.5h.01" />
    </>
  ),
  server: (
    <>
      <rect x="3" y="4" width="18" height="7" rx="2" />
      <rect x="3" y="13" width="18" height="7" rx="2" />
      <path d="M7 7.5h.01M7 16.5h.01" />
    </>
  ),
  wifi: (
    <>
      <path d="M2.5 8.5a14 14 0 0 1 19 0M5.5 12a9.5 9.5 0 0 1 13 0M8.5 15.5a5 5 0 0 1 7 0" />
      <path d="M12 19h.01" />
    </>
  ),
  monitor: (
    <>
      <rect x="2.5" y="4" width="19" height="12" rx="2" />
      <path d="M8 20h8M12 16v4" />
    </>
  ),
  printer: (
    <>
      <path d="M7 8V4h10v4" />
      <rect x="3.5" y="8" width="17" height="7" rx="2" />
      <path d="M7 12.5h10V20H7z" />
    </>
  ),
  volume: (
    <>
      <path d="M4 9.5h3.5L12 5.5v13L7.5 14.5H4z" />
      <path d="M16 9a4 4 0 0 1 0 6" />
    </>
  ),
  bluetooth: <path d="m7 7 10 10-5 4V3l5 4L7 17" />,
  grid: (
    <>
      <rect x="3.5" y="3.5" width="7" height="7" rx="1.5" />
      <rect x="13.5" y="3.5" width="7" height="7" rx="1.5" />
      <rect x="3.5" y="13.5" width="7" height="7" rx="1.5" />
      <rect x="13.5" y="13.5" width="7" height="7" rx="1.5" />
    </>
  ),
  camera: (
    <>
      <path d="M3 8.5h3l1.5-2h9L18 8.5h3v10H3z" />
      <circle cx="12" cy="13" r="3.2" />
    </>
  ),
  pen: <path d="M4 20h4L20 8l-4-4L4 16z" />,
  lock: (
    <>
      <rect x="4.5" y="10.5" width="15" height="10" rx="2.5" />
      <path d="M8 10.5V7.5a4 4 0 0 1 8 0v3" />
    </>
  ),
  power: (
    <>
      <path d="M12 3v9" />
      <path d="M7 6.5a7.5 7.5 0 1 0 10 0" />
    </>
  ),
  moon: <path d="M20 14.5A8.5 8.5 0 0 1 9.5 4 8.5 8.5 0 1 0 20 14.5z" />,
  user: (
    <>
      <circle cx="12" cy="8.5" r="3.8" />
      <path d="M4.5 20.5a7.5 7.5 0 0 1 15 0" />
    </>
  ),
  activity: <path d="M3 12h4l3-8 4 16 3-8h4" />,
  "file-text": (
    <>
      <path d="M6 3.5h7l5 5v11A1.5 1.5 0 0 1 16.5 21h-10A1.5 1.5 0 0 1 5 19.5v-15A1 1 0 0 1 6 3.5z" />
      <path d="M13 3.5V9h5M8.5 13h7M8.5 16.5h5" />
    </>
  ),
  filter: <path d="M3.5 5h17l-6.5 7.5V20l-4-2.5v-5z" />,
  "zoom-in": (
    <>
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.5-3.5M8.5 11h5M11 8.5v5" />
    </>
  ),
};

export function Icon({
  name,
  size = 18,
  strokeWidth = 1.7,
  className,
  style,
}: {
  name: string;
  size?: number;
  strokeWidth?: number;
  className?: string;
  style?: React.CSSProperties;
}) {
  const body = paths[name] ?? paths.file;
  return (
    <svg
      className={className}
      style={style}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {body}
    </svg>
  );
}
