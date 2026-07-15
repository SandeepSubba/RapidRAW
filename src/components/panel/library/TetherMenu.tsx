import { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { Aperture, Camera, Play, Square, Unplug } from 'lucide-react';
import Button from '../../ui/Button';
import Dropdown from '../../ui/Dropdown';
import Slider from '../../ui/Slider';
import Switch from '../../ui/Switch';
import Text from '../../ui/Text';
import { TextColors, TextVariants } from '../../../types/typography';
import { Invokes, Preset } from '../../ui/AppProperties';
import { usePresets, UserPreset } from '../../../hooks/usePresets';
import { INITIAL_ADJUSTMENTS } from '../../../utils/adjustments';
import { useEditorStore } from '../../../store/useEditorStore';
import { useLibraryStore } from '../../../store/useLibraryStore';
import { TetherCamera, TetherCameraConfig, useTetherStore } from '../../../store/useTetherStore';
import { DropdownMenu } from './LibraryHeader';

const NO_PRESET = '__none__';

// Numeric camera property as a slider; writes to the body are debounced so
// a drag doesn't spam PTP transactions.
function ConfigSlider({
  config,
  onCommit,
}: {
  config: TetherCameraConfig;
  onCommit: (value: string) => void;
}) {
  const range = config.range!;
  // Enumerated bodies (Fuji/Canon Kelvin lists) only accept their own values;
  // bound the slider by them and snap commits to the nearest one.
  const steps = config.choices.map(Number).filter((n) => !Number.isNaN(n));
  const min = steps.length ? Math.min(...steps) : range.min;
  const max = steps.length ? Math.max(...steps) : range.max;
  const snap = (v: number) =>
    steps.length ? steps.reduce((a, b) => (Math.abs(b - v) < Math.abs(a - v) ? b : a)) : v;
  const [value, setValue] = useState(() => Number(config.current) || min);
  const commitTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => {
    setValue(Number(config.current) || min);
  }, [config.current, min]);

  const handleChange = (e: any) => {
    const next = Number(e.target.value);
    setValue(next);
    clearTimeout(commitTimer.current);
    commitTimer.current = setTimeout(() => onCommit(String(snap(next))), 400);
  };

  return (
    <Slider
      label={config.label}
      min={min}
      max={max}
      step={range.step}
      defaultValue={snap(range.default)}
      value={value}
      onChange={handleChange}
    />
  );
}

// Direct-USB camera controls (needs the tether-usb build; the Detect button
// simply finds nothing on builds without it). Shots — app-triggered or via
// the body's shutter — download into the session folder, so the watcher
// ingests them like any other tethered shot. Rendered in the tether popover
// and, during an active session, as a section in the editor Controls panel.
export function CameraSection() {
  const { t } = useTranslation();
  const camera = useTetherStore((s) => s.camera);
  const folder = useTetherStore((s) => s.folder);
  const liveView = useTetherStore((s) => s.liveView);
  const setTether = useTetherStore((s) => s.setTether);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = async (label: string, action: () => Promise<void>) => {
    setBusy(label);
    setError(null);
    try {
      await action();
    } catch (err) {
      console.error(`Tether camera ${label} failed:`, err);
      setError(String(err));
    } finally {
      setBusy(null);
    }
  };

  const handleDetect = () =>
    run('detect', async () => {
      const found = await invoke<Array<{ model: string; port: string }>>(Invokes.TetherListCameras);
      if (!found.length) {
        setBusy('none');
        setTimeout(() => setBusy(null), 2500);
        return;
      }
      const connected = await invoke<TetherCamera>(Invokes.TetherConnectCamera, {
        model: found[0].model,
        port: found[0].port,
        downloadDir: folder,
      });
      setTether({ camera: connected });
    });

  const handleCapture = () => run('capture', () => invoke(Invokes.TetherTriggerCapture));

  // Detect automatically when the controls appear without a camera; the
  // button stays as a manual retry (e.g. plugged in after opening).
  const autoDetected = useRef(false);
  useEffect(() => {
    if (!camera && !autoDetected.current) {
      autoDetected.current = true;
      handleDetect();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleDisconnect = () =>
    run('disconnect', async () => {
      await invoke(Invokes.TetherDisconnectCamera);
      setTether({ camera: null, liveView: false });
    });

  const handleLiveView = (on: boolean) =>
    run('liveview', async () => {
      if (!on) {
        setTether({ liveView: false, liveViewError: null });
        await invoke(Invokes.TetherSetLiveView, { on: false }).catch(() => {});
        return;
      }
      try {
        await invoke(Invokes.TetherSetLiveView, { on: true });
        setTether({ liveView: true, liveViewError: null });
      } catch (err) {
        // Open the popup anyway as a camera-controls palette; the feed area
        // explains why the body refused to stream.
        setTether({ liveView: true, liveViewError: String(err) });
      }
    });

  const setConfigCurrent = (key: string, value: string) =>
    setTether((s) => ({
      camera: s.camera && {
        ...s.camera,
        configs: s.camera.configs.map((c) => (c.key === key ? { ...c, current: value } : c)),
      },
    }));

  const handleConfig = (key: string, value: string) => {
    const previous = camera?.configs.find((c) => c.key === key)?.current;
    setConfigCurrent(key, value); // optimistic; revert if the body refuses
    return run(key, () =>
      invoke(Invokes.TetherSetConfig, { key, value }).then(
        () => undefined,
        (err) => {
          if (previous !== undefined) setConfigCurrent(key, previous);
          throw err;
        },
      ),
    );
  };

  if (!camera) {
    return (
      <div className="flex flex-col gap-1.5">
        <Button className="bg-surface text-text-primary" disabled={busy === 'detect'} onClick={handleDetect}>
          <Camera size={16} />
          {busy === 'detect'
            ? t('editor.tether.camera.connecting')
            : busy === 'none'
              ? t('editor.tether.camera.noneFound')
              : t('editor.tether.camera.detect')}
        </Button>
        {error && (
          <Text variant={TextVariants.small} color={TextColors.secondary} className="break-words">
            {error}
          </Text>
        )}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2 p-3 rounded-lg bg-bg-tertiary">
      <div className="flex items-center justify-between">
        <Text variant={TextVariants.small} className="truncate">
          {camera.model}
        </Text>
        <button
          className="p-1 rounded-md text-text-secondary hover:text-text-primary transition-colors"
          onClick={handleDisconnect}
          data-tooltip={t('editor.tether.camera.disconnect')}
        >
          <Unplug size={14} />
        </button>
      </div>
      {camera.configs.map((config) =>
        config.range ? (
          <ConfigSlider key={config.key} config={config} onCommit={(value) => handleConfig(config.key, value)} />
        ) : (
          <div key={config.key} className="flex items-center gap-2">
            <Text variant={TextVariants.small} color={TextColors.secondary} className="w-16 shrink-0">
              {config.label}
            </Text>
            <Dropdown
              options={config.choices.map((c) => ({ label: c, value: c }))}
              value={config.current}
              onChange={(value: string) => handleConfig(config.key, value)}
            />
          </div>
        ),
      )}
      <Switch
        checked={liveView}
        disabled={busy === 'liveview' || !camera.liveViewSupported}
        label={t('editor.tether.camera.liveView')}
        tooltip={camera.liveViewSupported ? undefined : t('editor.tether.camera.liveViewUnsupported')}
        onChange={handleLiveView}
      />
      <Button className="bg-accent" disabled={busy === 'capture'} onClick={handleCapture}>
        <Aperture size={16} />
        {busy === 'capture' ? t('editor.tether.camera.capturing') : t('editor.tether.camera.capture')}
      </Button>
      {error && (
        <Text variant={TextVariants.small} color={TextColors.secondary} className="break-words">
          {error}
        </Text>
      )}
    </div>
  );
}

// Library-header tether control: the currently open library folder is the
// session folder — shots dropped into it by a vendor tether utility
// (X Acquire, EOS Utility, NX Tether...) are imported as they land.
export default function TetherMenu() {
  const { t } = useTranslation();
  const adjustments = useEditorStore((s) => s.adjustments);
  const currentFolderPath = useLibraryStore((s) => s.currentFolderPath);
  const { presets } = usePresets(adjustments);
  const { isActive, folder, presetId, autoSelect, shotCount, lastShotName, setTether } = useTetherStore();

  // Albums are virtual — only a real folder can be watched.
  const watchableFolder =
    currentFolderPath && !currentFolderPath.startsWith('Album: ') ? currentFolderPath : null;

  const flatPresets = useMemo<Preset[]>(
    () =>
      presets.flatMap((item: UserPreset) =>
        item.folder ? (item.folder.children as Preset[]) : item.preset ? [item.preset] : [],
      ),
    [presets],
  );

  const presetOptions = [
    { label: t('editor.tether.noPreset'), value: NO_PRESET },
    ...flatPresets.map((p) => ({ label: p.name, value: p.id })),
  ];

  const handleStart = async () => {
    if (!watchableFolder) return;
    const preset = flatPresets.find((p) => p.id === presetId);
    try {
      await invoke(Invokes.StartTetherSession, {
        folder: watchableFolder,
        presetAdjustments: preset ? { ...INITIAL_ADJUSTMENTS, ...preset.adjustments } : null,
      });
      setTether({ isActive: true, folder: watchableFolder, shotCount: 0, lastShotName: null });
    } catch (err) {
      console.error('Failed to start tether session:', err);
    }
  };

  const handleStop = async () => {
    try {
      await invoke(Invokes.TetherDisconnectCamera);
      await invoke(Invokes.StopTetherSession);
    } catch (err) {
      console.error('Failed to stop tether session:', err);
    }
    setTether({ isActive: false, camera: null });
  };

  return (
    <DropdownMenu
      buttonTitle={t('editor.tether.title')}
      contentClassName="w-72"
      buttonContent={
        <span className="relative flex items-center justify-center">
          {/* 22px: the camera glyph fills its viewbox edge-to-edge, so at the siblings' w-8 it reads oversized */}
          <Camera className="w-[22px] h-[22px]" />
          {isActive && (
            <span className="absolute -top-1 -right-1 w-2.5 h-2.5 rounded-full bg-green-500 animate-pulse" />
          )}
        </span>
      }
    >
      <div className="flex flex-col gap-3 p-4">
        <div className="flex items-center justify-between">
          <Text variant={TextVariants.heading}>{t('editor.tether.title')}</Text>
          {isActive && (
            <span className="flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
              <Text variant={TextVariants.small} color={TextColors.secondary}>
                {t('editor.tether.live')}
              </Text>
            </span>
          )}
        </div>

        {!isActive ? (
          <>
            <Text variant={TextVariants.small} color={TextColors.secondary}>
              {t('editor.tether.description')}
            </Text>
            <div className="flex flex-col gap-1.5">
              <Text variant={TextVariants.small} color={TextColors.secondary}>
                {t('editor.tether.presetOnImport')}
              </Text>
              <Dropdown
                options={presetOptions}
                value={presetId ?? NO_PRESET}
                onChange={(value: string) => setTether({ presetId: value === NO_PRESET ? null : value })}
              />
            </div>
            <Switch
              checked={autoSelect}
              label={t('editor.tether.autoSelect')}
              onChange={(val: boolean) => setTether({ autoSelect: val })}
            />
            <Button className="bg-accent" disabled={!watchableFolder} onClick={handleStart}>
              <Play size={16} />
              {t('editor.tether.start')}
            </Button>
            {!watchableFolder && (
              <Text variant={TextVariants.small} color={TextColors.secondary}>
                {t('editor.tether.needFolder')}
              </Text>
            )}
          </>
        ) : (
          <>
            <div className="flex flex-col gap-1 p-3 rounded-lg bg-bg-tertiary">
              <Text variant={TextVariants.small} color={TextColors.secondary} className="truncate">
                {folder?.split('/').pop()}
              </Text>
              <div className="flex items-center gap-2">
                <Camera size={16} className="text-text-secondary" />
                <Text>{t('editor.tether.shotCount', { count: shotCount })}</Text>
              </div>
              {lastShotName && (
                <Text variant={TextVariants.small} color={TextColors.secondary} className="truncate">
                  {t('editor.tether.lastShot', { name: lastShotName })}
                </Text>
              )}
            </div>
            <CameraSection />
            <Switch
              checked={autoSelect}
              label={t('editor.tether.autoSelect')}
              onChange={(val: boolean) => setTether({ autoSelect: val })}
            />
            <Button className="bg-surface text-text-primary" onClick={handleStop}>
              <Square size={16} />
              {t('editor.tether.stop')}
            </Button>
          </>
        )}
      </div>
    </DropdownMenu>
  );
}
