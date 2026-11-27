import { useCallback, useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';

import Button from '../ui/Button';
import Text from '../ui/Text';
import { TextVariants } from '../../types/typography';
import { useUIStore } from '../../store/useUIStore';
import { ExternalEditMode, runExternalEdit } from '../../utils/externalEdit';

// Remembered for the session, like Lightroom keeps the last choice selected.
let lastMode: ExternalEditMode = 'adjusted';

/**
 * Lightroom-style "Edit Photo with…" choice for non-raw files: render a copy
 * with RapidRAW's adjustments, copy the file as it is, or open the original.
 */
export default function ExternalEditModal() {
  const { t } = useTranslation();
  const { isOpen, path } = useUIStore((state) => state.externalEditModalState);
  const setUI = useUIStore((state) => state.setUI);
  const [mode, setMode] = useState<ExternalEditMode>(lastMode);
  const [isMounted, setIsMounted] = useState(false);
  const [show, setShow] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setMode(lastMode);
      setIsMounted(true);
      const timer = setTimeout(() => setShow(true), 10);
      return () => clearTimeout(timer);
    }
    setShow(false);
    const timer = setTimeout(() => setIsMounted(false), 300);
    return () => clearTimeout(timer);
  }, [isOpen]);

  const close = useCallback(() => {
    setUI((state) => ({ externalEditModalState: { ...state.externalEditModalState, isOpen: false } }));
  }, [setUI]);

  const confirm = useCallback(() => {
    if (!path) return;
    lastMode = mode;
    close();
    runExternalEdit(path, mode, t);
  }, [path, mode, close, t]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>) => {
      if (e.key === 'Enter' || e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        e.nativeEvent.stopImmediatePropagation();
        if (e.key === 'Enter') confirm();
        else close();
      }
    },
    [confirm, close],
  );

  if (!isMounted) return null;

  const name = path ? path.split('?')[0].split(/[\\/]/).pop() : '';
  const options: Array<{ value: ExternalEditMode; title: string; desc: string }> = [
    {
      value: 'adjusted',
      title: t('modals.externalEdit.adjustedTitle', 'Edit a Copy with RapidRAW Adjustments'),
      desc: t(
        'modals.externalEdit.adjustedDesc',
        'Renders a new -Edit file with your current adjustments applied. The original is not touched.',
      ),
    },
    {
      value: 'copy',
      title: t('modals.externalEdit.copyTitle', 'Edit a Copy'),
      desc: t(
        'modals.externalEdit.copyDesc',
        'Copies the original file as it is, without RapidRAW adjustments, into a new -Edit file.',
      ),
    },
    {
      value: 'original',
      title: t('modals.externalEdit.originalTitle', 'Edit Original'),
      desc: t(
        'modals.externalEdit.originalDesc',
        'Opens the original file itself. Saving in the editor overwrites it; RapidRAW adjustments stay and apply on top.',
      ),
    },
  ];

  const modalContent = (
    <div
      aria-labelledby="external-edit-modal-title"
      aria-modal="true"
      className={`
        fixed inset-0 flex items-center justify-center z-[9999]
        bg-black/30 backdrop-blur-xs
        transition-opacity duration-300 ease-in-out
        ${show ? 'opacity-100' : 'opacity-0'}
      `}
      onClick={close}
      role="dialog"
    >
      <div
        className={`
          bg-surface rounded-lg shadow-xl p-6 w-full max-w-lg text-text-primary
          transform transition-all duration-300 ease-out
          ${show ? 'scale-100 opacity-100 translate-y-0' : 'scale-95 opacity-0 -translate-y-4'}
        `}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
        tabIndex={-1}
      >
        <Text variant={TextVariants.title} id="external-edit-modal-title" className="mb-1">
          {t('modals.externalEdit.title', 'Edit Photo in External Editor')}
        </Text>
        <Text className="mb-4 text-text-secondary truncate" title={name}>
          {name}
        </Text>
        <div className="flex flex-col gap-2" role="radiogroup">
          {options.map((option) => {
            const selected = mode === option.value;
            return (
              <button
                key={option.value}
                type="button"
                role="radio"
                aria-checked={selected}
                onClick={() => setMode(option.value)}
                onDoubleClick={() => {
                  setMode(option.value);
                  lastMode = option.value;
                  if (path) {
                    close();
                    runExternalEdit(path, option.value, t);
                  }
                }}
                className={`
                  flex items-start gap-3 text-left rounded-md px-3 py-2.5 border transition-colors
                  focus:outline-hidden
                  ${selected ? 'border-accent bg-accent/10' : 'border-transparent bg-bg-primary hover:bg-bg-primary/70'}
                `}
              >
                <span
                  className={`
                    mt-1 h-3.5 w-3.5 shrink-0 rounded-full border-2
                    ${selected ? 'border-accent bg-accent' : 'border-text-secondary'}
                  `}
                />
                <span className="flex flex-col">
                  <span className="text-sm font-medium">{option.title}</span>
                  <span className="text-xs text-text-secondary mt-0.5">{option.desc}</span>
                </span>
              </button>
            );
          })}
        </div>
        <div className="flex justify-end gap-3 mt-5">
          <Button
            className="bg-bg-primary shadow-transparent hover:bg-bg-primary text-white shadow-none focus:outline-hidden focus:ring-0"
            onClick={close}
            variant="ghost"
          >
            {t('modals.confirm.cancel')}
          </Button>
          <Button
            onClick={confirm}
            variant="primary"
            autoFocus={true}
            className="focus:outline-hidden focus:ring-0 focus:ring-offset-0"
          >
            {t('modals.externalEdit.edit', 'Edit')}
          </Button>
        </div>
      </div>
    </div>
  );

  return typeof document !== 'undefined' ? createPortal(modalContent, document.body) : modalContent;
}
