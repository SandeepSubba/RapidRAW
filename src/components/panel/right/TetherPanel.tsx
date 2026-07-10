import { useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useTranslation } from 'react-i18next';
import { Camera, FolderOpen, Play, Square } from 'lucide-react';
import Button from '../../ui/Button';
import Dropdown from '../../ui/Dropdown';
import Switch from '../../ui/Switch';
import Text from '../../ui/Text';
import { TextColors, TextVariants } from '../../../types/typography';
import { Invokes, Preset } from '../../ui/AppProperties';
import { usePresets, UserPreset } from '../../../hooks/usePresets';
import { INITIAL_ADJUSTMENTS } from '../../../utils/adjustments';
import { useEditorStore } from '../../../store/useEditorStore';
import { useTetherStore } from '../../../store/useTetherStore';

const NO_PRESET = '__none__';

export default function TetherPanel() {
  const { t } = useTranslation();
  const adjustments = useEditorStore((s) => s.adjustments);
  const { presets } = usePresets(adjustments);
  const { isActive, folder, presetId, autoSelect, shotCount, lastShotName, setTether } = useTetherStore();

  // Flatten the preset tree (top-level presets + folder children) for the dropdown.
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

  const handlePickFolder = async () => {
    const picked = await open({ directory: true, multiple: false, title: t('editor.tether.chooseFolder') });
    if (typeof picked === 'string') {
      setTether({ folder: picked });
    }
  };

  const handleStart = async () => {
    if (!folder) return;
    const preset = flatPresets.find((p) => p.id === presetId);
    try {
      await invoke(Invokes.StartTetherSession, {
        folder,
        presetAdjustments: preset ? { ...INITIAL_ADJUSTMENTS, ...preset.adjustments } : null,
      });
      setTether({ isActive: true, shotCount: 0, lastShotName: null });
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

  const folderName = folder?.split('/').pop() || folder;

  return (
    <div className="flex flex-col h-full">
      <div className="p-4 flex justify-between items-center shrink-0 border-b border-surface">
        <Text variant={TextVariants.title}>{t('editor.tether.title')}</Text>
        {isActive && (
          <span className="flex items-center gap-1.5">
            <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            <Text variant={TextVariants.small} color={TextColors.secondary}>
              {t('editor.tether.live')}
            </Text>
          </span>
        )}
      </div>

      <div className="flex flex-col gap-4 p-4 overflow-y-auto">
        {!isActive ? (
          <>
            <Text variant={TextVariants.small} color={TextColors.secondary}>
              {t('editor.tether.description')}
            </Text>
            <Button className="bg-surface" onClick={handlePickFolder}>
              <FolderOpen size={16} />
              <span className="truncate">{folder ? folderName : t('editor.tether.chooseFolder')}</span>
            </Button>
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
            <Button className="bg-accent" disabled={!folder} onClick={handleStart}>
              <Play size={16} />
              {t('editor.tether.start')}
            </Button>
          </>
        ) : (
          <>
            <div className="flex flex-col gap-1 p-3 rounded-lg bg-surface">
              <Text variant={TextVariants.small} color={TextColors.secondary} className="truncate">
                {folderName}
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
    </div>
  );
}
