import { useCallback, useEffect, useMemo, useRef, useState } from "react";

type UseIncrementalItemsOptions = {
  initialCount: number;
  step: number;
  resetKey: unknown;
  rootMargin?: string;
};

export function useIncrementalItems<T>(
  items: readonly T[],
  {
    initialCount,
    step,
    resetKey,
    rootMargin = "720px",
  }: UseIncrementalItemsOptions
) {
  const normalizedInitialCount = Math.max(1, initialCount);
  const normalizedStep = Math.max(1, step);
  const [visibleCount, setVisibleCount] = useState(normalizedInitialCount);
  const observerRef = useRef<IntersectionObserver | null>(null);

  useEffect(() => {
    setVisibleCount((current) => (
      current === normalizedInitialCount ? current : normalizedInitialCount
    ));
  }, [normalizedInitialCount, resetKey]);

  useEffect(() => {
    return () => observerRef.current?.disconnect();
  }, []);

  const visibleItems = useMemo(
    () => items.slice(0, visibleCount),
    [items, visibleCount]
  );
  const hasMore = visibleCount < items.length;

  const loadMore = useCallback(() => {
    setVisibleCount((current) =>
      Math.min(current + normalizedStep, items.length)
    );
  }, [items.length, normalizedStep]);

  const sentinelRef = useCallback(
    (node: HTMLDivElement | null) => {
      observerRef.current?.disconnect();

      if (!node || !hasMore) {
        return;
      }

      if (typeof IntersectionObserver === "undefined") {
        loadMore();
        return;
      }

      observerRef.current = new IntersectionObserver(
        ([entry]) => {
          if (entry?.isIntersecting) {
            loadMore();
          }
        },
        { rootMargin }
      );
      observerRef.current.observe(node);
    },
    [hasMore, loadMore, rootMargin]
  );

  return {
    hasMore,
    loadMore,
    sentinelRef,
    visibleCount,
    visibleItems,
  };
}
