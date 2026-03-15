import { useEffect, useRef, useState } from 'react';
import type {
  AccountType,
  AssetType,
  Holding,
  HoldingInput,
  SymbolResult,
} from '../types/portfolio';
import { ACCOUNT_OPTIONS, SUPPORTED_CURRENCIES } from '../lib/constants';
import { Select } from './ui/Select';
import { SymbolSearch } from './ui/SymbolSearch';

const isTauri = (): boolean => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

// Mock prices for browser dev mode
const MOCK_PRICES: Record<string, number> = {
  AAPL: 189.3,
  MSFT: 415.5,
  NVDA: 875.4,
  GOOGL: 175.0,
  META: 510.2,
  AMZN: 195.6,
  TSLA: 175.8,
  VOO: 490.1,
  QQQ: 432.8,
  VTI: 238.4,
  'BTC-USD': 65000,
  'ETH-USD': 3400,
  'TD.TO': 78.5,
  'RY.TO': 132.4,
  'XIU.TO': 38.2,
  'VFV.TO': 117.6,
};

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onSave: (holding: HoldingInput) => void;
  editingHolding?: Holding;
}

interface FormState {
  symbol: string;
  name: string;
  assetType: AssetType;
  account: AccountType;
  quantity: string;
  costBasis: string;
  currency: string;
  exchange: string;
  targetWeight: string;
}

interface FormErrors {
  symbol?: string;
  name?: string;
  quantity?: string;
  costBasis?: string;
  targetWeight?: string;
}

const EMPTY_FORM: FormState = {
  symbol: '',
  name: '',
  assetType: 'stock',
  account: 'taxable',
  quantity: '',
  costBasis: '',
  currency: 'USD',
  exchange: '',
  targetWeight: '0',
};

const INPUT_STYLE: React.CSSProperties = {
  width: '100%',
  background: 'var(--bg-primary)',
  border: '1px solid var(--border-primary)',
  color: 'var(--text-primary)',
  padding: '7px 10px',
  fontSize: 13,
  fontFamily: 'var(--font-mono)',
  borderRadius: '2px',
  outline: 'none',
};

const LABEL_STYLE: React.CSSProperties = {
  display: 'block',
  fontSize: 10,
  color: 'var(--text-muted)',
  fontFamily: 'var(--font-mono)',
  textTransform: 'uppercase',
  letterSpacing: '0.08em',
  marginBottom: 4,
};

const ERROR_STYLE: React.CSSProperties = {
  fontSize: 11,
  color: 'var(--color-loss)',
  fontFamily: 'var(--font-mono)',
  marginTop: 3,
};

function Field({
  label,
  error,
  children,
}: {
  label: string;
  error?: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <label style={LABEL_STYLE}>{label}</label>
      {children}
      {error && <div style={ERROR_STYLE}>{error}</div>}
    </div>
  );
}

export function AddHoldingModal({ isOpen, onClose, onSave, editingHolding }: Props) {
  const [form, setForm] = useState<FormState>(EMPTY_FORM);
  const [errors, setErrors] = useState<FormErrors>({});
  const [saving, setSaving] = useState(false);
  const [priceFetching, setPriceFetching] = useState(false);
  const abortRef = useRef<AbortController | null>(null);
  const selectedSymbolRef = useRef<string>('');

  useEffect(() => {
    if (isOpen) {
      if (editingHolding) {
        setForm({
          symbol: editingHolding.symbol,
          name: editingHolding.name,
          assetType: editingHolding.assetType,
          account: editingHolding.account,
          quantity: String(editingHolding.quantity),
          costBasis: String(editingHolding.costBasis),
          currency: editingHolding.currency,
          exchange: editingHolding.exchange,
          targetWeight: String(editingHolding.targetWeight ?? 0),
        });
        selectedSymbolRef.current = editingHolding.symbol;
      } else {
        setForm(EMPTY_FORM);
        selectedSymbolRef.current = '';
      }
      setErrors({});
    } else {
      // Cancel any in-flight fetch when the modal closes
      abortRef.current?.abort();
      selectedSymbolRef.current = '';
      setPriceFetching(false);
    }
  }, [isOpen, editingHolding]);

  const isCash = form.assetType === 'cash';

  async function handleSymbolSelect(result: SymbolResult) {
    setForm((prev) => ({
      ...prev,
      symbol: result.symbol,
      name: result.name,
      assetType: result.assetType,
      currency: result.currency,
      exchange: result.exchange,
    }));
    setErrors((prev) => ({ ...prev, symbol: undefined }));

    // Cancel any in-flight price fetch
    abortRef.current?.abort();
    abortRef.current = new AbortController();

    // Record which symbol triggered this fetch so we can discard stale responses
    const fetchedForSymbol = result.symbol;
    selectedSymbolRef.current = result.symbol;

    setPriceFetching(true);
    try {
      let price: number | null = null;
      if (isTauri()) {
        const data = await tauriInvoke<{ price: number }>('get_symbol_price', {
          symbol: result.symbol,
        });
        price = data.price;
      } else {
        price = MOCK_PRICES[result.symbol] ?? null;
      }
      // Only apply if this response is still for the currently selected symbol
      if (price !== null && selectedSymbolRef.current === fetchedForSymbol) {
        setForm((prev) => ({ ...prev, costBasis: String(price) }));
      }
    } catch {
      // non-fatal: user can enter cost basis manually
    } finally {
      // Only clear the fetching indicator if we are still the active request
      if (selectedSymbolRef.current === fetchedForSymbol) {
        setPriceFetching(false);
      }
    }
  }

  function validate(): boolean {
    const next: FormErrors = {};
    if (!isCash && !form.symbol.trim()) next.symbol = 'Symbol is required';
    if (!form.name.trim()) next.name = 'Name is required';
    const qty = parseFloat(form.quantity);
    if (isNaN(qty) || qty <= 0) next.quantity = 'Quantity must be > 0';
    const cost = parseFloat(form.costBasis);
    if (isNaN(cost) || cost <= 0) next.costBasis = 'Cost basis must be > 0';
    const targetWeight = parseFloat(form.targetWeight);
    if (isNaN(targetWeight) || targetWeight < 0 || targetWeight > 100) {
      next.targetWeight = 'Target weight must be between 0 and 100';
    }
    setErrors(next);
    return Object.keys(next).length === 0;
  }

  async function handleSave() {
    if (!validate()) return;
    setSaving(true);
    try {
      const input: HoldingInput = {
        symbol: isCash ? `${form.currency}-CASH` : form.symbol.toUpperCase(),
        name: form.name,
        assetType: form.assetType,
        account: form.account,
        quantity: parseFloat(form.quantity),
        costBasis: parseFloat(form.costBasis),
        currency: form.currency,
        exchange: form.exchange.toUpperCase(),
        targetWeight: parseFloat(form.targetWeight),
      };
      await onSave(input);
      onClose();
    } finally {
      setSaving(false);
    }
  }

  function set(field: keyof FormState) {
    return (e: React.ChangeEvent<HTMLInputElement>) => {
      setForm((prev) => ({ ...prev, [field]: e.target.value }));
      setErrors((prev) => ({ ...prev, [field]: undefined }));
    };
  }

  function setSelect(field: keyof FormState) {
    return (value: string) => {
      setForm((prev) => {
        if (field === 'assetType') {
          const assetType = value as AssetType;
          return {
            ...prev,
            assetType,
            account:
              assetType === 'cash' ? 'cash' : prev.account === 'cash' ? 'taxable' : prev.account,
          };
        }
        return { ...prev, [field]: value };
      });
    };
  }

  if (!isOpen) return null;

  const isValid =
    (isCash || form.symbol.trim()) &&
    form.name.trim() &&
    parseFloat(form.quantity) > 0 &&
    parseFloat(form.costBasis) > 0 &&
    parseFloat(form.targetWeight) >= 0 &&
    parseFloat(form.targetWeight) <= 100;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(0,0,0,0.6)',
        backdropFilter: 'blur(4px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        style={{
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          width: '100%',
          maxWidth: 480,
          borderRadius: 0,
          padding: 24,
        }}
      >
        {/* Title */}
        <div style={{ marginBottom: 20 }}>
          <h2
            style={{
              fontFamily: 'var(--font-sans)',
              fontWeight: 600,
              fontSize: 15,
              color: 'var(--text-primary)',
            }}
          >
            {editingHolding ? 'Edit Holding' : 'Add Holding'}
          </h2>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
          {/* Type + Currency row */}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12 }}>
            <Field label="Asset Type">
              <Select
                value={form.assetType}
                onChange={setSelect('assetType')}
                options={[
                  { value: 'stock', label: 'Stock' },
                  { value: 'etf', label: 'ETF' },
                  { value: 'crypto', label: 'Crypto' },
                  { value: 'cash', label: 'Cash' },
                ]}
              />
            </Field>
            <Field label="Account">
              <Select
                value={form.account}
                onChange={setSelect('account')}
                options={ACCOUNT_OPTIONS.map((option) => ({
                  value: option.value,
                  label: option.label,
                }))}
              />
            </Field>
            <Field label="Currency">
              <Select
                value={form.currency}
                onChange={setSelect('currency')}
                options={[...SUPPORTED_CURRENCIES, 'AUD'].map((c) => ({ value: c, label: c }))}
              />
            </Field>
          </div>

          {/* Symbol (hidden for cash) */}
          {!isCash && (
            <Field label="Symbol" error={errors.symbol}>
              <SymbolSearch
                value={form.symbol}
                onChange={(v) => {
                  setForm((prev) => ({ ...prev, symbol: v.toUpperCase() }));
                  setErrors((prev) => ({ ...prev, symbol: undefined }));
                }}
                onSelect={handleSymbolSelect}
                disabled={saving}
              />
            </Field>
          )}

          {/* Name */}
          <Field label={isCash ? 'Description' : 'Name'} error={errors.name}>
            <input
              type="text"
              value={form.name}
              onChange={set('name')}
              placeholder={isCash ? 'US Dollar Cash' : 'Apple Inc.'}
              style={INPUT_STYLE}
              onFocus={(e) => (e.target.style.borderColor = 'var(--color-accent)')}
              onBlur={(e) => (e.target.style.borderColor = 'var(--border-primary)')}
            />
          </Field>

          {/* Exchange (non-cash only) */}
          {!isCash && (
            <Field label="Exchange (optional)">
              <input
                type="text"
                value={form.exchange}
                onChange={(e) =>
                  setForm((prev) => ({ ...prev, exchange: e.target.value.toUpperCase() }))
                }
                placeholder="NYSE"
                maxLength={10}
                style={INPUT_STYLE}
                onFocus={(e) => (e.target.style.borderColor = 'var(--color-accent)')}
                onBlur={(e) => (e.target.style.borderColor = 'var(--border-primary)')}
              />
            </Field>
          )}

          {/* Qty + Cost row */}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12 }}>
            <Field label={isCash ? 'Amount' : 'Quantity'} error={errors.quantity}>
              <input
                type="number"
                value={form.quantity}
                onChange={set('quantity')}
                placeholder="0.00"
                min="0"
                step="0.0001"
                style={INPUT_STYLE}
                onFocus={(e) => (e.target.style.borderColor = 'var(--color-accent)')}
                onBlur={(e) => (e.target.style.borderColor = 'var(--border-primary)')}
              />
            </Field>
            {!isCash && (
              <Field
                label={priceFetching ? 'Cost Per Unit (fetching…)' : 'Cost Per Unit'}
                error={errors.costBasis}
              >
                <input
                  type="number"
                  value={form.costBasis}
                  onChange={set('costBasis')}
                  placeholder="0.00"
                  min="0"
                  step="0.01"
                  style={INPUT_STYLE}
                  onFocus={(e) => (e.target.style.borderColor = 'var(--color-accent)')}
                  onBlur={(e) => (e.target.style.borderColor = 'var(--border-primary)')}
                />
              </Field>
            )}
            {isCash && (
              <Field label="Cost Basis" error={errors.costBasis}>
                <input
                  type="number"
                  value={form.costBasis}
                  onChange={set('costBasis')}
                  placeholder="1.00"
                  min="0"
                  step="0.01"
                  style={INPUT_STYLE}
                  onFocus={(e) => (e.target.style.borderColor = 'var(--color-accent)')}
                  onBlur={(e) => (e.target.style.borderColor = 'var(--border-primary)')}
                />
              </Field>
            )}
            <Field label="Target Weight %" error={errors.targetWeight}>
              <input
                type="number"
                value={form.targetWeight}
                onChange={set('targetWeight')}
                placeholder="0.0"
                min="0"
                max="100"
                step="0.1"
                style={INPUT_STYLE}
                onFocus={(e) => (e.target.style.borderColor = 'var(--color-accent)')}
                onBlur={(e) => (e.target.style.borderColor = 'var(--border-primary)')}
              />
            </Field>
          </div>
        </div>

        {/* Actions */}
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 10, marginTop: 24 }}>
          <button
            onClick={onClose}
            style={{
              padding: '7px 16px',
              background: 'transparent',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-secondary)',
              borderRadius: '2px',
              fontFamily: 'var(--font-sans)',
              fontSize: 13,
              cursor: 'pointer',
            }}
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            disabled={!isValid || saving}
            style={{
              padding: '7px 20px',
              background: isValid && !saving ? 'var(--color-accent)' : 'var(--border-primary)',
              border: 'none',
              color: isValid && !saving ? '#fff' : 'var(--text-muted)',
              borderRadius: '2px',
              fontFamily: 'var(--font-sans)',
              fontSize: 13,
              fontWeight: 600,
              cursor: isValid && !saving ? 'pointer' : 'not-allowed',
            }}
          >
            {saving ? 'Saving...' : editingHolding ? 'Save Changes' : 'Add Holding'}
          </button>
        </div>
      </div>
    </div>
  );
}
