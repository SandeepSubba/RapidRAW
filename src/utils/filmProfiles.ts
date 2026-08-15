// Film-stock conversion profiles (NegaFix-style): named parameter sets for the
// negative conversion. Params only — bounds are per-frame by design and never
// belong in a profile. Built-ins are pragmatic looks tuned by eye, not
// colorimetric stock emulation.

export interface FilmProfile {
  name: string;
  // Matched (case-insensitive substring) against the roll's film-stock EXIF
  // (ImageDescription, written by the scanner) to auto-suggest the profile.
  filmStock?: string;
  params: {
    redWeight: number;
    greenWeight: number;
    blueWeight: number;
    exposure: number;
    contrast: number;
  };
  clipBlack?: number; // fraction, e.g. 0.001
  clipWhite?: number;
  builtin?: boolean;
}

const p = (
  name: string,
  contrast: number,
  exposure = 0,
  redWeight = 1,
  greenWeight = 1,
  blueWeight = 1,
  filmStock?: string,
): FilmProfile => ({
  name,
  filmStock,
  params: { redWeight, greenWeight, blueWeight, exposure, contrast },
  builtin: true,
});

export const BUILTIN_FILM_PROFILES: FilmProfile[] = [
  p('Generic C-41', 1.0),
  p('Portrait — low contrast', 0.85, 0.1, 1.03, 1.0, 0.97, 'portra'),
  p('Consumer — punchy', 1.25, 0, 1.0, 1.0, 1.0, 'gold'),
  p('B&W Grade 1 (soft)', 0.7),
  p('B&W Grade 2', 1.0),
  p('B&W Grade 3', 1.3),
  p('B&W Grade 4 (hard)', 1.7),
];

export function allProfiles(userProfiles?: FilmProfile[] | null): FilmProfile[] {
  return [...BUILTIN_FILM_PROFILES, ...(userProfiles ?? [])];
}

// Best profile for a roll's film-stock string, or null.
export function suggestProfile(profiles: FilmProfile[], stock?: string | null): FilmProfile | null {
  if (!stock) return null;
  const s = stock.toLowerCase();
  return profiles.find((pr) => pr.filmStock && s.includes(pr.filmStock.toLowerCase())) ?? null;
}
