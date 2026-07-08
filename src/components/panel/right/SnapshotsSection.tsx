import { useState } from 'react';
import { v4 as uuidv4 } from 'uuid';
import { useTranslation } from 'react-i18next';
import { Check, FileEdit, History, Plus, RefreshCw, Trash2 } from 'lucide-react';
import { useEditorStore } from '../../../store/useEditorStore';
import { useEditorActions } from '../../../hooks/useEditorActions';
import { useContextMenu } from '../../../context/ContextMenuContext';
import { OPTION_SEPARATOR, Preset } from '../../ui/AppProperties';
import { Adjustments, AdjustmentSnapshot, INITIAL_ADJUSTMENTS } from '../../../utils/adjustments';
import PresetItemDisplay from './PresetItemDisplay';
import Text from '../../ui/Text';
import { TextColors, TextVariants, TextWeights } from '../../../types/typography';

// Preview id for a snapshot thumbnail. Embeds createdAt so an overwrite (which bumps
// createdAt) produces a fresh id, and the pipeline revokes the stale one.
export const snapshotPreviewId = (s: AdjustmentSnapshot) => `snapshot-${s.id}-${s.createdAt}`;

interface SnapshotsSectionProps {
  previews: Record<string, string | null>;
  isGeneratingPreviews: boolean;
}

// In-editor checkpoints of the full edit state, stored in the sidecar. Rendered as
// preset-style cards (reusing PresetItemDisplay) so they read as part of the panel.
export default function SnapshotsSection({ previews, isGeneratingPreviews }: SnapshotsSectionProps) {
  const adjustments = useEditorStore((s) => s.adjustments);
  const selectedImage = useEditorStore((s) => s.selectedImage);
  const { setAdjustments } = useEditorActions();
  const { showContextMenu } = useContextMenu();
  const { t } = useTranslation();
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [tempName, setTempName] = useState('');

  if (!selectedImage) {
    return null;
  }
  const snapshots: AdjustmentSnapshot[] = adjustments.snapshots || [];

  const snapshotState = (adj: Adjustments): Partial<Adjustments> => {
    const { snapshots: _snapshots, ...state } = adj;
    return structuredClone(state);
  };

  const handleSave = () => {
    setAdjustments((prev: Adjustments) => ({
      ...prev,
      snapshots: [
        ...(prev.snapshots || []),
        {
          id: uuidv4(),
          name: t('editor.presets.snapshots.defaultName', { count: (prev.snapshots?.length || 0) + 1 }),
          createdAt: Date.now(),
          state: snapshotState(prev),
        },
      ],
    }));
  };

  const handleApply = (version: AdjustmentSnapshot) => {
    setAdjustments((prev: Adjustments) => ({
      ...INITIAL_ADJUSTMENTS,
      ...version.state,
      snapshots: prev.snapshots,
    }));
  };

  const handleOverwrite = (version: AdjustmentSnapshot) => {
    setAdjustments((prev: Adjustments) => ({
      ...prev,
      snapshots: (prev.snapshots || []).map((v) =>
        v.id === version.id ? { ...v, state: snapshotState(prev), createdAt: Date.now() } : v,
      ),
    }));
  };

  const handleDelete = (version: AdjustmentSnapshot) => {
    setAdjustments((prev: Adjustments) => ({
      ...prev,
      snapshots: (prev.snapshots || []).filter((v) => v.id !== version.id),
    }));
  };

  const startRename = (version: AdjustmentSnapshot) => {
    setRenamingId(version.id);
    setTempName(version.name);
  };

  const commitRename = () => {
    const name = tempName.trim();
    if (name && renamingId) {
      setAdjustments((prev: Adjustments) => ({
        ...prev,
        snapshots: (prev.snapshots || []).map((v) => (v.id === renamingId ? { ...v, name } : v)),
      }));
    }
    setRenamingId(null);
  };

  const handleRowContextMenu = (event: React.MouseEvent, version: AdjustmentSnapshot) => {
    event.preventDefault();
    event.stopPropagation();
    showContextMenu(event.clientX, event.clientY, [
      { label: t('editor.presets.snapshots.apply'), icon: Check, onClick: () => handleApply(version) },
      { label: t('editor.presets.snapshots.overwrite'), icon: RefreshCw, onClick: () => handleOverwrite(version) },
      {
        label: t('editor.presets.snapshots.rename'),
        icon: FileEdit,
        onClick: () => startRename(version),
      },
      { type: OPTION_SEPARATOR },
      { label: t('editor.presets.snapshots.delete'), icon: Trash2, onClick: () => handleDelete(version) },
    ]);
  };

  const dateLabel = (createdAt: number) =>
    new Date(createdAt).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });

  return (
    <div className="mb-4 pb-4 border-b border-surface">
      <div className="flex items-center justify-between mb-2">
        <Text variant={TextVariants.heading} className="flex items-center gap-2">
          <History size={16} />
          {t('editor.presets.snapshots.title')}
        </Text>
        <button
          className="p-1.5 rounded-full hover:bg-surface transition-colors"
          onClick={handleSave}
          data-tooltip={t('editor.presets.snapshots.saveTooltip')}
        >
          <Plus size={16} />
        </button>
      </div>
      {snapshots.length === 0 ? (
        <Text color={TextColors.secondary} className="text-sm">
          {t('editor.presets.snapshots.empty')}
        </Text>
      ) : (
        <div className="space-y-2">
          {snapshots.map((version) => {
            const isRenaming = renamingId === version.id;
            return (
              <div
                key={version.id}
                className="relative group cursor-pointer"
                style={{ borderRadius: '10px' }}
                onClick={() => !isRenaming && handleApply(version)}
                onContextMenu={(e) => handleRowContextMenu(e, version)}
              >
                <PresetItemDisplay
                  preset={{ id: version.id, name: version.name, adjustments: version.state } as Preset}
                  previewUrl={previews[snapshotPreviewId(version)] || ''}
                  isGeneratingPreviews={isGeneratingPreviews}
                  subtitle={
                    <>
                      <History size={12} className="text-text-secondary" />
                      <Text variant={TextVariants.small} color={TextColors.secondary} weight={TextWeights.normal}>
                        {dateLabel(version.createdAt)}
                      </Text>
                    </>
                  }
                  nameSlot={
                    isRenaming ? (
                      <input
                        autoFocus
                        className="bg-transparent border-b border-text-secondary outline-none text-sm w-full"
                        value={tempName}
                        onChange={(e) => setTempName(e.target.value)}
                        onBlur={commitRename}
                        onClick={(e) => e.stopPropagation()}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') commitRename();
                          if (e.key === 'Escape') setRenamingId(null);
                        }}
                      />
                    ) : undefined
                  }
                />
                {!isRenaming && (
                  <button
                    className="absolute top-2 right-2 p-1 rounded-md bg-bg-tertiary text-text-secondary opacity-0 group-hover:opacity-100 hover:text-primary transition-opacity"
                    data-tooltip={t('editor.presets.snapshots.rename')}
                    onClick={(e) => {
                      e.stopPropagation();
                      startRename(version);
                    }}
                  >
                    <FileEdit size={14} />
                  </button>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
