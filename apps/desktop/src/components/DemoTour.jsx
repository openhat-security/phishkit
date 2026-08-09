import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  DEMO_TOUR_STEPS,
  DEMO_TOUR_STATUS,
  loadDemoTourState,
  resolveStepTestIds,
  saveDemoTourState,
} from "../lib/demoTour";

const PAD = 6;

function findTarget(testIds) {
  for (const id of testIds) {
    const el = document.querySelector(`[data-testid="${id}"]`);
    if (el) return el;
  }
  return null;
}

function tipPosition(rect, tipW, tipH) {
  const gap = 12;
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  let top = rect.bottom + gap;
  let left = rect.left + rect.width / 2 - tipW / 2;
  if (top + tipH > vh - 8) {
    top = rect.top - tipH - gap;
  }
  if (top < 8) top = 8;
  if (left < 8) left = 8;
  if (left + tipW > vw - 8) left = vw - tipW - 8;
  return { top, left };
}

/**
 * Spotlight coachmark overlay. Parent controls active via `open` + callbacks.
 */
export default function DemoTour({
  open,
  stepIndex,
  onStepChange,
  onClose,
  ctx,
}) {
  const [rect, setRect] = useState(null);
  const [waiting, setWaiting] = useState(false);
  const step = DEMO_TOUR_STEPS[stepIndex] || null;
  const total = DEMO_TOUR_STEPS.length;
  const ctxRef = useRef(ctx);
  ctxRef.current = ctx;

  const measure = useCallback(() => {
    if (!step) {
      setRect(null);
      return;
    }
    const el = findTarget(resolveStepTestIds(step));
    if (!el) {
      setRect(null);
      setWaiting(true);
      return;
    }
    setWaiting(false);
    const r = el.getBoundingClientRect();
    setRect({
      top: r.top - PAD,
      left: r.left - PAD,
      width: Math.max(r.width + PAD * 2, 24),
      height: Math.max(r.height + PAD * 2, 24),
    });
    try {
      el.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" });
    } catch {
      /* ignore */
    }
  }, [step]);

  useLayoutEffect(() => {
    if (!open || !step) return undefined;
    let cancelled = false;
    (async () => {
      try {
        await step.go?.(ctxRef.current);
      } catch {
        /* navigation best-effort */
      }
      if (cancelled) return;
      // Allow React to paint after go().
      requestAnimationFrame(() => {
        if (!cancelled) measure();
      });
      // Second pass after nav/state settles.
      setTimeout(() => {
        if (!cancelled) measure();
      }, 350);
    })();
    return () => {
      cancelled = true;
    };
  }, [open, stepIndex, step, measure]);

  useEffect(() => {
    if (!open) return undefined;
    const onWin = () => measure();
    window.addEventListener("resize", onWin);
    window.addEventListener("scroll", onWin, true);
    const iv = setInterval(() => {
      if (waiting) measure();
    }, 400);
    return () => {
      window.removeEventListener("resize", onWin);
      window.removeEventListener("scroll", onWin, true);
      clearInterval(iv);
    };
  }, [open, measure, waiting]);

  if (!open || !step) return null;

  const tipW = Math.min(340, window.innerWidth - 24);
  const tipH = 180;
  const tip = rect
    ? tipPosition(
        {
          top: rect.top,
          left: rect.left,
          width: rect.width,
          height: rect.height,
          bottom: rect.top + rect.height,
        },
        tipW,
        tipH
      )
    : { top: 80, left: Math.max(12, (window.innerWidth - tipW) / 2) };

  const finish = (status) => {
    saveDemoTourState({ status, step: stepIndex });
    onClose?.(status);
  };

  const goNext = () => {
    if (stepIndex >= total - 1) {
      finish(DEMO_TOUR_STATUS.completed);
      return;
    }
    onStepChange?.(stepIndex + 1);
  };

  const goBack = () => {
    if (stepIndex > 0) onStepChange?.(stepIndex - 1);
  };

  // Four dim panels around the hole so the spotlighted control stays clickable.
  const dims = [];
  if (rect) {
    const { top, left, width, height } = rect;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    dims.push(
      { key: "t", style: { top: 0, left: 0, width: vw, height: Math.max(0, top) } },
      {
        key: "l",
        style: { top, left: 0, width: Math.max(0, left), height },
      },
      {
        key: "r",
        style: {
          top,
          left: left + width,
          width: Math.max(0, vw - left - width),
          height,
        },
      },
      {
        key: "b",
        style: {
          top: top + height,
          left: 0,
          width: vw,
          height: Math.max(0, vh - top - height),
        },
      }
    );
  } else {
    dims.push({
      key: "full",
      style: { top: 0, left: 0, width: "100%", height: "100%" },
    });
  }

  return (
    <div className="demo-tour-root" data-testid="demo-tour" aria-live="polite">
      {dims.map((d) => (
        <div key={d.key} className="demo-tour-dim" style={d.style} />
      ))}
      {rect ? (
        <div
          className="demo-tour-hole"
          style={{
            top: rect.top,
            left: rect.left,
            width: rect.width,
            height: rect.height,
          }}
        />
      ) : null}
      <div
        className="demo-tour-tip"
        role="dialog"
        aria-label={step.title}
        style={{ top: tip.top, left: tip.left }}
      >
        <div className="demo-tour-meta">
          Step {stepIndex + 1} of {total}
        </div>
        <h3>{step.title}</h3>
        <p>{step.body}</p>
        {waiting ? (
          <p className="muted small">
            Waiting for this control… create or open the screen, or press Next
            {step.optional ? "" : " after it appears"}.
          </p>
        ) : null}
        <div className="row">
          <button
            type="button"
            className="ghost"
            data-testid="demo-tour-skip"
            onClick={() => finish(DEMO_TOUR_STATUS.skipped)}
          >
            Skip tour
          </button>
          <span className="grow" />
          <button
            type="button"
            className="ghost"
            data-testid="demo-tour-back"
            disabled={stepIndex === 0}
            onClick={goBack}
          >
            Back
          </button>
          <button type="button" data-testid="demo-tour-next" onClick={goNext}>
            {stepIndex >= total - 1 ? "Done" : "Next"}
          </button>
        </div>
      </div>
    </div>
  );
}

export function shouldAutoOfferDemoTour(assessmentCount) {
  const s = loadDemoTourState();
  if (s.status === DEMO_TOUR_STATUS.completed || s.status === DEMO_TOUR_STATUS.skipped) {
    return false;
  }
  return assessmentCount === 0;
}
