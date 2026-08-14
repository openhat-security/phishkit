/** Interactive first-run / restartable demo tour. */

export const DEMO_TOUR_STORAGE_KEY = "phishkit.demoTour";

export const DEMO_TOUR_STATUS = {
  idle: "idle",
  active: "active",
  completed: "completed",
  skipped: "skipped",
};

export function loadDemoTourState() {
  try {
    const raw = localStorage.getItem(DEMO_TOUR_STORAGE_KEY);
    if (!raw) return { status: DEMO_TOUR_STATUS.idle, step: 0 };
    const parsed = JSON.parse(raw);
    return {
      status: parsed.status || DEMO_TOUR_STATUS.idle,
      step: Number(parsed.step) || 0,
    };
  } catch {
    return { status: DEMO_TOUR_STATUS.idle, step: 0 };
  }
}

export function saveDemoTourState(state) {
  try {
    localStorage.setItem(DEMO_TOUR_STORAGE_KEY, JSON.stringify(state));
  } catch {
    /* ignore */
  }
}

/**
 * @typedef {object} DemoTourCtx
 * @property {(mode: string) => void} setMode
 * @property {(nav: string) => void} setNav
 * @property {(view: string) => void} [setReconView]
 * @property {() => void} [goHome]
 * @property {() => void} [openNewAssessment]
 * @property {'home'|'assessment'|'target'} mode
 * @property {string} nav
 * @property {object|null} activeAssessment
 * @property {object|null} activeTarget
 */

/**
 * @typedef {object} DemoTourStep
 * @property {string} id
 * @property {string} title
 * @property {string} body
 * @property {string|string[]} testId — primary (or fallbacks) data-testid to spotlight
 * @property {(ctx: DemoTourCtx) => void|Promise<void>} [go] — navigate before measuring
 * @property {boolean} [optional] — Next allowed even if target missing after wait
 */

/** @type {DemoTourStep[]} */
export const DEMO_TOUR_STEPS = [
  {
    id: "welcome",
    title: "Welcome to phishkit",
    body: "This is an alpha preview. You will create an Assessment, add a Target, open the proxy, then review Sessions. Authorized use only.",
    spotlight: '[data-testid="nav-assessments"]',
    mode: "home",
    nav: "assessments",
  },
  {
    id: "new-assessment",
    title: "Create an Assessment",
    body: "One primary domain per engagement. Click New assessment to begin.",
    spotlight: '[data-testid="assessment-new"]',
    mode: "home",
    nav: "assessments",
    action: "open-new-assessment",
  },
  {
    id: "assessment-form",
    title: "Name and primary domain",
    body: "Name the engagement and set the website you are authorized to assess.",
    spotlight: '[data-testid="assessment-name"]',
    mode: "home",
    nav: "assessments",
  },
  {
    id: "targets",
    title: "Add a Target",
    body: "Targets are the sites inside this assessment. Prefer a localhost demo for practice.",
    spotlight: '[data-testid="nav-targets"]',
    mode: "assessment",
    nav: "targets",
  },
  {
    id: "proxy",
    title: "Recon and Proxy",
    body: "Generate a Lure and start the AiTM proxy. Community packs stay under Advanced when you need them.",
    spotlight: '[data-testid="nav-recon"]',
    mode: "target",
    nav: "recon",
    reconView: "proxy",
  },
  {
    id: "campaigns",
    title: "Guided Campaign",
    body: "Use Guided Campaigns to send with safe defaults. Advanced composers stay available for experts.",
    spotlight: '[data-testid="nav-campaigns"]',
    mode: "assessment",
    nav: "campaigns",
  },
  {
    id: "sessions",
    title: "Sessions",
    body: "Captured sessions land here. Export or purge when the engagement ends. Archive keeps data; Delete erases it.",
    spotlight: '[data-testid="nav-sessions"]',
    mode: "target",
    nav: "sessions",
  },
];

export function resolveStepTestIds(step) {
  if (!step?.testId) return [];
  return Array.isArray(step.testId) ? step.testId : [step.testId];
}
