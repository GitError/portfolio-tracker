import { useEffect, useState } from 'react';
import type { AssetType, Holding, HoldingInput } from '../types/portfolio';
import { SUPPORTED_CURRENCIES } from '../lib/constants';

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
  quantity: string;
  costBasis: string;
  currency: string;
}

interface FormErrors {
  symbol?: string;
  name?: string;
  quantity?: string;
  costBasis?: string;
}

const EMPTY_FORM: FormState = {
  symbol: '',
  name: '',
  assetType: 'stock',
  quantity: '',
  costBasis: '',
  currency: 'USD',
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

  useEffect(() => {
    if (isOpen) {
      if (editingHolding) {
        setForm({
          symbol: editingHolding.symbol,
          name: editingHolding.name,
          assetType: editingHolding.assetType,
          quantity: String(editingHolding.quantity),
          costBasis: String(editingHolding.costBasis),
          currency: editingHolding.currency,
        });
      } else {
        setForm(EMPTY_FORM);
      }
      setErrors({});
    }
  }, [isOpen, editingHolding]);

  const isCash = form.assetType === 'cash';

  function validate(): boolean {
    const next: FormErrors = {};
    if (!isCash && !form.symbol.trim()) next.symbol = 'Symbol is required';
    if (!form.name.trim()) next.name = 'Name is required';
    const qty = parseFloat(form.quantity);
    if (isNaN(qty) || qty <= 0) next.quantity = 'Quantity must be > 0';
    const cost = parseFloat(form.costBasis);
    if (isNaN(cost) || cost <= 0) next.costBasis = 'Cost basis must be > 0';
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
        quantity: parseFloat(form.quantity),
        costBasis: parseFloat(form.costBasis),
        currency: form.currency,
      };
      await onSave(input);
      onClose();
    } finally {
      setSaving(false);
    }
  }

  function set(field: keyof FormState) {
    return (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
      setForm((prev) => ({ ...prev, [field]: e.target.value }));
      setErrors((prev) => ({ ...prev, [field]: undefined }));
    };
  }

  if (!isOpen) return null;

  const isValid =
    (isCash || form.symbol.trim()) &&
    form.name.trim() &&
    parseFloat(form.quantity) > 0 &&
    parseFloat(form.costBasis) > 0;

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
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
            <Field label="Asset Type">
              <select
                value={form.assetType}
                onChange={set('assetType')}
                style={{ ...INPUT_STYLE, cursor: 'pointer' }}
              >
                <option value="stock">Stock</option>
                <option value="etf">ETF</option>
                <option value="crypto">Crypto</option>
                <option value="cash">Cash</option>
              </select>
            </Field>
            <Field label="Currency">
              <select
                value={form.currency}
                onChange={set('currency')}
                style={{ ...INPUT_STYLE, cursor: 'pointer' }}
              >
                {SUPPORTED_CURRENCIES.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
                <option value="AUD">AUD</option>
              </select>
            </Field>
          </div>

          {/* Symbol (hidden for cash) */}
          {!isCash && (
            <Field label="Symbol" error={errors.symbol}>
              <input
                type="text"
                value={form.symbol}
                onChange={set('symbol')}
                placeholder="AAPL"
                style={{ ...INPUT_STYLE, textTransform: 'uppercase' }}
                onFocus={(e) => (e.target.style.borderColor = 'var(--color-accent)')}
                onBlur={(e) => (e.target.style.borderColor = 'var(--border-primary)')}
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

          {/* Qty + Cost row */}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
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
              <Field label="Cost Per Unit" error={errors.costBasis}>
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
