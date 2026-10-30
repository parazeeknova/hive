"use client";

import "./scenes.css";
import { Icon } from "../../../components/icon";
import { useLoopInView } from "../../../lib/hooks/use-loop-in-view";
import { SceneHexagon } from "./scene-hexagon";

const query = "Extract tables from PDFs";
const results = [
  { name: "pdf_table_extract", price: "$0.05", tag: "MCP" },
  { name: "doc_parser", price: "$0.02", tag: "API" },
  { name: "invoice_reader", price: "$0.03", tag: "Workflow" },
];

export const DiscoverScene = () => {
  const ref = useLoopInView();
  return (
    <figure
      ref={ref}
      aria-label="Search for PDF table extraction, compare three capabilities, and select the tested MCP tool at five cents per call."
      data-loop="idle"
      className="hive-scene discover-scene"
    >
      <div className="scene-track discover-board" aria-hidden="true">
        <div className="scene-surface discover-search">
          <Icon name="search" size={16} />
          <span className="discover-query">
            {query}
            <span className="discover-type-mask">
              <span className="discover-caret" />
            </span>
          </span>
        </div>
        <div className="discover-results">
          {results.map((result, index) => (
            <div
              key={result.name}
              className={`discover-row discover-row-${index}`}
            >
              {index === 0 && <div className="discover-highlight" />}
              <div className="discover-row-content">
                <SceneHexagon className="discover-hex" />
                <div className="discover-name">
                  <span>{result.name}</span>
                  <div className="discover-tags">
                    <span className="scene-tag">{result.tag}</span>
                    {index === 0 && (
                      <span className="scene-done discover-tested">
                        <Icon name="check" size={11} />
                        Tested
                      </span>
                    )}
                  </div>
                </div>
                <span className="scene-mono discover-price">
                  {result.price}
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </figure>
  );
};

export default DiscoverScene;
