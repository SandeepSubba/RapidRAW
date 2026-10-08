import { invoke } from '@tauri-apps/api/core';
import { Crop } from 'react-image-crop';
import { Adjustments, GuideLine } from './adjustments';
import { getOrientedDimensions, guidedRectToCrop } from './cropUtils';

type SetAdjustments = (fn: (prev: Adjustments) => Adjustments) => void;

const sameCrop = (a?: Crop | null, b?: Crop | null) =>
  !!a && !!b && a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;

/** True when there is no crop, or the crop is the whole frame (what image load writes). */
const isUncropped = (prev: Adjustments, imageWidth: number, imageHeight: number) => {
  const crop = prev.crop;
  if (!crop) return true;
  const { width, height } = getOrientedDimensions(imageWidth, imageHeight, prev.orientationSteps || 0);
  return crop.x <= 1 && crop.y <= 1 && crop.width >= width - 2 && crop.height >= height - 2;
};

/** Whether auto-crop may write the crop: it is untouched by the user since we last set it. */
const cropIsOurs = (prev: Adjustments, imageWidth: number, imageHeight: number) =>
  isUncropped(prev, imageWidth, imageHeight) || sameCrop(prev.crop, prev.guidedPerspective?.appliedCrop);

/**
 * Trim the empty border the perspective correction leaves, using the solver's
 * largest inscribed rectangle. The result lands asynchronously, so it is
 * dropped if the guides changed meanwhile, auto-crop was switched off, or the
 * user has cropped by hand.
 */
export async function applyGuidedAutoCrop(
  lines: GuideLine[],
  setAdjustments: SetAdjustments,
  imageWidth: number,
  imageHeight: number,
) {
  let rect: [number, number, number, number];
  try {
    const res: any = await invoke('calculate_guided_perspective', { lines, width: imageWidth, height: imageHeight });
    if (!res?.valid || !Array.isArray(res.crop)) return;
    rect = res.crop;
  } catch (e) {
    console.error('Guided perspective auto-crop failed', e);
    return;
  }

  setAdjustments((prev) => {
    const gp = prev.guidedPerspective;
    if (!gp?.autoCrop || gp.lines !== lines) return prev;
    // Fine rotation adds empty corners of its own that this rectangle ignores.
    if (Math.abs(prev.rotation || 0) > 0.01) return prev;
    if (!cropIsOurs(prev, imageWidth, imageHeight)) return prev;

    const crop = guidedRectToCrop(
      rect,
      imageWidth,
      imageHeight,
      prev.orientationSteps || 0,
      !!prev.flipHorizontal,
      !!prev.flipVertical,
      prev.aspectRatio ?? null,
    );
    return { ...prev, crop, guidedPerspective: { ...gp, appliedCrop: crop } };
  });
}

/** Restore the full frame if the current crop is the one auto-crop wrote. */
export const withdrawAutoCrop = (prev: Adjustments): Pick<Adjustments, 'crop'> & { appliedCrop: null } => ({
  crop: sameCrop(prev.crop, prev.guidedPerspective?.appliedCrop) ? null : prev.crop,
  appliedCrop: null,
});

/**
 * Single entry point for every guide edit — draw, endpoint drag, delete,
 * clear — so the correction and its auto-crop stay in step.
 */
export function commitGuideLines(
  lines: GuideLine[],
  setAdjustments: SetAdjustments,
  imageWidth: number,
  imageHeight: number,
) {
  const active = lines.length >= 2;
  setAdjustments((prev) => {
    const gp = prev.guidedPerspective;
    if (active) {
      return { ...prev, guidedPerspective: { ...gp, lines, enabled: true } };
    }
    const { crop, appliedCrop } = withdrawAutoCrop(prev);
    return { ...prev, crop, guidedPerspective: { ...gp, lines, enabled: false, appliedCrop } };
  });
  if (active) {
    applyGuidedAutoCrop(lines, setAdjustments, imageWidth, imageHeight);
  }
}

/** Toggle auto-crop, applying or withdrawing the crop right away. */
export function setGuidedAutoCrop(
  enabled: boolean,
  setAdjustments: SetAdjustments,
  imageWidth: number,
  imageHeight: number,
) {
  let lines: GuideLine[] = [];
  setAdjustments((prev) => {
    const gp = prev.guidedPerspective;
    lines = gp?.lines ?? [];
    if (enabled) {
      return { ...prev, guidedPerspective: { ...gp, autoCrop: true } };
    }
    const { crop, appliedCrop } = withdrawAutoCrop(prev);
    return { ...prev, crop, guidedPerspective: { ...gp, autoCrop: false, appliedCrop } };
  });
  if (enabled && lines.length >= 2) {
    applyGuidedAutoCrop(lines, setAdjustments, imageWidth, imageHeight);
  }
}
