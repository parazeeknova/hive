import Image from "next/image";
import Link from "next/link";

export const Brand = ({
  small = false,
  markClassName = "size-8",
  wordmarkClassName = "text-3xl",
}: {
  small?: boolean;
  markClassName?: string;
  wordmarkClassName?: string;
}) => (
  <Link
    href="/"
    className="inline-flex shrink-0 items-center gap-2 leading-none"
    aria-label="Hive home"
  >
    <Image
      src="/hive-mark.png"
      alt=""
      width={36}
      height={33}
      priority
      className={`block self-center object-contain ${
        small ? "size-7" : markClassName
      }`}
    />
    <span
      className={`font-display inline-flex items-baseline leading-none font-extrabold tracking-tighter ${
        small ? "text-2xl" : wordmarkClassName
      }`}
    >
      hive<span className="text-amber-500">.</span>
    </span>
  </Link>
);
