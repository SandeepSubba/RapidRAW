import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Loader2, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import Text from '../../ui/Text';
import { TextColors, TextVariants } from '../../../types/typography';
import { Invokes } from '../../ui/AppProperties';
import { useTetherStore } from '../../../store/useTetherStore';

// Floating live-view feed while USB tethering; frames arrive as base64 JPEGs
// from the camera thread. Rendered app-wide so it survives view switches.
export default function LiveViewOverlay() {
  const { t } = useTranslation();
  const liveView = useTetherStore((s) => s.liveView);
  const model = useTetherStore((s) => s.camera?.model);
  const setTether = useTetherStore((s) => s.setTether);
  const [frame, setFrame] = useState<string | null>(null);

  useEffect(() => {
    if (!liveView) {
      setFrame(null);
      return;
    }
    const unlistenFrame = listen('tether-preview-frame', (event: any) => setFrame(event.payload));
    const unlistenStopped = listen('tether-live-view-stopped', () => setTether({ liveView: false }));
    return () => {
      unlistenFrame.then((f) => f());
      unlistenStopped.then((f) => f());
    };
  }, [liveView, setTether]);

  if (!liveView) {
    return null;
  }

  const handleClose = () => {
    invoke(Invokes.TetherSetLiveView, { on: false }).catch(() => {});
    setTether({ liveView: false });
  };

  return (
    <div className="fixed bottom-6 right-6 z-50 w-[440px] rounded-lg overflow-hidden shadow-2xl bg-bg-secondary border border-surface">
      <div className="flex items-center justify-between px-3 py-2">
        <span className="flex items-center gap-2">
          <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
          <Text variant={TextVariants.small} color={TextColors.secondary} className="truncate">
            {model} — {t('editor.tether.camera.liveView')}
          </Text>
        </span>
        <button
          className="p-1 rounded-md text-text-secondary hover:text-text-primary transition-colors"
          onClick={handleClose}
        >
          <X size={14} />
        </button>
      </div>
      <div className="bg-black flex items-center justify-center min-h-[248px]">
        {frame ? (
          <img src={`data:image/jpeg;base64,${frame}`} alt="Live view" className="w-full" />
        ) : (
          <Loader2 size={24} className="animate-spin text-text-secondary" />
        )}
      </div>
    </div>
  );
}
