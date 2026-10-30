"use client";

import "./scenes.css";
import type { CSSProperties } from "react";

import { Icon } from "../../../components/icon";
import { useLoopInView } from "../../../lib/hooks/use-loop-in-view";
import { SceneHexagon } from "./scene-hexagon";

const inputs = ["report.pdf", "invoice.pdf", "lease.pdf"];
const tables = [
  { cells: 6, columns: 2, rows: 3 },
  { cells: 8, columns: 4, rows: 2 },
  { cells: 12, columns: 3, rows: 4 },
];
const totals = ["$0.05", "$0.10", "$0.15"];

export const InvokeScene = () => {
  const ref = useLoopInView();
  return (
    <figure
      ref={ref}
      aria-label="Report, invoice, and lease PDFs pass through the same hosted extract_tables capability, producing different tables. Total cost rises from five to fifteen cents."
      data-loop="idle"
      className="hive-scene invoke-scene"
    >
      <div className="scene-track invoke-board" aria-hidden="true">
        <div className="invoke-caption">
          <span>Fresh inputs</span>
          <span>Structured output</span>
        </div>
        <svg
          className="invoke-wires"
          viewBox="0 0 320 240"
          preserveAspectRatio="none"
          fill="none"
          aria-hidden="true"
        >
          <path
            d="M28 112H128M176 112H284"
            stroke="currentColor"
            strokeWidth="1.2"
          />
        </svg>
        <div className="invoke-input-lane">
          {inputs.map((input, index) => (
            <div
              key={input}
              className={`scene-surface input-chip input-chip-${index}`}
            >
              <Icon name="file" size={14} />
              <span>{input}</span>
            </div>
          ))}
        </div>
        <div className="invoke-capability">
          <div className="invoke-node">
            <SceneHexagon className="invoke-node-shape" />
            <span className="invoke-ring" />
            <Icon name="code" size={23} />
          </div>
          <span className="invoke-node-label">extract_tables</span>
          <span className="invoke-hosted scene-mono">hosted</span>
        </div>
        <div className="scene-surface invoke-output">
          <span className="invoke-output-title">Tables</span>
          <div className="invoke-table-area">
            {tables.map((table, index) => (
              <div
                key={table.cells}
                className={`invoke-table invoke-table-${index}`}
                style={
                  {
                    "--columns": table.columns,
                    "--rows": table.rows,
                  } as CSSProperties
                }
              >
                {Array.from({ length: table.cells }, (_, cell) => (
                  <span
                    key={`${table.cells}-${cell}`}
                    className={`table-cell table-cell-${index}`}
                    style={{ "--cell": cell } as CSSProperties}
                  />
                ))}
              </div>
            ))}
          </div>
          <div className="invoke-billing">
            <span className="scene-done scene-mono invoke-price">
              {totals.map((total, index) => (
                <span
                  key={total}
                  className={`invoke-total invoke-total-${index}`}
                >
                  {total}
                </span>
              ))}
            </span>
          </div>
        </div>
      </div>
    </figure>
  );
};

export default InvokeScene;
