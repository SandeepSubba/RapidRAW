import { useEffect, useRef } from 'react';
import { useProcessStore } from '../../../store/useProcessStore';
import { useThumbnails } from '../../../hooks/useThumbnails';

interface LazyThumbProps {
  path: string;
  className?: string;
  imgClassName?: string;
}

/**
 * Renders a thumbnail for `path`, requesting it from the backend only when the element
 * scrolls into view (IntersectionObserver). Avoids flooding the decoder when hundreds
 * of cells mount at once.
 */
export default function LazyThumb({ path, className, imgClassName }: LazyThumbProps) {
  const url = useProcessStore((s) => s.thumbnails[path]);
  const { requestThumbnails } = useThumbnails();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (url || !ref.current) return;
    const el = ref.current;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          requestThumbnails([path]);
          observer.disconnect();
        }
      },
      { rootMargin: '300px' },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [path, url, requestThumbnails]);

  return (
    <div ref={ref} className={className}>
      {url ? (
        <img src={url} alt="" draggable={false} className={imgClassName ?? 'w-full h-full object-cover'} />
      ) : (
        <div className="w-full h-full bg-surface animate-pulse" />
      )}
    </div>
  );
}
