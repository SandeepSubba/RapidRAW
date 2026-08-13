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
    out[exifKey] = String(value).trim();
  }
  return out;
}

const VALID_COLORS = new Set(['red', 'yellow', 'green', 'blue', 'purple']);

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

function formatPatch(patch: Record<string, number>): string {
  return Object.entries(patch)
    .map(([k, v]) => `${k} ${v > 0 ? '+' : ''}${v}`)
    .join(', ');
}

// Fetch a blob: URL (the viewer's processed preview) and turn it into the
// base64 payload the backend expects, so the assistant can "see" the open image.
async function blobUrlToImage(url: string): Promise<{ mediaType: string; data: string } | null> {
  try {
    const resp = await fetch(url);
    const blob = await resp.blob();
    const dataUrl: string = await new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = reject;
      reader.readAsDataURL(blob);
    });
    const comma = dataUrl.indexOf(',');
    const data = comma >= 0 ? dataUrl.slice(comma + 1) : dataUrl;
    const mediaType = dataUrl.slice(5, dataUrl.indexOf(';')) || blob.type || 'image/png';
    return { mediaType, data };
  } catch {
    return null;
  }
}

function fileToAttachment(file: File): Promise<Attachment> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const dataUrl = reader.result as string;
      const comma = dataUrl.indexOf(',');
      const data = comma >= 0 ? dataUrl.slice(comma + 1) : dataUrl;
      const mediaType = dataUrl.slice(5, dataUrl.indexOf(';')) || file.type || 'image/png';
      resolve({ id: nextMessageId(), dataUrl, mediaType, data });
    };
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
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
  const providerLabel = provider === 'openai' ? 'OpenAI' : provider === 'anthropic' ? 'Anthropic' : 'LM Studio';
  const selectedModel = appSettings?.assistantModel || '';

  const { setAdjustments } = useEditorActions();
  const { handleUpdateExif, handleRate, handleSetColorLabel, handleTagsChanged, handleRenameToName } =
    useLibraryActions();
  const selectedImage = useEditorStore((s) => s.selectedImage);
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

  const addFiles = useCallback((files: Array<File>) => {
    const images = files.filter((f) => f.type.startsWith('image/'));
    if (images.length === 0) return;
    Promise.all(images.map(fileToAttachment)).then((atts) =>
      setAttachments((prev) => [...prev, ...atts]),
    );
  }, []);

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
    async (response: any, path: string): Promise<{ metaPatch: Record<string, string> | null; org: string | null }> => {
      const metaPatch = sanitizeMetadata(response?.metadata);
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

      if (typeof response?.rating === 'number') {
        const r = Math.max(0, Math.min(5, Math.round(response.rating)));
        handleRate(r, [path]);
        orgParts.push(r === 0 ? 'rating cleared' : `rating ${r}★`);
      }

      if (typeof response?.colorLabel === 'string') {
        const c = response.colorLabel.trim().toLowerCase();
        if (c === 'none' || c === 'null' || c === '') {
          handleSetColorLabel(null, [path]);
          orgParts.push('label cleared');
        } else if (VALID_COLORS.has(c)) {
          const curTags = useLibraryStore.getState().imageList.find((i) => i.path === path)?.tags || [];
          const curColor = curTags.find((tg: string) => tg.startsWith('color:'))?.slice(6) || null;
          if (curColor !== c) handleSetColorLabel(c, [path]);
          orgParts.push(`label ${c}`);
        }
      }

      if (typeof response?.filename === 'string' && response.filename.trim()) {
        const newPath = await handleRenameToName(path, response.filename);
        if (newPath) orgParts.push(`renamed to ${newPath.split(/[\\/]/).pop() || response.filename}`);
      }

      return { metaPatch: hasMeta ? metaPatch : null, org: orgParts.length ? orgParts.join(' · ') : null };
    },
    [handleUpdateExif, handleRate, handleSetColorLabel, handleTagsChanged, handleRenameToName],
  );

  const send = useCallback(async () => {
    const text = input.trim();
    if ((!text && attachments.length === 0) || isLoading) return;
    cancelRef.current = false;

    const { selectedImage: currentImage, adjustments, finalPreviewUrl, uncroppedAdjustedPreviewUrl } =
      useEditorStore.getState();
    const { multiSelectedPaths: selectedPaths, imageList } = useLibraryStore.getState();
    const outgoing = [...attachments];

    // Batch mode: several library images selected and no manual attachment — OCR
    // and apply to each of them individually (matches how the Metadata panel
    // treats a multi-selection). A manual attachment falls back to single.
    const doBatch = outgoing.length === 0 && selectedPaths.length > 1;

    const viewerUrl = finalPreviewUrl || uncroppedAdjustedPreviewUrl || currentImage?.thumbnailUrl || null;
    const willAttachViewer = !doBatch && outgoing.length === 0 && !!currentImage && !!viewerUrl;

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
            const prepared: any = await invoke(Invokes.AssistantPrepareImage, { path, maxDim: 2000 });
            const response: any = await invoke(Invokes.AssistantChat, {
              messages: batchHistory,
              adjustments: null,
              currentMetadata: readCurrentMetadata(imageList.find((i) => i.path === path)?.exif || {}),
              images: [{ mediaType: prepared.mediaType, data: prepared.data }],
              model: selectedModel || null,
            });
            if (cancelRef.current) break;
            const { metaPatch, org } = await applyMetaOrg(response, path);
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
      if (willAttachViewer && viewerUrl) {
        const viewer = await blobUrlToImage(viewerUrl);
        if (viewer) images = [viewer];
      }
      const response: any = await invoke(Invokes.AssistantChat, {
        messages: history,
        adjustments: currentImage ? adjustments : null,
        currentMetadata: currentImage ? readCurrentMetadata(currentImage.exif) : null,
        images,
        model: selectedModel || null,
      });

      if (cancelRef.current) {
        addMessage({ id: nextMessageId(), role: 'assistant', content: t('editor.assistant.stoppedShort', 'Stopped.') });
        return;
      }

      const patch = currentImage ? sanitizePatch(response?.adjustments) : {};
      const hasPatch = Object.keys(patch).length > 0;
      if (hasPatch) {
        setAdjustments((prev: any) => ({ ...prev, ...patch }));
      }

      let metaPatch: Record<string, string> | null = null;
      let appliedOrganization: string | null = null;
      if (currentImage) {
        const res = await applyMetaOrg(response, currentImage.path);
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
  }, [input, attachments, isLoading, addMessage, setLoading, setAdjustments, applyMetaOrg, selectedModel, t]);

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

        {!selectedImage && selectedCount <= 1 && (
          <Text color={TextColors.secondary} className="text-xs mb-2">
            {t('editor.assistant.noImage', 'Open an image to let the assistant apply edits.')}
          </Text>
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
