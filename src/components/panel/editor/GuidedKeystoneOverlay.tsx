import { useRef, useState } from 'react';
import { useEditorStore } from '../../../store/useEditorStore';

interface RenderSize {
  width: number;
  height: number;
  offsetX: number;
  offsetY: number;
}

interface Line {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

const MAX_LINES = 4;
const MIN_LEN = 0.03; // normalized; ignore accidental taps

// Vertical guide if it is taller than it is wide.
const isVertical = (l: Line) => Math.abs(l.y2 - l.y1) >= Math.abs(l.x2 - l.x1);

export default function GuidedKeystoneOverlay({ renderSize }: { renderSize: RenderSize }) {
  const lines = useEditorStore((s) => s.keystoneLines);
  const setEditor = useEditorStore((s) => s.setEditor);
  const svgRef = useRef<SVGSVGElement | null>(null);
  const [draft, setDraft] = useState<Line | null>(null);
  const drawingRef = useRef(false);

  const { width, height, offsetX, offsetY } = renderSize;
  if (!width || !height) return null;

  // client px -> normalized 0..1 image coords (getScreenCTM handles pan/zoom)
  const toNorm = (clientX: number, clientY: number): { x: number; y: number } | null => {
    const svg = svgRef.current;
    if (!svg) return null;
    const ctm = svg.getScreenCTM();
    if (!ctm) return null;
    const pt = svg.createSVGPoint();
    pt.x = clientX;
    pt.y = clientY;
    const p = pt.matrixTransform(ctm.inverse());
    return { x: clamp01(p.x / width), y: clamp01(p.y / height) };
  };

  const onPointerDown = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    if (lines.length >= MAX_LINES) return;
    const p = toNorm(e.clientX, e.clientY);
    if (!p) return;
    drawingRef.current = true;
    setDraft({ x1: p.x, y1: p.y, x2: p.x, y2: p.y });
    (e.target as Element).setPointerCapture?.(e.pointerId);
  };

  const onPointerMove = (e: React.PointerEvent) => {
    if (!drawingRef.current || !draft) return;
    e.stopPropagation();
    const p = toNorm(e.clientX, e.clientY);
    if (!p) return;
    setDraft({ ...draft, x2: p.x, y2: p.y });
  };

  const onPointerUp = (e: React.PointerEvent) => {
    if (!drawingRef.current) return;
    e.stopPropagation();
    drawingRef.current = false;
    const d = draft;
    setDraft(null);
    if (!d) return;
    const len = Math.hypot(d.x2 - d.x1, d.y2 - d.y1);
    if (len < MIN_LEN) return;
    setEditor({ keystoneLines: [...lines, d].slice(-MAX_LINES) });
  };

  const px = (l: Line) => ({ x1: l.x1 * width, y1: l.y1 * height, x2: l.x2 * width, y2: l.y2 * height });
  const color = (l: Line) => (isVertical(l) ? 'var(--color-accent, #3b82f6)' : '#22c55e');

  const all = draft ? [...lines, draft] : lines;

  return (
    <svg
      ref={svgRef}
      viewBox={`0 0 ${width} ${height}`}
      width={width}
      height={height}
      style={{ position: 'absolute', left: offsetX, top: offsetY, cursor: 'crosshair', touchAction: 'none', zIndex: 5 }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
    >
      {all.map((l, i) => {
        const c = px(l);
        const col = color(l);
        return (
          <g key={i}>
            <line
              x1={c.x1}
              y1={c.y1}
              x2={c.x2}
              y2={c.y2}
              stroke={col}
              strokeWidth={2}
              vectorEffect="non-scaling-stroke"
            />
            {[
              [c.x1, c.y1],
              [c.x2, c.y2],
            ].map(([cx, cy], j) => (
              <circle key={j} cx={cx} cy={cy} r={5} fill={col} stroke="#fff" strokeWidth={1.5} vectorEffect="non-scaling-stroke" />
            ))}
          </g>
        );
      })}
    </svg>
  );
}

function clamp01(v: number): number {
  return Math.max(0, Math.min(1, v));
}
