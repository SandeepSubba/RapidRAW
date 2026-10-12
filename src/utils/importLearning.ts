import { invoke } from '@tauri-apps/api/core';
import { CullingSuggestions, Invokes } from '../components/ui/AppProperties';
import { useImportStore } from '../store/useImportStore';
import { useSettingsStore } from '../store/useSettingsStore';
import { computeVisibleSet } from '../components/views/import/importFilters';

// How the import window's culling learns from finished imports.
//
// - Auto-select best: for every scored similar group the backend learns which frames you
//   kept (cue weights per kind of photo) and your keep policy — how many frames of a burst
//   you keep and how good a burst must be. Auto-select then applies that policy instead of
//   always keeping exactly one frame above a fixed grade.
// - The Assistant: its import-window cull decisions are remembered; at import, the ones you
//   overrode (unmarked a pick, rescued a reject, changed its stars) are recorded and quoted
//   in later cull requests so it adapts to your taste.
//
// Both learn only while "Personalize selection" is on, and only from what you import.

/** What culling has learned (mirrors cull_model::LearningSummary). */
export interface CullLearning {
  imports: number;
  groups: number;
  cueSamples: number;
  /** P(keep a burst) by its best frame's grade 0–5. */
  keepGrade: number[];
  /** P(keep a frame | burst kept) by rank: best, 2nd, 3rd, 4th+. */
  keepRank: number[];
  typicalKeep: number | null;
}

/** An assistant cull decision made in the import window, kept until import. */
export interface AiDecision {
  flag: 'pick' | 'reject' | null;
  rating: number | null;
  reason: string | null;
}

const fileNameOf = (p: string) => p.split(/[\\/]/).pop() || p;

interface RankedGroup {
  ranked: string[];
  bestGrade: number;
}

/**
 * Similar groups best-first by the app's score, with the best frame's 0–5 grade on the
 * same min–max scale as the grid badges. `exclude` drops photos that can't be imported.
 */
export function rankedGroups(
  suggestions: CullingSuggestions,
  exclude: (p: string) => boolean = () => false,
): RankedGroup[] {
  const all: number[] = [];
  suggestions.similarGroups.forEach((g) =>
    all.push(g.representative.qualityScore, ...g.duplicates.map((d) => d.qualityScore)),
  );
  const min = all.length ? Math.min(...all) : 0;
  const max = all.length ? Math.max(...all) : 1;
  const grade = (s: number) => Math.round(max > min ? ((s - min) / (max - min)) * 5 : 5);

  return suggestions.similarGroups
    .map((g) => {
      const members = [g.representative, ...g.duplicates]
        .filter((m) => !exclude(m.path))
        .sort((a, b) => b.qualityScore - a.qualityScore);
      return { ranked: members.map((m) => m.path), bestGrade: members.length ? grade(members[0].qualityScore) : 0 };
    })
    .filter((g) => g.ranked.length > 0);
}

/** Which frames of a ranked burst the (learned or default) policy keeps. */
export function keepersOf(group: RankedGroup, learning: CullLearning | null): string[] {
  const keepGrade = learning?.keepGrade ?? [0, 0, 0, 1, 1, 1];
  const keepRank = learning?.keepRank ?? [1, 0, 0, 0];
  if ((keepGrade[Math.min(5, group.bestGrade)] ?? 0) < 0.5) return [];
  const kept = group.ranked.filter((_, r) => (keepRank[Math.min(keepRank.length - 1, r)] ?? 0) >= 0.5);
  // A burst worth keeping always keeps its best frame.
  return kept.length ? kept : group.ranked.slice(0, 1);
}

export async function loadCullLearning(): Promise<CullLearning | null> {
  try {
    return await invoke<CullLearning>(Invokes.GetCullLearning);
  } catch (e) {
    console.warn('Could not load culling learning:', e);
    return null;
  }
}

/** One assistant decision the photographer overrode (mirrors assistant::CullCorrection). */
interface CullCorrection {
  file: string;
  aiFlag: string | null;
  aiRating: number | null;
  aiReason: string | null;
  kept: boolean;
  rating: number | null;
}

/** Compare the assistant's import-window decisions with what is being imported. */
export function assistantFeedback(
  decisions: Record<string, AiDecision>,
  importable: (p: string) => boolean,
  keptPaths: Set<string>,
  ratings: Record<string, number>,
): { corrections: CullCorrection[]; agreed: number } {
  const corrections: CullCorrection[] = [];
  let agreed = 0;
  Object.entries(decisions).forEach(([path, d]) => {
    if (!importable(path) || (!d.flag && !d.rating)) return;
    const kept = keptPaths.has(path);
    const rating = ratings[path] || 0;
    const flagOk = !d.flag || (d.flag === 'pick') === kept;
    const ratingOk = !d.rating || !kept || d.rating === rating;
    if (flagOk && ratingOk) {
      agreed++;
    } else {
      corrections.push({
        file: fileNameOf(path),
        aiFlag: d.flag,
        aiRating: d.rating,
        aiReason: d.reason,
        kept,
        rating: rating || null,
      });
    }
  });
  return { corrections, agreed };
}

/**
 * Learn from the import that is starting: the keep/skip outcome of every scored burst,
 * and how the assistant's decisions fared. Best-effort — never blocks the import.
 * Returns the updated learning summary when the backend learned anything.
 */
export async function learnFromImport(): Promise<CullLearning | null> {
  const s = useImportStore.getState();
  if (!s.personalizeSelection) return null;
  const rawExts = useSettingsStore.getState().supportedTypes?.raw ?? [];
  // Hidden file types (e.g. the JPEG of a RAW+JPEG pair) and photos already in the
  // destination can't be imported, so skipping them says nothing about taste.
  const typeVisible = computeVisibleSet(s.scannedPaths, s.fileTypeFilter, rawExts);
  const importable = (p: string) => !s.alreadyImported.has(p) && (!typeVisible || typeVisible.has(p));

  let summary: CullLearning | null = null;
  if (s.suggestions && s.scoresReady) {
    const groups = rankedGroups(s.suggestions, (p) => !importable(p))
      .filter((g) => g.ranked.length >= 2)
      .map((g) => ({ ranked: g.ranked, kept: g.ranked.map((p) => s.keptPaths.has(p)), bestGrade: g.bestGrade }));
    if (groups.length) {
      try {
        summary = await invoke<CullLearning>(Invokes.RecordCullPicks, { groups });
      } catch (e) {
        console.warn('Culling-preference learning skipped:', e);
      }
    }
  }

  const { corrections, agreed } = assistantFeedback(s.aiDecisions, importable, s.keptPaths, s.ratings);
  if (corrections.length || agreed) {
    try {
      await invoke(Invokes.RecordAssistantCullFeedback, { corrections, agreed });
    } catch (e) {
      console.warn('Assistant cull feedback skipped:', e);
    }
  }
  return summary;
}

/** One-line description of what has been learned, for the import window. */
export function describeLearning(l: CullLearning | null): string | null {
  if (!l || l.groups === 0) return null;
  const parts = [
    `Learned from ${l.groups} burst${l.groups === 1 ? '' : 's'} over ${l.imports} import${l.imports === 1 ? '' : 's'}`,
  ];
  if (l.typicalKeep) parts.push(`you keep ~${l.typicalKeep.toFixed(1).replace(/\.0$/, '')} per burst`);
  const keepsN = l.keepRank.filter((p) => p >= 0.5).length;
  if (keepsN > 1) parts.push(`auto-select keeps the top ${keepsN}`);
  const minGrade = l.keepGrade.findIndex((p) => p >= 0.5);
  if (minGrade !== -1 && minGrade !== 3) parts.push(`keeps bursts graded ${minGrade}+`);
  return parts.join(' · ');
}
