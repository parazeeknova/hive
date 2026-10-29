"use client";

import "./scenes.css";
import { useId } from "react";

import { Icon } from "../../../components/icon";
import type { IconName } from "../../../components/icon";
import { useLoopInView } from "../../../lib/hooks/use-loop-in-view";
import { SceneHexagon } from "./scene-hexagon";

const nodes: { icon: IconName; label: string }[] = [
  { icon: "globe", label: "Fetch" },
  { icon: "file", label: "Parse" },
  { icon: "layers", label: "Summarize" },
];

export const ComposeScene = () => {
  const ref = useLoopInView();
  const pattern = useId();
  return (
    <figure
      ref={ref}
      aria-label="Fetch, Parse, and Summarize hexagons combine into doc_to_summary version one at four cents, become published, and give the next capability a place to connect."
      data-loop="idle"
      className="hive-scene compose-scene"
    >
      <div className="scene-track compose-board" aria-hidden="true">
        <svg
          className="compose-grid"
          viewBox="0 0 320 240"
          preserveAspectRatio="xMidYMid slice"
          fill="none"
          aria-hidden="true"
        >
          <defs>
            <pattern
              id={pattern}
              width="36"
              height="62.4"
              patternUnits="userSpaceOnUse"
            >
              <path
                d="M18 0 36 10.4V31.2L18 41.6 0 31.2V10.4ZM0 31.2V52L18 62.4 36 52V31.2"
                stroke="currentColor"
                strokeWidth=".7"
              />
            </pattern>
          </defs>
          <rect width="320" height="240" fill={`url(#${pattern})`} />
        </svg>
        <div className="compose-cluster">
          <svg
            className="compose-connections"
            viewBox="0 0 320 240"
            preserveAspectRatio="none"
            fill="none"
            aria-hidden="true"
          >
            <path
              className="compose-line"
              d="M109 72H181L145 134"
              pathLength="100"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
            <circle className="compose-data-dot" r="3" fill="currentColor" />
            <rect
              className="compose-outline-fill"
              x="61"
              y="21"
              width="167"
              height="167"
              rx="23"
              fill="currentColor"
            />
            <rect
              className="compose-outline"
              x="61"
              y="21"
              width="167"
              height="167"
              rx="23"
              pathLength="100"
              stroke="currentColor"
              strokeWidth="1.2"
            />
          </svg>
          {nodes.map((node, index) => (
            <div
              key={node.label}
              className={`compose-node compose-node-${index}`}
            >
              <div className="compose-token">
                <SceneHexagon />
                <div className="compose-node-copy">
                  <Icon name={node.icon} size={17} />
                  <span>{node.label}</span>
                </div>
              </div>
            </div>
          ))}
          <div className="compose-combined">
            <span>doc_to_summary</span>
            <div className="scene-mono">
              <span>v1</span>
              <span>$0.04</span>
            </div>
          </div>
        </div>
        <span className="scene-done compose-published">
          <Icon name="check" size={12} />
          Published
        </span>
        <div className="compose-next-slot">
          <svg viewBox="0 0 100 100" fill="none" aria-hidden="true">
            <path
              d="m50 3 41 23.5v47L50 97 9 73.5v-47L50 3Z"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeDasharray="4 5"
            />
          </svg>
        </div>
        <div className="compose-next-node">
          <SceneHexagon />
          <Icon name="plus" size={19} />
        </div>
      </div>
    </figure>
  );
};

export default ComposeScene;
