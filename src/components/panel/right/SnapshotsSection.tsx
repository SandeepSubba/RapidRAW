import { useState } from 'react';
import { v4 as uuidv4 } from 'uuid';
import { useTranslation } from 'react-i18next';
import { Check, FileEdit, History, Plus, RefreshCw, Trash2 } from 'lucide-react';
import { useEditorStore } from '../../../store/useEditorStore';
import { useEditorActions } from '../../../hooks/useEditorActions';
import { useContextMenu } from '../../../context/ContextMenuContext';
import { OPTION_SEPARATOR } from '../../ui/AppProperties';
import { Adjustments, AdjustmentSnapshot, INITIAL_ADJUSTMENTS } from '../../../utils/adjustments';
import Text from '../../ui/Text';
import { TextColors, TextVariants, TextWeights } from '../../../types/typography';

// In-editor checkpoints of the full edit state, stored inside the sidecar. They
// complement library-level Virtual Copies: snapshots switch looks in place while
// you work, without creating new library entries. Applying, renaming and deleting all go through
// setAdjustments and are therefore undoable and auto-saved like any other edit.
export default function SnapshotsSection() {
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
        onClick: () => {
          setRenamingId(version.id);
          setTempName(version.name);
        },
      },
      { type: OPTION_SEPARATOR },
      { label: t('editor.presets.snapshots.delete'), icon: Trash2, onClick: () => handleDelete(version) },
    ]);
  };

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
        <div className="space-y-1">
          {snapshots.map((version) => (
            <button
              key={version.id}
              className="w-full flex items-center justify-between p-2 rounded-md bg-surface hover:bg-card-active cursor-pointer transition-colors group text-left"
              onClick={() => handleApply(version)}
              onContextMenu={(e) => handleRowContextMenu(e, version)}
            >
              {renamingId === version.id ? (
                <input
                  autoFocus
                  className="bg-transparent border-b border-text-secondary outline-none text-sm w-full mr-2"
                  value={tempName}
                  onChange={(e) => setTempName(e.target.value)}
                  onBlur={commitRename}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') commitRename();
                    if (e.key === 'Escape') setRenamingId(null);
                  }}
                  onClick={(e) => e.stopPropagation()}
                />
              ) : (
                <Text weight={TextWeights.medium} className="truncate text-sm">
                  {version.name}
                </Text>
              )}
              <Text color={TextColors.secondary} className="text-xs shrink-0 ml-2">
                {new Date(version.createdAt).toLocaleDateString(undefined, {
                  month: 'short',
                  day: 'numeric',
                })}
              </Text>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
