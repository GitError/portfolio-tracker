import { Component, type ErrorInfo, type ReactNode } from 'react';

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { hasError: false };

  static getDerivedStateFromError(): ErrorBoundaryState {
    return { hasError: true };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    // eslint-disable-next-line no-console
    console.error('Uncaught render error:', error, errorInfo);
  }

  render(): ReactNode {
    if (this.state.hasError) {
      return (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: 16,
            height: '100vh',
            padding: 32,
            textAlign: 'center',
            background: 'var(--bg-primary)',
            fontFamily: 'var(--font-sans)',
          }}
        >
          <div>
            <h1
              style={{
                color: 'var(--text-primary)',
                fontSize: 18,
                margin: '0 0 8px',
              }}
            >
              Something went wrong
            </h1>
            <p
              style={{
                color: 'var(--text-secondary)',
                fontSize: 13,
                margin: 0,
              }}
            >
              Portfolio Tracker hit an unexpected error. Your data is safe — reloading should fix
              it.
            </p>
          </div>
          <button
            onClick={() => window.location.reload()}
            style={{
              padding: '10px 20px',
              background: 'var(--color-accent)',
              border: 'none',
              color: '#fff',
              cursor: 'pointer',
              fontFamily: 'var(--font-mono)',
              fontSize: 13,
              borderRadius: 2,
            }}
          >
            Reload
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
