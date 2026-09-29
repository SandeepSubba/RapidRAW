import { useState, useRef, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Bot,
  Send,
  Trash2,
  Loader2,
  AlertTriangle,
  Sparkles,
  Paperclip,
  X,
  RefreshCw,
  Tag,
  MessageSquarePlus,
  History,
  MessageSquare,
  Pencil,
  Check,
  Layers,
  Square,
  Wrench,
} from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import { toast } from 'react-toastify';
import clsx from 'clsx';
import { Invokes } from '../../ui/AppProperties';
import Text from '../../ui/Text';
import { TextColors, TextVariants } from '../../../types/typography';
import { useEditorStore } from '../../../store/useEditorStore';
import { getOrientedDimensions } from '../../../utils/cropUtils';
import { useImportStore } from '../../../store/useImportStore';
import { useScannerStore } from '../../../store/useScannerStore';
import { rerenderScanPreviewNow } from '../../views/import/ScannerPane';
import { useUIStore } from '../../../store/useUIStore';
import { useLibraryStore } from '../../../store/useLibraryStore';
import { useSettingsStore } from '../../../store/useSettingsStore';
import { useAssistantStore, nextMessageId, AssistantMessage } from '../../../store/useAssistantStore';
import { useEditorActions } from '../../../hooks/useEditorActions';
import { useLibraryActions } from '../../../hooks/useLibraryActions';
import { getTransformAdjustments } from '../../../hooks/useAiMasking';
import { INITIAL_MASK_ADJUSTMENTS, INITIAL_MASK_CONTAINER, MAX_POINT_COLORS } from '../../../utils/adjustments';
import { useAuth } from '@clerk/react';
import { createSubMask } from '../../../utils/maskUtils';
import { Mask, SubMaskMode } from './Masks';
import { v4 as uuidv4 } from 'uuid';

// The develop-slider fields the assistant is allowed to set, with their valid
// ranges. Values coming back from the model are clamped to these before we
// apply them, so a hallucinated 999 can't push a slider out of bounds.
// Everything the assistant may set, each clamped to the range its UI slider
// allows, so a model value can never go further than a user could drag.
// Not here, and ignored if proposed: masks, AI patches, point colours, LUTs,
// lens blur (needs a depth map), guided-perspective lines, point curves, and
// film-negative conversion (sidecar-owned; it has its own commands).
const ADJUSTMENT_RANGES: Record<string, [number, number]> = {
  // Basic
  exposure: [-5, 5],
  brightness: [-5, 5],
  contrast: [-100, 100],
  highlights: [-100, 100],
  shadows: [-100, 100],
  whites: [-100, 100],
  blacks: [-100, 100],
  // Color
  temperature: [-100, 100],
  tint: [-100, 100],
  vibrance: [-100, 100],
  saturation: [-100, 100],
  hue: [-180, 180],
  // Details
  sharpness: [-100, 100],
  sharpnessThreshold: [0, 80],
  clarity: [-100, 100],
  dehaze: [-100, 100],
  structure: [-100, 100],
  centré: [-100, 100],
  lumaNoiseReduction: [0, 100],
  colorNoiseReduction: [0, 100],
  chromaticAberrationRedCyan: [-100, 100],
  chromaticAberrationBlueYellow: [-100, 100],
  skinSmoothing: [0, 100],
  skinTexture: [0, 100],
  skinSmoothingScale: [0, 100],
  // Effects
  glowAmount: [0, 100],
  halationAmount: [0, 100],
  flareAmount: [0, 100],
  vignetteAmount: [-100, 100],
  vignetteMidpoint: [0, 100],
  vignetteRoundness: [-100, 100],
  vignetteFeather: [0, 100],
  grainAmount: [0, 100],
  grainSize: [0, 100],
  grainRoughness: [0, 100],
  // Geometry
  rotation: [-45, 45],
  transformDistortion: [-100, 100],
  transformVertical: [-100, 100],
  transformHorizontal: [-100, 100],
  transformRotate: [-45, 45],
  transformAspect: [-100, 100],
  transformScale: [50, 150],
  transformXOffset: [-100, 100],
  transformYOffset: [-100, 100],
  // Lens profile correction strengths (only act when a profile is found)
  lensDistortionAmount: [0, 200],
  lensTcaAmount: [0, 200],
  lensVignetteAmount: [0, 200],
};
const INTEGER_ADJUSTMENTS: Record<string, [number, number]> = { orientationSteps: [0, 3] };
const BOOLEAN_ADJUSTMENTS = new Set([
  'flipHorizontal',
  'flipVertical',
  'lensDistortionEnabled',
  'lensTcaEnabled',
  'lensVignetteEnabled',
]);
const ENUM_ADJUSTMENTS: Record<string, string[]> = { toneMapper: ['basic', 'agx'] };

// Nested groups, as a spec of field -> range (or sub-spec). Merged into the
// current value, because the apply step is a shallow spread: sending just
// {hsl: {blues: {saturation: -20}}} must not wipe the other seven colours.
type AdjustmentSpec = { [key: string]: [number, number] | AdjustmentSpec };
const HSL_FIELDS: AdjustmentSpec = { hue: [-100, 100], saturation: [-100, 100], luminance: [-100, 100] };
const GRADING_WHEEL: AdjustmentSpec = { hue: [0, 360], saturation: [0, 100], luminance: [-100, 100] };
const CURVE_FIELDS: AdjustmentSpec = {
  darks: [-100, 100],
  shadows: [-100, 100],
  highlights: [-100, 100],
  lights: [-100, 100],
  whiteLevel: [-100, 0],
  blackLevel: [0, 100],
};
const NESTED_ADJUSTMENTS: Record<string, AdjustmentSpec> = {
  hsl: Object.fromEntries(
    ['reds', 'oranges', 'yellows', 'greens', 'aquas', 'blues', 'purples', 'magentas'].map((c) => [c, HSL_FIELDS]),
  ),
  colorGrading: {
    shadows: GRADING_WHEEL,
    midtones: GRADING_WHEEL,
    highlights: GRADING_WHEEL,
    global: GRADING_WHEEL,
    blending: [0, 100],
    balance: [-100, 100],
  },
  colorCalibration: {
    shadowsTint: [-100, 100],
    redHue: [-100, 100],
    redSaturation: [-100, 100],
    greenHue: [-100, 100],
    greenSaturation: [-100, 100],
    blueHue: [-100, 100],
    blueSaturation: [-100, 100],
  },
  parametricCurve: { luma: CURVE_FIELDS, red: CURVE_FIELDS, green: CURVE_FIELDS, blue: CURVE_FIELDS },
};

// Geometry is where a single guess is rarely right (how far a distortion value
// bends depends on the lens), so these trigger a rendered-result check.
const GEOMETRY_KEYS = new Set([
  'rotation',
  'transformDistortion',
  'transformVertical',
  'transformHorizontal',
  'transformRotate',
  'transformAspect',
  'transformScale',
  'transformXOffset',
  'transformYOffset',
  'lensDistortionAmount',
  'lensDistortionEnabled',
  'flipHorizontal',
  'flipVertical',
  'orientationSteps',
]);
const MAX_REVIEW_ROUNDS = 2;

type AdjustmentValue = number | string | boolean;

interface Attachment {
  id: string;
  dataUrl: string;
  mediaType: string;
  data: string; // base64 without the data: prefix
}

// The text metadata fields the assistant may write, mapped from the friendly
// names it uses in its JSON to the EXIF keys the backend/metadata panel expect.
const METADATA_FIELDS: Record<string, string> = {
  title: 'ImageDescription',
  author: 'Artist',
  copyright: 'Copyright',
  comments: 'UserComment',
};
const EXIF_TO_FRIENDLY: Record<string, string> = Object.fromEntries(
  Object.entries(METADATA_FIELDS).map(([friendly, exifKey]) => [exifKey, friendly]),
);

// The model doesn't always use the exact lowercase keys, so map a range of
// spellings/synonyms (case- and separator-insensitive) onto the EXIF keys.
// Without this, a returned {"Title": "..."} would be silently dropped.
const METADATA_ALIASES: Record<string, string> = {
  title: 'ImageDescription',
  imagetitle: 'ImageDescription',
  description: 'ImageDescription',
  imagedescription: 'ImageDescription',
  caption: 'ImageDescription',
  author: 'Artist',
  artist: 'Artist',
  creator: 'Artist',
  copyright: 'Copyright',
  rights: 'Copyright',
  comments: 'UserComment',
  comment: 'UserComment',
  usercomment: 'UserComment',
  notes: 'UserComment',
  note: 'UserComment',
};

function metaKeyToExif(key: string): string | null {
  const norm = key.toLowerCase().replace(/[\s_-]/g, '');
  if (METADATA_ALIASES[norm]) return METADATA_ALIASES[norm];
  if (EXIF_TO_FRIENDLY[key]) return key; // a raw EXIF key passed straight through
  return null;
}

// Keep only the whitelisted metadata fields and coerce every value to a string.
function sanitizeMetadata(raw: any): Record<string, string> {
  const out: Record<string, string> = {};
  if (!raw || typeof raw !== 'object') return out;
  for (const [key, value] of Object.entries(raw)) {
    const exifKey = metaKeyToExif(key);
    if (!exifKey || value == null) continue;
    // Skip empty/whitespace values: schema-constrained models fill unchanged
    // fields with "" and we must not blank existing metadata because of that.
    const str = String(value).trim();
    if (str === '') continue;
    out[exifKey] = str;
  }
  return out;
}

const VALID_COLORS = new Set(['red', 'yellow', 'green', 'blue', 'purple']);

// Slash commands available in the chat input.
const SLASH_COMMANDS: Array<{ cmd: string; desc: string }> = [
  { cmd: '/compact', desc: 'Summarize this chat to shrink its context' },
  { cmd: '/clear', desc: 'Clear this conversation' },
  { cmd: '/new', desc: 'Start a new conversation' },
  { cmd: '/reset', desc: 'Delete ALL chat history and start fresh' },
  { cmd: '/help', desc: 'List commands' },
];

// Normalize the model's tags field (either an array of adds, or {add,remove})
// into clean, lowercased add/remove lists.
function normalizeTags(raw: any): { add: Array<string>; remove: Array<string> } {
  const clean = (arr: any): Array<string> =>
    (Array.isArray(arr) ? arr : []).map((t) => String(t).trim().toLowerCase()).filter(Boolean);
  if (Array.isArray(raw)) return { add: clean(raw), remove: [] };
  if (raw && typeof raw === 'object') return { add: clean(raw.add), remove: clean(raw.remove) };
  return { add: [], remove: [] };
}

// The current text metadata of the open image, as friendly names, so the model
// can make sensible partial edits (e.g. append to an existing copyright).
function readCurrentMetadata(exif: any): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [friendly, exifKey] of Object.entries(METADATA_FIELDS)) {
    const clean = (exif?.[exifKey] ?? '').toString().replace(/^"|"$/g, '').trim();
    if (clean && clean.toLowerCase() !== 'default') out[friendly] = clean;
  }
  return out;
}

function formatMetadata(patch: Record<string, string>): string {
  return Object.entries(patch)
    .map(([k, v]) => `${EXIF_TO_FRIENDLY[k] || k}: ${v || '(cleared)'}`)
    .join(', ');
}

const toNumber = (v: any) => (typeof v === 'number' ? v : parseFloat(v));
const clampTo = (v: number, [lo, hi]: [number, number]) => Math.max(lo, Math.min(hi, v));

function mergeNested(
  raw: any,
  current: any,
  spec: AdjustmentSpec,
  path: string,
  changes: Record<string, AdjustmentValue>,
): any | null {
  if (!raw || typeof raw !== 'object') return null;
  let out: any = null;
  for (const [key, sub] of Object.entries(spec)) {
    if (!(key in raw)) continue;
    let next: any;
    if (Array.isArray(sub)) {
      const n = toNumber(raw[key]);
      if (!Number.isFinite(n)) continue;
      next = clampTo(n, sub);
      changes[`${path}.${key}`] = next;
    } else {
      next = mergeNested(raw[key], current?.[key], sub, `${path}.${key}`, changes);
      if (next === null) continue;
    }
    out = out ?? { ...(current ?? {}) };
    out[key] = next;
  }
  return out;
}

// Validate a model's proposal against the current adjustments. `patch` is
// ready to spread over them; `changes` is the flat list shown in the chat.
function sanitizeAdjustments(
  raw: any,
  current: any,
): { patch: Record<string, any>; changes: Record<string, AdjustmentValue> } {
  const patch: Record<string, any> = {};
  const changes: Record<string, AdjustmentValue> = {};
  if (!raw || typeof raw !== 'object') return { patch, changes };
  for (const [key, value] of Object.entries(raw)) {
    if (ADJUSTMENT_RANGES[key]) {
      const n = toNumber(value);
      if (Number.isFinite(n)) patch[key] = changes[key] = clampTo(n, ADJUSTMENT_RANGES[key]);
    } else if (INTEGER_ADJUSTMENTS[key]) {
      const n = Math.round(toNumber(value));
      if (Number.isFinite(n)) patch[key] = changes[key] = clampTo(n, INTEGER_ADJUSTMENTS[key]);
    } else if (BOOLEAN_ADJUSTMENTS.has(key)) {
      if (typeof value === 'boolean') patch[key] = changes[key] = value;
    } else if (ENUM_ADJUSTMENTS[key]) {
      const v = String(value).toLowerCase();
      if (ENUM_ADJUSTMENTS[key].includes(v)) patch[key] = changes[key] = v;
    }
  }
  for (const [key, spec] of Object.entries(NESTED_ADJUSTMENTS)) {
    const merged = mergeNested(raw[key], current?.[key], spec, key, changes);
    if (merged) patch[key] = merged;
  }
  // A parametric curve only renders in parametric mode.
  if (patch.parametricCurve && current?.curveMode !== 'parametric') {
    patch.curveMode = changes.curveMode = 'parametric';
  }
  return { patch, changes };
}

// Validate + clamp a model-proposed crop rectangle (pixels, oriented image
// space). Returns null for anything degenerate so a hallucinated rect can't
// blank the image.
// `inspect: {"target": "label"}` asks the app to locate the printed tag itself
// rather than supplying coordinates. Accepted loosely — models reach for
// "auto"/"tag" as readily as the documented "label".
// "Don't scan/ocr or send any images" is a standing rule for the chat, but a
// later request to look overrides it: the newest user turn that says either way
// decides. Checking only the message being sent let "do the same" fall back on
// a prohibition from many turns earlier, and a loose keyword match read
// targeted exclusions ("read the tag but don't read the price code") as bans.
// Clause punctuation ends the span, so "skip 1160, read the tag" is a request.
const SCAN_VERB = String.raw`(?:scan(?:ning)?|ocr|inspect(?:ing)?|zoom(?:ing)?|read(?:ing)?|look(?:ing)?\s+at|attach(?:ing)?|send(?:ing)?)`;
const SCAN_PROHIBITION = new RegExp(
  String.raw`\b(?:don['’]?t|do(?:es)?\s+not|to\s+not|never|no(?:\s+need\s+to)?|without|avoid|skip|stop)\b[^.!?,;\n]{0,25}?\b${SCAN_VERB}\b(?:\s*(?:\/|,|\bor\b|\band\b|\bnor\b)\s*${SCAN_VERB}\b)*`,
  'g',
);
const SCAN_REQUEST = /\b(?:scan|ocr|read|inspect|zoom in|look at|describe|analy[sz]e|identify)\b/;

// 'ask' when the text asks to look at the image once its prohibited phrases are
// removed, 'forbid' when it only prohibits, null when it says neither.
function scanStance(text: string): 'forbid' | 'ask' | null {
  const lower = (text || '').toLowerCase();
  const stripped = lower.replace(SCAN_PROHIBITION, ' ');
  if (SCAN_REQUEST.test(stripped)) return 'ask';
  return stripped !== lower ? 'forbid' : null;
}

// Photos go to the model only when the user asks it to look ("scan/ocr this
// image", "read the tag"). Titles, renames, tags and folder questions don't
// need the picture, and attaching it anyway pinned every answer to that one
// image ("I can only inspect the single image attached"). A bare follow-up
// ("do the same", "again") inherits the last stance so a scan workflow repeats.
const REPEAT_FOLLOW_UP = /\b(?:same|again|repeat|redo|continue|next one)\b/;

// Edits that can only be done by looking (straightening, perspective, "make
// it look better") need the photo even without "scan" or "look at". An
// explicit "don't scan" still wins, since scanStance runs first.
const VISUAL_EDIT =
  /\b(?:mask(?:s|ed|ing)?|local(?:ly)?|selective(?:ly)?|paint|brush|remove|erase|get rid of|bokeh|lens blur|blur the background|point colou?r|perspective|straight(?:en)?|keystone|distort(?:ion)?|barrel|tilt(?:ed)?|horizon|crooked|lean(?:ing)?|converg\w*|verticals?|enhance|improve|retouch|auto[- ]?edit|look (?:better|nicer|good)|white balance|colou?r (?:cast|correct\w*))\b/;

function wantsImage(text: string, priorUserText: string[]): boolean {
  const own = scanStance(text);
  if (own !== null) return own === 'ask';
  if (VISUAL_EDIT.test((text || '').toLowerCase())) return true;
  if (!REPEAT_FOLLOW_UP.test((text || '').toLowerCase())) return false;
  return [...priorUserText].reverse().map(scanStance).find((s) => s !== null) === 'ask';
}

// Folder-wide questions ("is any image missing a title?", "which ones have no
// tags?") are answered from an app-supplied listing, never from pixels.
const LIBRARY_REQUEST =
  /\b(?:folder|library|all (?:the |these |of the )?(?:images|photos|pictures|files)|every (?:image|photo|picture|file)|any (?:image|photo|picture|file)s?|which (?:images|photos|pictures|files|ones)|how many)\b/;
const LIBRARY_LIMIT = 2000;

const fileNameOf = (p: string) => (p.split(/[\\/]/).pop() || p).split('?vc=')[0];

// One compact entry per image in the open folder, empty fields omitted. Titles
// come from the background EXIF pass; anything it hasn't reached yet is read
// now, so "no title" can't be reported just because loading was still running.
async function buildLibraryContext(): Promise<Record<string, any> | null> {
  const { imageList, imageRatings, currentFolderPath } = useLibraryStore.getState();
  if (imageList.length === 0) return null;
  const list = imageList.slice(0, LIBRARY_LIMIT);
  const missing = list.filter((img) => !img.exif).map((img) => img.path);
  const fetched: Record<string, Record<string, string>> = {};
  for (let i = 0; i < missing.length; i += 100) {
    try {
      Object.assign(fetched, await invoke(Invokes.ReadExifForPaths, { paths: missing.slice(i, i + 100) }));
    } catch {
      // An unreadable file is listed without a title rather than failing the question.
    }
  }
  const images = list.map((img) => {
    const title = readCurrentMetadata(img.exif || fetched[img.path] || null).title;
    const rating = imageRatings[img.path] ?? img.rating;
    const allTags = img.tags || [];
    const label = allTags.find((tg) => tg.startsWith('color:'))?.slice(6);
    const tags = allTags
      .filter((tg) => !tg.startsWith('color:'))
      .map((tg) => (tg.startsWith('user:') ? tg.slice(5) : tg));
    return {
      file: fileNameOf(img.path),
      ...(title ? { title } : {}),
      ...(rating ? { rating } : {}),
      ...(label ? { label } : {}),
      ...(tags.length ? { tags } : {}),
      ...(img.is_edited ? { edited: true } : {}),
    };
  });
  return { folder: currentFolderPath, total: imageList.length, truncated: imageList.length > list.length, images };
}

function wantsLabelInspect(inspect: any): boolean {
  const target = inspect?.target;
  if (typeof target !== 'string') return false;
  const t = target.trim().toLowerCase();
  return t === 'label' || t === 'tag' || t === 'auto';
}

function sanitizeCropPatch(
  raw: any,
  imageW: number,
  imageH: number,
): { crop: { unit: 'px'; x: number; y: number; width: number; height: number }; aspectRatio: number } | null {
  if (!raw || typeof raw !== 'object') return null;
  const n = (v: any) => (typeof v === 'number' ? v : parseFloat(v));
  let x = Math.round(n(raw.x));
  let y = Math.round(n(raw.y));
  let width = Math.round(n(raw.width));
  let height = Math.round(n(raw.height));
  if (![x, y, width, height].every(Number.isFinite)) return null;
  x = Math.min(Math.max(0, x), imageW - 1);
  y = Math.min(Math.max(0, y), imageH - 1);
  width = Math.min(width, imageW - x);
  height = Math.min(height, imageH - y);
  const MIN_SIDE = 16;
  if (width < MIN_SIDE || height < MIN_SIDE) return null;
  return { crop: { unit: 'px', x, y, width, height }, aspectRatio: width / height };
}

// Scan-preview mode: the pane's own controls, described to the model through
// the adjustments context and mapped back from its patch.
function scannerContext(sc: any): any {
  const ctx: any = {
    _mode:
      'FILM SCANNER PREVIEW — the only controls are: brightness (exposure, EV -3..3), ' +
      'contrast (-100..100)' +
      (sc.filmType !== 'e6'
        ? ', negativeConversion.redWeight/greenWeight/blueWeight (color timing, 0.5..1.5) and negativeConversion.contrast (print grade, 0.5..2.5)'
        : '') +
      '. Return changes under those exact keys in "adjustments". Metadata/tags/rating/filename cannot be changed here.',
    brightness: sc.exposureOffset,
    contrast: sc.contrast,
  };
  if (sc.filmType !== 'e6') {
    ctx.negativeConversion = {
      enabled: true,
      redWeight: sc.redWeight,
      greenWeight: sc.greenWeight,
      blueWeight: sc.blueWeight,
      contrast: sc.curveContrast,
    };
  }
  return ctx;
}

function applyScannerPatch(raw: any): Record<string, number> {
  const applied: Record<string, number> = {};
  if (!raw || typeof raw !== 'object') return applied;
  const clamp = (v: any, lo: number, hi: number) => {
    const n = typeof v === 'number' ? v : parseFloat(v);
    return Number.isFinite(n) ? Math.max(lo, Math.min(hi, n)) : null;
  };
  const patch: any = {};
  const b = clamp(raw.brightness ?? raw.exposure, -3, 3);
  if (b !== null) { patch.exposureOffset = b; applied.exposure = b; }
  const c = clamp(raw.contrast, -100, 100);
  if (c !== null) { patch.contrast = c; applied.contrast = c; }
  const nc = raw.negativeConversion;
  if (nc && typeof nc === 'object') {
    const rw = clamp(nc.redWeight, 0.5, 1.5);
    const gw = clamp(nc.greenWeight, 0.5, 1.5);
    const bw = clamp(nc.blueWeight, 0.5, 1.5);
    const pg = clamp(nc.contrast, 0.5, 2.5);
    if (rw !== null) { patch.redWeight = rw; applied.redWeight = rw; }
    if (gw !== null) { patch.greenWeight = gw; applied.greenWeight = gw; }
    if (bw !== null) { patch.blueWeight = bw; applied.blueWeight = bw; }
    if (pg !== null) { patch.curveContrast = pg; applied.printGrade = pg; }
    if (Object.keys(patch).some((k) => ['redWeight', 'greenWeight', 'blueWeight', 'curveContrast'].includes(k))) {
      patch.scanAdvanced = true; // reveal what changed
    }
  }
  if (Object.keys(patch).length > 0) useScannerStore.getState().setScanner(patch);
  return applied;
}

function dataUrlToImage(url: string): { mediaType: string; data: string } | null {
  const m = url.match(/^data:([^;]+);base64,(.*)$/s);
  return m ? { mediaType: m[1], data: m[2] } : null;
}

function formatPatch(patch: Record<string, AdjustmentValue>): string {
  return Object.entries(patch)
    .map(([k, v]) =>
      typeof v === 'number' ? `${k} ${v > 0 ? '+' : ''}${Math.round(v * 100) / 100}` : `${k} ${v}`,
    )
    .join(', ');
}

// Fetch a blob: URL (the viewer's processed preview) and turn it into the
// base64 payload the backend expects, so the assistant can "see" the open image.
// Local models are often loaded with a small context window (LM Studio defaults
// to ~4k). A full-size preview can blow past it, and the server silently
// truncates the request — the model then drops fields (e.g. filename) or emits
// stale values. Cap the longest edge so the image + prompt fit comfortably.
const ASSISTANT_IMAGE_MAX_DIM = 2048;
// Cloud vision models (Kimi/OpenAI/Claude) aren't VRAM/context-limited like a
// local model, so we can send much larger images — critical for reading small
// text (e.g. tiny fabric labels) that gets crushed at 2048px.
const ASSISTANT_IMAGE_MAX_DIM_CLOUD = 4096;

async function downscaleBlob(blob: Blob, maxDim: number): Promise<{ mediaType: string; data: string }> {
  const noop = async () => {
    const dataUrl: string = await new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = reject;
      reader.readAsDataURL(blob);
    });
    const comma = dataUrl.indexOf(',');
    return {
      mediaType: dataUrl.slice(5, dataUrl.indexOf(';')) || blob.type || 'image/png',
      data: comma >= 0 ? dataUrl.slice(comma + 1) : dataUrl,
    };
  };
  try {
    const bitmap = await createImageBitmap(blob);
    const longest = Math.max(bitmap.width, bitmap.height);
    if (longest <= maxDim) {
      bitmap.close?.();
      return noop();
    }
    const scale = maxDim / longest;
    const w = Math.max(1, Math.round(bitmap.width * scale));
    const h = Math.max(1, Math.round(bitmap.height * scale));
    const canvas = document.createElement('canvas');
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      bitmap.close?.();
      return noop();
    }
    ctx.drawImage(bitmap, 0, 0, w, h);
    bitmap.close?.();
    const dataUrl = canvas.toDataURL('image/jpeg', 0.9);
    return { mediaType: 'image/jpeg', data: dataUrl.slice(dataUrl.indexOf(',') + 1) };
  } catch {
    return noop();
  }
}

async function blobUrlToImage(
  url: string,
  maxDim: number = ASSISTANT_IMAGE_MAX_DIM,
): Promise<{ mediaType: string; data: string } | null> {
  try {
    const resp = await fetch(url);
    const blob = await resp.blob();
    return await downscaleBlob(blob, maxDim);
  } catch {
    return null;
  }
}

// Render the image with the given adjustments. The editor's own preview can't
// be used for the result check: with the GPU renderer it isn't refreshed after
// an edit, so the model would be shown its own starting point.
async function renderForReview(
  path: string,
  adjustments: any,
  maxDim: number,
): Promise<{ mediaType: string; data: string } | null> {
  try {
    const bytes = await invoke<Uint8Array>(Invokes.GeneratePreviewForPath, { path, jsAdjustments: adjustments });
    return await downscaleBlob(new Blob([new Uint8Array(bytes)], { type: 'image/jpeg' }), maxDim);
  } catch {
    return null;
  }
}

// Mask types the chat can create, by the name the model uses. "background"
// is the foreground mask inverted.
const MASK_TYPES: Record<string, Mask> = {
  subject: Mask.AiSubject,
  object: Mask.AiSubject,
  sky: Mask.AiSky,
  foreground: Mask.AiForeground,
  background: Mask.AiForeground,
  radial: Mask.Radial,
  linear: Mask.Linear,
  gradient: Mask.Linear,
  luminance: Mask.Luminance,
  color: Mask.Color,
  colour: Mask.Color,
  eyes: Mask.AiEyes,
  mouth: Mask.AiMouth,
  depth: Mask.AiDepth,
  all: Mask.All,
  brush: Mask.Brush,
  paint: Mask.Brush,
};
const MAX_MASKS_PER_TURN = 6;
const MAX_REMOVALS_PER_TURN = 4;

// LUT library requests carry the list of installed LUT names.
const LUT_REQUEST = /\b(?:luts?|look[- ]?up table|film (?:look|emulation|stock)|cinematic)\b/;

interface LutEntry {
  name: string;
  path: string;
  isBuiltIn: boolean;
}

const pointFrom = (p: any) => (Array.isArray(p) ? { x: p[0], y: p[1] } : p);

// Fill a polygon with overlapping horizontal brush strokes, inset by the brush
// radius so the round caps land on the edge instead of spilling past it.
function fillPolygon(pts: Array<{ x: number; y: number }>, size: number, feather: number): any[] {
  const ys = pts.map((p) => p.y);
  const minY = Math.min(...ys);
  const maxY = Math.max(...ys);
  const step = Math.max(1, size * 0.6);
  const r = size / 2;
  const out: any[] = [];
  for (let y = minY + step / 2; y < maxY && out.length < 2000; y += step) {
    const xs: number[] = [];
    for (let i = 0; i < pts.length; i++) {
      const a = pts[i];
      const b = pts[(i + 1) % pts.length];
      if ((a.y <= y && b.y > y) || (b.y <= y && a.y > y)) xs.push(a.x + ((y - a.y) * (b.x - a.x)) / (b.y - a.y));
    }
    xs.sort((p, q) => p - q);
    for (let i = 0; i + 1 < xs.length; i += 2) {
      const x1 = xs[i] + r;
      const x2 = xs[i + 1] - r;
      const points = x2 > x1 ? [{ x: x1, y }, { x: x2, y }] : [{ x: (xs[i] + xs[i + 1]) / 2, y }];
      out.push({ tool: 'brush', brushSize: size, feather, points: points.length === 1 ? [points[0], points[0]] : points });
    }
  }
  return out;
}

// Brush strokes (and filled polygons) from view space into image-space lines,
// the shape the canvas brush writes: {tool, brushSize, feather 0..1, points}.
function brushLines(spec: any, ctx: MaskContext): any[] {
  const shortSide = ctx.canvas ? Math.min(ctx.canvas.width, ctx.canvas.height) : 1000;
  const defSize = Math.max(2, numberOr(spec?.size, shortSide * 0.03));
  const defFeather = clampTo(numberOr(spec?.feather, 50), [0, 100]) / 100;
  const lines: any[] = [];
  for (const s of Array.isArray(spec?.strokes) ? spec.strokes.slice(0, 200) : []) {
    const pts = (Array.isArray(s?.points) ? s.points : [])
      .map((p: any) => viewPoint(pointFrom(p), ctx))
      .filter(Boolean) as Array<{ x: number; y: number }>;
    if (pts.length === 0) continue;
    lines.push({
      tool: s?.erase ? 'eraser' : 'brush',
      brushSize: Math.max(2, numberOr(s?.size, defSize)),
      feather: s?.feather !== undefined ? clampTo(numberOr(s.feather, 50), [0, 100]) / 100 : defFeather,
      points: pts.length === 1 ? [pts[0], pts[0]] : pts,
    });
  }
  for (const poly of Array.isArray(spec?.fill) ? spec.fill.slice(0, 20) : []) {
    const raw = Array.isArray(poly) ? poly : poly?.points || [];
    const pts = raw.map((p: any) => viewPoint(pointFrom(p), ctx)).filter(Boolean) as Array<{ x: number; y: number }>;
    if (pts.length >= 3) lines.push(...fillPolygon(pts, defSize, defFeather));
  }
  return lines;
}

// Everything needed to turn the model's view-space coordinates into mask
// parameters. Masks live in full (oriented, uncropped) image pixels — the view
// plus the crop offset — the same space the Masks panel writes.
interface MaskContext {
  path: string;
  adjustments: any;
  offsetX: number;
  offsetY: number;
  canvas: { width: number; height: number } | null;
}

function viewPoint(p: any, ctx: MaskContext): { x: number; y: number } | null {
  const x = toNumber(p?.x);
  const y = toNumber(p?.y);
  if (!Number.isFinite(x) || !Number.isFinite(y)) return null;
  const cx = ctx.canvas ? clampTo(x, [0, ctx.canvas.width]) : x;
  const cy = ctx.canvas ? clampTo(y, [0, ctx.canvas.height]) : y;
  return { x: cx + ctx.offsetX, y: cy + ctx.offsetY };
}

const numberOr = (v: any, fallback: number) => {
  const n = toNumber(v);
  return Number.isFinite(n) ? n : fallback;
};

// A mask's own adjustments: the global sanitizer, limited to the keys a mask
// accepts (tone, colour, detail, HSL, grading, curve — no geometry).
function sanitizeMaskAdjustments(raw: any, current: any) {
  const { patch, changes } = sanitizeAdjustments(raw, current);
  for (const k of Object.keys(patch)) if (!(k in INITIAL_MASK_ADJUSTMENTS)) delete patch[k];
  for (const k of Object.keys(changes)) if (!(k.split('.')[0] in INITIAL_MASK_ADJUSTMENTS)) delete changes[k];
  return { patch, changes };
}

// Build one mask container from a model spec. AI masks are generated here, so
// nothing reaches the image until every requested mask exists.
async function buildMaskContainer(spec: any, ctx: MaskContext): Promise<{ container: any; label: string }> {
  const kind = String(spec?.type || '').toLowerCase();
  const type = MASK_TYPES[kind];
  if (!type) throw new Error(`unknown mask type "${spec?.type}"`);
  const invert = Boolean(spec?.invert) !== (kind === 'background');
  const view = {
    rotation: ctx.adjustments.rotation || 0,
    flipHorizontal: !!ctx.adjustments.flipHorizontal,
    flipVertical: !!ctx.adjustments.flipVertical,
    orientationSteps: ctx.adjustments.orientationSteps || 0,
  };
  const aiArgs = { jsAdjustments: getTransformAdjustments(ctx.adjustments), ...view };
  const sub: any = createSubMask(
    type,
    { width: ctx.canvas?.width ?? 1000, height: ctx.canvas?.height ?? 1000 } as any,
    SubMaskMode.Additive,
  );
  const grow = clampTo(numberOr(spec?.grow, 0), [-100, 100]);
  const aiFeather = clampTo(numberOr(spec?.feather, 0), [0, 100]);
  let params: any = { ...(sub.parameters || {}) };

  switch (type) {
    case Mask.AiSubject: {
      const b = spec?.box;
      const a = viewPoint(b, ctx);
      const e = viewPoint({ x: toNumber(b?.x) + toNumber(b?.width), y: toNumber(b?.y) + toNumber(b?.height) }, ctx);
      if (!a || !e) throw new Error('a subject mask needs a "box"');
      const out: any = await invoke(Invokes.GenerateAiSubjectMask, {
        ...aiArgs,
        path: ctx.path,
        startPoint: [a.x, a.y],
        endPoint: [e.x, e.y],
      });
      params = { ...params, grow, feather: aiFeather, startX: a.x, startY: a.y, endX: e.x, endY: e.y, ...out };
      break;
    }
    case Mask.AiSky:
      params = { ...params, grow, feather: aiFeather, ...((await invoke(Invokes.GenerateAiSkyMask, aiArgs)) as any) };
      break;
    case Mask.AiForeground:
      params = {
        ...params,
        grow,
        feather: aiFeather,
        ...((await invoke(Invokes.GenerateAiForegroundMask, aiArgs)) as any),
      };
      break;
    case Mask.AiEyes:
    case Mask.AiMouth:
      params = {
        ...params,
        ...((await invoke(Invokes.GenerateAiFaceRegionMask, {
          ...aiArgs,
          region: type === Mask.AiEyes ? 'eyes' : 'mouth',
        })) as any),
      };
      break;
    case Mask.AiDepth: {
      const depth = {
        minDepth: clampTo(numberOr(spec?.minDepth, 20), [0, 100]),
        maxDepth: clampTo(numberOr(spec?.maxDepth, 80), [0, 100]),
        minFade: 15,
        maxFade: 15,
        feather: 10,
      };
      params = {
        ...params,
        ...depth,
        ...((await invoke('generate_ai_depth_mask', { ...aiArgs, path: ctx.path, ...depth })) as any),
      };
      break;
    }
    case Mask.Radial: {
      const c = viewPoint(spec?.center, ctx);
      if (!c) throw new Error('a radial mask needs a "center"');
      const rx = Math.max(1, numberOr(spec?.radius?.x, 200));
      const ry = Math.max(1, numberOr(spec?.radius?.y, rx));
      const f = numberOr(spec?.feather, 0.5);
      params = {
        centerX: c.x,
        centerY: c.y,
        radiusX: rx,
        radiusY: ry,
        rotation: clampTo(numberOr(spec?.rotation, 0), [-180, 180]),
        // 0..1 here; accept a percentage too.
        feather: clampTo(f > 1 ? f / 100 : f, [0, 1]),
      };
      break;
    }
    case Mask.Linear: {
      // The model drags "from" (full effect) to "to" (none), like a graduated
      // filter. The stored line is the gradient's midline, with the full-effect
      // side on its left (mask_generation's perpendicular), so convert.
      const f = viewPoint(spec?.from, ctx);
      const tt = viewPoint(spec?.to, ctx);
      if (!f || !tt) throw new Error('a linear mask needs "from" and "to"');
      const gx = tt.x - f.x;
      const gy = tt.y - f.y;
      const len = Math.hypot(gx, gy);
      if (len < 1) throw new Error('linear mask "from" and "to" are the same point');
      const ux = gx / len;
      const uy = gy / len;
      const mx = (f.x + tt.x) / 2;
      const my = (f.y + tt.y) / 2;
      const half = len / 2;
      params = { startX: mx, startY: my, endX: mx + uy * half, endY: my - ux * half, range: half };
      break;
    }
    case Mask.Brush: {
      const lines = brushLines(spec, ctx);
      if (lines.length === 0) throw new Error('a brush mask needs "strokes" or "fill"');
      params = { lines };
      break;
    }
    case Mask.Luminance:
    case Mask.Color: {
      const p = viewPoint(spec?.target, ctx);
      if (!p) throw new Error(`a ${kind} mask needs a "target" point`);
      params = {
        targetX: p.x,
        targetY: p.y,
        tolerance: clampTo(numberOr(spec?.tolerance, 20), [0, 100]),
        feather: clampTo(numberOr(spec?.feather, 35), [0, 100]),
        grow: 0,
        ...view,
      };
      break;
    }
    default:
      params = {};
  }

  const { patch, changes } = sanitizeMaskAdjustments(spec?.adjustments, INITIAL_MASK_ADJUSTMENTS);
  const name = String(spec?.name || '').trim().slice(0, 40) || `${kind} mask`;
  const container = {
    ...INITIAL_MASK_CONTAINER,
    id: uuidv4(),
    name,
    invert,
    opacity: clampTo(numberOr(spec?.opacity, 100), [0, 100]),
    subMasks: [{ ...sub, parameters: params }],
    adjustments: { ...INITIAL_MASK_ADJUSTMENTS, ...patch },
  };
  const detail = formatPatch(changes) || 'no adjustments yet';
  return { container, label: `mask "${name}" (${kind}${invert ? ', inverted' : ''}): ${detail}` };
}

// Remove an object (or replace it, given a "prompt") with an AI patch: the
// quick-eraser flow. The region is a box the AI subject model outlines, or
// brush strokes/fill for thin or irregular things (wires, stains). Without a
// prompt it inpaints locally (LaMa); with one it's generative replace, which
// needs the AI connector or cloud.
async function buildRemovalPatch(
  spec: any,
  ctx: MaskContext,
  current: any,
  getToken: () => Promise<string | null>,
): Promise<{ patch: any; label: string }> {
  const prompt = String(spec?.prompt || '').trim().slice(0, 300);
  const what = String(spec?.name || '').trim().slice(0, 40) || 'object';
  const view = {
    rotation: ctx.adjustments.rotation || 0,
    flipHorizontal: !!ctx.adjustments.flipHorizontal,
    flipVertical: !!ctx.adjustments.flipVertical,
    orientationSteps: ctx.adjustments.orientationSteps || 0,
  };
  const dims = { width: ctx.canvas?.width ?? 1000, height: ctx.canvas?.height ?? 1000 } as any;
  let sub: any;
  if (spec?.box) {
    const b = spec.box;
    const a = viewPoint(b, ctx);
    const e = viewPoint({ x: toNumber(b?.x) + toNumber(b?.width), y: toNumber(b?.y) + toNumber(b?.height) }, ctx);
    if (!a || !e) throw new Error('the removal "box" is invalid');
    const out: any = await invoke(Invokes.GenerateAiSubjectMask, {
      jsAdjustments: getTransformAdjustments(ctx.adjustments),
      ...view,
      path: ctx.path,
      startPoint: [a.x, a.y],
      endPoint: [e.x, e.y],
    });
    sub = createSubMask(Mask.QuickEraser, dims, SubMaskMode.Additive);
    sub = { ...sub, parameters: { ...sub.parameters, startX: a.x, startY: a.y, endX: e.x, endY: e.y, ...out } };
  } else {
    const lines = brushLines(spec, ctx);
    if (lines.length === 0) throw new Error('a removal needs a "box", "strokes" or "fill"');
    sub = createSubMask(Mask.Brush, dims, SubMaskMode.Additive);
    sub = { ...sub, parameters: { lines } };
  }
  const patch: any = {
    id: uuidv4(),
    invert: false,
    isLoading: false,
    name: (prompt ? `Replace ${what}` : `Remove ${what}`).slice(0, 40),
    patchData: null,
    prompt,
    subMasks: [sub],
    visible: true,
  };
  const token = prompt ? await getToken().catch(() => null) : null;
  const json: any = await invoke(Invokes.InvokeGenerativeReplaseWithMaskDef, {
    currentAdjustments: { ...current, aiPatches: [...(current.aiPatches || []), patch] },
    patchDefinition: { ...patch, prompt },
    path: ctx.path,
    useFastInpaint: !prompt,
    token: token || null,
  });
  patch.patchData = typeof json === 'string' ? JSON.parse(json) : json;
  return { patch, label: prompt ? `replaced ${what} with "${prompt}"` : `removed ${what}` };
}

// The eyedropper's conversion: average sRGB -> linear, hue/saturation from
// the linear max/min, luminance back in the perceptual scale.
function rgbToPointColor(r: number, g: number, b: number) {
  const lr = Math.pow(r / 255, 2.2);
  const lg = Math.pow(g / 255, 2.2);
  const lb = Math.pow(b / 255, 2.2);
  const mx = Math.max(lr, lg, lb);
  const mn = Math.min(lr, lg, lb);
  const d = mx - mn;
  let h = 0;
  if (d > 1e-6) {
    if (mx === lr) h = 60 * (((lg - lb) / d) % 6);
    else if (mx === lg) h = 60 * ((lb - lr) / d + 2);
    else h = 60 * ((lr - lg) / d + 4);
  }
  h = (h + 360) % 360;
  return {
    hue: Math.round(h * 10) / 10,
    saturation: Math.round((mx > 1e-6 ? d / mx : 0) * 1000) / 10,
    luminance: Math.round(Math.pow(mx, 1 / 2.2) * 1000) / 10,
  };
}

// Sample a 7x7 average of the attached view at a _canvas point.
async function sampleViewColor(
  image: { mediaType: string; data: string } | null,
  canvas: { width: number; height: number } | null,
  x: number,
  y: number,
): Promise<{ hue: number; saturation: number; luminance: number } | null> {
  if (!image || !canvas) return null;
  try {
    const blob = await (await fetch(`data:${image.mediaType};base64,${image.data}`)).blob();
    const bmp = await createImageBitmap(blob);
    const sx = Math.round((x * bmp.width) / canvas.width);
    const sy = Math.round((y * bmp.height) / canvas.height);
    const oc = new OffscreenCanvas(bmp.width, bmp.height);
    const g = oc.getContext('2d');
    if (!g) return null;
    g.drawImage(bmp, 0, 0);
    const r = 3;
    const x0 = Math.max(0, Math.min(bmp.width - 1, sx - r));
    const y0 = Math.max(0, Math.min(bmp.height - 1, sy - r));
    const w = Math.max(1, Math.min(2 * r + 1, bmp.width - x0));
    const h = Math.max(1, Math.min(2 * r + 1, bmp.height - y0));
    const px = g.getImageData(x0, y0, w, h).data;
    let rs = 0;
    let gs = 0;
    let bs = 0;
    const n = px.length / 4;
    for (let i = 0; i < px.length; i += 4) {
      rs += px[i];
      gs += px[i + 1];
      bs += px[i + 2];
    }
    return rgbToPointColor(rs / n, gs / n, bs / n);
  } catch {
    return null;
  }
}

const POINT_COLOR_RANGES: Record<string, [number, number]> = {
  hue: [0, 360],
  saturation: [0, 100],
  luminance: [0, 100],
  hueRange: [2, 120],
  satRange: [5, 100],
  lumRange: [5, 100],
  hueShift: [-180, 180],
  satShift: [-100, 100],
  lumShift: [-100, 100],
};

function pointColorFields(spec: any, base: any) {
  const out = { ...base };
  const changed: string[] = [];
  for (const [k, range] of Object.entries(POINT_COLOR_RANGES)) {
    const n = toNumber(spec?.[k]);
    if (Number.isFinite(n)) {
      out[k] = clampTo(n, range);
      changed.push(`${k} ${out[k]}`);
    }
  }
  return { point: out, changed };
}

// New point colours ("pointColors", up to 8 in total) and edits by 0-based
// index ("pointColorUpdates"). A new point's reference colour is sampled at
// "target" on the attached view, as the eyedropper does, or given directly.
async function applyPointColors(
  response: any,
  list: any[],
  ctx: MaskContext,
  viewImage: { mediaType: string; data: string } | null,
  create: boolean,
): Promise<{ list: any[]; labels: string[] }> {
  const next = [...list];
  const labels: string[] = [];
  for (const u of Array.isArray(response?.pointColorUpdates) ? response.pointColorUpdates : []) {
    const idx = Math.round(toNumber(u?.index));
    if (!Number.isFinite(idx) || idx < 0 || idx >= next.length) continue;
    if (u?.delete === true) {
      next.splice(idx, 1);
      labels.push(`deleted point colour ${idx + 1}`);
      continue;
    }
    const { point, changed } = pointColorFields(u, next[idx]);
    next[idx] = point;
    if (changed.length) labels.push(`point colour ${idx + 1}: ${changed.join(', ')}`);
  }
  if (create) {
    for (const spec of Array.isArray(response?.pointColors) ? response.pointColors : []) {
      if (next.length >= MAX_POINT_COLORS) {
        labels.push(`point colour skipped: the limit is ${MAX_POINT_COLORS}`);
        break;
      }
      let base: any = {
        hue: 0,
        saturation: 50,
        luminance: 50,
        hueRange: 30,
        satRange: 50,
        lumRange: 50,
        hueShift: 0,
        satShift: 0,
        lumShift: 0,
      };
      const t = spec?.target;
      if (t) {
        const sampled = await sampleViewColor(viewImage, ctx.canvas, toNumber(t?.x), toNumber(t?.y));
        if (!sampled) {
          labels.push('point colour skipped: could not sample the target (no image attached)');
          continue;
        }
        base = { ...base, ...sampled };
      } else if (!Number.isFinite(toNumber(spec?.hue))) {
        labels.push('point colour skipped: needs a "target" point or a "hue"');
        continue;
      }
      const { point, changed } = pointColorFields(spec, base);
      next.push(point);
      labels.push(`point colour ${next.length} (hue ${Math.round(point.hue)}): ${changed.join(', ') || 'added'}`);
    }
  }
  return { list: next, labels };
}

// "lut": {"name", "intensity"} picks from the LUT library by name;
// {"intensity"} alone changes the current one; {"remove": true} clears it.
async function applyLut(spec: any, current: any): Promise<{ patch: any; label: string } | null> {
  if (!spec || typeof spec !== 'object') return null;
  if (spec.remove === true) {
    return {
      patch: { lutPath: null, lutName: null, lutData: null, lutSize: 0, lutIntensity: 100, lutIsSceneReferred: false },
      label: 'LUT removed',
    };
  }
  const intensity = Number.isFinite(toNumber(spec.intensity)) ? clampTo(toNumber(spec.intensity), [0, 100]) : null;
  const wanted = String(spec.name || '').trim().toLowerCase();
  if (!wanted) {
    if (intensity === null || !current.lutPath) return null;
    return { patch: { lutIntensity: intensity }, label: `LUT intensity ${intensity}` };
  }
  const luts = await invoke<LutEntry[]>('list_luts');
  const stem = (s: string) => s.toLowerCase().replace(/\.(cube|3dl|png)$/i, '');
  const entry =
    luts.find((l) => l.name.toLowerCase() === wanted) ||
    luts.find((l) => stem(l.name) === stem(wanted)) ||
    luts.find((l) => l.name.toLowerCase().includes(wanted));
  if (!entry) throw new Error(`no LUT named "${spec.name}" in the library`);
  const result: { size: number } = await invoke('load_and_parse_lut', { path: entry.path });
  return {
    patch: {
      lutPath: entry.path,
      lutName: entry.path.split(/[\\/]/).pop() || entry.name,
      lutSize: result.size,
      lutIntensity: intensity ?? 100,
      lutIsSceneReferred: entry.isBuiltIn,
      sectionVisibility: { ...(current.sectionVisibility || {}), effects: true },
    },
    label: `LUT ${entry.name}${intensity !== null ? ` at ${intensity}%` : ''}`,
  };
}

// "lensBlur": background blur from an AI depth map, generated on first use.
// "focus" is the depth band kept sharp, 0 = nearest .. 100 = farthest; it's
// stored inverted (lensBlurMinDepth = 100 - far), as the Effects panel does.
async function applyLensBlur(spec: any, current: any): Promise<{ patch: any; label: string } | null> {
  if (!spec || typeof spec !== 'object') return null;
  if (spec.enabled === false) return { patch: { lensBlurEnabled: false }, label: 'lens blur off' };
  const patch: any = { lensBlurEnabled: true };
  const parts: string[] = [];
  if (Number.isFinite(toNumber(spec.amount))) {
    patch.lensBlurAmount = clampTo(toNumber(spec.amount), [0, 100]);
    parts.push(`amount ${patch.lensBlurAmount}`);
  }
  if (Number.isFinite(toNumber(spec.diffusion))) {
    patch.lensBlurDiffusion = clampTo(toNumber(spec.diffusion), [0, 100]);
    parts.push(`diffusion ${patch.lensBlurDiffusion}`);
  }
  if (['circle', 'hexagon', 'octagon', 'ring'].includes(String(spec.shape))) {
    patch.lensBlurShape = String(spec.shape);
    parts.push(`${patch.lensBlurShape} bokeh`);
  }
  const near = toNumber(spec.focus?.near);
  const far = toNumber(spec.focus?.far);
  if (Number.isFinite(near) && Number.isFinite(far)) {
    const lo = clampTo(Math.min(near, far), [0, 100]);
    const hi = clampTo(Math.max(near, far), [0, 100]);
    patch.lensBlurMinDepth = 100 - hi;
    patch.lensBlurMaxDepth = 100 - lo;
    parts.push(`sharp from ${lo} to ${hi}`);
  }
  if (Number.isFinite(toNumber(spec.fade))) {
    patch.lensBlurMinFade = patch.lensBlurMaxFade = clampTo(toNumber(spec.fade), [0, 100]);
  }
  if (!current.lensBlurDepthMap) {
    patch.lensBlurDepthMap = await invoke<string>('generate_full_image_depth_map', { jsAdjustments: current });
  }
  return { patch, label: `lens blur${parts.length ? `: ${parts.join(', ')}` : ''}` };
}

// Every non-slider visual edit in one pass: masks, object removal, point
// colours, LUT, lens blur. Failures become chat notes, never exceptions, so
// one bad item doesn't block the rest. `create` is false in review rounds,
// which may only adjust what exists.
async function applyVisualExtras(
  response: any,
  current: any,
  ctx: MaskContext,
  opts: {
    create: boolean;
    viewImage: { mediaType: string; data: string } | null;
    getToken: () => Promise<string | null>;
    isCancelled: () => boolean;
  },
): Promise<{ next: any; labels: string[]; changed: boolean }> {
  let next: any = { ...current };
  const labels: string[] = [];
  let changed = false;
  const note = (what: string, e: any) => labels.push(`${what}: ${e?.message || e}`);

  if (opts.create) {
    const built: any[] = [];
    for (const spec of (Array.isArray(response?.masks) ? response.masks : []).slice(0, MAX_MASKS_PER_TURN)) {
      if (opts.isCancelled()) break;
      try {
        const b = await buildMaskContainer(spec, { ...ctx, adjustments: next });
        built.push(b.container);
        labels.push(b.label);
      } catch (e) {
        note(`mask "${spec?.name || spec?.type}" not created`, e);
      }
    }
    if (built.length) {
      next = { ...next, masks: [...(next.masks || []), ...built] };
      changed = true;
    }
  }
  const mu = applyMaskUpdates(response?.maskUpdates, next.masks || []);
  if (mu.labels.length) {
    next = { ...next, masks: mu.masks };
    labels.push(...mu.labels);
    changed = true;
  }

  if (opts.create) {
    for (const spec of (Array.isArray(response?.remove) ? response.remove : []).slice(0, MAX_REMOVALS_PER_TURN)) {
      if (opts.isCancelled()) break;
      try {
        const r = await buildRemovalPatch(spec, { ...ctx, adjustments: next }, next, opts.getToken);
        next = { ...next, aiPatches: [...(next.aiPatches || []), r.patch] };
        labels.push(r.label);
        changed = true;
      } catch (e) {
        note(`couldn't remove ${spec?.name || 'object'}`, e);
      }
    }
  }

  const pc = await applyPointColors(response, next.pointColors || [], ctx, opts.viewImage, opts.create);
  if (pc.labels.length) {
    next = { ...next, pointColors: pc.list };
    labels.push(...pc.labels);
    changed = true;
  }

  if (response?.lut) {
    try {
      const r = await applyLut(response.lut, next);
      if (r) {
        next = { ...next, ...r.patch };
        labels.push(r.label);
        changed = true;
      }
    } catch (e) {
      note('LUT not applied', e);
    }
  }

  if (response?.lensBlur) {
    try {
      const r = await applyLensBlur(response.lensBlur, next);
      if (r) {
        next = { ...next, ...r.patch };
        labels.push(r.label);
        changed = true;
      }
    } catch (e) {
      note('lens blur not applied', e);
    }
  }

  return { next, labels, changed };
}

// The keys of `next` that differ from `prev` — what to spread into the editor.
function changedKeys(next: any, prev: any): Record<string, any> {
  return Object.fromEntries(Object.entries(next).filter(([k, v]) => v !== prev?.[k]));
}

// Edit or delete existing masks, matched by id or name.
function applyMaskUpdates(updates: any, masks: any[]): { masks: any[]; labels: string[] } {
  if (!Array.isArray(updates)) return { masks, labels: [] };
  const next = [...masks];
  const labels: string[] = [];
  for (const u of updates.slice(0, 20)) {
    const key = String(u?.id ?? u?.name ?? '');
    const idx = next.findIndex((m) => m.id === key || m.name === key);
    if (idx < 0) continue;
    const m = next[idx];
    if (u?.delete === true) {
      next.splice(idx, 1);
      labels.push(`deleted mask "${m.name}"`);
      continue;
    }
    const { patch, changes } = sanitizeMaskAdjustments(u?.adjustments, m.adjustments);
    const updated = { ...m, adjustments: { ...m.adjustments, ...patch } };
    if (typeof u?.invert === 'boolean') updated.invert = u.invert;
    if (typeof u?.visible === 'boolean') updated.visible = u.visible;
    const op = toNumber(u?.opacity);
    if (Number.isFinite(op)) updated.opacity = clampTo(op, [0, 100]);
    next[idx] = updated;
    labels.push(`mask "${m.name}": ${formatPatch(changes) || 'updated'}`);
  }
  return { masks: next, labels };
}

async function fileToAttachment(file: File, maxDim: number): Promise<Attachment> {
  // Downscale manual attachments too (a File is a Blob), so a full-size photo
  // doesn't blow up the request body — matches the viewer/batch paths.
  const { mediaType, data } = await downscaleBlob(file, maxDim);
  return { id: nextMessageId(), dataUrl: `data:${mediaType};base64,${data}`, mediaType, data };
}

export default function AssistantPanel() {
  const { t } = useTranslation();
  const [input, setInput] = useState('');
  // Up/Down recall of previously sent messages, shell-style. null = not
  // navigating; the draft holds whatever was typed before recall started.
  const [historyIndex, setHistoryIndex] = useState<number | null>(null);
  const historyDraftRef = useRef('');
  const [attachments, setAttachments] = useState<Array<Attachment>>([]);
  const [models, setModels] = useState<Array<string>>([]);
  const [modelsError, setModelsError] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  // Set by the Stop button; checked between batch images and before applying a
  // single response so an in-flight run can be abandoned.
  const cancelRef = useRef(false);
  // Developer mode: route the message to the Claude Code CLI running inside
  // the user's RapidRAW checkout, so the assistant can change the app itself.
  const [devMode, setDevMode] = useState(false);
  const [devProgress, setDevProgress] = useState<string[]>([]);
  const devBusyRef = useRef(false);

  const conversations = useAssistantStore((s) => s.conversations);
  const activeId = useAssistantStore((s) => s.activeId);
  const isLoading = useAssistantStore((s) => s.isLoading);
  const addMessage = useAssistantStore((s) => s.addMessage);
  const setLoading = useAssistantStore((s) => s.setLoading);
  const newConversation = useAssistantStore((s) => s.newConversation);
  const selectConversation = useAssistantStore((s) => s.selectConversation);
  const renameConversation = useAssistantStore((s) => s.renameConversation);
  const deleteConversation = useAssistantStore((s) => s.deleteConversation);
  const clearActive = useAssistantStore((s) => s.clearActive);
  const clearAll = useAssistantStore((s) => s.clearAll);
  const replaceActiveMessages = useAssistantStore((s) => s.replaceActiveMessages);

  const activeConversation = conversations.find((c) => c.id === activeId) || null;
  const messages = activeConversation?.messages ?? [];

  const [historyOpen, setHistoryOpen] = useState(false);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');

  const startRename = (c: { id: string; title: string }) => {
    setRenamingId(c.id);
    setRenameValue(c.title);
  };
  const commitRename = () => {
    if (renamingId) renameConversation(renamingId, renameValue);
    setRenamingId(null);
  };

  const appSettings = useSettingsStore((s) => s.appSettings);
  const handleSettingsChange = useSettingsStore((s) => s.handleSettingsChange);
  const provider = appSettings?.assistantProvider || 'lmstudio';
  const providerLabel =
    provider === 'openai'
      ? 'OpenAI'
      : provider === 'anthropic'
        ? 'Anthropic'
        : provider === 'claudecode'
          ? 'Claude Code'
          : 'LM Studio';
  const selectedModel = appSettings?.assistantModel || '';
  // Local (LM Studio) is VRAM/context-limited, so keep images small; cloud
  // providers can take much larger images for better small-text OCR.
  const imageMaxDim = provider === 'lmstudio' ? ASSISTANT_IMAGE_MAX_DIM : ASSISTANT_IMAGE_MAX_DIM_CLOUD;

  const { setAdjustments } = useEditorActions();
  // Generative replace (object removal with a prompt) may need the cloud token.
  const { getToken } = useAuth();
  const { handleUpdateExif, handleRate, handleSetColorLabel, handleTagsChanged, handleRenameToName } =
    useLibraryActions();
  const selectedImage = useEditorStore((s) => s.selectedImage);
  const scanPreviewReady = useScannerStore((st) => !!st.previewData);
  // Hooks must be unconditional — deriving the flag after both reads keeps the
  // hook order stable (the short-circuited form crashed the whole tree).
  const importViewActive = useUIStore((st) => st.isImportViewActive);
  const importStage = useImportStore((st) => st.stage);
  const scannerOpen = importViewActive && importStage === 'scanner';
  const multiSelectedPaths = useLibraryStore((s) => s.multiSelectedPaths);
  const selectedCount = multiSelectedPaths.length;

  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [messages, isLoading]);

  const refreshModels = useCallback(async () => {
    setModelsError(false);
    try {
      const list: any = await invoke(Invokes.AssistantListModels);
      setModels(Array.isArray(list) ? list : []);
    } catch {
      setModels([]);
      setModelsError(true);
    }
  }, []);

  // Load the model list when the panel mounts or the provider changes.
  useEffect(() => {
    refreshModels();
  }, [refreshModels, provider]);

  const addFiles = useCallback(
    (files: Array<File>) => {
      const images = files.filter((f) => f.type.startsWith('image/'));
      if (images.length === 0) return;
      Promise.all(images.map((f) => fileToAttachment(f, imageMaxDim))).then((atts) =>
        setAttachments((prev) => [...prev, ...atts]),
      );
    },
    [imageMaxDim],
  );

  const handlePaste = useCallback(
    (e: any) => {
      const items = e.clipboardData?.items;
      if (!items) return;
      const files: Array<File> = [];
      for (const item of items) {
        if (item.type && item.type.startsWith('image/')) {
          const f = item.getAsFile();
          if (f) files.push(f);
        }
      }
      if (files.length > 0) {
        e.preventDefault();
        addFiles(files);
      }
    },
    [addFiles],
  );

  // Apply a model response's metadata + organization (tags/rating/color/rename)
  // to one image path. Returns a summary so the chat can show what changed.
  const applyMetaOrg = useCallback(
    async (
      response: any,
      path: string,
      intent?: { rename: boolean; title: boolean },
    ): Promise<{ metaPatch: Record<string, string> | null; org: string | null }> => {
      const metaPatch = sanitizeMetadata(response?.metadata);
      const modelFilename =
        typeof response?.filename === 'string' && response.filename.trim() ? response.filename.trim() : null;

      // Local models are unreliable at filling BOTH title and filename when asked
      // to do both (they fill one, or narrate in `reply`). This workflow uses the
      // same value for both, so mirror whichever one the model produced into the
      // other when the user's intent for it is clear.
      if (intent?.title && !metaPatch['ImageDescription'] && modelFilename) {
        metaPatch['ImageDescription'] = modelFilename;
      }

      const hasMeta = Object.keys(metaPatch).length > 0;
      if (hasMeta) await handleUpdateExif([path], metaPatch);

      const orgParts: Array<string> = [];

      const { add, remove } = normalizeTags(response?.tags);
      if (add.length || remove.length) {
        const img = useLibraryStore.getState().imageList.find((i) => i.path === path);
        let tagObjs = (img?.tags || [])
          .filter((tg: string) => !tg.startsWith('color:'))
          .map((tg: string) => ({ tag: tg.startsWith('user:') ? tg.slice(5) : tg, isUser: tg.startsWith('user:') }));
        for (const tag of add) {
          if (!tagObjs.some((o) => o.tag === tag)) {
            await invoke(Invokes.AddTagForPaths, { paths: [path], tag: `user:${tag}` });
            tagObjs.push({ tag, isUser: true });
          }
        }
        for (const tag of remove) {
          const existing = tagObjs.find((o) => o.tag === tag);
          if (existing) {
            await invoke(Invokes.RemoveTagForPaths, { paths: [path], tag: existing.isUser ? `user:${tag}` : tag });
            tagObjs = tagObjs.filter((o) => o.tag !== tag);
          }
        }
        handleTagsChanged([path], tagObjs);
        if (add.length) orgParts.push(`tags +${add.join(', +')}`);
        if (remove.length) orgParts.push(`tags -${remove.join(', -')}`);
      }

      // Only ACT on a positive rating. Schema-constrained models often emit 0
      // (or null) for fields they aren't changing, so treat 0 as "no change"
      // rather than wiping an existing rating on every edit.
      if (typeof response?.rating === 'number') {
        const r = Math.max(0, Math.min(5, Math.round(response.rating)));
        if (r >= 1) {
          handleRate(r, [path]);
          orgParts.push(`rating ${r}★`);
        }
      }

      // Only SET a real color; ignore none/null/empty so the assistant never
      // clears a label just because the model filled the field with "none".
      if (typeof response?.colorLabel === 'string') {
        const c = response.colorLabel.trim().toLowerCase();
        if (VALID_COLORS.has(c)) {
          const curTags = useLibraryStore.getState().imageList.find((i) => i.path === path)?.tags || [];
          const curColor = curTags.find((tg: string) => tg.startsWith('color:'))?.slice(6) || null;
          if (curColor !== c) handleSetColorLabel(c, [path]);
          orgParts.push(`label ${c}`);
        }
      }

      // Rename to the model's filename, or — if the user asked to rename but the
      // model only produced a title — mirror the title into the filename.
      const renameTo = modelFilename || (intent?.rename ? metaPatch['ImageDescription'] || null : null);
      if (renameTo) {
        const newPath = await handleRenameToName(path, renameTo);
        if (newPath) orgParts.push(`renamed to ${newPath.split(/[\\/]/).pop() || renameTo}`);
      }

      return { metaPatch: hasMeta ? metaPatch : null, org: orgParts.length ? orgParts.join(' · ') : null };
    },
    [handleUpdateExif, handleRate, handleSetColorLabel, handleTagsChanged, handleRenameToName],
  );

  // Summarize the active conversation into a single message so future turns carry
  // the gist with far less context. Powered by the same model.
  const compactConversation = useCallback(async () => {
    const st = useAssistantStore.getState();
    const conv = st.conversations.find((c) => c.id === st.activeId);
    const msgs = conv?.messages ?? [];
    if (msgs.length === 0) {
      addMessage({ id: nextMessageId(), role: 'assistant', content: t('editor.assistant.nothingToCompact', 'Nothing to compact yet.') });
      return;
    }
    setLoading(true);
    try {
      const history = msgs.filter((m) => !m.isError).map((m) => ({ role: m.role, content: m.content }));
      history.push({
        role: 'user',
        content:
          'Summarize our conversation so far as a concise briefing that preserves the key facts, decisions, and any established workflow/conventions needed to continue. Do not apply any edits — just write the summary in your reply.',
      });
      const response: any = await invoke(Invokes.AssistantChat, {
        messages: history,
        adjustments: null,
        currentMetadata: null,
        images: [],
        model: selectedModel || null,
      });
      const summary = (response?.reply || '').trim() || t('editor.assistant.summaryUnavailable', '(summary unavailable)');
      replaceActiveMessages([
        {
          id: nextMessageId(),
          role: 'assistant',
          content: `🗜️ ${t('editor.assistant.compacted', 'Compacted summary')}:\n\n${summary}`,
        },
      ]);
      toast.success(t('editor.assistant.compactedToast', 'Conversation compacted'));
    } catch (err: any) {
      addMessage({
        id: nextMessageId(),
        role: 'assistant',
        content: typeof err === 'string' ? err : err?.message || String(err),
        isError: true,
      });
    } finally {
      setLoading(false);
    }
  }, [addMessage, setLoading, replaceActiveMessages, selectedModel, t]);

  const runCommand = useCallback(
    async (raw: string) => {
      const command = raw.slice(1).trim().split(/\s+/)[0].toLowerCase();
      switch (command) {
        case 'compact':
          await compactConversation();
          return;
        case 'clear':
          clearActive();
          return;
        case 'new':
          newConversation();
          return;
        case 'reset':
          clearAll();
          // Belt-and-suspenders: also wipe the persisted copy directly, so the
          // history can't rehydrate from localStorage on the next load.
          try {
            (useAssistantStore as any).persist?.clearStorage?.();
          } catch {
            /* ignore */
          }
          toast.success(t('editor.assistant.resetDone', 'Cleared all chat history.'));
          return;
        case 'help':
        case '?':
          addMessage({
            id: nextMessageId(),
            role: 'assistant',
            content:
              `${t('editor.assistant.commandsTitle', 'Commands')}:\n` +
              SLASH_COMMANDS.map((c) => `${c.cmd} — ${c.desc}`).join('\n'),
          });
          return;
        default:
          addMessage({
            id: nextMessageId(),
            role: 'assistant',
            content: t('editor.assistant.unknownCommand', 'Unknown command "/{{command}}". Type /help.', { command }),
            isError: true,
          });
      }
    },
    [compactConversation, clearActive, clearAll, newConversation, addMessage, t],
  );

  const send = useCallback(async () => {
    const text = input.trim();
    if ((!text && attachments.length === 0) || isLoading) return;
    // Slash commands are handled locally, not sent to the model.
    if (text.startsWith('/') && attachments.length === 0) {
      setInput('');
      await runCommand(text);
      return;
    }

    // Developer mode: the request is about the app, not a photo — hand it to
    // the Claude Code CLI working inside the configured RapidRAW checkout and
    // stream its progress under the spinner.
    if (devMode) {
      setInput('');
      addMessage({ id: nextMessageId(), role: 'user', content: text });
      setLoading(true);
      setDevProgress([]);
      devBusyRef.current = true;
      const unlisten = await listen('assistant-dev-progress', (e: any) => {
        const line = String(e.payload ?? '').trim();
        if (line) setDevProgress((p) => [...p.slice(-11), line]);
      });
      try {
        const result = await invoke<string>(Invokes.AssistantDevChat, { prompt: text });
        addMessage({ id: nextMessageId(), role: 'assistant', content: result || 'Done.' });
      } catch (err: any) {
        addMessage({
          id: nextMessageId(),
          role: 'assistant',
          content: String(err?.message || err),
          isError: true,
        });
      } finally {
        unlisten();
        devBusyRef.current = false;
        setDevProgress([]);
        setLoading(false);
      }
      return;
    }
    cancelRef.current = false;

    const { selectedImage: currentImage, adjustments, finalPreviewUrl, uncroppedAdjustedPreviewUrl } =
      useEditorStore.getState();
    const { multiSelectedPaths: selectedPaths, imageList } = useLibraryStore.getState();
    const outgoing = [...attachments];

    // Scan-preview mode: the film-scanner pane is open with a previewed frame —
    // the assistant drives the scan controls instead of editor adjustments.
    const scanState = useScannerStore.getState();
    const scannerMode =
      useUIStore.getState().isImportViewActive &&
      useImportStore.getState().stage === 'scanner' &&
      !!scanState.previewData;

    // Batch mode: several library images selected and no manual attachment — OCR
    // and apply to each of them individually (matches how the Metadata panel
    // treats a multi-selection). A manual attachment falls back to single.
    // A folder question is about the listing, so it never fans out per image
    // even with several images selected.
    const libraryRequest = !scannerMode && LIBRARY_REQUEST.test((text || '').toLowerCase());
    const doBatch = !scannerMode && !libraryRequest && outgoing.length === 0 && selectedPaths.length > 1;

    const priorUserText = (() => {
      const s = useAssistantStore.getState();
      const msgs = s.conversations.find((c) => c.id === s.activeId)?.messages ?? [];
      return msgs.filter((m) => m.role === 'user').map((m) => m.content);
    })();
    // Images are opt-in (see wantsImage); computed before the badge so the
    // count reflects what is really sent. Manual attachments and the scanner
    // preview (the scan controls are about that frame) are always sent.
    // `let`: a model that asks for the photo ("needsImage") turns images on.
    let imagesOff = !wantsImage(text, priorUserText);

    const viewerUrl = finalPreviewUrl || uncroppedAdjustedPreviewUrl || currentImage?.thumbnailUrl || null;
    const willAttachViewer = scannerMode
      ? outgoing.length === 0
      : !doBatch && !imagesOff && outgoing.length === 0 && !!currentImage && !!viewerUrl;

    const userMessage: AssistantMessage = {
      id: nextMessageId(),
      role: 'user',
      content: text || t('editor.assistant.imageOnly', '(image)'),
      // Count what is actually sent: with scanning ruled out nothing is
      // attached, and showing "4 image(s)" made it look like the app had
      // ignored the instruction.
      imageCount: doBatch
        ? imagesOff
          ? 0
          : selectedPaths.length
        : outgoing.length + (willAttachViewer ? 1 : 0),
    };
    addMessage(userMessage);
    setInput('');
    setAttachments([]);
    setLoading(true);

    const st = useAssistantStore.getState();
    const activeMessages = st.conversations.find((c) => c.id === st.activeId)?.messages ?? [];
    // Error lines (failed runs, per-item batch reports, past refusals) are the
    // app talking to the user, not turns the model authored. Replayed as
    // assistant turns they read as an established finding — one refusal then
    // teaches every later run to refuse the same way.
    const history = activeMessages
      .filter((m) => !m.isError)
      .map((m) => ({
        role: m.role,
        content: m.content,
      }));

    // Infer what the user wants written, scanning recent user turns so "do it
    // again"/"do the same" follow-ups inherit the intent from earlier messages.
    // Used to mirror title<->filename when a weak model fills only one of them.
    const recentUserText = activeMessages
      .filter((m) => m.role === 'user')
      .slice(-8)
      .map((m) => m.content)
      .join('\n')
      .toLowerCase();
    const intent = {
      rename: /\b(rename|renamed|file ?name|file'?s name)\b/.test(recentUserText),
      title: /\btitle\b/.test(recentUserText),
    };
    // Requests that read text off the image get a hard verification gate in
    // batch: values are not accepted from the downscaled overview alone.
    //
    // A user forbidding OCR names it to do so ("don't scan or ocr these"), which
    // a bare keyword match reads as asking for it — the gate then fired and told
    // the model to inspect, contradicting the user's own standing instruction.
    // Arriving as an "[app]" turn it looks precisely like an injection, and the
    // model rightly refused it. So an explicit prohibition wins over the
    // keywords.
    // "increase the number after every two images" is arithmetic over a
    // sequence, and asking the model to carry a counter across independent
    // per-image calls gave it one chance to drift per image. When it repeated a
    // name the collision handler dutifully appended -001, -002 … -035, turning a
    // mistake into a plausible-looking file. The app computes these names
    // instead; the model is left to read the tag and decide content.
    const NUMBER_WORDS: Record<string, number> = { one: 1, two: 2, three: 3, four: 4, five: 5 };
    const numberingRule = recentUserText.match(
      /(?:increase|increment|bump|advance|change)[^.!?]{0,40}?after\s+(?:every\s+)?(one|two|three|four|five|\d+)\s*(?:image|photo|shot|frame)/,
    );
    const imagesPerNumber = numberingRule
      ? (NUMBER_WORDS[numberingRule[1]] ?? parseInt(numberingRule[1], 10)) || null
      : null;

    const ocrIntent =
      !imagesOff &&
      /\b(read|ocr|label|code|weight|gms|gsm|extract|text|number)\b/.test(recentUserText);

    try {
      if (doBatch) {
        const paths = [...selectedPaths];
        // Nudge the model to act on the attached image only, so it OCRs each
        // image's own label instead of reusing values from earlier ones. When
        // the user has ruled scanning out, the nudge drops the OCR half rather
        // than ordering the very thing they forbade.
        const batchNudge = imagesOff
          ? 'Apply the workflow to THIS image only; do not reuse values from other images.'
          : 'Apply the workflow to the ATTACHED image only. Read/OCR its own label; do not reuse values from other images.';
        const batchHistory = history.map((m, i) =>
          i === history.length - 1 && m.role === 'user'
            ? { ...m, content: `${m.content}\n\n${batchNudge}` }
            : m,
        );

        let done = 0;
        // Each image runs in its own conversation, so without this the model has
        // no idea where it sits in the batch. Rules the user states as a sequence
        // ("bump the suffix every two images", "continue the numbering") are then
        // unsatisfiable — not a reasoning failure, just missing information.
        // Position and the previous result are carried across explicitly.
        let batchIndex = 0;
        let previousOutcome: string | null = null;
        // Seeded from the name the model gives the FIRST image, so a code it
        // read off that tag still sets the series; every later name is computed,
        // not proposed.
        let numberSeed: { prefix: string; value: number; digits: number } | null = null;
        for (const path of paths) {
          if (cancelRef.current) break;
          batchIndex += 1;
          const name = (path.split(/[\\/]/).pop() || path).split('?vc=')[0];
          try {
            // "Don't send any images" is taken literally: nothing is decoded or
            // attached, which also skips the inspect loop below (it needs a
            // canvas) and saves the upload on every item in the batch.
            let prepared: any = imagesOff
              ? null
              : await invoke(Invokes.AssistantPrepareImage, {
                  path,
                  maxDim: imageMaxDim,
                });
            let canvas =
              prepared?.fullWidth && prepared?.fullHeight
                ? { width: prepared.fullWidth, height: prepared.fullHeight }
                : null;
            const meta = readCurrentMetadata(imageList.find((i) => i.path === path)?.exif || {});
            // Same inspect loop as single-image chat: small label text ("gms",
            // codes) is often illegible at attachment size — let the model pull
            // a native-resolution region before it commits values to tags.
            // Batch position rides in the structured context, NOT as an "[app]"
            // pseudo-user turn. Appended text claiming to be from the app is
            // indistinguishable from an injection — the model flagged exactly
            // that and refused it, rightly. The context payload is app-supplied
            // by construction and the field is documented in the system prompt,
            // so it carries authority a fabricated user message cannot.
            // 1-based, matching how people count ("every two images").
            const batchContext = {
              index: batchIndex,
              total: paths.length,
              file: name,
              ...(previousOutcome ? { previous: previousOutcome } : {}),
            };
            let itemHistory = batchHistory;
            let itemImages = prepared ? [{ mediaType: prepared.mediaType, data: prepared.data }] : [];
            // App-driven follow-ups (inspect deliveries, the accuracy gate) ride
            // in the structured context like _batch — a chat line claiming app
            // authority is indistinguishable from an injection and gets refused.
            let itemAppTurn: any = null;
            let response: any;
            // The final-chance turn is offered once; without this a model that
            // keeps requesting inspections would loop forever.
            let outOfInspections = false;
            let itemAutoAttached = false;
            for (let round = 0; ; round++) {
              response = await invoke(Invokes.AssistantChat, {
                messages: itemHistory,
                adjustments: {
                  ...(canvas ? { _canvas: canvas } : {}),
                  _batch: batchContext,
                  ...(itemAppTurn ? { _appTurn: itemAppTurn } : {}),
                },
                currentMetadata: meta,
                images: itemImages,
                model: selectedModel || null,
              });
              if (cancelRef.current) break;
              // Asked to see this image: prepare and attach it, then ask again —
              // the user never has to add "scan" to the request.
              if (response?.needsImage && prepared && !itemAutoAttached) {
                itemAutoAttached = true;
                itemAppTurn = { kind: 'image_attached' };
                itemHistory = [
                  ...itemHistory,
                  { role: 'assistant', content: response?.reply || '(needs the photo)' },
                  { role: 'user', content: '(app follow-up turn)' },
                ];
                continue;
              }
              if (response?.needsImage && !prepared && !itemAutoAttached) {
                itemAutoAttached = true;
                prepared = await invoke(Invokes.AssistantPrepareImage, { path, maxDim: imageMaxDim }).catch(
                  () => null,
                );
                if (prepared) {
                  canvas =
                    prepared.fullWidth && prepared.fullHeight
                      ? { width: prepared.fullWidth, height: prepared.fullHeight }
                      : null;
                  itemImages = [{ mediaType: prepared.mediaType, data: prepared.data }];
                  itemAppTurn = { kind: 'image_attached' };
                  itemHistory = [
                    ...itemHistory,
                    { role: 'assistant', content: response?.reply || '(needs the photo)' },
                    { role: 'user', content: '(app follow-up turn)' },
                  ];
                  continue;
                }
              }
              const wantsLabel = wantsLabelInspect(response?.inspect);
              if (wantsLabel && canvas && round < 5) {
                const lab: any = await invoke(Invokes.AssistantPrepareLabel, {
                  path,
                  orientationSteps: 0,
                  maxDim: Math.min(imageMaxDim, 1568),
                  canvasWidth: canvas?.width,
                  canvasHeight: canvas?.height,
                });
                itemAppTurn = lab?.found
                  ? { kind: 'label_attached', x: lab.x, y: lab.y, width: lab.width, height: lab.height }
                  : { kind: 'label_not_found' };
                itemHistory = [
                  ...itemHistory,
                  { role: 'assistant', content: response?.reply || '(inspecting the label)' },
                  { role: 'user', content: '(app follow-up turn)' },
                ];
                itemImages = [{ mediaType: lab.mediaType, data: lab.data }];
                continue;
              }
              const region =
                canvas && round < 5 ? sanitizeCropPatch(response?.inspect, canvas.width, canvas.height) : null;
              if (!region) {
                // Out of inspections while still asking for one. Breaking here
                // applied the last response, which — per the prompt's rule that
                // an inspect turn nulls every other field — carried no values at
                // all. The image was silently skipped while the chat still
                // showed a normal-looking reply. Give it one final turn to
                // commit what it has seen, or to say plainly that it cannot
                // read it.
                const stillWantsToInspect = !!response?.inspect;
                if (stillWantsToInspect && canvas && round >= 5 && !outOfInspections) {
                  outOfInspections = true;
                  itemAppTurn = { kind: 'out_of_inspections' };
                  itemHistory = [
                    ...itemHistory,
                    { role: 'assistant', content: response?.reply || '(inspecting)' },
                    { role: 'user', content: '(app follow-up turn)' },
                  ];
                  continue;
                }
                // Accuracy gate: an OCR-style request that proposed values
                // without a single close-up look gets sent back once.
                const wroteValues = !!response?.metadata || !!response?.tags || !!response?.filename;
                if (ocrIntent && canvas && round === 0 && wroteValues) {
                  itemAppTurn = { kind: 'accuracy_gate' };
                  itemHistory = [
                    ...itemHistory,
                    { role: 'assistant', content: response?.reply || '(values proposed)' },
                    { role: 'user', content: '(app follow-up turn)' },
                  ];
                  continue;
                }
                break;
              }
              const att: any = await invoke(Invokes.AssistantPrepareRegion, {
                path,
                x: region.crop.x,
                y: region.crop.y,
                width: region.crop.width,
                height: region.crop.height,
                orientationSteps: 0,
                maxDim: Math.min(imageMaxDim, 1568),
                canvasWidth: canvas?.width,
                canvasHeight: canvas?.height,
              });
              itemAppTurn = {
                kind: 'region_attached',
                x: region.crop.x,
                y: region.crop.y,
                width: region.crop.width,
                height: region.crop.height,
              };
              itemHistory = [
                ...itemHistory,
                { role: 'assistant', content: response?.reply || '(inspecting)' },
                { role: 'user', content: '(app follow-up turn)' },
              ];
              itemImages = [{ mediaType: att.mediaType, data: att.data }];
              continue;
            }
            if (cancelRef.current) break;

            // App-assigned numbering. The first image seeds the series from
            // whatever the model named it; from then on the name is derived from
            // the batch position, so a 200-image run is exact by construction and
            // a single wrong guess cannot cascade.
            if (imagesPerNumber) {
              const proposed = typeof response?.filename === 'string' ? response.filename : '';
              if (!numberSeed) {
                const m = proposed.match(/^(.*?)(\d+)$/);
                if (m) numberSeed = { prefix: m[1], value: parseInt(m[2], 10), digits: m[2].length };
              }
              if (numberSeed) {
                const step = Math.floor((batchIndex - 1) / imagesPerNumber);
                const within = (batchIndex - 1) % imagesPerNumber;
                const stem =
                  numberSeed.prefix + String(numberSeed.value + step).padStart(numberSeed.digits, '0');
                // Second and later images of a group repeat the number with a
                // suffix — the same shape the collision handler produced, now
                // chosen deliberately rather than as a side effect of a clash.
                const assigned = within === 0 ? stem : `${stem}-${String(within).padStart(3, '0')}`;
                response = {
                  ...response,
                  filename: assigned,
                  // Keep the title on the number, not the suffix: both images of
                  // a pair are the same product.
                  metadata:
                    intent.title && response?.metadata
                      ? { ...response.metadata, ImageDescription: stem }
                      : response?.metadata,
                };
              }
            }

            const { metaPatch, org } = await applyMetaOrg(response, path, intent);
            done += 1;
            // Carry forward what actually landed on disk, not what the model
            // proposed — a rename may have been given a -001 suffix to dodge a
            // collision, and the next image has to continue from the real name.
            previousOutcome =
              [org, metaPatch?.ImageDescription ? `title "${metaPatch.ImageDescription}"` : null]
                .filter(Boolean)
                .join(' · ') || 'no changes';
            // An OCR request that wrote nothing is a skipped image, not a
            // result. It used to read like any other line in the run, so a
            // handful of misses in a long batch went unnoticed until the
            // filenames were checked later.
            //
            // A reply that still asks to inspect carries no values by the
            // prompt's rules, so it is a miss whatever the request was. With
            // images switched off the model can only ask to look and the app can
            // only refuse; say why, or the run reads as "Zooming into the tag…"
            // with nothing behind it.
            const wroteNothing = !metaPatch && !org;
            const skipped = wroteNothing && (ocrIntent || !!response?.inspect);
            addMessage({
              id: nextMessageId(),
              role: 'assistant',
              isError: skipped,
              content: !skipped
                ? `${name}: ${response?.reply || 'done'}`
                : imagesOff && response?.inspect
                  ? t(
                      'editor.assistant.skippedNoImages',
                      '{{name}}: skipped — it needs to see the image, but no image was sent. Images are only sent when you ask it to look, e.g. "scan the tag" or "read the image".',
                      { name },
                    )
                  : `${name}: nothing applied — ${response?.reply || 'no values returned'}`,
              appliedMetadata: metaPatch,
              appliedOrganization: org,
            });
          } catch (err: any) {
            addMessage({
              id: nextMessageId(),
              role: 'assistant',
              content: `${name}: ${typeof err === 'string' ? err : err?.message || String(err)}`,
              isError: true,
            });
          }
        }
        if (cancelRef.current) {
          addMessage({
            id: nextMessageId(),
            role: 'assistant',
            content: t('editor.assistant.stopped', 'Stopped — processed {{done}} of {{total}}.', {
              done,
              total: paths.length,
            }),
          });
        } else {
          toast.success(
            t('editor.assistant.batchDoneToast', 'Processed {{done}}/{{total}} images', { done, total: paths.length }),
          );
        }
        return;
      }

      let images = outgoing.map((a) => ({ mediaType: a.mediaType, data: a.data }));
      if (willAttachViewer) {
        if (scannerMode && scanState.previewData) {
          const preview = dataUrlToImage(scanState.previewData);
          if (preview) images = [preview];
        } else if (viewerUrl) {
          const viewer = await blobUrlToImage(viewerUrl, imageMaxDim);
          if (viewer) images = [viewer];
        }
      }
      const orientedDims =
        currentImage?.width && currentImage?.height
          ? getOrientedDimensions(currentImage.width, currentImage.height, adjustments?.orientationSteps ?? 0)
          : null;
      // The attached viewer image is the CURRENT VIEW (crop applied), so all
      // rectangle talk with the model happens in that space.
      const existingCrop =
        adjustments?.crop && adjustments.crop.width > 0 && adjustments.crop.height > 0 ? adjustments.crop : null;
      const viewCanvas = existingCrop
        ? { width: Math.round(existingCrop.width), height: Math.round(existingCrop.height) }
        : orientedDims;
      const chatContext = scannerMode
        ? scannerContext(scanState)
        : currentImage
          ? {
              ...adjustments,
              // Tells the model the pixel space its crop / inspect rectangles live in.
              _canvas: viewCanvas,
            }
          : null;
      // Folder questions carry the listing; works with no image open too.
      const libraryContext = libraryRequest ? await buildLibraryContext() : null;
      let baseContext: any = libraryContext ? { ...(chatContext || {}), _library: libraryContext } : chatContext;
      // LUT requests carry the installed LUT names, the only ones "lut" accepts.
      if (!scannerMode && currentImage && LUT_REQUEST.test((text || '').toLowerCase())) {
        try {
          const luts = await invoke<LutEntry[]>('list_luts');
          baseContext = { ...(baseContext || {}), _luts: luts.map((l) => l.name) };
        } catch {
          // No library: the model is told none are available.
        }
      }

      // Inspect loop: the model may ask to zoom into a region (small text the
      // downscaled attachment can't resolve). Each round crops that region from
      // the original at native resolution and continues the same conversation.
      const MAX_INSPECT_ROUNDS = 5;
      let loopHistory = history;
      let loopImages = images;
      // Same rule as batch: app follow-ups are structured context, not chat text.
      let loopAppTurn: any = null;
      let autoAttached = false;
      let response: any;
      for (let round = 0; ; round++) {
        response = await invoke(Invokes.AssistantChat, {
          messages: loopHistory,
          adjustments: loopAppTurn ? { ...baseContext, _appTurn: loopAppTurn } : baseContext,
          currentMetadata: scannerMode || !currentImage ? null : readCurrentMetadata(currentImage.exif),
          images: loopImages,
          model: selectedModel || null,
        });
        if (cancelRef.current) break;
        // The model asked to see the photo: attach the view and ask again,
        // once, instead of making the user retype the request with "scan".
        // If the photo WAS attached, the model missed it (with the Claude Code
        // transport it arrives as a file it has to open): say so and re-ask.
        if (response?.needsImage && loopImages.length > 0 && !scannerMode && !autoAttached) {
          autoAttached = true;
          loopHistory = [
            ...loopHistory,
            { role: 'assistant', content: response?.reply || '(needs the photo)' },
            { role: 'user', content: '(app follow-up turn)' },
          ];
          loopAppTurn = { kind: 'image_attached' };
          continue;
        }
        if (response?.needsImage && loopImages.length === 0 && !scannerMode && currentImage && !autoAttached) {
          autoAttached = true;
          const viewer = viewerUrl ? await blobUrlToImage(viewerUrl, imageMaxDim) : null;
          if (viewer) {
            imagesOff = false;
            images = [viewer];
            addMessage({
              id: nextMessageId(),
              role: 'assistant',
              content: t('editor.assistant.lookingAtPhoto', 'Taking a look at the photo…'),
            });
            loopHistory = [
              ...loopHistory,
              { role: 'assistant', content: response?.reply || '(needs the photo)' },
              { role: 'user', content: '(app follow-up turn)' },
            ];
            loopImages = [viewer];
            loopAppTurn = { kind: 'image_attached' };
            continue;
          }
        }
        const wantsLabel = wantsLabelInspect(response?.inspect);
        const region =
          !scannerMode && currentImage && viewCanvas && round < MAX_INSPECT_ROUNDS && !wantsLabel
            ? sanitizeCropPatch(response?.inspect, viewCanvas.width, viewCanvas.height)
            : null;
        const canInspectLabel =
          wantsLabel && !scannerMode && !!currentImage && round < MAX_INSPECT_ROUNDS;
        if (!region && !canInspectLabel) break;
        addMessage({
          id: nextMessageId(),
          role: 'assistant',
          content: response?.reply || t('editor.assistant.inspecting', 'Zooming in for a closer look…'),
        });
        if (!currentImage) break;

        let att: any;
        let note: string;
        if (canInspectLabel) {
          // The app locates the card itself — the model picking coordinates off
          // the downscaled overview tends to bracket the QR block instead.
          const lab: any = await invoke(Invokes.AssistantPrepareLabel, {
            path: currentImage.path,
            orientationSteps: adjustments?.orientationSteps ?? 0,
            maxDim: Math.min(imageMaxDim, 1568),
            canvasWidth: viewCanvas?.width,
            canvasHeight: viewCanvas?.height,
          });
          att = lab;
          loopAppTurn = lab?.found
            ? { kind: 'label_attached', x: lab.x, y: lab.y, width: lab.width, height: lab.height }
            : { kind: 'label_not_found' };
          note = '(app follow-up turn)';
        } else {
          att = await invoke(Invokes.AssistantPrepareRegion, {
            path: currentImage.path,
            x: region!.crop.x,
            y: region!.crop.y,
            width: region!.crop.width,
            height: region!.crop.height,
            orientationSteps: adjustments?.orientationSteps ?? 0,
            maxDim: Math.min(imageMaxDim, 1568),
            canvasWidth: viewCanvas?.width,
            canvasHeight: viewCanvas?.height,
          });
          loopAppTurn = {
            kind: 'region_attached',
            x: region!.crop.x,
            y: region!.crop.y,
            width: region!.crop.width,
            height: region!.crop.height,
          };
          note = '(app follow-up turn)';
        }
        loopHistory = [
          ...loopHistory,
          { role: 'assistant', content: response?.reply || '(inspecting)' },
          { role: 'user', content: note },
        ];
        loopImages = [{ mediaType: att.mediaType, data: att.data }];
      }

      if (cancelRef.current) {
        addMessage({ id: nextMessageId(), role: 'assistant', content: t('editor.assistant.stoppedShort', 'Stopped.') });
        return;
      }
      // Still only asking for the photo after it was given: without this the
      // chat ends on "Taking a look…" and nothing happens, which reads as a hang.
      if (response?.needsImage) {
        addMessage({
          id: nextMessageId(),
          role: 'assistant',
          isError: true,
          content: t(
            'editor.assistant.couldNotSeePhoto',
            "The assistant couldn't read the photo, so nothing was changed. Please try the request again.",
          ),
        });
        return;
      }

      if (scannerMode) {
        const applied = applyScannerPatch(response?.adjustments);
        const hasScanPatch = Object.keys(applied).length > 0;
        if (hasScanPatch) await rerenderScanPreviewNow();
        addMessage({
          id: nextMessageId(),
          role: 'assistant',
          content: response?.reply || t('editor.assistant.emptyReply', 'Done.'),
          appliedAdjustments: hasScanPatch ? applied : null,
        });
        return;
      }

      const baseAdjustments: any = useEditorStore.getState().adjustments;
      const sanitized = currentImage
        ? sanitizeAdjustments(response?.adjustments, baseAdjustments)
        : { patch: {} as Record<string, any>, changes: {} as Record<string, AdjustmentValue> };
      const patch = sanitized.patch;
      const changes = sanitized.changes;
      const viewCropPatch =
        currentImage && viewCanvas ? sanitizeCropPatch(response?.crop, viewCanvas.width, viewCanvas.height) : null;
      // A rect proposed inside the current view refines the existing crop, so
      // offset it back into absolute (oriented full-image) coordinates.
      const cropPatch =
        viewCropPatch && orientedDims
          ? (() => {
              const baseX = existingCrop ? Math.round(existingCrop.x) : 0;
              const baseY = existingCrop ? Math.round(existingCrop.y) : 0;
              const x = Math.min(baseX + viewCropPatch.crop.x, orientedDims.width - 1);
              const y = Math.min(baseY + viewCropPatch.crop.y, orientedDims.height - 1);
              const width = Math.min(viewCropPatch.crop.width, orientedDims.width - x);
              const height = Math.min(viewCropPatch.crop.height, orientedDims.height - y);
              return { crop: { unit: 'px' as const, x, y, width, height }, aspectRatio: width / height };
            })()
          : null;
      const hasPatch = Object.keys(patch).length > 0 || !!cropPatch;
      if (hasPatch) {
        setAdjustments((prev: any) => ({ ...prev, ...patch, ...(cropPatch ?? {}) }));
      }
      if (cropPatch) {
        toast.success(
          t('editor.assistant.croppedToast', 'Cropped to {{width}} × {{height}}', {
            width: cropPatch.crop.width,
            height: cropPatch.crop.height,
          }),
        );
      }

      // Visual extras: masks (incl. brush), object removal, point colours,
      // LUT, lens blur. Coordinates are in the view the model saw, so they're
      // offset by the crop that was in place then.
      const maskLabels: string[] = [];
      let extrasApplied: Record<string, any> | null = null;
      const maskCtx: MaskContext | null = currentImage
        ? {
            path: currentImage.path,
            adjustments: { ...baseAdjustments, ...patch, ...(cropPatch ?? {}) },
            offsetX: existingCrop ? Math.round(existingCrop.x) : 0,
            offsetY: existingCrop ? Math.round(existingCrop.y) : 0,
            canvas: viewCanvas,
          }
        : null;
      const extraOpts = {
        getToken: async () => (await getToken()) ?? null,
        isCancelled: () => cancelRef.current,
      };
      if (maskCtx) {
        const before = maskCtx.adjustments;
        const res = await applyVisualExtras(response, before, maskCtx, {
          ...extraOpts,
          create: true,
          viewImage: images[0] ?? null,
        });
        maskLabels.push(...res.labels);
        if (res.changed) {
          const applied = changedKeys(res.next, before);
          extrasApplied = applied;
          setAdjustments((prev: any) => ({ ...prev, ...applied }));
        }
      }

      // Result check for every visual edit (geometry, masks, removals, point
      // colours, LUT, lens blur): the model sees the rendered edit and may
      // correct it (absolute values) until it's satisfied or the rounds run
      // out. The backend renders the check image itself, so this runs even
      // when the request sent no photo — the model verifies its own work
      // instead of asking the user to "scan" afterwards.
      let finalReply: string = response?.reply || t('editor.assistant.emptyReply', 'Done.');
      if (
        currentImage &&
        (Object.keys(changes).some((k) => GEOMETRY_KEYS.has(k)) || extrasApplied !== null) &&
        !cancelRef.current
      ) {
        addMessage({
          id: nextMessageId(),
          role: 'assistant',
          content: finalReply,
          appliedAdjustments: Object.keys(changes).length ? { ...changes } : null,
          appliedOrganization: maskLabels.length ? maskLabels.join(' · ') : null,
        });
        let current: any = {
          ...baseAdjustments,
          ...patch,
          ...(cropPatch ?? {}),
          ...(extrasApplied ?? {}),
        };
        let reviewHistory = [...loopHistory, { role: 'assistant', content: finalReply }];
        for (let round = 1; round <= MAX_REVIEW_ROUNDS && !cancelRef.current; round++) {
          addMessage({
            id: nextMessageId(),
            role: 'assistant',
            content: t('editor.assistant.checkingResult', 'Checking the result…'),
          });
          const rendered = await renderForReview(currentImage.path, current, imageMaxDim);
          if (!rendered || cancelRef.current) break;
          reviewHistory = [...reviewHistory, { role: 'user', content: '(app follow-up turn)' }];
          const review: any = await invoke(Invokes.AssistantChat, {
            messages: reviewHistory,
            adjustments: {
              ...current,
              _canvas: viewCanvas,
              _appTurn: { kind: 'result_attached', round, maxRounds: MAX_REVIEW_ROUNDS },
            },
            currentMetadata: readCurrentMetadata(currentImage.exif),
            images: [rendered],
            model: selectedModel || null,
          });
          if (cancelRef.current) break;
          finalReply = review?.reply || finalReply;
          reviewHistory = [...reviewHistory, { role: 'assistant', content: review?.reply || '(reviewed)' }];
          const next = sanitizeAdjustments(review?.adjustments, current);
          if (Object.keys(next.patch).length > 0) {
            setAdjustments((prev: any) => ({ ...prev, ...next.patch }));
            current = { ...current, ...next.patch };
            Object.assign(patch, next.patch);
            Object.assign(changes, next.changes);
          }
          // Review rounds may tune what exists (masks, point colours, LUT
          // strength, lens blur) but not add new masks or removals.
          const fix = maskCtx
            ? await applyVisualExtras(review, current, maskCtx, { ...extraOpts, create: false, viewImage: rendered })
            : { next: current, labels: [] as string[], changed: false };
          if (fix.changed) {
            const applied = changedKeys(fix.next, current);
            setAdjustments((prev: any) => ({ ...prev, ...applied }));
            current = fix.next;
            maskLabels.push(...fix.labels);
          }
          if (Object.keys(next.patch).length === 0 && !fix.changed) break;
        }
      }

      let metaPatch: Record<string, string> | null = null;
      let appliedOrganization: string | null = null;
      if (currentImage) {
        const res = await applyMetaOrg(response, currentImage.path, intent);
        metaPatch = res.metaPatch;
        appliedOrganization = res.org;
      }
      if (maskLabels.length > 0) {
        const masksNote = maskLabels.join(' · ');
        appliedOrganization = appliedOrganization ? `${appliedOrganization} · ${masksNote}` : masksNote;
      }

      // "select": filenames from _library the model picked out. Selecting them
      // makes the user's next request run over exactly those images as a batch
      // — the chat can't write other images directly, but it can hand them over.
      if (Array.isArray(response?.select) && response.select.length > 0) {
        const wanted = new Set(response.select.map((f: any) => String(f).trim().toLowerCase()));
        const { imageList: list, setLibrary } = useLibraryStore.getState();
        const picked = list.filter((img) => wanted.has(fileNameOf(img.path).toLowerCase())).map((img) => img.path);
        if (picked.length > 0) {
          setLibrary({ multiSelectedPaths: picked });
          const note = `selected ${picked.length} image${picked.length === 1 ? '' : 's'}`;
          appliedOrganization = appliedOrganization ? `${appliedOrganization} · ${note}` : note;
        }
      }

      if (metaPatch || appliedOrganization) {
        const summary = [
          ...Object.entries(metaPatch || {}).map(([k, v]) => `${EXIF_TO_FRIENDLY[k] || k}: ${v || '(cleared)'}`),
          ...(appliedOrganization ? [appliedOrganization] : []),
        ].join(' · ');
        toast.success(t('editor.assistant.appliedToast', 'Updated {{summary}}', { summary }));
      }

      addMessage({
        id: nextMessageId(),
        role: 'assistant',
        content: finalReply,
        appliedAdjustments: Object.keys(changes).length > 0 ? changes : null,
        appliedMetadata: metaPatch,
        appliedOrganization,
      });
    } catch (err: any) {
      addMessage({
        id: nextMessageId(),
        role: 'assistant',
        content: typeof err === 'string' ? err : err?.message || String(err),
        isError: true,
      });
    } finally {
      setLoading(false);
    }
  }, [
    input,
    attachments,
    isLoading,
    runCommand,
    addMessage,
    setLoading,
    setAdjustments,
    applyMetaOrg,
    selectedModel,
    imageMaxDim,
    t,
    devMode,
  ]);

  const stop = useCallback(() => {
    cancelRef.current = true;
    if (devBusyRef.current) {
      invoke(Invokes.AssistantDevCancel).catch(() => {});
    }
  }, []);

  const handleKeyDown = (e: any) => {
    e.stopPropagation();
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      setHistoryIndex(null);
      send();
      return;
    }
    // Up/Down recall previously sent messages. Up only enters recall from an
    // empty input, so it never hijacks caret movement while composing.
    if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      const st = useAssistantStore.getState();
      const sent = (st.conversations.find((c) => c.id === st.activeId)?.messages ?? [])
        .filter((m) => m.role === 'user' && !m.isError)
        .map((m) => m.content);
      if (e.key === 'ArrowUp' && (historyIndex !== null || input === '')) {
        if (sent.length === 0) return;
        e.preventDefault();
        if (historyIndex === null) historyDraftRef.current = input;
        const next = historyIndex === null ? sent.length - 1 : Math.max(0, historyIndex - 1);
        setHistoryIndex(next);
        setInput(sent[next]);
      } else if (e.key === 'ArrowDown' && historyIndex !== null) {
        e.preventDefault();
        const next = historyIndex + 1;
        if (next >= sent.length) {
          setHistoryIndex(null);
          setInput(historyDraftRef.current);
        } else {
          setHistoryIndex(next);
          setInput(sent[next]);
        }
      }
    }
  };

  const onModelChange = (value: string) => {
    handleSettingsChange({ ...(appSettings as any), assistantModel: value });
  };

  return (
    <div className="flex flex-col h-full">
      <div className="p-3 flex justify-between items-center shrink-0 border-b border-surface gap-2">
        <Text variant={TextVariants.title}>{t('editor.assistant.title', 'Assistant')}</Text>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={() => {
              newConversation();
              setHistoryOpen(false);
            }}
            title={t('editor.assistant.newChat', 'New chat')}
            className="p-1.5 rounded-md hover:bg-surface text-text-secondary hover:text-text-primary transition-colors"
          >
            <MessageSquarePlus size={16} />
          </button>
          <button
            type="button"
            onClick={() => setHistoryOpen((v) => !v)}
            title={t('editor.assistant.history', 'Chat history')}
            className={clsx(
              'p-1.5 rounded-md hover:bg-surface transition-colors',
              historyOpen ? 'text-accent' : 'text-text-secondary hover:text-text-primary',
            )}
          >
            <History size={16} />
          </button>
          {messages.length > 0 && (
            <button
              type="button"
              onClick={clearActive}
              title={t('editor.assistant.clear', 'Clear conversation')}
              className="p-1.5 rounded-md hover:bg-surface text-text-secondary hover:text-text-primary transition-colors"
            >
              <Trash2 size={16} />
            </button>
          )}
        </div>
      </div>

      {historyOpen && (
        <div className="shrink-0 border-b border-surface max-h-64 overflow-y-auto custom-scrollbar">
          {conversations.length === 0 ? (
            <div className="px-3 py-3">
              <Text color={TextColors.secondary} className="text-xs">
                {t('editor.assistant.noHistory', 'No conversations yet.')}
              </Text>
            </div>
          ) : (
            conversations.map((c) => (
              <div
                key={c.id}
                onClick={() => {
                  if (renamingId !== c.id) {
                    selectConversation(c.id);
                    setHistoryOpen(false);
                  }
                }}
                className={clsx(
                  'group flex items-center gap-2 px-3 py-2 cursor-pointer transition-colors',
                  c.id === activeId ? 'bg-surface' : 'hover:bg-surface/60',
                )}
              >
                {renamingId === c.id ? (
                  <>
                    <input
                      autoFocus
                      value={renameValue}
                      onChange={(e) => setRenameValue(e.target.value)}
                      onClick={(e) => e.stopPropagation()}
                      onKeyDown={(e) => {
                        e.stopPropagation();
                        if (e.key === 'Enter') commitRename();
                        if (e.key === 'Escape') setRenamingId(null);
                      }}
                      className="grow min-w-0 rounded-sm bg-bg-primary border border-accent px-1.5 py-0.5 text-xs text-text-primary focus:outline-none"
                    />
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        commitRename();
                      }}
                      title={t('editor.assistant.saveName', 'Save')}
                      className="p-1 rounded-sm text-text-secondary hover:text-accent shrink-0"
                    >
                      <Check size={13} />
                    </button>
                  </>
                ) : (
                  <>
                    <MessageSquare size={13} className="shrink-0 text-text-secondary" />
                    <span className="grow truncate text-xs text-text-primary">{c.title}</span>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        startRename(c);
                      }}
                      title={t('editor.assistant.rename', 'Rename')}
                      className="p-1 rounded-sm text-text-secondary hover:text-text-primary shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
                    >
                      <Pencil size={12} />
                    </button>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteConversation(c.id);
                      }}
                      title={t('editor.assistant.delete', 'Delete')}
                      className="p-1 rounded-sm text-text-secondary hover:text-red-400 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
                    >
                      <Trash2 size={12} />
                    </button>
                  </>
                )}
              </div>
            ))
          )}
        </div>
      )}

      {/* Model picker */}
      <div className="px-3 py-2 flex items-center gap-2 shrink-0 border-b border-surface">
        <Text color={TextColors.secondary} className="text-xs shrink-0">
          {t('editor.assistant.model', 'Model')}
        </Text>
        <select
          value={selectedModel}
          onChange={(e) => onModelChange(e.target.value)}
          onKeyDown={(e) => e.stopPropagation()}
          className="grow min-w-0 rounded-md bg-bg-primary border border-border-color px-2 py-1 text-xs text-text-primary focus:outline-none focus:border-accent"
        >
          <option value="">{t('editor.assistant.defaultModel', 'Provider default')}</option>
          {selectedModel && !models.includes(selectedModel) && <option value={selectedModel}>{selectedModel}</option>}
          {models.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
        <button
          type="button"
          onClick={refreshModels}
          title={t('editor.assistant.refreshModels', 'Refresh models')}
          className="p-1 rounded-md hover:bg-surface text-text-secondary hover:text-text-primary transition-colors shrink-0"
        >
          <RefreshCw size={14} />
        </button>
        {/* Developer mode only makes sense against the dev server, where the
            source edits hot-reload into the running app — a packaged build
            would edit the checkout without ever showing the result. */}
        {import.meta.env.DEV && (
          <button
            type="button"
            onClick={() => setDevMode((v) => !v)}
            title={t(
              'editor.assistant.devModeTip',
              'Developer mode — change the app itself: edits the RapidRAW source, verifies, commits, and pushes',
            )}
            className={clsx(
              'p-1 rounded-md transition-colors shrink-0',
              devMode
                ? 'bg-accent text-button-text'
                : 'hover:bg-surface text-text-secondary hover:text-text-primary',
            )}
          >
            <Wrench size={14} />
          </button>
        )}
      </div>
      {modelsError && (
        <div className="px-3 pt-2 shrink-0">
          <Text color={TextColors.secondary} className="text-xs">
            {t('editor.assistant.modelsError', "Couldn't list models — check the provider in Settings.")}
          </Text>
        </div>
      )}

      <div ref={scrollRef} className="grow overflow-y-auto p-3 custom-scrollbar flex flex-col gap-3">
        {messages.length === 0 && (
          <div className="flex flex-col items-center justify-center h-full text-center gap-3 px-4">
            <div className="p-3 rounded-full bg-surface">
              <Bot size={28} className="text-accent" />
            </div>
            <Text variant={TextVariants.heading}>{t('editor.assistant.emptyTitle', 'Chat with the editor')}</Text>
            <Text color={TextColors.secondary} className="text-sm">
              {t(
                'editor.assistant.emptyBody',
                'Ask for edits in plain language, like “warm it up and lift the shadows”, and I’ll adjust the sliders on the open image. You can paste or attach a reference image too.',
              )}
            </Text>
            <Text color={TextColors.secondary} className="text-xs mt-1">
              {t('editor.assistant.providerHint', 'Using {{provider}} — change it in Settings → AI Assistant.', {
                provider: providerLabel,
              })}
            </Text>
          </div>
        )}

        {messages.map((m) => (
          <div key={m.id} className={clsx('flex', m.role === 'user' ? 'justify-end' : 'justify-start')}>
            <div
              className={clsx(
                'max-w-[85%] rounded-xl px-3 py-2 text-sm whitespace-pre-wrap break-words select-text cursor-text',
                m.role === 'user' && 'bg-accent text-button-text',
                m.role === 'assistant' && !m.isError && 'bg-surface text-text-primary',
                m.isError && 'bg-surface border border-red-500/50 text-text-primary',
              )}
            >
              {m.isError && (
                <div className="flex items-center gap-1.5 mb-1 text-red-400">
                  <AlertTriangle size={13} />
                  <span className="text-xs font-semibold">{t('editor.assistant.error', 'Error')}</span>
                </div>
              )}
              {m.content}
              {!!m.imageCount && (
                <div className="mt-1.5 flex items-center gap-1.5 text-xs opacity-80">
                  <Paperclip size={12} />
                  <span>{t('editor.assistant.attachedCount', '{{count}} image(s)', { count: m.imageCount })}</span>
                </div>
              )}
              {m.appliedAdjustments && (
                <div className="mt-2 pt-2 border-t border-border-color/40 flex items-start gap-1.5 text-xs text-text-secondary">
                  <Sparkles size={13} className="mt-0.5 shrink-0 text-accent" />
                  <span>{formatPatch(m.appliedAdjustments)}</span>
                </div>
              )}
              {m.appliedMetadata && (
                <div className="mt-2 pt-2 border-t border-border-color/40 flex items-start gap-1.5 text-xs text-text-secondary">
                  <Tag size={13} className="mt-0.5 shrink-0 text-accent" />
                  <span>{formatMetadata(m.appliedMetadata)}</span>
                </div>
              )}
              {m.appliedOrganization && (
                <div className="mt-2 pt-2 border-t border-border-color/40 flex items-start gap-1.5 text-xs text-text-secondary">
                  <Tag size={13} className="mt-0.5 shrink-0 text-accent" />
                  <span>{m.appliedOrganization}</span>
                </div>
              )}
            </div>
          </div>
        ))}

        {isLoading && (
          <div className="flex justify-start">
            <div className="bg-surface rounded-xl px-3 py-2 flex items-center gap-2 text-text-secondary text-sm">
              <Loader2 size={15} className="animate-spin" />
              {t('editor.assistant.thinking', 'Thinking…')}
            </div>
          </div>
        )}
        {isLoading && devProgress.length > 0 && (
          <div className="pl-1 space-y-0.5">
            {devProgress.map((l, i) => (
              <div key={i} className="text-[11px] text-text-secondary font-mono truncate">
                {l}
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="p-3 border-t border-surface shrink-0">
        {input.startsWith('/') && (
          <div className="mb-2 rounded-lg border border-border-color bg-bg-primary overflow-hidden">
            {SLASH_COMMANDS.filter((c) => c.cmd.startsWith(input.split(/\s+/)[0].toLowerCase())).map((c) => (
              <button
                key={c.cmd}
                type="button"
                onClick={() => {
                  setInput('');
                  void runCommand(c.cmd);
                }}
                className="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-surface transition-colors text-left"
              >
                <span className="text-xs font-semibold text-accent shrink-0">{c.cmd}</span>
                <Text color={TextColors.secondary} className="text-xs truncate">
                  {c.desc}
                </Text>
              </button>
            ))}
          </div>
        )}

        {devMode && (
          <div className="flex items-center gap-1.5 mb-2 px-1 text-accent">
            <Wrench size={13} className="shrink-0" />
            <Text color={TextColors.secondary} className="text-xs">
              {appSettings?.assistantDevRepoPath
                ? t('editor.assistant.devModeOn', 'Developer mode — changes the app source at {{path}}', {
                    path: appSettings.assistantDevRepoPath,
                  })
                : t(
                    'editor.assistant.devModeNoRepo',
                    'Developer mode — set the repository path in Settings → AI Assistant first',
                  )}
            </Text>
          </div>
        )}

        {!devMode && selectedCount > 1 && attachments.length === 0 && (
          <div className="flex items-center gap-1.5 mb-2 px-1 text-accent">
            <Layers size={13} className="shrink-0" />
            <Text color={TextColors.secondary} className="text-xs">
              {t('editor.assistant.applyingToSelected', 'Will apply to all {{count}} selected images', {
                count: selectedCount,
              })}
            </Text>
          </div>
        )}

        {scannerOpen ? (
          <Text color={TextColors.secondary} className="text-xs mb-2">
            {scanPreviewReady
              ? t('editor.assistant.scanPreview', 'Editing the scan preview — ask for exposure, contrast, or color changes.')
              : t('editor.assistant.scanNoPreview', 'Run a preview to let the assistant tune the scan.')}
          </Text>
        ) : (
          !selectedImage &&
          selectedCount <= 1 && (
            <Text color={TextColors.secondary} className="text-xs mb-2">
              {t('editor.assistant.noImage', 'Open an image to let the assistant apply edits.')}
            </Text>
          )
        )}

        {attachments.length > 0 && (
          <div className="flex flex-wrap gap-2 mb-2">
            {attachments.map((a) => (
              <div key={a.id} className="relative group">
                <img src={a.dataUrl} alt="attachment" className="h-14 w-14 object-cover rounded-md border border-border-color" />
                <button
                  type="button"
                  onClick={() => setAttachments((prev) => prev.filter((x) => x.id !== a.id))}
                  className="absolute -top-1.5 -right-1.5 bg-bg-primary border border-border-color rounded-full p-0.5 text-text-secondary hover:text-text-primary"
                  title={t('editor.assistant.removeImage', 'Remove')}
                >
                  <X size={12} />
                </button>
              </div>
            ))}
          </div>
        )}

        <div className="flex items-end gap-2">
          <input
            ref={fileInputRef}
            type="file"
            accept="image/*"
            multiple
            className="hidden"
            onChange={(e) => {
              addFiles(Array.from(e.target.files || []));
              e.target.value = '';
            }}
          />
          <button
            type="button"
            onClick={() => fileInputRef.current?.click()}
            title={t('editor.assistant.attach', 'Attach image')}
            className="p-2.5 rounded-lg bg-surface text-text-secondary hover:text-text-primary transition-colors shrink-0"
          >
            <Paperclip size={16} />
          </button>
          <textarea
            value={input}
            onChange={(e) => {
              setInput(e.target.value);
              setHistoryIndex(null);
            }}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
            placeholder={
              devMode
                ? t('editor.assistant.devPlaceholder', 'Describe the app change…')
                : t('editor.assistant.placeholder', 'Ask for an edit…')
            }
            rows={2}
            className="grow resize-none rounded-lg bg-bg-primary border border-border-color px-3 py-2 text-sm text-text-primary placeholder:text-text-secondary focus:outline-none focus:border-accent custom-scrollbar"
          />
          {isLoading ? (
            <button
              type="button"
              onClick={stop}
              title={t('editor.assistant.stop', 'Stop')}
              className="p-2.5 rounded-lg bg-surface text-text-primary border border-border-color hover:text-red-400 hover:border-red-400/50 transition-all shrink-0"
            >
              <Square size={16} className="fill-current" />
            </button>
          ) : (
            <button
              type="button"
              onClick={send}
              disabled={!input.trim() && attachments.length === 0}
              title={t('editor.assistant.send', 'Send')}
              className="p-2.5 rounded-lg bg-accent text-button-text disabled:opacity-40 disabled:cursor-not-allowed hover:brightness-110 transition-all shrink-0"
            >
              <Send size={16} />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
