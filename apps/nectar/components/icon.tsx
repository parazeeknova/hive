const paths = {
  activity: "M3 12h4l3-8 4 16 3-8h4",
  arrow: "M5 12h14m-6-6 6 6-6 6",
  bolt: "m13 2-9 12h7l-1 8 10-13h-7l1-7Z",
  book: "M12 5v16m0-16C9 3 5 3 2 4v15c3-1 7-1 10 2 3-3 7-3 10-2V4c-3-1-7-1-10 1Z",
  box: "m12 3 9 5v9l-9 5-9-5V8l9-5ZM3 8l9 5 9-5m-9 5v9M8 5l9 5",
  check: "m5 12 4 4L19 6",
  chevron: "m9 5 7 7-7 7",
  clock: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9-5v5l3 2",
  close: "m6 6 12 12M6 18 18 6",
  code: "m8 7-5 5 5 5m8-10 5 5-5 5m-3-13-2 20",
  compose: "M5 5h5v5H5zM14 14h5v5h-5zM10 7h6v7M7 10v6h7",
  copy: "M9 9h12v12H9zM15 9V3H3v12h6",
  down: "m6 9 6 6 6-6",
  external: "M14 3h7v7m0-7L10 14M9 3H3v18h18v-6",
  file: "M14 2H5v20h14V7l-5-5Zm0 0v6h5M8 12h8m-8 4h6",
  git: "M9 18c-5 2-5-2-7-2m14 6v-4c0-1-.3-2-1-2 4 0 7-2 7-6 0-2-1-3-2-4 0-1 0-3-1-4-2 0-4 2-4 2-3-1-6-1-8 0 0 0-2-2-4-2-1 1-1 3-1 4-1 1-2 2-2 4 0 4 3 6 7 6-.7 0-1 1-1 2v4",
  globe:
    "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM3 12h18M12 3c5 5 5 13 0 18-5-5-5-13 0-18Z",
  help: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM9 9a3 3 0 1 1 4 3c-1 0-1 1-1 2m0 3h.01",
  layers: "m12 3 10 5-10 5L2 8l10-5ZM2 12l10 5 10-5M2 16l10 5 10-5",
  logout: "M9 3H3v18h6m5-15 6 6-6 6m-6-6h12",
  menu: "M4 7h16M4 12h16M4 17h16",
  play: "m9 5 11 7-11 7V5Z",
  plus: "M12 5v14M5 12h14",
  search: "m21 21-5-5M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0",
  settings:
    "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8Zm-2-5h4l1 3 3-1 2 3-2 3 2 3-2 3-3-1-1 3h-4l-1-3-3 1-2-3 2-3-2-3 2-3 3 1 1-3Z",
  shield: "m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6l8-3Zm-4 9 3 3 5-6",
  wallet: "M20 8V5H5a2 2 0 0 0 0 4h16v11H5a2 2 0 0 1-2-2V7m18 6h-5v3h5",
} as const;

export type IconName = keyof typeof paths;

export const Icon = ({
  name,
  size = 20,
  className,
}: {
  name: IconName;
  size?: number;
  className?: string;
}) => (
  <svg
    aria-hidden="true"
    width={size}
    height={size}
    className={className}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.65"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d={paths[name]} />
  </svg>
);
