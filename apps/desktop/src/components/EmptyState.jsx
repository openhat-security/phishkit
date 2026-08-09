/**
 * Consistent empty-state block: icon, one-line title, optional short body,
 * and a primary call-to-action. Keep the body to a single sentence — push
 * any detail into a Hint tooltip on the surrounding heading instead.
 */
export default function EmptyState({ icon, title, children, action, compact }) {
  return (
    <div className={`empty-state${compact ? " compact" : ""}`}>
      {icon ? <div className="empty-ico">{icon}</div> : null}
      {title ? <h3 className="empty-title">{title}</h3> : null}
      {children ? <p className="empty-body muted">{children}</p> : null}
      {action ? <div className="empty-actions">{action}</div> : null}
    </div>
  );
}
