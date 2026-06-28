import { Star } from 'lucide-react';
import { COLOR_HEX } from './importFilters';

/** Small overlay showing a photo's color label dot + star rating (for grid/viewer cells). */
export function RatingColor({ rating, color }: { rating?: number; color?: string }) {
  if (!rating && !color) return null;
  return (
    <div className="absolute top-1.5 left-1.5 flex items-center gap-1 pointer-events-none">
      {color && (
        <span className="w-3 h-3 rounded-full border border-white/60" style={{ background: COLOR_HEX[color] ?? '#888' }} />
      )}
      {!!rating && rating > 0 && (
        <span className="flex items-center gap-px bg-black/55 rounded px-1 py-0.5">
          {Array.from({ length: rating }).map((_, i) => (
            <Star key={i} size={9} className="fill-yellow-400 text-yellow-400" />
          ))}
        </span>
      )}
    </div>
  );
}
