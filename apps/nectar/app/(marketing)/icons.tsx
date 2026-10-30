interface IconProps {
  className?: string;
}

export const ArrowRight = ({ className }: IconProps) => (
  <svg
    aria-hidden="true"
    className={className}
    fill="none"
    stroke="currentColor"
    strokeLinecap="round"
    strokeLinejoin="round"
    strokeWidth={1.75}
    viewBox="0 0 24 24"
  >
    <path d="M5 12h14M13 6l6 6-6 6" />
  </svg>
);

export const PlayCircle = ({ className }: IconProps) => (
  <svg
    aria-hidden="true"
    className={className}
    fill="none"
    stroke="currentColor"
    strokeLinecap="round"
    strokeLinejoin="round"
    strokeWidth={1.75}
    viewBox="0 0 24 24"
  >
    <circle cx="12" cy="12" r="9" />
    <path d="M10 8.8v6.4l5.2-3.2z" fill="currentColor" stroke="none" />
  </svg>
);

export const Sparkle = ({ className }: IconProps) => (
  <svg
    aria-hidden="true"
    className={className}
    fill="currentColor"
    viewBox="0 0 24 24"
  >
    <path d="M12 2c.6 5.2 4.8 9.4 10 10-5.2.6-9.4 4.8-10 10-.6-5.2-4.8-9.4-10-10C7.2 11.4 11.4 7.2 12 2z" />
  </svg>
);

export const Hexagon = ({ className }: IconProps) => (
  <svg aria-hidden="true" className={className} fill="none" viewBox="0 0 24 24">
    <path
      d="M12 2.8 19.9 7.4v9.2L12 21.2l-7.9-4.6V7.4z"
      stroke="currentColor"
      strokeLinejoin="round"
      strokeWidth={1.75}
    />
  </svg>
);

export const Bolt = ({ className }: IconProps) => (
  <svg aria-hidden="true" className={className} fill="none" viewBox="0 0 24 24">
    <path
      d="M13.5 2.5 5 13.5h5.5L10 21.5 19 10.5h-5.5z"
      stroke="currentColor"
      strokeLinejoin="round"
      strokeWidth={1.75}
    />
  </svg>
);

export const ShieldCheck = ({ className }: IconProps) => (
  <svg
    aria-hidden="true"
    className={className}
    fill="none"
    stroke="currentColor"
    strokeLinecap="round"
    strokeLinejoin="round"
    strokeWidth={1.75}
    viewBox="0 0 24 24"
  >
    <path d="M12 2.8 19.5 6v6c0 4.6-3.2 7.6-7.5 9.2C7.7 19.6 4.5 16.6 4.5 12V6z" />
    <path d="m8.8 11.9 2.2 2.2 4.2-4.3" />
  </svg>
);

export const Menu = ({ className }: IconProps) => (
  <svg
    aria-hidden="true"
    className={className}
    fill="none"
    stroke="currentColor"
    strokeLinecap="round"
    strokeWidth={1.75}
    viewBox="0 0 24 24"
  >
    <path d="M4 8h16M4 16h16" />
  </svg>
);
