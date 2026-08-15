import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { Pipette, Plus, RotateCcw, Trash2, X } from 'lucide-react';
import Slider from '../ui/Slider';
import Switch from '../ui/Switch';
import { Invokes } from '../ui/AppProperties';
import Dropdown from '../ui/Dropdown';
import { useEditorStore } from '../../store/useEditorStore';
import { useSettingsStore } from '../../store/useSettingsStore';
import { allProfiles, suggestProfile, FilmProfile } from '../../utils/filmProfiles';

interface FilmParams {
  redWeight: number;
  greenWeight: number;
  blueWeight: number;
  exposure: number;
  contrast: number;
}

const DEFAULT_PARAMS: FilmParams = {
  redWeight: 1.0,
  greenWeight: 1.0,
  blueWeight: 1.0,
  exposure: 0.0,
  contrast: 1.0,
};

// Clip percentiles live inside the bounds analysis; shown as % in Advanced.
const DEFAULT_CLIP_BLACK = 0.1; // percent
const DEFAULT_CLIP_WHITE = 99.9; // percent

// Film-negative conversion tuning. The params are sidecar-owned (a dedicated
// command writes them, never setAdjustments) so they stay out of undo history
// and presets; each commit re-decodes the base image.
export default function FilmPanel({ adjustments }: any) {
  const { t } = useTranslation();
  const nc = adjustments?.negativeConversion;
  const selectedImage = useEditorStore((s) => s.selectedImage);
  const appSettings = useSettingsStore((s) => s.appSettings);
  const handleSettingsChange = useSettingsStore((s) => s.handleSettingsChange);

  const [params, setParams] = useState<FilmParams>(DEFAULT_PARAMS);
  const [clipBlack, setClipBlack] = useState(DEFAULT_CLIP_BLACK);
  const [clipWhite, setClipWhite] = useState(DEFAULT_CLIP_WHITE);
  const [clipDirty, setClipDirty] = useState(false);
  const [rawPreview, setRawPreview] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const commitTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const advanced = !!appSettings?.filmPanelAdvanced;
  const setAdvanced = (on: boolean) =>
    appSettings && handleSettingsChange({ ...appSettings, filmPanelAdvanced: on });

  // Seed local state from the sidecar-owned values whenever the image changes.
  useEffect(() => {
    setParams({
      redWeight: nc?.redWeight ?? 1.0,
      greenWeight: nc?.greenWeight ?? 1.0,
      blueWeight: nc?.blueWeight ?? 1.0,
      exposure: nc?.exposure ?? 0.0,
      contrast: nc?.contrast ?? 1.0,
    });
    setClipBlack(nc?.clipBlack != null ? nc.clipBlack * 100 : DEFAULT_CLIP_BLACK);
    setClipWhite(nc?.clipWhite != null ? nc.clipWhite * 100 : DEFAULT_CLIP_WHITE);
    setClipDirty(false);
    setRawPreview(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedImage?.path]);

  const reloadImage = useCallback(async () => {
    const { selectedImage: img, setEditor } = useEditorStore.getState();
    if (!img) return;
    setEditor({ hasRenderedFirstFrame: false });
    // Re-decode only — history untouched, unlike the enable/disable toggle.
    await invoke(Invokes.LoadImage, { path: img.path });
  }, []);

  const commit = useCallback(
    (next: FilmParams, clip: { black: number; white: number } | null) => {
      if (commitTimer.current) clearTimeout(commitTimer.current);
      commitTimer.current = setTimeout(async () => {
        const { selectedImage: img } = useEditorStore.getState();
        if (!img) return;
        setBusy(true);
        try {
          await invoke(Invokes.UpdateNegativeConversion, {
            paths: [img.path],
            params: next,
            blackPoint: clip ? clip.black / 100 : null,
            whitePoint: clip ? clip.white / 100 : null,
            regenThumbnails: false,
          });
          await reloadImage();
        } catch (e) {
          console.error('Film tuning failed', e);
        } finally {
          setBusy(false);
        }
      }, 150);
    },
    [reloadImage],
  );

  const onParam = (key: keyof FilmParams) => (e: any) => {
    const value = parseFloat(e.target.value);
    setParams((p) => ({ ...p, [key]: value }));
  };
  // Sliders commit on release; sliding only updates local state.
  // Track latest values for the release callback without re-subscribing.
  const latest = useRef({ params, clipBlack, clipWhite, clipDirty });
  latest.current = { params, clipBlack, clipWhite, clipDirty };
  const onDragStateChange = (dragging: boolean) => {
    if (!dragging) {
      const l = latest.current;
      commit(l.params, l.clipDirty ? { black: l.clipBlack, white: l.clipWhite } : null);
    }
  };

  // --- Film-stock profiles (params only; bounds stay per-frame) ---
  const profiles = allProfiles(appSettings?.negativeProfiles as FilmProfile[] | undefined);
  const [profileName, setProfileName] = useState<string | null>(null);
  const [savingName, setSavingName] = useState<string | null>(null); // null = closed

  // Auto-suggest by the roll's film-stock EXIF once per image.
  useEffect(() => {
    const stock = (selectedImage as any)?.exif?.ImageDescription;
    const hit = suggestProfile(profiles, stock);
    setProfileName(hit ? hit.name : null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedImage?.path]);

  const applyProfile = (name: string) => {
    const pr = profiles.find((x) => x.name === name);
    if (!pr) return;
    setProfileName(name);
    setParams({ ...pr.params });
    const clip =
      pr.clipBlack != null && pr.clipWhite != null
        ? { black: pr.clipBlack * 100, white: pr.clipWhite * 100 }
        : null;
    if (clip) {
      setClipBlack(clip.black);
      setClipWhite(clip.white);
      setClipDirty(true);
    }
    commit({ ...pr.params }, clip);
  };

  const saveProfile = () => {
    const name = savingName?.trim();
    if (!name || !appSettings) return;
    const user = ((appSettings.negativeProfiles as FilmProfile[]) ?? []).filter((x) => x.name !== name);
    user.push({
      name,
      filmStock: (selectedImage as any)?.exif?.ImageDescription || undefined,
      params: { ...params },
      clipBlack: clipDirty ? clipBlack / 100 : undefined,
      clipWhite: clipDirty ? clipWhite / 100 : undefined,
    });
    handleSettingsChange({ ...appSettings, negativeProfiles: user });
    setProfileName(name);
    setSavingName(null);
  };

  const deleteProfile = () => {
    if (!appSettings || !profileName) return;
    const pr = profiles.find((x) => x.name === profileName);
    if (!pr || pr.builtin) return;
    const user = ((appSettings.negativeProfiles as FilmProfile[]) ?? []).filter(
      (x) => x.name !== profileName,
    );
    handleSettingsChange({ ...appSettings, negativeProfiles: user });
    setProfileName(null);
  };

  const resetToAuto = () => {
    setParams(DEFAULT_PARAMS);
    setClipBlack(DEFAULT_CLIP_BLACK);
    setClipWhite(DEFAULT_CLIP_WHITE);
    setClipDirty(true); // re-run bounds at default percentiles (also clears a pinned base)
    commit(DEFAULT_PARAMS, { black: DEFAULT_CLIP_BLACK, white: DEFAULT_CLIP_WHITE });
  };

  const startEyedropper = async () => {
    const { selectedImage: img } = useEditorStore.getState();
    if (!img) return;
    try {
      const url: string = await invoke(Invokes.GetNegativeRawPreview, { path: img.path });
      setRawPreview(url);
    } catch (e) {
      console.error('Raw preview failed', e);
    }
  };

  const onPickBase = async (e: React.MouseEvent<HTMLImageElement>) => {
    const { selectedImage: img } = useEditorStore.getState();
    if (!img) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const x = (e.clientX - rect.left) / rect.width;
    const y = (e.clientY - rect.top) / rect.height;
    setRawPreview(null);
    setBusy(true);
    try {
      await invoke(Invokes.SetNegativeFilmBase, { path: img.path, x, y });
      await reloadImage();
    } catch (err) {
      console.error('Film base sampling failed', err);
    } finally {
      setBusy(false);
    }
  };

  if (!nc?.enabled) return null;

  return (
    <div className={busy ? 'opacity-70 pointer-events-none' : ''}>
      <div className="flex items-center gap-1 mb-2">
        <div className="flex-1 min-w-0">
          <Dropdown
            options={profiles.map((pr) => ({ label: pr.name, value: pr.name }))}
            value={profileName}
            onChange={(name: string) => applyProfile(name)}
          />
        </div>
        <button
          className="p-1.5 rounded-md text-text-secondary hover:bg-surface hover:text-text-primary"
          onClick={() => setSavingName(savingName === null ? '' : null)}
          title={t('editor.adjustments.film.saveProfile')}
        >
          <Plus size={14} />
        </button>
        {profileName && !profiles.find((x) => x.name === profileName)?.builtin && (
          <button
            className="p-1.5 rounded-md text-text-secondary hover:bg-surface hover:text-text-primary"
            onClick={deleteProfile}
            title={t('editor.adjustments.film.deleteProfile')}
          >
            <Trash2 size={14} />
          </button>
        )}
      </div>
      {savingName !== null && (
        <div className="flex items-center gap-1 mb-2">
          <input
            className="flex-1 min-w-0 px-2 py-1 rounded-md bg-surface text-sm text-text-primary outline-none"
            placeholder={t('editor.adjustments.film.profileNamePlaceholder')}
            value={savingName}
            autoFocus
            onChange={(e) => setSavingName(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && saveProfile()}
          />
          <button
            className="px-2 py-1 rounded-md bg-surface text-sm text-text-secondary hover:text-text-primary"
            onClick={saveProfile}
          >
            {t('editor.adjustments.film.save')}
          </button>
        </div>
      )}
      <Slider
        label={t('modals.negativeConversion.exposure')}
        min={-1}
        max={1}
        step={0.01}
        value={params.exposure}
        defaultValue={0}
        onChange={onParam('exposure')}
        onDragStateChange={onDragStateChange}
      />
      <Slider
        label={t('modals.negativeConversion.printGrade')}
        min={0.5}
        max={2.5}
        step={0.01}
        value={params.contrast}
        defaultValue={1}
        onChange={onParam('contrast')}
        onDragStateChange={onDragStateChange}
      />

      <div className="flex items-center justify-between mt-2 mb-1">
        <Switch
          label={t('editor.adjustments.film.advanced')}
          checked={advanced}
          onChange={setAdvanced}
        />
        <button
          className="p-1.5 rounded-md text-text-secondary hover:bg-surface hover:text-text-primary"
          onClick={resetToAuto}
          title={t('modals.negativeConversion.resetTooltip')}
        >
          <RotateCcw size={14} />
        </button>
      </div>

      {advanced && (
        <>
          <p className="text-xs text-text-secondary mt-2 mb-1">
            {t('modals.negativeConversion.colorTiming')}
          </p>
          <Slider
            label={t('modals.negativeConversion.redWeight')}
            min={0.5}
            max={1.5}
            step={0.01}
            value={params.redWeight}
            defaultValue={1}
            onChange={onParam('redWeight')}
            onDragStateChange={onDragStateChange}
          />
          <Slider
            label={t('modals.negativeConversion.greenWeight')}
            min={0.5}
            max={1.5}
            step={0.01}
            value={params.greenWeight}
            defaultValue={1}
            onChange={onParam('greenWeight')}
            onDragStateChange={onDragStateChange}
          />
          <Slider
            label={t('modals.negativeConversion.blueWeight')}
            min={0.5}
            max={1.5}
            step={0.01}
            value={params.blueWeight}
            defaultValue={1}
            onChange={onParam('blueWeight')}
            onDragStateChange={onDragStateChange}
          />
          <Slider
            label={t('editor.adjustments.film.clipBlack')}
            min={0}
            max={1}
            step={0.05}
            value={clipBlack}
            defaultValue={DEFAULT_CLIP_BLACK}
            suffix="%"
            onChange={(e: any) => {
              setClipBlack(parseFloat(e.target.value));
              setClipDirty(true);
            }}
            onDragStateChange={onDragStateChange}
          />
          <Slider
            label={t('editor.adjustments.film.clipWhite')}
            min={99}
            max={100}
            step={0.05}
            value={clipWhite}
            defaultValue={DEFAULT_CLIP_WHITE}
            suffix="%"
            onChange={(e: any) => {
              setClipWhite(parseFloat(e.target.value));
              setClipDirty(true);
            }}
            onDragStateChange={onDragStateChange}
          />

          <button
            className="mt-2 flex items-center gap-2 text-sm text-text-secondary hover:text-text-primary"
            onClick={rawPreview ? () => setRawPreview(null) : startEyedropper}
          >
            {rawPreview ? <X size={14} /> : <Pipette size={14} />}
            {t('editor.adjustments.film.sampleBase')}
          </button>
          {rawPreview && (
            <div className="mt-2">
              <p className="text-xs text-text-secondary mb-1">
                {t('editor.adjustments.film.sampleBaseHint')}
              </p>
              <img
                src={rawPreview}
                className="w-full rounded-md cursor-crosshair"
                onClick={onPickBase}
                alt={t('modals.negativeConversion.originalLabel')}
              />
            </div>
          )}
        </>
      )}
    </div>
  );
}
