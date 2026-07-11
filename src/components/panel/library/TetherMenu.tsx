import { useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { Camera, Play, Square } from 'lucide-react';
import Button from '../../ui/Button';
import Dropdown from '../../ui/Dropdown';
import Switch from '../../ui/Switch';
import Text from '../../ui/Text';
import { TextColors, TextVariants } from '../../../types/typography';
import { Invokes, Preset } from '../../ui/AppProperties';
import { usePresets, UserPreset } from '../../../hooks/usePresets';
import { INITIAL_ADJUSTMENTS } from '../../../utils/adjustments';
import { useEditorStore } from '../../../store/useEditorStore';
import { useLibraryStore } from '../../../store/useLibraryStore';
import { useTetherStore } from '../../../store/useTetherStore';
import { DropdownMenu } from './LibraryHeader';

const NO_PRESET = '__none__';

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
      await invoke(Invokes.StopTetherSession);
    } catch (err) {
      console.error('Failed to stop tether session:', err);
    }
    setTether({ isActive: false });
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
            <Switch
              checked={autoSelect}
              label={t('editor.tether.autoSelect')}
              onChange={(val: boolean) => setTether({ autoSelect: val })}
            />
            <Button className="bg-surface" onClick={handleStop}>
              <Square size={16} />
              {t('editor.tether.stop')}
            </Button>
          </>
        )}
      </div>
    </DropdownMenu>
  );
}
