import { useId } from "react";

import { Icon } from "../../components/icon";
import type { IconName } from "../../components/icon";
import type { RegistryCapability } from "../../lib/hiveway";

import styles from "./capability-details.module.css";

const runtimeIcons: Record<RegistryCapability["runtime"], IconName> = {
  connected_mcp: "globe",
  dependency_inventory: "layers",
  repository_scaffold: "box",
  repository_stack: "git",
  wasm: "code",
  workflow: "compose",
};

/** A decorative circuit that changes with the selected capability, never live telemetry. */
export const CapabilityArtwork = ({ tool }: { tool: RegistryCapability }) => {
  const id = useId();
  const pattern = `${id}-grid`;
  const icon =
    tool.metadata?.integration?.slug === "context7"
      ? "book"
      : runtimeIcons[tool.runtime];
  const variant =
    [...tool.id].reduce(
      (total, character) => total + (character.codePointAt(0) ?? 0),
      0
    ) % 3;
  return (
    <div className={styles.artwork} data-variant={variant} aria-hidden="true">
      <svg viewBox="0 0 520 196" fill="none" focusable="false">
        <defs>
          <pattern
            id={pattern}
            width="24"
            height="24"
            patternUnits="userSpaceOnUse"
          >
            <circle cx="1" cy="1" r="1" fill="currentColor" />
          </pattern>
        </defs>
        <rect
          width="520"
          height="196"
          fill={`url(#${pattern})`}
          opacity=".15"
        />
        <circle cx="260" cy="98" r="126" stroke="currentColor" opacity=".09" />
        <circle
          cx="260"
          cy="98"
          r="92"
          stroke="currentColor"
          opacity=".15"
          strokeDasharray="3 7"
        />
        <g className={styles.circuit} stroke="currentColor" strokeWidth="1.2">
          <path d="M-8 61h125q15 0 15 15v8q0 14 15 14h67" />
          <path d="M306 98h57q15 0 15-15V52q0-14 15-14h135" />
          <path d="M-8 156h166q15 0 15-15v-28q0-15 15-15h26" />
          <path d="M306 98h24q15 0 15 15v30q0 15 15 15h168" />
        </g>
        <g
          className={styles.signals}
          stroke="#b58b33"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeDasharray="7 310"
        >
          <path d="M-8 61h125q15 0 15 15v8q0 14 15 14h67" />
          <path d="M306 98h57q15 0 15-15V52q0-14 15-14h135" />
          <path d="M-8 156h166q15 0 15-15v-28q0-15 15-15h26" />
          <path d="M306 98h24q15 0 15 15v30q0 15 15 15h168" />
        </g>
        <g className={styles.satellite}>
          <rect
            x="71"
            y="40"
            width="42"
            height="42"
            rx="14"
            fill="#fffefb"
            stroke="#d9d9c8"
          />
          <path
            d="m86 55-6 6 6 6m12-12 6 6-6 6m-5-15-3 18"
            stroke="#697457"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
          <circle cx="408" cy="38" r="18" fill="#e1e7d5" stroke="#cbd4bb" />
          <path
            d="m401 38 5 5 9-10"
            stroke="#61734b"
            strokeWidth="1.7"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
          <rect
            x="393"
            y="138"
            width="51"
            height="40"
            rx="13"
            fill="#fffefb"
            stroke="#d9d9c8"
          />
          <path
            d="M406 153h25m-25 6h17m-17 6h22"
            stroke="#8a927b"
            strokeWidth="2"
            strokeLinecap="round"
          />
          <circle cx="145" cy="156" r="5" fill="#f8cf60" stroke="#cfad4c" />
        </g>
        <g className={styles.core}>
          <rect
            x="213"
            y="51"
            width="94"
            height="94"
            rx="30"
            fill="#f8cf60"
            stroke="#d9b754"
          />
          <rect
            x="220"
            y="58"
            width="80"
            height="80"
            rx="24"
            stroke="#fff7d2"
            strokeWidth="1.5"
          />
          <g transform="translate(243 81)">
            <Icon name={icon} size={34} />
          </g>
        </g>
        <path
          d="M346 74v-10m-5 5h10M194 39v-6m-3 3h6M365 121v-6m-3 3h6"
          stroke="#929c80"
          strokeWidth="1.5"
          strokeLinecap="round"
        />
      </svg>
    </div>
  );
};
