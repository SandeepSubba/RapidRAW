import { invoke } from '@tauri-apps/api/core';
import { Invokes } from '../components/ui/AppProperties';

// Comparative culling from the Assistant chat. Per-image batch chat sends every
// photo in its own conversation, so the model can never compare frames. Here
// the photos are grouped into bursts of similar shots and each group goes to
// the model in ONE request, labelled by filename, so it can pick the best
// frame and say why. Unrelated photos go in "set" chunks, rated on one scale.

/**
 * Requests that mean "cull/rate these" rather than an edit or a question.
 * Only the plural nouns count ("find the keepers / rejects"): "reject this"
 * or "it's a keeper" is a flag on the photos in hand, not a cull.
 */
export const CULL_REQUEST =
  /\b(?:cull\w*|keepers|rejects|reject (?:the|any|all)|(?:pick|choose|select|find|keep)(?: only)? the best|best (?:shot|shots|frame|frames|one|ones|of (?:each|every|the))|rate (?:these|them|each|all|every|the (?:photos|images|pictures|shots))|star[- ]rate|give (?:each|them|these) (?:a )?(?:star )?ratings?)\b/;

/** Photos per request: enough to compare a burst, small enough for every provider. */
export const MAX_PHOTOS_PER_REQUEST = 12;
const CULL_IMAGE_DIM = 768;
const PREPARE_CONCURRENCY = 4;

export interface ShotInfo {
  path: string;
  quality: number;
  sharpness: number;
}

export interface ShotGroups {
  groups: ShotInfo[][];
  singles: ShotInfo[];
  failed: string[];
}

export interface CullDecision {
  file: string;
  rating: number | null;
  flag: 'pick' | 'reject' | null;
  colorLabel: string | null;
  reason: string | null;
}

/** Current state shown to the model, and the place decisions are written. */
export interface CullTarget {
  mode: 'library' | 'import';
  stateOf(path: string): { rating?: number; flag?: string | null; label?: string | null; kept?: boolean };
  /** Applies one decision; returns what actually changed, for the summary. */
  apply(path: string, decision: CullDecision): string[];
}

export interface CullChunk {
  kind: 'group' | 'set';
  shots: ShotInfo[];
}

export interface ChunkReport {
  index: number;
  total: number;
  kind: 'group' | 'set';
  text: string;
  isError?: boolean;
}

const fileNameOf = (p: string) => (p.split(/[\\/]/).pop() || p).split('?vc=')[0];

/** Groups first (split when larger than one request holds), then the rest as sets. */
export function planCullChunks(shots: ShotGroups, max = MAX_PHOTOS_PER_REQUEST): CullChunk[] {
  const chunks: CullChunk[] = [];
  for (const group of shots.groups) {
    for (let i = 0; i < group.length; i += max) {
      const part = group.slice(i, i + max);
      // A leftover single frame has nothing to be compared with in its chunk.
      chunks.push({ kind: part.length > 1 ? 'group' : 'set', shots: part });
    }
  }
  for (let i = 0; i < shots.singles.length; i += max) {
    chunks.push({ kind: 'set', shots: shots.singles.slice(i, i + max) });
  }
  return chunks;
}

/**
 * Names the model sees for a chunk's photos. Virtual copies and same-named files
 * from different folders would collide, so repeats get " (2)", " (3)".
 */
export function uniqueNames(paths: string[]): string[] {
  const seen = new Map<string, number>();
  return paths.map((p) => {
    const base = fileNameOf(p);
    const n = (seen.get(base) ?? 0) + 1;
    seen.set(base, n);
    return n === 1 ? base : `${base} (${n})`;
  });
}

/** Map a decision's "file" back to a path: exact, then case-insensitive. */
export function matchDecision(file: string, names: string[]): number {
  const exact = names.indexOf(file);
  if (exact !== -1) return exact;
  const lower = file.trim().toLowerCase();
  return names.findIndex((n) => n.toLowerCase() === lower);
}

async function prepareAll(paths: string[], isCancelled: () => boolean) {
  const out: Array<{ mediaType: string; data: string } | null> = new Array(paths.length).fill(null);
  let next = 0;
  const worker = async () => {
    while (next < paths.length && !isCancelled()) {
      const i = next++;
      try {
        const prepared: any = await invoke(Invokes.AssistantPrepareImage, { path: paths[i], maxDim: CULL_IMAGE_DIM });
        out[i] = { mediaType: prepared.mediaType, data: prepared.data };
      } catch (e) {
        console.error('Cull: could not prepare', paths[i], e);
      }
    }
  };
  await Promise.all(Array.from({ length: Math.min(PREPARE_CONCURRENCY, paths.length) }, worker));
  return out;
}

function decisionLine(name: string, d: CullDecision | undefined, changes: string[]): string {
  if (!d) return `· ${name} — no decision returned`;
  const mark = d.flag === 'pick' ? '✓' : d.flag === 'reject' ? '✗' : '·';
  const stars = d.rating ? ` ${'★'.repeat(d.rating)}` : '';
  const label = d.colorLabel ? ` [${d.colorLabel}]` : '';
  const reason = d.reason ? ` — ${d.reason}` : '';
  const unchanged = changes.length === 0 && (d.flag || d.rating || d.colorLabel) ? ' (already set)' : '';
  return `${mark} ${name}${stars}${label}${reason}${unchanged}`;
}

export interface RunCullOptions {
  shots: ShotGroups;
  instruction: string;
  model: string | null;
  target: CullTarget;
  /** The app's own 0-5 quality grade per path, when it has one (import window). */
  qGrades?: Record<string, number>;
  isCancelled(): boolean;
  onReport(report: ChunkReport): void;
}

/** Run the cull chunk by chunk; returns how many photos got a decision. */
export async function runAssistantCull(opts: RunCullOptions): Promise<{ decided: number; total: number }> {
  const chunks = planCullChunks(opts.shots);
  const total = chunks.reduce((n, c) => n + c.shots.length, 0);
  let decided = 0;
  let groupNo = 0;
  let setNo = 0;
  const groupCount = chunks.filter((c) => c.kind === 'group').length;
  const setCount = chunks.length - groupCount;

  for (let ci = 0; ci < chunks.length; ci++) {
    if (opts.isCancelled()) break;
    const chunk = chunks[ci];
    const heading =
      chunk.kind === 'group'
        ? `Group ${++groupNo} of ${groupCount} · ${chunk.shots.length} similar shots`
        : `Photos ${++setNo} of ${setCount} · ${chunk.shots.length} photo${chunk.shots.length === 1 ? '' : 's'}`;
    const paths = chunk.shots.map((s) => s.path);
    const names = uniqueNames(paths);

    try {
      const prepared = await prepareAll(paths, opts.isCancelled);
      if (opts.isCancelled()) break;
      const usable = paths.map((_, i) => i).filter((i) => prepared[i]);
      if (usable.length === 0) throw new Error('none of these photos could be read');

      const images = usable.map((i, n) => ({ ...prepared[i]!, label: `Photo ${n + 1}: ${names[i]}` }));
      const sharpVals = chunk.shots.map((s) => s.sharpness);
      const context = {
        mode: opts.target.mode,
        kind: chunk.kind,
        chunk: { index: ci + 1, total: chunks.length },
        files: usable.map((i) => {
          const state = opts.target.stateOf(paths[i]);
          const q = opts.qGrades?.[paths[i]];
          return {
            file: names[i],
            ...(sharpVals.some((v) => v > 0) ? { sharpness: Math.round(chunk.shots[i].sharpness) } : {}),
            ...(q !== undefined ? { q } : {}),
            ...(state.rating ? { rating: state.rating } : {}),
            ...(state.flag ? { flag: state.flag } : {}),
            ...(state.label ? { label: state.label } : {}),
            ...(state.kept !== undefined ? { kept: state.kept } : {}),
          };
        }),
      };

      const res: any = await invoke(Invokes.AssistantCull, {
        instruction: opts.instruction,
        images,
        context,
        model: opts.model,
      });
      if (opts.isCancelled()) break;

      const byIndex = new Map<number, CullDecision>();
      for (const d of (res?.results ?? []) as CullDecision[]) {
        const i = matchDecision(d.file, names);
        if (i !== -1 && prepared[i] && !byIndex.has(i)) byIndex.set(i, d);
      }
      const lines = usable.map((i) => {
        const d = byIndex.get(i);
        const changes = d ? opts.target.apply(paths[i], d) : [];
        if (d) decided++;
        return decisionLine(names[i], d, changes);
      });
      const unreadable = paths.map((_, i) => i).filter((i) => !prepared[i]);
      unreadable.forEach((i) => lines.push(`· ${names[i]} — could not be read`));

      const reply = typeof res?.reply === 'string' && res.reply.trim() ? `\n${res.reply.trim()}` : '';
      opts.onReport({
        index: ci + 1,
        total: chunks.length,
        kind: chunk.kind,
        text: `${heading}${reply}\n${lines.join('\n')}`,
      });
    } catch (e: any) {
      opts.onReport({
        index: ci + 1,
        total: chunks.length,
        kind: chunk.kind,
        text: `${heading}: ${typeof e === 'string' ? e : e?.message || String(e)}`,
        isError: true,
      });
    }
  }
  return { decided, total };
}
