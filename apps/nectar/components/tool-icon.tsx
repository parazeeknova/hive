const tints = {
  blue: "bg-sky-50 text-sky-400",
  butter: "bg-amber-50 text-amber-600",
  lavender: "bg-violet-50 text-violet-400",
  peach: "bg-orange-50 text-orange-300",
  rose: "bg-rose-50 text-rose-300",
  sage: "bg-lime-50 text-lime-700",
} as const;

export const toolTint = (color: string) =>
  tints[color as keyof typeof tints] ?? tints.sage;

export const toolIconBase =
  "inline-flex size-10 shrink-0 items-center justify-center rounded-xl border border-black/5";
