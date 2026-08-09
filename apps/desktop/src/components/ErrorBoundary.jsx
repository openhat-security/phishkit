import { Component } from "react";

/**
 * Catches render/commit errors in a subtree so one screen crashing does not
 * tear down the whole app (an uncaught error unmounts the React root, and
 * Vite HMR cannot recover it without a full reload). Resets automatically when
 * `resetKey` changes (e.g. on navigation) so switching views clears the error.
 */
export default class ErrorBoundary extends Component {
  constructor(props) {
    super(props);
    this.state = { error: null };
  }

  static getDerivedStateFromError(error) {
    return { error };
  }

  componentDidCatch(error, info) {
    this.props.onError?.(error, info);
  }

  componentDidUpdate(prev) {
    if (prev.resetKey !== this.props.resetKey && this.state.error) {
      this.setState({ error: null });
    }
  }

  render() {
    if (this.state.error) {
      const msg = String(this.state.error?.message || this.state.error);
      return (
        <section className="card error-boundary">
          <h2>Something went wrong on this screen</h2>
          <p className="muted">
            The rest of the app is still running — retry this screen or switch
            views from the sidebar.
          </p>
          <pre className="hint-panel small">{msg}</pre>
          <button onClick={() => this.setState({ error: null })}>
            Try again
          </button>
        </section>
      );
    }
    return this.props.children;
  }
}
