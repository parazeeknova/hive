/** A ring spinner that inherits the surrounding text color. */
export const Spinner = ({
  size = 16,
  className = "",
  label,
}: {
  size?: number;
  className?: string;
  /** Announced to screen readers; omit when nearby text already says what is loading. */
  label?: string;
}) => (
  <svg
    className={`shrink-0 animate-spin motion-reduce:animate-[spin_1.6s_linear_infinite] ${className}`}
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    role={label ? "img" : undefined}
    aria-label={label}
    aria-hidden={label ? undefined : true}
  >
    <circle
      cx="12"
      cy="12"
      r="9"
      stroke="currentColor"
      strokeWidth="2.5"
      className="opacity-20"
    />
    <path
      d="M21 12a9 9 0 0 0-9-9"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
    />
  </svg>
);

/**
 * A centered loading state. `screen` fills the viewport (route transitions);
 * otherwise it fills the panel it is placed in.
 */
export const LoadingState = ({
  label,
  screen = false,
  className = "",
}: {
  label: string;
  screen?: boolean;
  className?: string;
}) => (
  <output
    aria-live="polite"
    className={`flex w-full flex-col items-center justify-center gap-3 text-center text-[12px] text-[#686f60] ${screen ? "min-h-dvh" : "min-h-[180px] py-8"} ${className}`}
  >
    <Spinner size={screen ? 28 : 22} className="text-[#697f53]" />
    <span>{label}</span>
  </output>
);
