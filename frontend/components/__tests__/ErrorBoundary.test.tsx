import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ErrorBoundary } from '../ErrorBoundary';

function ThrowingChild(): never {
  throw new Error('boom');
}

describe('ErrorBoundary', () => {
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>;
  let reloadSpy: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    // React logs caught render errors to console.error; silence that expected noise.
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    reloadSpy = vi.fn();
    Object.defineProperty(window, 'location', {
      configurable: true,
      value: { ...window.location, reload: reloadSpy },
    });
  });

  afterEach(() => {
    consoleErrorSpy.mockRestore();
  });

  it('renders children when no error occurs', () => {
    render(
      <ErrorBoundary>
        <div>All good</div>
      </ErrorBoundary>
    );

    expect(screen.getByText('All good')).toBeTruthy();
  });

  it('renders a friendly fallback instead of crashing when a child throws', () => {
    render(
      <ErrorBoundary>
        <ThrowingChild />
      </ErrorBoundary>
    );

    expect(screen.queryByText('All good')).toBeNull();
    expect(screen.getByRole('button', { name: /reload/i })).toBeTruthy();
  });

  it('reloads the page when the Reload button is clicked', () => {
    render(
      <ErrorBoundary>
        <ThrowingChild />
      </ErrorBoundary>
    );

    fireEvent.click(screen.getByRole('button', { name: /reload/i }));

    expect(reloadSpy).toHaveBeenCalledTimes(1);
  });
});
