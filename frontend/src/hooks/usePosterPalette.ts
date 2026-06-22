import { useEffect, useState } from "react";
import { Vibrant } from "node-vibrant/browser";

type PosterPalette = {
  primary: string;
  secondary: string;
  tertiary: string;
};

const fallbackPalette: PosterPalette = {
  primary: "rgba(242, 242, 247, 0.92)",
  secondary: "rgba(255, 255, 255, 0.86)",
  tertiary: "rgba(232, 232, 237, 0.88)",
};
const paletteCache = new Map<string, PosterPalette>();
const MAX_PALETTE_CACHE = 96;

function rgb(rgb?: [number, number, number], alpha = 1) {
  if (!rgb) return null;
  return `rgba(${Math.round(rgb[0])}, ${Math.round(rgb[1])}, ${Math.round(rgb[2])}, ${alpha})`;
}

export function usePosterPalette(src?: string): PosterPalette {
  const [palette, setPalette] = useState<PosterPalette>(() => (
    src ? paletteCache.get(src) ?? fallbackPalette : fallbackPalette
  ));

  useEffect(() => {
    let alive = true;
    if (!src) {
      setPalette(fallbackPalette);
      return () => {
        alive = false;
      };
    }
    const cached = paletteCache.get(src);
    if (cached) {
      setPalette(cached);
      return () => {
        alive = false;
      };
    }

    Vibrant.from(src)
      .maxColorCount(48)
      .quality(4)
      .getPalette()
      .then((result) => {
        if (!alive) return;
        const nextPalette = {
          primary: rgb(result.Muted?.rgb ?? result.Vibrant?.rgb, 0.82) ?? fallbackPalette.primary,
          secondary: rgb(result.LightMuted?.rgb ?? result.LightVibrant?.rgb, 0.78) ?? fallbackPalette.secondary,
          tertiary: rgb(result.DarkMuted?.rgb ?? result.DarkVibrant?.rgb, 0.58) ?? fallbackPalette.tertiary,
        };
        paletteCache.set(src, nextPalette);
        while (paletteCache.size > MAX_PALETTE_CACHE) {
          const oldest = paletteCache.keys().next().value;
          if (!oldest) break;
          paletteCache.delete(oldest);
        }
        setPalette(nextPalette);
      })
      .catch(() => {
        if (alive) setPalette(fallbackPalette);
      });

    return () => {
      alive = false;
    };
  }, [src]);

  return palette;
}
