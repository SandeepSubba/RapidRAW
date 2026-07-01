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

type Drag =
  | { kind: 'new' }
  | { kind: 'end'; index: number; end: 1 | 2 }
  | { kind: 'move'; index: number; lastX: number; lastY: number };

const MAX_LINES = 4;
const MIN_LEN = 0.03; // normalized; ignore accidental taps
const HANDLE_R = 6;

// Vertical guide if it is taller than it is wide.
const isVertical = (l: Line) => Math.abs(l.y2 - l.y1) >= Math.abs(l.x2 - l.x1);

export default function GuidedKeystoneOverlay({ renderSize }: { renderSize: RenderSize }) {
  const lines = useEditorStore((s) => s.keystoneLines);
  const setEditor = useEditorStore((s) => s.setEditor);
  const svgRef = useRef<SVGSVGElement | null>(null);
  const dragRef = useRef<Drag | null>(null);
  const [draft, setDraft] = useState<Line | null>(null);

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

  const getLines = () => useEditorStore.getState().keystoneLines;
  const writeLines = (ls: Line[]) => setEditor({ keystoneLines: ls });
  const capture = (e: React.PointerEvent) => svgRef.current?.setPointerCapture?.(e.pointerId);

  // pointer down on empty space -> start a new line
  const beginNew = (e: React.PointerEvent) => {
    if (e.button !== 0 || getLines().length >= MAX_LINES) return;
    const p = toNorm(e.clientX, e.clientY);
    if (!p) return;
    dragRef.current = { kind: 'new' };
    setDraft({ x1: p.x, y1: p.y, x2: p.x, y2: p.y });
    capture(e);
  };

  // pointer down on an endpoint handle -> drag/rotate that end
  const beginEnd = (e: React.PointerEvent, index: number, end: 1 | 2) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    dragRef.current = { kind: 'end', index, end };
    capture(e);
  };

  // pointer down on the line body -> translate the whole line
  const beginMove = (e: React.PointerEvent, index: number) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    const p = toNorm(e.clientX, e.clientY);
    if (!p) return;
    dragRef.current = { kind: 'move', index, lastX: p.x, lastY: p.y };
    capture(e);
  };

  const onMove = (e: React.PointerEvent) => {
    const d = dragRef.current;
    if (!d) return;
    const p = toNorm(e.clientX, e.clientY);
    if (!p) return;
    if (d.kind === 'new') {
      setDraft((prev) => (prev ? { ...prev, x2: p.x, y2: p.y } : prev));
    } else if (d.kind === 'end') {
      writeLines(
        getLines().map((l, i) =>
          i === d.index ? (d.end === 1 ? { ...l, x1: p.x, y1: p.y } : { ...l, x2: p.x, y2: p.y }) : l,
        ),
      );
    } else {
      const dx = p.x - d.lastX;
      const dy = p.y - d.lastY;
      d.lastX = p.x;
      d.lastY = p.y;
      writeLines(
        getLines().map((l, i) =>
          i === d.index
            ? { x1: clamp01(l.x1 + dx), y1: clamp01(l.y1 + dy), x2: clamp01(l.x2 + dx), y2: clamp01(l.y2 + dy) }
            : l,
        ),
      );
    }
  };

  const onUp = () => {
    const d = dragRef.current;
    dragRef.current = null;
    if (d?.kind === 'new') {
      const dr = draft;
      setDraft(null);
      if (dr && Math.hypot(dr.x2 - dr.x1, dr.y2 - dr.y1) >= MIN_LEN) {
        writeLines([...getLines(), dr].slice(-MAX_LINES));
      }
    }
  };

  const px = (l: Line) => ({ x1: l.x1 * width, y1: l.y1 * height, x2: l.x2 * width, y2: l.y2 * height });
  const color = (l: Line) => (isVertical(l) ? 'var(--color-accent, #3b82f6)' : '#22c55e');

  return (
    <svg
      ref={svgRef}
      viewBox={`0 0 ${width} ${height}`}
      width={width}
      height={height}
      style={{ position: 'absolute', left: offsetX, top: offsetY, cursor: 'crosshair', touchAction: 'none', zIndex: 5 }}
      onPointerDown={beginNew}
      onPointerMove={onMove}
      onPointerUp={onUp}
      onPointerCancel={onUp}
    >
      {lines.map((l, i) => {
        const c = px(l);
        const col = color(l);
        return (
          <g key={i}>
            {/* wide invisible hit area for translating the line */}
            <line
              x1={c.x1}
              y1={c.y1}
              x2={c.x2}
              y2={c.y2}
              stroke="transparent"
              strokeWidth={16}
              style={{ cursor: 'move' }}
              onPointerDown={(e) => beginMove(e, i)}
            />
            <line x1={c.x1} y1={c.y1} x2={c.x2} y2={c.y2} stroke={col} strokeWidth={2} vectorEffect="non-scaling-stroke" />
            {([1, 2] as const).map((end) => {
              const hx = end === 1 ? c.x1 : c.x2;
              const hy = end === 1 ? c.y1 : c.y2;
              return (
                <circle
                  key={end}
                  cx={hx}
                  cy={hy}
                  r={HANDLE_R}
                  fill={col}
                  stroke="#fff"
                  strokeWidth={1.5}
                  vectorEffect="non-scaling-stroke"
                  style={{ cursor: 'grab' }}
                  onPointerDown={(e) => beginEnd(e, i, end)}
                />
              );
            })}
          </g>
        );
      })}

      {draft && (
        <line
          x1={draft.x1 * width}
          y1={draft.y1 * height}
          x2={draft.x2 * width}
          y2={draft.y2 * height}
          stroke={color(draft)}
          strokeWidth={2}
          strokeDasharray="6 4"
          vectorEffect="non-scaling-stroke"
        />
      )}
    </svg>
  );
}

function clamp01(v: number): number {
  return Math.max(0, Math.min(1, v));
}
