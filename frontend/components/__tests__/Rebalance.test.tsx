import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Rebalance } from '../Rebalance';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../lib/tauri', () => ({
  isTauri: () => false,
  tauriInvoke: vi.fn().mockResolvedValue([]),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe('Rebalance', () => {
  it('renders without crashing', () => {
    const { container } = render(<Rebalance />);
    expect(container).toBeTruthy();
  });

  it('shows the Rebalance heading', () => {
    render(<Rebalance />);
    expect(screen.getByText(/rebalance/i)).toBeTruthy();
  });

  it('renders drift threshold input', () => {
    render(<Rebalance />);
    expect(screen.getByRole('spinbutton')).toBeTruthy();
  });
});
