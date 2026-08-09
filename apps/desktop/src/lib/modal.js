import { useEffect } from "react";

const FOCUSABLE =
  'a[href],button:not([disabled]),input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])';

/**
 * Wire up standard modal/dialog keyboard behavior for an open overlay:
 *  - Escape closes it
 *  - Tab is trapped inside the container
 *  - focus moves into the dialog on open and is restored to the previously
 *    focused element on close
 *
 * `active` toggles the behavior; `ref` points at the dialog container.
 */
export function useModalBehavior(active, onClose, ref) {
  useEffect(() => {
    if (!active) return undefined;
    const previouslyFocused = document.activeElement;
    const el = ref?.current;

    const focusables = () =>
      el ? Array.from(el.querySelectorAll(FOCUSABLE)) : [];

    // Move focus in unless something inside is already focused (e.g. autoFocus).
    if (el && !el.contains(document.activeElement)) {
      const first = focusables()[0];
      if (first) first.focus();
    }

    const onKey = (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose?.();
        return;
      }
      if (e.key === "Tab" && el) {
        const list = focusables();
        if (!list.length) return;
        const first = list[0];
        const last = list[list.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };

    document.addEventListener("keydown", onKey, true);
    return () => {
      document.removeEventListener("keydown", onKey, true);
      if (previouslyFocused && typeof previouslyFocused.focus === "function") {
        previouslyFocused.focus();
      }
    };
  }, [active, onClose, ref]);
}
