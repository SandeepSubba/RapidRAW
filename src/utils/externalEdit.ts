import { invoke } from '@tauri-apps/api/core';
import { toast } from 'react-toastify';
import { TFunction } from 'i18next';

import { Invokes } from '../types/invokes';

/** What the external editor gets, as in Lightroom's "Edit Photo with…" dialog. */
export type ExternalEditMode = 'adjusted' | 'copy' | 'original';

/**
 * Files that can be handed to the editor as-is (Edit Original / Edit a Copy).
 * Raw files can't: an editor needs rendered pixels, so they always go out as
 * a copy with RapidRAW's adjustments.
 */
const EDITABLE_ORIGINAL = /\.(tiff?|psd|psb|jpe?g|png|webp)$/i;

export function canEditOriginal(path: string): boolean {
  const source = path.split('?')[0];
  return EDITABLE_ORIGINAL.test(source);
}

export function runExternalEdit(path: string, mode: ExternalEditMode, t: TFunction) {
  if (mode === 'adjusted') {
    toast.info(t('contextMenus.toasts.externalEditRendering', 'Rendering for the external editor…'));
  }
  return invoke<{ outputPath: string; editor: string }>(Invokes.StartExternalEdit, { path, mode })
    .then((res) => {
      const name = res.outputPath.split(/[\\/]/).pop();
      toast.success(
        mode === 'original'
          ? t(
              'contextMenus.toasts.externalEditOpenedOriginal',
              'Opened the original {{name}} in {{editor}} — saving there updates it here.',
              { name, editor: res.editor },
            )
          : t(
              'contextMenus.toasts.externalEditOpened',
              'Opened {{name}} in {{editor}} — saves appear in the library automatically.',
              { name, editor: res.editor },
            ),
      );
    })
    .catch((err) => toast.error(String(err)));
}
