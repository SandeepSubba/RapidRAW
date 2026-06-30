import { useEffect } from 'react';
import { useSdImportActions } from './useSdImportActions';

// Shift + 1-5 → color labels (red/yellow/green/blue/purple); Shift + 0 clears.
const SHIFT_COLORS: Record<string, string> = { '1': 'red', '2': 'yellow', '3': 'green', '4': 'blue', '5': 'purple' };

/**
 * Culling keyboard shortcuts active while the importer is open. Acts on the focused
 * (active) photo: 1-5 stars, 0 clears rating, Shift+1-5 color labels, P keep, X skip.
 * Mirrors RapidRAW's library rating keybinds.
 */
export function useImportKeyboard() {
  const { rateActive, colorActive, keepActive, skipActive } = useSdImportActions();

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT' || t.isContentEditable)) {
        return;
      }
      if (e.metaKey || e.ctrlKey || e.altKey) return;

      // Accept both the number row (DigitN) and the numeric keypad (NumpadN).
      const digit = e.code.startsWith('Digit')
        ? e.code.slice(5)
        : /^Numpad[0-9]$/.test(e.code)
          ? e.code.slice(6)
          : null;
      if (digit !== null) {
        if (e.shiftKey) {
          if (digit === '0') {
            e.preventDefault();
            colorActive(null);
          } else if (SHIFT_COLORS[digit]) {
            e.preventDefault();
            colorActive(SHIFT_COLORS[digit]);
          }
        } else if (/^[0-5]$/.test(digit)) {
          e.preventDefault();
          rateActive(Number(digit));
        }
        return;
      }
      if (e.code === 'KeyP') {
        e.preventDefault();
        keepActive();
      } else if (e.code === 'KeyX') {
        e.preventDefault();
        skipActive();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [rateActive, colorActive, keepActive, skipActive]);
}
