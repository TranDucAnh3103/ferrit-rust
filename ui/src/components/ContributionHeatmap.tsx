import React, { useMemo } from "react";
import type { HeatmapDay } from "../types/ipc";

interface ContributionHeatmapProps {
  days: HeatmapDay[];
}

export const ContributionHeatmap: React.FC<ContributionHeatmapProps> = ({
  days,
}) => {
  const weeks = useMemo(() => {
    const cols: HeatmapDay[][] = [];
    for (let i = 0; i < days.length; i += 7) {
      cols.push(days.slice(i, i + 7));
    }
    return cols;
  }, [days]);

  const totalCommits = useMemo(
    () => days.reduce((acc, d) => acc + d.count, 0),
    [days]
  );

  return (
    <div className="primer-card p-3">
      <div className="d-flex flex-justify-between flex-items-center mb-2 flex-wrap gap-2">
        <div className="f5 text-bold">
          <span className="tabular-nums">{totalCommits.toLocaleString()}</span>{" "}
          contributions in the last 365 days
        </div>

        <div className="d-flex flex-items-center gap-1 f6 text-muted">
          <span>Less</span>
          {[0, 1, 2, 3, 4].map((tier) => (
            <span
              key={tier}
              className="heatmap-cell d-inline-block"
              style={{
                backgroundColor: `var(--color-heatmap-${tier})`,
                pointerEvents: "none",
              }}
            />
          ))}
          <span>More</span>
        </div>
      </div>

      <div className="overflow-auto pb-1">
        <div className="d-flex" style={{ gap: 3 }}>
          {weeks.map((week, wIdx) => (
            <div key={wIdx} className="d-flex flex-column" style={{ gap: 3 }}>
              {week.map((day) => (
                <div
                  key={day.date}
                  className="heatmap-cell"
                  title={`${day.count} commits on ${day.date}`}
                  aria-label={`${day.count} commits on ${day.date}`}
                  style={{
                    backgroundColor: `var(--color-heatmap-${Math.min(
                      4,
                      Math.max(0, day.tier)
                    )})`,
                  }}
                />
              ))}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
