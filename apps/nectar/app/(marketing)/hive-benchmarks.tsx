// Comparison figures shown in the landing comparison cards.
// Placeholder figures, not measured results: replace them with real benchmark
// data before this page is published.
export interface Benchmark {
  metric: string;
  unit: string;
  format: (value: number) => string;
  gain: string;
  rows: { model: string; without: number; with: number }[];
}

const money = (value: number) => `$${value.toFixed(2)}`;
export const benchmarks: Benchmark[] = [
  {
    format: (value) => `${value.toFixed(1)} min`,
    gain: "faster",
    metric: "Task execution time",
    rows: [
      { model: "GPT-6.1 Sol · medium", with: 4.1, without: 18.4 },
      { model: "GPT-6.1 Sol · high, fast", with: 2.9, without: 12.6 },
      { model: "DeepSeek V4.1 Flash", with: 3.4, without: 15.2 },
    ],
    unit: "minutes per task",
  },
  {
    format: (value) => `${Math.round(value)}k`,
    gain: "fewer tokens",
    metric: "Token consumption",
    rows: [
      { model: "GPT-6.1 Sol · medium", with: 86, without: 412 },
      { model: "GPT-6.1 Sol · high, fast", with: 109, without: 538 },
      { model: "DeepSeek V4.1 Flash", with: 81, without: 366 },
    ],
    unit: "thousand tokens per task",
  },
  {
    format: money,
    gain: "cheaper",
    metric: "Price per completed task",
    rows: [
      { model: "GPT-6.1 Sol · medium", with: 0.61, without: 2.84 },
      { model: "GPT-6.1 Sol · high, fast", with: 0.86, without: 4.12 },
      { model: "DeepSeek V4.1 Flash", with: 0.13, without: 0.58 },
    ],
    unit: "USD per completed task",
  },
];

// No viewBox: the bar width is a percentage of the real element width and the
// end caps use a fixed pixel radius, so they stay round at every length.
const Bar = ({
  value,
  max,
  strong,
}: {
  value: number;
  max: number;
  strong: boolean;
}) => (
  <svg
    className={`bench__bar ${strong ? "bench__bar--with" : "bench__bar--without"}`}
    width="100%"
    height="8"
    aria-hidden="true"
  >
    <rect
      className="bench__fill"
      width={`${Math.max(3, (value / max) * 100)}%`}
      height="8"
      rx="4"
    />
  </svg>
);

/** Paired horizontal bars, one group per model; values are also given as text. */
export const BenchmarkChart = ({ data }: { data: Benchmark }) => {
  const max = Math.max(...data.rows.map((row) => row.without));
  const ratio =
    data.rows.reduce((sum, row) => sum + row.without / row.with, 0) /
    data.rows.length;
  return (
    <figure className="bench">
      <figcaption className="bench__head">
        <span className="hive-text-4 bench__metric">{data.metric}</span>
        <strong className="bench__gain">
          {ratio.toFixed(1)}× {data.gain}
        </strong>
      </figcaption>
      <dl className="bench__rows">
        {data.rows.map((row) => (
          <div className="bench__row" key={row.model}>
            <dt className="bench__model">{row.model}</dt>
            <dd className="bench__pair">
              <span className="bench__line">
                <Bar value={row.without} max={max} strong={false} />
                <span className="bench__value">
                  <span className="sr-only">Without Hive: </span>
                  {data.format(row.without)}
                </span>
              </span>
              <span className="bench__line">
                <Bar value={row.with} max={max} strong />
                <span className="bench__value bench__value--with">
                  <span className="sr-only">With Hive: </span>
                  {data.format(row.with)}
                </span>
              </span>
            </dd>
          </div>
        ))}
      </dl>
      <p className="bench__legend">
        <span className="bench__key bench__key--without" aria-hidden="true" />
        Without Hive
        <span className="bench__key bench__key--with" aria-hidden="true" />
        With Hive
        <span className="bench__unit">· {data.unit}</span>
      </p>
    </figure>
  );
};
