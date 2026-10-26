import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { RotateCcw, Copy, ClipboardPaste, PencilSparkles, ChartArea, LayoutList, ChevronDown } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import clsx from 'clsx';
import { useTranslation } from 'react-i18next';
import CollapsibleSection from '../../ui/CollapsibleSection';
import AdjustmentSection from '../../adjustments/AdjustmentSection';
import ConnectedWaveform from '../editor/ConnectedWaveform';
import Resizer from '../../ui/Resizer';
import AdjustmentSectionsSubMenu from './AdjustmentSectionsSubMenu';
import {
  ActiveTools,
  Adjustments,
  INITIAL_ADJUSTMENTS,
  ADJUSTMENT_SECTIONS,
  getActiveTools,
  getVisibleAdjustmentSections,
  isToolActive,
  setToolActive,
  toggleToolActive,
} from '../../../utils/adjustments';
import { invoke } from '@tauri-apps/api/core';
import { useContextMenu } from '../../../context/ContextMenuContext';
import { AutoAdjustOptions, Invokes, OPTION_SEPARATOR, Orientation, resolveAutoAdjust } from '../../ui/AppProperties';
import Switch from '../../ui/Switch';
import Text from '../../ui/Text';
import { TextVariants, TextColors, TextWeights } from '../../../types/typography';
import { useShallow } from 'zustand/react/shallow';
import { useEditorStore } from '../../../store/useEditorStore';
import { useSettingsStore } from '../../../store/useSettingsStore';
import { useTetherStore } from '../../../store/useTetherStore';
import { CameraSection } from '../library/TetherMenu';
import { useUIStore } from '../../../store/useUIStore';
import { buildAutoAdjustModeOptions, useEditorActions } from '../../../hooks/useEditorActions';
import { useWaveformControls } from '../../../hooks/useWaveformControls';

const AUTO_TOOLS: Array<Exclude<keyof AutoAdjustOptions, 'strength' | 'keepLook'>> = [
  'exposure',
  'brightness',
  'contrast',
  'highlights',
  'shadows',
  'whites',
  'blacks',
  'whiteBalance',
  'vibrance',
  'clarity',
  'dehaze',
  'vignette',
];

/**
 * Capture One–style checklist for the Auto button: which tools it may write and
 * whether it corrects on top of the current look. Saved in settings, so the shortcut
 * and the library's batch Auto follow the same choice.
 */
/** What Auto has learned from the user's edits (mirrors app_settings::AutoProfile). */
interface AutoLearning {
  faceSamples: number;
  globalSamples: number;
  styleSamples: number;
  rangeTarget: number;
  saturationTarget: number;
  wbResidualTemperature: number;
  wbResidualTint: number;
}

/** One line on what Auto has picked up, or null while it still uses stock targets. */
function describeAutoLearning(l: AutoLearning | null, t: (k: any, o?: any) => string): string | null {
  if (!l) return null;
  const edits = Math.max(l.styleSamples, l.faceSamples + l.globalSamples);
  if (edits === 0) return null;
  const parts = [t('editor.adjustments.auto.learnedFrom', { count: edits })];
  if (l.styleSamples >= 3) {
    if (l.rangeTarget < 195) parts.push(t('editor.adjustments.auto.learnedSoft'));
    else if (l.rangeTarget > 235) parts.push(t('editor.adjustments.auto.learnedPunchy'));
    if (l.saturationTarget > 0.26) parts.push(t('editor.adjustments.auto.learnedVivid'));
    else if (l.saturationTarget < 0.14) parts.push(t('editor.adjustments.auto.learnedMuted'));
    if (l.wbResidualTemperature >= 3) parts.push(t('editor.adjustments.auto.learnedWarm'));
    else if (l.wbResidualTemperature <= -3) parts.push(t('editor.adjustments.auto.learnedCool'));
  }
  if (l.faceSamples >= 3) parts.push(t('editor.adjustments.auto.learnedFaces', { count: l.faceSamples }));
  return parts.join(' · ');
}

function AutoAdjustOptionsPopover({ onClose }: { onClose(): void }) {
  const { t } = useTranslation();
  const ref = useRef<HTMLDivElement>(null);
  const [learning, setLearning] = useState<AutoLearning | null>(null);
  useEffect(() => {
    invoke<AutoLearning>(Invokes.GetAutoLearning)
      .then(setLearning)
      .catch(() => setLearning(null));
  }, []);
  const learned = describeAutoLearning(learning, t);
  const resetLearning = async () => {
    try {
      await invoke(Invokes.ResetAutoProfile);
      setLearning(null);
    } catch (e) {
      console.error('Failed to reset auto learning:', e);
    }
  };
  const { appSettings, handleSettingsChange } = useSettingsStore(
    useShallow((state) => ({ appSettings: state.appSettings, handleSettingsChange: state.handleSettingsChange })),
  );
  const options = resolveAutoAdjust(appSettings?.autoAdjust);
  const update = (patch: Partial<AutoAdjustOptions>) => {
    if (!appSettings) return;
    handleSettingsChange({ ...appSettings, autoAdjust: { ...options, ...patch } });
  };
  const noneSelected = AUTO_TOOLS.every((tool) => !options[tool]);

  useEffect(() => {
    const onPointerDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
      }
    };
    document.addEventListener('mousedown', onPointerDown);
    window.addEventListener('keydown', onKey, true);
    return () => {
      document.removeEventListener('mousedown', onPointerDown);
      window.removeEventListener('keydown', onKey, true);
    };
  }, [onClose]);

  return (
    <motion.div
      ref={ref}
      initial={{ opacity: 0, y: -4 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -4 }}
      transition={{ duration: 0.15 }}
      // Spans the panel (anchored to its header) rather than hanging off the button:
      // the panel clips anything wider than itself.
      className="absolute left-3 right-3 top-full z-50 bg-surface border border-border-color rounded-lg shadow-xl p-3 flex flex-col gap-2"
    >
      <Text variant={TextVariants.label} color={TextColors.primary}>
        {t('editor.adjustments.auto.title')}
      </Text>
      <Text variant={TextVariants.small} color={TextColors.secondary}>
        {t('editor.adjustments.auto.tools')}
      </Text>
      <div className="grid grid-cols-2 gap-x-4 gap-y-1.5">
        {AUTO_TOOLS.map((tool) => (
          <Switch
            key={tool}
            label={t(`editor.adjustments.auto.${tool}`)}
            checked={options[tool]}
            onChange={(checked) => update({ [tool]: checked })}
          />
        ))}
      </div>
      {noneSelected && (
        <Text variant={TextVariants.small} color={TextColors.secondary} className="italic">
          {t('editor.adjustments.auto.nothingSelected')}
        </Text>
      )}
      <div className="border-t border-border-color pt-2 mt-1 flex flex-col gap-1.5">
        <Text variant={TextVariants.small} color={TextColors.secondary}>
          {t('editor.adjustments.auto.strength')}
        </Text>
        <div className="grid grid-cols-3 gap-1 bg-bg-primary rounded-md p-0.5">
          {(['subtle', 'normal', 'strong'] as const).map((level) => (
            <button
              key={level}
              onClick={() => update({ strength: level })}
              className={clsx(
                'text-xs py-1 rounded transition-colors',
                options.strength === level
                  ? 'bg-accent text-button-text'
                  : 'text-text-secondary hover:text-text-primary hover:bg-surface',
              )}
            >
              {t(`contextMenus.autoAdjustModes.${level}`)}
            </button>
          ))}
        </div>
      </div>
      <div className="border-t border-border-color pt-2 mt-1 flex flex-col gap-1">
        <Switch
          label={t('editor.adjustments.auto.keepLook')}
          checked={options.keepLook}
          onChange={(checked) => update({ keepLook: checked })}
        />
        <Text variant={TextVariants.small} color={TextColors.secondary}>
          {t('editor.adjustments.auto.keepLookDesc')}
        </Text>
        <Text variant={TextVariants.small} color={TextColors.secondary} className="opacity-70">
          {t('editor.adjustments.auto.batchNote')}
        </Text>
      </div>
      <div className="border-t border-border-color pt-2 mt-1">
        <Text variant={TextVariants.small} color={TextColors.secondary}>
          {learned ?? t('editor.adjustments.auto.learnedNothing')}
          {learned && (
            <>
              {' '}
              <button onClick={resetLearning} className="text-accent hover:underline">
                {t('editor.adjustments.auto.resetLearning')}
              </button>
            </>
          )}
        </Text>
      </div>
    </motion.div>
  );
}

function AdjustmentSectionContent(props: {
  sectionName: string;
  activeTools: ActiveTools;
  onToggleTool(id: string): void;
  [key: string]: unknown;
}) {
  const adjustments = useEditorStore((state) => state.adjustments);
  return <AdjustmentSection {...props} adjustments={adjustments} />;
}

export default function Controls() {
  const { t } = useTranslation();
  const [isAutoOptionsOpen, setIsAutoOptionsOpen] = useState(false);
  const closeAutoOptions = useCallback(() => setIsAutoOptionsOpen(false), []);
  const isTetherActive = useTetherStore((s) => s.isActive);
  const [isTetherOpen, setIsTetherOpen] = useState(true);
  const { showContextMenu } = useContextMenu();
  const { isResizingWaveform, onToggleWaveform, setActiveWaveformChannel, handleWaveformResize } =
    useWaveformControls();
  const { setAdjustments, handleAutoAdjustments, handleLutSelect, setLutPreviewOverride } = useEditorActions();

  const { appSettings, theme } = useSettingsStore(
    useShallow((state) => ({
      appSettings: state.appSettings,
      theme: state.theme,
    })),
  );

  const visibleSections = getVisibleAdjustmentSections(appSettings?.adjustmentLayout);

  const { collapsibleSectionsState, setUI } = useUIStore(
    useShallow((state) => ({
      collapsibleSectionsState: state.collapsibleSectionsState,
      setUI: state.setUI,
    })),
  );

  const {
    activeToolsState,
    sectionVisibility,
    copiedSectionAdjustments,
    selectedImage,
    isWbPickerActive,
    isPointPickerActive,
    isWaveformVisible,
    waveformHeight,
    setEditor,
    isNegative,
  } = useEditorStore(
    useShallow((state) => ({
      isNegative: !!(state.adjustments as any).negativeConversion?.enabled,
      activeToolsState: state.adjustments.activeTools,
      sectionVisibility: state.adjustments.sectionVisibility,
      copiedSectionAdjustments: state.copiedSectionAdjustments,
      selectedImage: state.selectedImage,
      isWbPickerActive: state.isWbPickerActive,
      isPointPickerActive: state.isPointPickerActive,
      isWaveformVisible: state.isWaveformVisible,
      waveformHeight: state.waveformHeight,
      setEditor: state.setEditor,
    })),
  );

  const activeTools = useMemo(
    () => getActiveTools({ activeTools: activeToolsState, sectionVisibility }),
    [activeToolsState, sectionVisibility],
  );

  const setCopiedSectionAdjustments = useCallback(
    (val: any) => setEditor({ copiedSectionAdjustments: val }),
    [setEditor],
  );

  const toggleWbPicker = useCallback(
    () =>
      setEditor((state) => ({
        isWbPickerActive: !state.isWbPickerActive,
        isRelightPickerActive: false,
        activeCenterMarker: null,
      })),
    [setEditor],
  );

  const togglePointPicker = useCallback(
    () => setEditor((state) => ({ isPointPickerActive: !state.isPointPickerActive })),
    [setEditor],
  );

  const onDragStateChange = useCallback(
    (isDragging: boolean) => setEditor({ isSliderDragging: isDragging }),
    [setEditor],
  );

  const setCollapsibleState = useCallback(
    (updater: any) =>
      setUI((state) => ({
        collapsibleSectionsState: typeof updater === 'function' ? updater(state.collapsibleSectionsState) : updater,
      })),
    [setUI],
  );

  const handleToggleVisibility = useCallback(
    (id: string) => {
      setAdjustments((prev: Adjustments) => ({
        ...prev,
        activeTools: toggleToolActive(getActiveTools(prev), id),
      }));
    },
    [setAdjustments],
  );

  const handleResetAdjustments = () => {
    setAdjustments((prev: Adjustments) => ({
      ...prev,
      ...Object.keys(ADJUSTMENT_SECTIONS)
        .flatMap((s) => ADJUSTMENT_SECTIONS[s])
        .reduce((acc: any, key: string) => {
          acc[key] = INITIAL_ADJUSTMENTS[key as keyof Adjustments];
          return acc;
        }, {}),
      activeTools: { ...INITIAL_ADJUSTMENTS.activeTools },
    }));
  };

  const handleAutoAdjustContextMenu = (event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    showContextMenu(event.clientX, event.clientY, buildAutoAdjustModeOptions(t, handleAutoAdjustments));
  };

  const handleToggleSection = (section: string) => {
    setCollapsibleState((prev: any) => {
      const isOpening = !prev[section];
      if (appSettings?.enableFocusMode && isOpening) {
        const newState = { ...prev };
        Object.keys(newState).forEach((key) => {
          newState[key] = false;
        });
        newState[section] = true;
        return newState;
      }
      return { ...prev, [section]: !prev[section] };
    });
  };

  const handleSectionContextMenu = (event: any, sectionName: string) => {
    event.preventDefault();
    event.stopPropagation();

    const sectionKeys = ADJUSTMENT_SECTIONS[sectionName];
    if (!sectionKeys) {
      return;
    }

    const handleCopy = () => {
      const { adjustments } = useEditorStore.getState();
      const adjustmentsToCopy: any = {};
      for (const key of sectionKeys) {
        if (Object.prototype.hasOwnProperty.call(adjustments, key)) {
          adjustmentsToCopy[key] = JSON.parse(JSON.stringify(adjustments[key as keyof Adjustments]));
        }
      }
      setCopiedSectionAdjustments({ section: sectionName, values: adjustmentsToCopy });
    };

    const handlePaste = () => {
      if (!copiedSectionAdjustments || copiedSectionAdjustments.section !== sectionName) {
        return;
      }
      setAdjustments((prev: Adjustments) => ({
        ...prev,
        ...copiedSectionAdjustments.values,
        activeTools: setToolActive(getActiveTools(prev), sectionName, true),
      }));
    };

    const handleReset = () => {
      const resetValues: any = {};
      for (const key of sectionKeys) {
        resetValues[key] = JSON.parse(JSON.stringify(INITIAL_ADJUSTMENTS[key as keyof Adjustments]));
      }
      setAdjustments((prev: Adjustments) => ({
        ...prev,
        ...resetValues,
        activeTools: setToolActive(getActiveTools(prev), sectionName, true),
      }));
    };

    const isPasteAllowed = copiedSectionAdjustments && copiedSectionAdjustments.section === sectionName;
    const translatedSection = t(`editor.adjustments.sections.${sectionName}`);

    const pasteLabel = copiedSectionAdjustments
      ? t('editor.adjustments.actions.pasteLabel', { section: translatedSection })
      : t('editor.adjustments.actions.pasteSettings');

    const options: any = [
      {
        label: t('editor.adjustments.actions.copySectionSettings', { section: translatedSection }),
        icon: Copy,
        onClick: handleCopy,
      },
      { label: pasteLabel, icon: ClipboardPaste, onClick: handlePaste, disabled: !isPasteAllowed },
      { type: OPTION_SEPARATOR },
      {
        label: t('editor.adjustments.actions.resetSectionSettings', { section: translatedSection }),
        icon: RotateCcw,
        onClick: handleReset,
      },
      { type: OPTION_SEPARATOR },
      {
        label: t('editor.adjustments.actions.customizePanels'),
        icon: LayoutList,
        submenu: [{ customComponent: AdjustmentSectionsSubMenu }],
      },
    ];

    showContextMenu(event.clientX, event.clientY, options);
  };

  return (
    <div className="flex flex-col h-full">
      <div className="p-3 flex justify-between items-center shrink-0 border-b border-surface relative">
        <Text variant={TextVariants.title}>{t('editor.adjustments.title')}</Text>
        <div className="flex items-center gap-1">
          <div className="flex items-center rounded-full hover:bg-surface transition-colors">
            <button
              className="p-2 pr-1 rounded-l-full disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              disabled={!selectedImage}
              onClick={() => handleAutoAdjustments()}
              onContextMenu={(e) => {
                e.preventDefault();
                setIsAutoOptionsOpen((open) => !open);
              }}
              data-tooltip={t('editor.adjustments.tooltips.autoAdjust')}
            >
              <PencilSparkles size={18} />
            </button>
            <button
              className={clsx(
                'p-1 pr-1.5 rounded-r-full transition-colors',
                isAutoOptionsOpen ? 'text-text-primary' : 'text-text-secondary hover:text-text-primary',
              )}
              onClick={() => setIsAutoOptionsOpen((open) => !open)}
              data-tooltip={t('editor.adjustments.auto.options')}
              aria-expanded={isAutoOptionsOpen}
            >
              <ChevronDown size={14} />
            </button>
          </div>
          <AnimatePresence>
            {isAutoOptionsOpen && <AutoAdjustOptionsPopover onClose={closeAutoOptions} />}
          </AnimatePresence>
          <button
            className={clsx(
              'p-2 rounded-full transition-colors',
              isWaveformVisible ? 'bg-surface hover:bg-card-active' : 'hover:bg-surface',
            )}
            onClick={onToggleWaveform}
            data-tooltip={t('editor.adjustments.tooltips.toggleAnalytics')}
          >
            <ChartArea size={18} />
          </button>
          <button
            className="p-2 rounded-full hover:bg-surface disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            disabled={!selectedImage}
            onClick={handleResetAdjustments}
            data-tooltip={t('editor.adjustments.tooltips.resetAdjustments')}
          >
            <RotateCcw size={18} />
          </button>
        </div>
      </div>

      <AnimatePresence initial={false}>
        {isWaveformVisible && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: waveformHeight || 256, opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: isResizingWaveform ? 0 : 0.2, ease: 'easeOut' }}
            className="shrink-0 flex flex-col relative border-b border-surface overflow-hidden"
          >
            <div className="grow w-full h-full p-3 pb-2 min-h-0">
              <ConnectedWaveform
                setDisplayMode={setActiveWaveformChannel}
                setAdjustments={setAdjustments}
                theme={theme}
              />
            </div>
            <Resizer direction={Orientation.Horizontal} onMouseDown={handleWaveformResize} />
          </motion.div>
        )}
      </AnimatePresence>

      <div className="grow overflow-y-scroll p-3 flex flex-col gap-2">
        {isTetherActive && (
          <div className="shrink-0">
            <CollapsibleSection
              isContentVisible={true}
              isOpen={isTetherOpen}
              onToggle={() => setIsTetherOpen((open) => !open)}
              title={t('editor.tether.title')}
            >
              <CameraSection />
            </CollapsibleSection>
          </div>
        )}
        {selectedImage ? (
          visibleSections.map((sectionName: string) => {
            // Film tuning only exists for converted negatives.
            if (sectionName === 'film' && !isNegative) {
              return null;
            }
            const title = t(`editor.adjustments.sections.${sectionName}`);

            return (
              <div className="shrink-0 group" key={sectionName}>
                <CollapsibleSection
                  isContentVisible={isToolActive(activeTools, sectionName)}
                  isOpen={collapsibleSectionsState[sectionName as keyof typeof collapsibleSectionsState]}
                  onContextMenu={(e: any) => handleSectionContextMenu(e, sectionName)}
                  onToggle={() => handleToggleSection(sectionName)}
                  onToggleVisibility={() => handleToggleVisibility(sectionName)}
                  title={title}
                >
                  <AdjustmentSectionContent
                    sectionName={sectionName}
                    activeTools={activeTools}
                    onToggleTool={handleToggleVisibility}
                    setAdjustments={setAdjustments}
                    theme={theme}
                    handleLutSelect={handleLutSelect}
                    onLutHover={setLutPreviewOverride}
                    appSettings={appSettings}
                    asShotWhiteBalance={selectedImage.asShotWhiteBalance}
                    isWbPickerActive={isWbPickerActive}
                    toggleWbPicker={toggleWbPicker}
                    isPointPickerActive={isPointPickerActive}
                    togglePointPicker={togglePointPicker}
                    onDragStateChange={onDragStateChange}
                  />
                </CollapsibleSection>
              </div>
            );
          })
        ) : (
          <div className="flex items-center justify-center h-full">
            <Text
              variant={TextVariants.heading}
              color={TextColors.secondary}
              weight={TextWeights.normal}
              className="text-center"
            >
              {t('editor.ai.noImageSelected')}
            </Text>
          </div>
        )}
      </div>
    </div>
  );
}
