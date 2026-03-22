import { useState, useEffect, useRef, useCallback } from 'react';
import { config } from '../../lib/config';
import type { SymbolResult } from '../../types/portfolio';
import { isTauri, tauriInvoke } from '../../lib/tauri';

const MOCK_RESULTS: SymbolResult[] = [
  { symbol: 'AAPL', name: 'Apple Inc.', assetType: 'stock', exchange: 'NMS', currency: 'USD' },
  { symbol: 'AMZN', name: 'Amazon.com Inc.', assetType: 'stock', exchange: 'NMS', currency: 'USD' },
  {
    symbol: 'MSFT',
    name: 'Microsoft Corporation',
    assetType: 'stock',
    exchange: 'NMS',
    currency: 'USD',
  },
  { symbol: 'GOOGL', name: 'Alphabet Inc.', assetType: 'stock', exchange: 'NMS', currency: 'USD' },
  {
    symbol: 'META',
    name: 'Meta Platforms Inc.',
    assetType: 'stock',
    exchange: 'NMS',
    currency: 'USD',
  },
  {
    symbol: 'NVDA',
    name: 'NVIDIA Corporation',
    assetType: 'stock',
    exchange: 'NMS',
    currency: 'USD',
  },
  { symbol: 'TSLA', name: 'Tesla Inc.', assetType: 'stock', exchange: 'NMS', currency: 'USD' },
  {
    symbol: 'VOO',
    name: 'Vanguard S&P 500 ETF',
    assetType: 'etf',
    exchange: 'PCX',
    currency: 'USD',
  },
  { symbol: 'QQQ', name: 'Invesco QQQ Trust', assetType: 'etf', exchange: 'NMS', currency: 'USD' },
  {
    symbol: 'VTI',
    name: 'Vanguard Total Stock Market ETF',
    assetType: 'etf',
    exchange: 'PCX',
    currency: 'USD',
  },
  { symbol: 'BTC-USD', name: 'Bitcoin USD', assetType: 'crypto', exchange: 'CCC', currency: 'USD' },
  {
    symbol: 'ETH-USD',
    name: 'Ethereum USD',
    assetType: 'crypto',
    exchange: 'CCC',
    currency: 'USD',
  },
  {
    symbol: 'TD.TO',
    name: 'Toronto-Dominion Bank',
    assetType: 'stock',
    exchange: 'TRT',
    currency: 'CAD',
  },
  {
    symbol: 'RY.TO',
    name: 'Royal Bank of Canada',
    assetType: 'stock',
    exchange: 'TRT',
    currency: 'CAD',
  },
  {
    symbol: 'XIU.TO',
    name: 'iShares S&P/TSX 60 Index ETF',
    assetType: 'etf',
    exchange: 'TRT',
    currency: 'CAD',
  },
  {
    symbol: 'VFV.TO',
    name: 'Vanguard S&P 500 Index ETF',
    assetType: 'etf',
    exchange: 'TRT',
    currency: 'CAD',
  },
];

interface Props {
  value: string;
  onChange: (value: string) => void;
  onSelect: (result: SymbolResult) => void;
  placeholder?: string;
  disabled?: boolean;
  inputRef?: React.RefObject<HTMLInputElement | null>;
}

export function SymbolSearch({
  value,
  onChange,
  onSelect,
  placeholder = 'AAPL',
  disabled,
  inputRef,
}: Props) {
  const [query, setQuery] = useState(value);
  const [results, setResults] = useState<SymbolResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  // Tracks the most recent query string so stale async responses can be discarded.
  const currentQueryRef = useRef('');

  // Sync parent-controlled value (e.g. when editing an existing holding)
  useEffect(() => {
    setQuery(value);
  }, [value]);

  // Close on outside click
  useEffect(() => {
    function handleOutside(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener('mousedown', handleOutside);
    return () => document.removeEventListener('mousedown', handleOutside);
  }, []);

  const search = useCallback(async (q: string) => {
    const trimmed = q.trim();
    currentQueryRef.current = q;
    if (trimmed.length < config.symbolSearchMinChars) {
      setResults([]);
      setOpen(false);
      return;
    }
    setLoading(true);
    try {
      if (isTauri()) {
        const res = await tauriInvoke<SymbolResult[]>('search_symbols', { query: trimmed });
        // Discard the response if the user has already typed something newer.
        if (q !== currentQueryRef.current) return;
        setResults(res);
        setOpen(res.length > 0);
      } else {
        const lower = trimmed.toLowerCase();
        const filtered = MOCK_RESULTS.filter(
          (r) => r.symbol.toLowerCase().startsWith(lower) || r.name.toLowerCase().includes(lower)
        ).slice(0, 8);
        // Discard the response if the user has already typed something newer.
        if (q !== currentQueryRef.current) return;
        setResults(filtered);
        setOpen(filtered.length > 0);
      }
    } catch {
      if (q !== currentQueryRef.current) return;
      setResults([]);
      setOpen(false);
    } finally {
      setLoading(false);
    }
  }, []);

  function handleInput(e: React.ChangeEvent<HTMLInputElement>) {
    const q = e.target.value.toUpperCase();
    setQuery(q);
    onChange(q);
    setActiveIndex(-1);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => search(q), config.symbolSearchDebounceMs);
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (!open || results.length === 0) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setActiveIndex((i) => Math.min(i + 1, results.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setActiveIndex((i) => Math.max(i - 1, -1));
    } else if (e.key === 'Enter' && activeIndex >= 0) {
      e.preventDefault();
      const selected = results[activeIndex];
      if (selected) handleSelect(selected);
    } else if (e.key === 'Escape') {
      setOpen(false);
    }
  }

  function handleSelect(result: SymbolResult) {
    setQuery(result.symbol);
    onChange(result.symbol);
    setOpen(false);
    setResults([]);
    setActiveIndex(-1);
    onSelect(result);
  }

  return (
    <div ref={containerRef} style={{ position: 'relative' }}>
      <div style={{ position: 'relative' }}>
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={handleInput}
          onKeyDown={handleKeyDown}
          onFocus={(e) => {
            (e.target as HTMLInputElement).style.borderColor = 'var(--color-accent)';
            if (results.length > 0) setOpen(true);
          }}
          onBlur={(e) => {
            (e.target as HTMLInputElement).style.borderColor = 'var(--border-primary)';
          }}
          placeholder={placeholder}
          disabled={disabled}
          style={{
            width: '100%',
            background: 'var(--bg-primary)',
            border: '1px solid var(--border-primary)',
            color: 'var(--text-primary)',
            padding: '7px 32px 7px 10px',
            fontSize: 13,
            fontFamily: 'var(--font-mono)',
            borderRadius: '2px',
            outline: 'none',
            textTransform: 'uppercase',
            boxSizing: 'border-box',
          }}
        />
        {loading && (
          <span
            style={{
              position: 'absolute',
              right: 8,
              top: '50%',
              transform: 'translateY(-50%)',
              fontSize: 10,
              color: 'var(--text-muted)',
              fontFamily: 'var(--font-mono)',
              pointerEvents: 'none',
            }}
          >
            ...
          </span>
        )}
      </div>

      {open && (
        <div
          style={{
            position: 'absolute',
            top: '100%',
            left: 0,
            right: 0,
            background: 'var(--bg-primary)',
            border: '1px solid var(--border-primary)',
            borderTop: 'none',
            zIndex: 1100,
            maxHeight: 280,
            overflowY: 'auto',
            scrollbarWidth: 'thin',
            scrollbarColor: 'var(--border-primary) transparent',
          }}
        >
          {results.length === 0 ? (
            <div
              style={{
                padding: '8px 10px',
                fontSize: 12,
                color: 'var(--text-muted)',
                fontFamily: 'var(--font-mono)',
              }}
            >
              No results
            </div>
          ) : (
            results.map((r, i) => (
              <div
                key={r.symbol}
                onMouseDown={(e) => {
                  e.preventDefault(); // keep input focused
                  handleSelect(r);
                }}
                onMouseEnter={() => setActiveIndex(i)}
                style={{
                  padding: '7px 10px',
                  cursor: 'pointer',
                  background: i === activeIndex ? 'var(--bg-surface-hover)' : 'transparent',
                  borderBottom: '1px solid var(--border-subtle)',
                  display: 'flex',
                  gap: 8,
                  alignItems: 'center',
                }}
              >
                <span
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontWeight: 600,
                    fontSize: 12,
                    color: 'var(--text-primary)',
                    minWidth: 76,
                    flexShrink: 0,
                  }}
                >
                  {r.symbol}
                </span>
                <span
                  style={{
                    fontFamily: 'var(--font-sans)',
                    fontSize: 11,
                    color: 'var(--text-secondary)',
                    flex: 1,
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {r.name}
                </span>
                <span
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: 10,
                    color: 'var(--text-muted)',
                    whiteSpace: 'nowrap',
                    flexShrink: 0,
                  }}
                >
                  {r.exchange} · {r.currency}
                </span>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}
