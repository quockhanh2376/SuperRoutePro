import { useMemo } from "react";

interface LatencyChartProps {
  data: number[];
  height?: number;
}

export default function LatencyChart({ data, height = 80 }: LatencyChartProps) {
  const svgContent = useMemo(() => {
    const width = 100; // viewBox width (percentage-based)
    const h = height;
    const maxVal = Math.max(...data, 100);
    const points = data.map((val, i) => {
      const x = (i / (data.length - 1)) * width;
      const y = val > 0 ? h - (val / maxVal) * (h - 10) : h;
      return { x, y, val };
    });

    // Create smooth path
    const pathD = points
      .map((p, i) => (i === 0 ? `M ${p.x} ${p.y}` : `L ${p.x} ${p.y}`))
      .join(" ");

    // Area fill path
    const areaD = `${pathD} L ${width} ${h} L 0 ${h} Z`;

    // Get color for current value
    const lastVal = data[data.length - 1];
    const color =
      lastVal === 0 ? "#ef4444" :
      lastVal < 50 ? "#10b981" :
      lastVal < 100 ? "#eab308" : "#f97316";

    return { pathD, areaD, color, points, width: width, height: h };
  }, [data, height]);

  return (
    <div className="w-full rounded-lg bg-[#0c1220] border border-slate-700/50 overflow-hidden">
      <svg
        viewBox={`0 0 ${svgContent.width} ${svgContent.height}`}
        preserveAspectRatio="none"
        className="w-full block"
        style={{ height: `${height}px` }}
      >
        {/* Grid lines */}
        {[0.25, 0.5, 0.75].map((pct) => (
          <line
            key={pct}
            x1="0"
            y1={height * pct}
            x2={svgContent.width}
            y2={height * pct}
            stroke="#1e293b"
            strokeWidth="0.3"
          />
        ))}

        {/* Area fill */}
        <defs>
          <linearGradient id="areaGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={svgContent.color} stopOpacity="0.3" />
            <stop offset="100%" stopColor={svgContent.color} stopOpacity="0.02" />
          </linearGradient>
        </defs>
        <path d={svgContent.areaD} fill="url(#areaGrad)" />

        {/* Line */}
        <path
          d={svgContent.pathD}
          fill="none"
          stroke={svgContent.color}
          strokeWidth="1"
          strokeLinecap="round"
          strokeLinejoin="round"
        />

        {/* Last point dot */}
        {svgContent.points.length > 0 && (
          <circle
            cx={svgContent.points[svgContent.points.length - 1].x}
            cy={svgContent.points[svgContent.points.length - 1].y}
            r="2"
            fill={svgContent.color}
          >
            <animate
              attributeName="opacity"
              values="1;0.4;1"
              dur="1.5s"
              repeatCount="indefinite"
            />
          </circle>
        )}
      </svg>
    </div>
  );
}
