import { useId, useState } from "react";
import { IconInfo } from "../lib/icons";

/** Hover / focus info tip. Pass a string, or { title?, body }. */
export default function Hint({ hint, size = 15 }) {
  const id = useId();
  const [open, setOpen] = useState(false);
  if (!hint) return null;
  const tip = typeof hint === "string" ? { body: hint } : hint;
  const label = tip.title || (typeof tip.body === "string" ? tip.body : "More info");

  return (
    <span
      className={`hint ${open ? "open" : ""}`}
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
      onFocus={() => setOpen(true)}
      onBlur={() => setOpen(false)}
    >
      <button
        type="button"
        className="hint-btn"
        aria-describedby={open ? id : undefined}
        aria-label={label}
      >
        <IconInfo size={size} />
      </button>
      {open && (
        <span className="hint-tip" id={id} role="tooltip">
          {tip.title && <strong className="hint-title">{tip.title}</strong>}
          <span className="hint-body">{tip.body}</span>
        </span>
      )}
    </span>
  );
}

export function LabelWithHint({ children, hint }) {
  return (
    <span className="label-with-hint">
      {children}
      <Hint hint={hint} />
    </span>
  );
}

/** Section heading with an optional inline info tip. */
export function SectionTitle({ as: Tag = "h2", hint, children, actions }) {
  return (
    <div className="section-head">
      <Tag className="section-head-title">
        {children}
        {hint ? <Hint hint={hint} /> : null}
      </Tag>
      {actions ? <div className="section-head-actions">{actions}</div> : null}
    </div>
  );
}
