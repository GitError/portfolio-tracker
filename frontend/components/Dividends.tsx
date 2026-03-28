import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { DollarSign, Plus, Trash2, TrendingUp } from 'lucide-react';
import type { Dividend, DividendInput, Holding, HoldingWithPrice } from '../types/portfolio';
import { usePortfolio } from '../hooks/usePortfolio';
import { formatCurrency, formatNumber } from '../lib/format';
import { MOCK_DIVIDENDS, MOCK_HOLDINGS } from '../lib/mockData';
import { EmptyState } from './ui/EmptyState';
import { Select } from './ui/Select';
import { Spinner } from './ui/Spinner';
import { useToast } from './ui/Toast';
import { isTauri, tauriInvoke } from '../lib/tauri';
import { SUPPORTED_CURRENCIES } from '../lib/constants';

interface AddDividendFormProps {
  holdings: Holding[];
  onAdd: (input: DividendInput) => void;
  onCancel: () => void;
}

function AddDividendForm({ holdings, onAdd, onCancel }: AddDividendFormProps) {
  const { t } = useTranslation();
  const [holdingId, setHoldingId] = useState(holdings[0]?.id ?? '');
  const [amount, setAmount] = useState('');
  const [currency, setCurrency] = useState('CAD');
  const [exDate, setExDate] = useState('');
  const [payDate, setPayDate] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const a = parseFloat(amount);
    if (!holdingId || isNaN(a) || a <= 0 || !exDate || !payDate) return;
    onAdd({ holdingId, amountPerUnit: a, currency, exDate, payDate });
  };

  const inputStyle: React.CSSProperties = {
    background: 'var(--bg-surface-alt)',
    border: '1px solid var(--border-primary)',
    color: 'var(--text-primary)',
    fontSize: 13,
    padding: '6px 10px',
    borderRadius: 2,
    outline: 'none',
    fontFamily: 'var(--font-sans)',
    width: '100%',
    boxSizing: 'border-box',
  };

  return (
    <form
      onSubmit={handleSubmit}
      style={{
        background: 'var(--bg-surface)',
        border: '1px solid var(--border-primary)',
        padding: 16,
        borderRadius: 2,
        marginBottom: 16,
        display: 'flex',
        flexDirection: 'column',
        gap: 12,
      }}
    >
      <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)' }}>
        Record Dividend
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 1fr', gap: 10 }}>
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>
            {t('dividends.holding').toUpperCase()}
          </div>
          <Select
            value={holdingId}
            onChange={setHoldingId}
            options={holdings.map((h) => ({ value: h.id, label: `${h.symbol} — ${h.name}` }))}
          />
        </div>
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>
            {t('dividends.amountPerUnit').toUpperCase()}
          </div>
          <input
            style={{ ...inputStyle, fontFamily: 'var(--font-mono)' }}
            type="number"
            min="0"
            step="any"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="0.00"
            required
          />
        </div>
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>
            {t('dividends.currency').toUpperCase()}
          </div>
          <Select
            value={currency}
            onChange={setCurrency}
            options={SUPPORTED_CURRENCIES.map((c) => ({ value: c, label: c }))}
          />
        </div>
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>
            {t('dividends.exDate').toUpperCase()}
          </div>
          <input
            style={inputStyle}
            type="date"
            value={exDate}
            onChange={(e) => setExDate(e.target.value)}
            placeholder="YYYY-MM-DD"
            required
            onFocus={(e) => (e.currentTarget.style.borderColor = 'var(--color-accent)')}
            onBlur={(e) => (e.currentTarget.style.borderColor = 'var(--border-primary)')}
          />
        </div>
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>
            {t('dividends.payDate').toUpperCase()}
          </div>
          <input
            style={inputStyle}
            type="date"
            value={payDate}
            onChange={(e) => setPayDate(e.target.value)}
            placeholder="YYYY-MM-DD"
            required
            onFocus={(e) => (e.currentTarget.style.borderColor = 'var(--color-accent)')}
            onBlur={(e) => (e.currentTarget.style.borderColor = 'var(--border-primary)')}
          />
        </div>
      </div>
      <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
        <button
          type="button"
          onClick={onCancel}
          style={{
            background: 'transparent',
            border: '1px solid var(--border-primary)',
            color: 'var(--text-secondary)',
            fontSize: 13,
            padding: '6px 14px',
            borderRadius: 2,
            cursor: 'pointer',
          }}
        >
          {t('common.cancel')}
        </button>
        <button
          type="submit"
          style={{
            background: 'var(--color-accent)',
            border: 'none',
            color: '#fff',
            fontSize: 13,
            fontWeight: 500,
            padding: '6px 14px',
            borderRadius: 2,
            cursor: 'pointer',
          }}
        >
          Record
        </button>
      </div>
    </form>
  );
}

export function Dividends() {
  const { t } = useTranslation();
  const [dividends, setDividends] = useState<Dividend[]>([]);
  const [holdings, setHoldings] = useState<Holding[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const { showToast } = useToast();
  const { portfolio } = usePortfolio();

  const loadData = useCallback(async () => {
    try {
      if (isTauri()) {
        const [divs, holds] = await Promise.all([
          tauriInvoke<Dividend[]>('get_dividends'),
          tauriInvoke<Holding[]>('get_holdings'),
        ]);
        setDividends(divs);
        setHoldings(holds);
      } else {
        setDividends(MOCK_DIVIDENDS);
        setHoldings(MOCK_HOLDINGS);
      }
    } catch (err) {
      showToast(`Failed to load dividends: ${String(err)}`, 'error');
    } finally {
      setLoading(false);
    }
  }, [showToast]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  const handleAdd = useCallback(
    async (input: DividendInput) => {
      try {
        if (isTauri()) {
          const div = await tauriInvoke<Dividend>('add_dividend', { dividend: input });
          setDividends((prev) => [div, ...prev]);
        } else {
          const holding = holdings.find((h) => h.id === input.holdingId);
          const mock: Dividend = {
            id: Date.now(),
            symbol: holding?.symbol ?? '',
            ...input,
            createdAt: new Date().toISOString(),
          };
          setDividends((prev) => [mock, ...prev]);
        }
        setShowForm(false);
        showToast('Dividend recorded', 'success');
      } catch (err) {
        showToast(`Failed to record dividend: ${String(err)}`, 'error');
      }
    },
    [holdings, showToast]
  );

  const handleDelete = useCallback(
    async (id: number) => {
      try {
        if (isTauri()) {
          await tauriInvoke<boolean>('delete_dividend', { id });
        }
        setDividends((prev) => prev.filter((d) => d.id !== id));
        showToast('Dividend deleted', 'success');
      } catch (err) {
        showToast(`Failed to delete: ${String(err)}`, 'error');
      }
    },
    [showToast]
  );

  // Forward income: holdings with indicatedAnnualDividend set
  const forwardIncomeRows = useMemo(() => {
    const holdingsWithPrice: HoldingWithPrice[] = portfolio?.holdings ?? [];
    return holdingsWithPrice
      .filter((h) => h.indicatedAnnualDividend != null && h.indicatedAnnualDividend > 0)
      .map((h) => ({
        id: h.id,
        symbol: h.symbol,
        name: h.name,
        currency: h.indicatedAnnualDividendCurrency ?? h.currency,
        frequency: h.dividendFrequency,
        iadPerUnit: h.indicatedAnnualDividend as number,
        quantity: h.quantity,
        estimatedAnnualIncome: (h.indicatedAnnualDividend as number) * h.quantity,
      }));
  }, [portfolio]);

  // Summary stats
  const summary = useMemo(() => {
    const bySymbol: Record<string, { total: number; currency: string; count: number }> = {};
    for (const div of dividends) {
      if (!bySymbol[div.symbol]) {
        bySymbol[div.symbol] = { total: 0, currency: div.currency, count: 0 };
      }
      const entry = bySymbol[div.symbol]!;
      entry.total += div.amountPerUnit;
      entry.count += 1;
    }
    return bySymbol;
  }, [dividends]);

  if (loading) {
    return (
      <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Spinner />
      </div>
    );
  }

  return (
    <div style={{ flex: 1, overflow: 'auto', padding: '24px 32px', maxWidth: 900 }}>
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: 24,
        }}
      >
        <div>
          <h1
            style={{
              fontSize: 18,
              fontWeight: 600,
              color: 'var(--text-primary)',
              margin: 0,
              marginBottom: 4,
            }}
          >
            {t('dividends.title')}
          </h1>
          <p style={{ fontSize: 13, color: 'var(--text-secondary)', margin: 0 }}>
            Record and track dividend income across your holdings.
          </p>
        </div>
        <button
          onClick={() => setShowForm(true)}
          disabled={holdings.length === 0}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            background: 'var(--color-accent)',
            border: 'none',
            color: '#fff',
            fontSize: 13,
            fontWeight: 500,
            padding: '8px 14px',
            borderRadius: 2,
            cursor: holdings.length === 0 ? 'not-allowed' : 'pointer',
            opacity: holdings.length === 0 ? 0.5 : 1,
          }}
        >
          <Plus size={14} />
          Record Dividend
        </button>
      </div>

      {/* Summary by symbol */}
      {Object.keys(summary).length > 0 && (
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))',
            gap: 1,
            background: 'var(--border-primary)',
            border: '1px solid var(--border-primary)',
            marginBottom: 24,
          }}
        >
          {Object.entries(summary).map(([symbol, data]) => (
            <div key={symbol} style={{ background: 'var(--bg-surface)', padding: '14px 16px' }}>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  fontWeight: 600,
                  color: 'var(--text-primary)',
                }}
              >
                {symbol}
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 16,
                  fontWeight: 600,
                  color: 'var(--color-gain)',
                  marginTop: 4,
                }}
              >
                {formatCurrency(data.total, data.currency)}
              </div>
              <div style={{ fontSize: 11, color: 'var(--text-muted)', marginTop: 2 }}>
                {data.count} payment{data.count !== 1 ? 's' : ''} recorded
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Forward Income section */}
      {forwardIncomeRows.length > 0 && (
        <div style={{ marginBottom: 24 }}>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              marginBottom: 10,
            }}
          >
            <TrendingUp size={14} style={{ color: 'var(--color-gain)' }} />
            <span
              style={{
                fontSize: 12,
                fontWeight: 600,
                fontFamily: 'var(--font-sans)',
                color: 'var(--text-primary)',
              }}
            >
              Forward Income
            </span>
            <span
              style={{
                fontSize: 11,
                color: 'var(--text-muted)',
                fontFamily: 'var(--font-mono)',
              }}
            >
              (based on indicated annual dividend × quantity)
            </span>
          </div>
          <div style={{ border: '1px solid var(--border-primary)', overflow: 'hidden' }}>
            {/* Header */}
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: '90px 1fr 110px 90px 100px 120px',
                padding: '7px 14px',
                background: 'var(--bg-surface-alt)',
                borderBottom: '1px solid var(--border-primary)',
              }}
            >
              {['SYMBOL', 'NAME', 'FREQUENCY', 'CURRENCY', 'IAD / UNIT', 'EST. ANNUAL'].map((h) => (
                <div
                  key={h}
                  style={{
                    fontSize: 10,
                    fontWeight: 600,
                    textTransform: 'uppercase',
                    letterSpacing: '0.06em',
                    color: 'var(--text-muted)',
                    fontFamily: 'var(--font-mono)',
                  }}
                >
                  {h}
                </div>
              ))}
            </div>
            {forwardIncomeRows.map((row, i) => (
              <div
                key={row.id}
                style={{
                  display: 'grid',
                  gridTemplateColumns: '90px 1fr 110px 90px 100px 120px',
                  padding: '9px 14px',
                  alignItems: 'center',
                  background: i % 2 === 0 ? 'var(--bg-surface)' : 'var(--bg-surface-alt)',
                  borderBottom:
                    i < forwardIncomeRows.length - 1 ? '1px solid var(--border-subtle)' : 'none',
                }}
              >
                <div
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: 13,
                    fontWeight: 700,
                    color: 'var(--text-primary)',
                  }}
                >
                  {row.symbol}
                </div>
                <div
                  style={{
                    fontSize: 12,
                    color: 'var(--text-secondary)',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {row.name}
                </div>
                <div
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: 11,
                    color: row.frequency ? 'var(--text-secondary)' : 'var(--text-muted)',
                    textTransform: 'capitalize',
                  }}
                >
                  {row.frequency ?? '—'}
                </div>
                <div
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: 11,
                    color: 'var(--text-secondary)',
                  }}
                >
                  {row.currency}
                </div>
                <div
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: 12,
                    color: 'var(--color-gain)',
                    textAlign: 'right',
                  }}
                >
                  {formatCurrency(row.iadPerUnit, row.currency)}
                </div>
                <div
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: 13,
                    fontWeight: 600,
                    color: 'var(--color-gain)',
                    textAlign: 'right',
                  }}
                >
                  {formatCurrency(row.estimatedAnnualIncome, row.currency)}
                </div>
              </div>
            ))}
            {/* Total row */}
            {forwardIncomeRows.length > 1 && (
              <div
                style={{
                  display: 'grid',
                  gridTemplateColumns: '90px 1fr 110px 90px 100px 120px',
                  padding: '9px 14px',
                  background: 'var(--bg-surface-alt)',
                  borderTop: '2px solid var(--border-primary)',
                }}
              >
                <div
                  style={{
                    gridColumn: '1 / 6',
                    fontFamily: 'var(--font-mono)',
                    fontSize: 10,
                    color: 'var(--text-muted)',
                    textTransform: 'uppercase',
                    letterSpacing: '0.06em',
                  }}
                >
                  Total ({forwardIncomeRows.length} holdings)
                </div>
                <div
                  style={{
                    fontFamily: 'var(--font-mono)',
                    fontSize: 13,
                    fontWeight: 700,
                    color: 'var(--color-gain)',
                    textAlign: 'right',
                  }}
                >
                  {(() => {
                    const byCurrency = forwardIncomeRows.reduce<Record<string, number>>(
                      (acc, r) => {
                        const cur = r.currency;
                        acc[cur] = (acc[cur] ?? 0) + r.estimatedAnnualIncome;
                        return acc;
                      },
                      {}
                    );
                    return Object.entries(byCurrency).map(([cur, total]) => (
                      <div key={cur}>{formatCurrency(total, cur)}</div>
                    ));
                  })()}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {showForm && (
        <AddDividendForm
          holdings={holdings}
          onAdd={handleAdd}
          onCancel={() => setShowForm(false)}
        />
      )}

      {/* Dividend history table */}
      {dividends.length === 0 ? (
        <EmptyState
          message={
            holdings.length === 0
              ? 'Add holdings first to record dividends.'
              : 'No dividends recorded yet. Click "Record Dividend" to add one.'
          }
        />
      ) : (
        <div
          style={{
            border: '1px solid var(--border-primary)',
            overflow: 'hidden',
          }}
        >
          {/* Header row */}
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: '120px 1fr 120px 110px 110px 48px',
              padding: '8px 16px',
              background: 'var(--bg-surface-alt)',
              borderBottom: '1px solid var(--border-primary)',
            }}
          >
            {[
              t('holdings.columns.symbol'),
              t('dividends.exDate'),
              t('dividends.payDate'),
              t('dividends.amountPerUnit'),
              t('dividends.currency'),
              '',
            ].map((h) => (
              <div
                key={h}
                style={{
                  fontSize: 10,
                  fontWeight: 600,
                  textTransform: 'uppercase',
                  letterSpacing: '0.06em',
                  color: 'var(--text-muted)',
                }}
              >
                {h}
              </div>
            ))}
          </div>

          {dividends.map((div, i) => (
            <div
              key={div.id}
              style={{
                display: 'grid',
                gridTemplateColumns: '120px 1fr 120px 110px 110px 48px',
                padding: '10px 16px',
                alignItems: 'center',
                background: i % 2 === 0 ? 'var(--bg-surface)' : 'var(--bg-surface-alt)',
                borderBottom: i < dividends.length - 1 ? '1px solid var(--border-subtle)' : 'none',
              }}
            >
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  fontWeight: 600,
                  color: 'var(--text-primary)',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                }}
              >
                <DollarSign size={12} style={{ color: 'var(--color-gain)', flexShrink: 0 }} />
                {div.symbol}
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 12,
                  color: 'var(--text-primary)',
                }}
              >
                {div.exDate}
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 12,
                  color: 'var(--text-secondary)',
                }}
              >
                {div.payDate}
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 13,
                  fontWeight: 500,
                  color: 'var(--color-gain)',
                  textAlign: 'right',
                }}
              >
                +{formatNumber(div.amountPerUnit, 4)}
              </div>
              <div
                style={{
                  fontFamily: 'var(--font-mono)',
                  fontSize: 12,
                  color: 'var(--text-secondary)',
                  textAlign: 'center',
                }}
              >
                {div.currency}
              </div>
              <div style={{ display: 'flex', justifyContent: 'center' }}>
                <button
                  onClick={() => handleDelete(div.id)}
                  title="Delete"
                  style={{
                    background: 'transparent',
                    border: '1px solid var(--border-primary)',
                    color: 'var(--text-secondary)',
                    cursor: 'pointer',
                    padding: '4px 6px',
                    borderRadius: 2,
                    display: 'flex',
                    alignItems: 'center',
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLButtonElement).style.color = 'var(--color-loss)';
                    (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--color-loss)';
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLButtonElement).style.color = 'var(--text-secondary)';
                    (e.currentTarget as HTMLButtonElement).style.borderColor =
                      'var(--border-primary)';
                  }}
                >
                  <Trash2 size={12} />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
