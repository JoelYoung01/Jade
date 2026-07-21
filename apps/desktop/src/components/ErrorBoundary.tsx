import * as React from "react";

import { ErrorFallback } from "@/components/ErrorFallback";

type ErrorBoundaryProps = {
  children: React.ReactNode;
};

type ErrorBoundaryState = {
  error: Error | null;
  componentStack: string | null;
};

export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  override state: ErrorBoundaryState = {
    error: null,
    componentStack: null,
  };

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return { error };
  }

  override componentDidCatch(error: Error, info: React.ErrorInfo): void {
    this.setState({
      error,
      componentStack: info.componentStack ?? null,
    });
    console.error("Jade error boundary caught:", error, info);
  }

  reset = (): void => {
    this.setState({ error: null, componentStack: null });
  };

  override render(): React.ReactNode {
    const { error, componentStack } = this.state;
    if (error) {
      return (
        <ErrorFallback error={error} componentStack={componentStack} onReset={this.reset} />
      );
    }
    return this.props.children;
  }
}
