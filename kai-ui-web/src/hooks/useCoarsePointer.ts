"use client";

import { useEffect, useState } from "react";

const QUERY = "(pointer: coarse)";

/** True en tablets/teléfonos táctiles (no depende del ancho de pantalla). */
export function useCoarsePointer(): boolean {
  const [isCoarse, setIsCoarse] = useState(false);

  useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return;
    const mq = window.matchMedia(QUERY);
    const update = () => setIsCoarse(mq.matches);
    update();
    mq.addEventListener("change", update);
    return () => mq.removeEventListener("change", update);
  }, []);

  return isCoarse;
}
