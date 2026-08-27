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
} from 'lucide-react';
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

// The develop-slider fields the assistant is allowed to set, with their valid
// ranges. Values coming back from the model are clamped to these before we
// apply them, so a hallucinated 999 can't push a slider out of bounds.
const ADJUSTMENT_RANGES: Record<string, [number, number]> = {
  exposure: [-5, 5],
  contrast: [-100, 100],
  highlights: [-100, 100],
  shadows: [-100, 100],
  whites: [-100, 100],
  blacks: [-100, 100],
  temperature: [-100, 100],
  tint: [-100, 100],
  vibrance: [-100, 100],
  saturation: [-100, 100],
  hue: [-180, 180],
  clarity: [-100, 100],
  dehaze: [-100, 100],
  structure: [-100, 100],
  sharpness: [-100, 100],
};

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

function sanitizePatch(raw: any): Record<string, number> {
  const out: Record<string, number> = {};
  if (!raw || typeof raw !== 'object') return out;
  for (const [key, value] of Object.entries(raw)) {
    const range = ADJUSTMENT_RANGES[key];
    const num = typeof value === 'number' ? value : parseFloat(value as any);
    if (!range || !Number.isFinite(num)) continue;
    out[key] = Math.max(range[0], Math.min(range[1], num));
  }
  return out;
}

// Validate + clamp a model-proposed crop rectangle (pixels, oriented image
// space). Returns null for anything degenerate so a hallucinated rect can't
// blank the image.
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

function formatPatch(patch: Record<string, number>): string {
  return Object.entries(patch)
    .map(([k, v]) => `${k} ${v > 0 ? '+' : ''}${v}`)
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

async function fileToAttachment(file: File, maxDim: number): Promise<Attachment> {
  // Downscale manual attachments too (a File is a Blob), so a full-size photo
  // doesn't blow up the request body — matches the viewer/batch paths.
  const { mediaType, data } = await downscaleBlob(file, maxDim);
  return { id: nextMessageId(), dataUrl: `data:${mediaType};base64,${data}`, mediaType, data };
}

export default function AssistantPanel() {
  const { t } = useTranslation();
  const [input, setInput] = useState('');
  const [attachments, setAttachments] = useState<Array<Attachment>>([]);
  const [models, setModels] = useState<Array<string>>([]);
  const [modelsError, setModelsError] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  // Set by the Stop button; checked between batch images and before applying a
  // single response so an in-flight run can be abandoned.
  const cancelRef = useRef(false);

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
      const history = msgs.map((m) => ({ role: m.role, content: m.content }));
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
    const doBatch = !scannerMode && outgoing.length === 0 && selectedPaths.length > 1;

    const viewerUrl = finalPreviewUrl || uncroppedAdjustedPreviewUrl || currentImage?.thumbnailUrl || null;
    const willAttachViewer = scannerMode
      ? outgoing.length === 0
      : !doBatch && outgoing.length === 0 && !!currentImage && !!viewerUrl;

    const userMessage: AssistantMessage = {
      id: nextMessageId(),
      role: 'user',
      content: text || t('editor.assistant.imageOnly', '(image)'),
      imageCount: doBatch ? selectedPaths.length : outgoing.length + (willAttachViewer ? 1 : 0),
    };
    addMessage(userMessage);
    setInput('');
    setAttachments([]);
    setLoading(true);

    const st = useAssistantStore.getState();
    const activeMessages = st.conversations.find((c) => c.id === st.activeId)?.messages ?? [];
    const history = activeMessages.map((m) => ({
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

    try {
      if (doBatch) {
        const paths = [...selectedPaths];
        // Nudge the model to act on the attached image only, so it OCRs each
        // image's own label instead of reusing values from earlier ones.
        const batchHistory = history.map((m, i) =>
          i === history.length - 1 && m.role === 'user'
            ? {
                ...m,
                content: `${m.content}\n\nApply the workflow to the ATTACHED image only. Read/OCR its own label; do not reuse values from other images.`,
              }
            : m,
        );

        let done = 0;
        for (const path of paths) {
          if (cancelRef.current) break;
          const name = (path.split(/[\\/]/).pop() || path).split('?vc=')[0];
          try {
            const prepared: any = await invoke(Invokes.AssistantPrepareImage, {
              path,
              maxDim: imageMaxDim,
            });
            const response: any = await invoke(Invokes.AssistantChat, {
              messages: batchHistory,
              adjustments: null,
              currentMetadata: readCurrentMetadata(imageList.find((i) => i.path === path)?.exif || {}),
              images: [{ mediaType: prepared.mediaType, data: prepared.data }],
              model: selectedModel || null,
            });
            if (cancelRef.current) break;
            const { metaPatch, org } = await applyMetaOrg(response, path, intent);
            done += 1;
            addMessage({
              id: nextMessageId(),
              role: 'assistant',
              content: `${name}: ${response?.reply || 'done'}`,
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
      const response: any = await invoke(Invokes.AssistantChat, {
        messages: history,
        adjustments: scannerMode
          ? scannerContext(scanState)
          : currentImage
            ? {
                ...adjustments,
                // Tells the model the pixel space its crop rectangle lives in.
                _canvas:
                  currentImage.width && currentImage.height
                    ? getOrientedDimensions(currentImage.width, currentImage.height, adjustments.orientationSteps ?? 0)
                    : null,
              }
            : null,
        currentMetadata: scannerMode || !currentImage ? null : readCurrentMetadata(currentImage.exif),
        images,
        model: selectedModel || null,
      });

      if (cancelRef.current) {
        addMessage({ id: nextMessageId(), role: 'assistant', content: t('editor.assistant.stoppedShort', 'Stopped.') });
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

      const patch = currentImage ? sanitizePatch(response?.adjustments) : {};
      const orientedDims =
        currentImage?.width && currentImage?.height
          ? getOrientedDimensions(currentImage.width, currentImage.height, adjustments?.orientationSteps ?? 0)
          : null;
      const cropPatch =
        currentImage && orientedDims
          ? sanitizeCropPatch(response?.crop, orientedDims.width, orientedDims.height)
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

      let metaPatch: Record<string, string> | null = null;
      let appliedOrganization: string | null = null;
      if (currentImage) {
        const res = await applyMetaOrg(response, currentImage.path, intent);
        metaPatch = res.metaPatch;
        appliedOrganization = res.org;
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
        content: response?.reply || t('editor.assistant.emptyReply', 'Done.'),
        appliedAdjustments: hasPatch ? patch : null,
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
  ]);

  const stop = useCallback(() => {
    cancelRef.current = true;
  }, []);

  const handleKeyDown = (e: any) => {
    e.stopPropagation();
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      send();
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

        {selectedCount > 1 && attachments.length === 0 && (
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
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
            placeholder={t('editor.assistant.placeholder', 'Ask for an edit…')}
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
