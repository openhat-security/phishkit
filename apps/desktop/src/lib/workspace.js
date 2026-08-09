/** Navigation subsection ids per shell mode */
export const HOME_NAV = [
  { id: "assessments", label: "Assessments" },
  { id: "delivery", label: "Delivery" },
  { id: "settings", label: "Settings" },
];

export const ASSESSMENT_NAV = [
  { id: "overview", label: "Overview" },
  { id: "targets", label: "Targets" },
  { id: "templates", label: "Templates" },
  { id: "recipients", label: "Recipients" },
  { id: "campaigns", label: "Campaigns" },
  { id: "results", label: "Results" },
  { id: "delivery", label: "Delivery" },
];

export const TARGET_NAV = [
  { id: "overview", label: "Overview" },
  { id: "recon", label: "Recon & Proxy" },
  { id: "sessions", label: "Sessions" },
];

export function assessmentMeta(nav) {
  return ASSESSMENT_NAV.find((n) => n.id === nav) || ASSESSMENT_NAV[0];
}

export function homeMeta(nav) {
  return HOME_NAV.find((n) => n.id === nav) || HOME_NAV[0];
}

export function targetMeta(nav) {
  return TARGET_NAV.find((n) => n.id === nav) || TARGET_NAV[0];
}

/** Normalize Assessment fields from API (camelCase from serde) */
export function fmtAssessment(a) {
  if (!a) return null;
  return {
    ...a,
    primaryDomain: a.primaryDomain ?? a.primary_domain ?? "",
    targetCount: a.targetCount ?? a.target_count ?? 0,
    campaignCount: a.campaignCount ?? a.campaign_count ?? 0,
    sessionCount: a.sessionCount ?? a.session_count ?? 0,
    authorizationRef: a.authorizationRef ?? a.authorization_ref ?? "",
    authorizedBy: a.authorizedBy ?? a.authorized_by ?? "",
  };
}

/** Normalize Profile / Target fields */
export function fmtProfile(p) {
  if (!p) return null;
  return {
    ...p,
    targetDomain: p.targetDomain ?? p.target_domain ?? "",
    dryrunDomain: p.dryrunDomain ?? p.dryrun_domain ?? "",
    lureUrl: p.lureUrl ?? p.lure_url ?? "",
    assessmentId: p.assessmentId ?? p.assessment_id ?? "",
  };
}
