import { Component, type ErrorInfo, type ReactNode } from "react";
import { reportError } from "../touch/errors";
import { Icon } from "./Icon";
import { C, caption, ring } from "./tokens";

interface ErrorBoundaryProps {
  /** Names the failed part in the log and on screen. */
  name: string;
  children: ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

/** Keeps one panel's render error from blanking the whole stage; shows it in that panel's place. */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    reportError(`${this.props.name} panel`, Object.assign(error, { stack: `${error.stack ?? ""}${info.componentStack ?? ""}` }));
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    return (
      <div
        style={{
          borderRadius: 16,
          background: C.panel,
          boxShadow: ring(C.red),
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          gap: 8,
          padding: 24,
          color: C.red,
          textAlign: "center",
        }}
      >
        <Icon name="error" size={28} />
        <span style={{ fontSize: 13, fontWeight: 600 }}>{this.props.name} failed</span>
        <span style={{ ...caption, overflowWrap: "anywhere" }}>{error.message}</span>
      </div>
    );
  }
}
