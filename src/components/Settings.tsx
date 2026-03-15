import { useEffect, useState } from 'react';
import { useConfig } from '../hooks/useConfig';

// ── Types ────────────────────────────────────────────────────────────────────

interface SelectSettingProps {
  label: string;
  description?: string;
  value: string;
  options: { value: string; label: string; disabled?: boolean; badge?: string }[];
  onChange: (value: string) => void;
  disabled?: boolean;
}

interface ToggleSettingProps {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}

interface SectionProps {
  title: string;
  children: React.ReactNode;
}

// ── Sub-components ────────────────────────────────────────────────────────────

function Section({ title, children }: SectionProps) {
  return (
    <div
      style={{
        background: 'var(--bg-surface)',
        border: '1px solid var(--border-primary)',
        marginBottom: 24,
      }}
    >
      <div
        style={{
          padding: '12px 20px',
          borderBottom: '1px solid var(--border-primary)',
        }}
      >
        <span
          style={{
            fontSize: 11,
            fontWeight: 600,
            textTransform: 'uppercase',
            letterSpacing: '0.08em',
            color: 'var(--text-secondary)',
            fontFamily: 'var(--font-sans)',
          }}
        >
          {title}
        </span>
      </div>
      <div>{children}</div>
    </div>
  );
}

function SettingRow({ children, last }: { children: React.ReactNode; last?: boolean }) {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '14px 20px',
        borderBottom: last ? 'none' : '1px solid var(--border-subtle)',
        gap: 24,
      }}
    >
      {children}
    </div>
  );
}

function SettingLabel({ label, description }: { label: string; description?: string }) {
  return (
    <div style={{ flex: 1 }}>
      <div style={{ fontSize: 13, color: 'var(--text-primary)', fontWeight: 500 }}>{label}</div>
      {description && (
        <div style={{ fontSize: 11, color: 'var(--text-secondary)', marginTop: 2 }}>
          {description}
        </div>
      )}
    </div>
  );
}

function SavedIndicator({ visible }: { visible: boolean }) {
  return (
    <span
      style={{
        fontSize: 11,
        color: 'var(--color-gain)',
        opacity: visible ? 1 : 0,
        transition: 'opacity 200ms',
        fontFamily: 'var(--font-mono)',
        minWidth: 42,
        textAlign: 'right',
      }}
    >
      Saved
    </span>
  );
}

function SelectSetting({ label, description, value, options, onChange, disabled }: SelectSettingProps) {
  const [showSaved, setShowSaved] = useState(false);

  function handleChange(e: React.ChangeEvent<HTMLSelectElement>) {
    onChange(e.target.value);
    setShowSaved(true);
    setTimeout(() => setShowSaved(false), 1000);
  }

  return (
    <SettingRow>
      <SettingLabel label={label} description={description} />
      <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <SavedIndicator visible={showSaved} />
        <select
          value={value}
          onChange={handleChange}
          disabled={disabled}
          style={{
            background: 'var(--bg-surface-alt)',
            border: '1px solid var(--border-primary)',
            borderRadius: 2,
            color: 'var(--text-primary)',
            fontSize: 12,
            padding: '6px 10px',
            fontFamily: 'var(--font-sans)',
            cursor: disabled ? 'not-allowed' : 'pointer',
            opacity: disabled ? 0.5 : 1,
            minWidth: 140,
            outline: 'none',
          }}
          onFocus={(e) => {
            (e.currentTarget as HTMLSelectElement).style.borderColor = 'var(--color-accent)';
          }}
          onBlur={(e) => {
            (e.currentTarget as HTMLSelectElement).style.borderColor = 'var(--border-primary)';
          }}
        >
          {options.map((opt) => (
            <option key={opt.value} value={opt.value} disabled={opt.disabled}>
              {opt.label}
              {opt.badge ? ` (${opt.badge})` : ''}
            </option>
          ))}
        </select>
      </div>
    </SettingRow>
  );
}

function ToggleSetting({ label, description, checked, onChange }: ToggleSettingProps) {
  const [showSaved, setShowSaved] = useState(false);

  function handleChange() {
    onChange(!checked);
    setShowSaved(true);
    setTimeout(() => setShowSaved(false), 1000);
  }

  return (
    <SettingRow>
      <SettingLabel label={label} description={description} />
      <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <SavedIndicator visible={showSaved} />
        <button
          role="switch"
          aria-checked={checked}
          onClick={handleChange}
          style={{
            position: 'relative',
            display: 'inline-flex',
            alignItems: 'center',
            width: 40,
            height: 22,
            borderRadius: 11,
            border: 'none',
            cursor: 'pointer',
            padding: 0,
            background: checked ? 'var(--color-accent)' : 'var(--border-primary)',
            transition: 'background 200ms',
            flexShrink: 0,
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLButtonElement).style.opacity = '0.85';
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLButtonElement).style.opacity = '1';
          }}
        >
          <span
            style={{
              position: 'absolute',
              top: 3,
              left: checked ? 21 : 3,
              width: 16,
              height: 16,
              borderRadius: '50%',
              background: 'var(--text-primary)',
              transition: 'left 200ms',
            }}
          />
        </button>
      </div>
    </SettingRow>
  );
}

// ── Main component ────────────────────────────────────────────────────────────

export function Settings() {
  // General
  const { value: baseCurrency, setValue: setBaseCurrency, ready: bcReady } = useConfig(
    'base_currency',
    'CAD'
  );
  const { value: autoRefresh, setValue: setAutoRefresh, ready: arReady } = useConfig(
    'auto_refresh_interval',
    'off'
  );
  const { value: marketHoursOnly, setValue: setMarketHoursOnly, ready: mhReady } = useConfig(
    'auto_refresh_market_hours_only',
    'false'
  );

  // Portfolio
  const { value: costBasisMethod, setValue: setCostBasisMethod, ready: cbReady } = useConfig(
    'cost_basis_method',
    'avco'
  );
  const { value: defaultRange, setValue: setDefaultRange, ready: drReady } = useConfig(
    'default_perf_range',
    '1M'
  );

  // Display
  const { value: compactNumbers, setValue: setCompactNumbers, ready: cnReady } = useConfig(
    'compact_numbers',
    'false'
  );

  const allReady = bcReady && arReady && mhReady && cbReady && drReady && cnReady;

  // Scroll to top on mount
  useEffect(() => {
    window.scrollTo(0, 0);
  }, []);

  return (
    <div
      style={{
        padding: '28px 32px',
        maxWidth: 680,
        overflowY: 'auto',
        height: '100%',
      }}
    >
      {/* Page header */}
      <div style={{ marginBottom: 28 }}>
        <h1
          style={{
            fontSize: 18,
            fontWeight: 600,
            color: 'var(--text-primary)',
            marginBottom: 4,
          }}
        >
          Settings
        </h1>
        <p style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
          Configure your portfolio tracker preferences. Changes are saved automatically.
        </p>
      </div>

      {!allReady ? (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            paddingTop: 80,
            color: 'var(--text-secondary)',
            fontSize: 13,
          }}
        >
          Loading settings…
        </div>
      ) : (
        <>
          {/* ── General ───────────────────────────────────────────── */}
          <Section title="General">
            <SelectSetting
              label="Base Currency"
              description="All portfolio values are converted to this currency."
              value={baseCurrency}
              onChange={setBaseCurrency}
              options={[
                { value: 'CAD', label: 'CAD — Canadian Dollar' },
                { value: 'USD', label: 'USD — US Dollar' },
                { value: 'EUR', label: 'EUR — Euro' },
                { value: 'GBP', label: 'GBP — British Pound' },
                { value: 'AUD', label: 'AUD — Australian Dollar' },
              ]}
            />
            <SelectSetting
              label="Auto-Refresh Interval"
              description="How often to automatically refresh prices in the background."
              value={autoRefresh}
              onChange={setAutoRefresh}
              options={[
                { value: 'off', label: 'Off' },
                { value: '1', label: 'Every 1 minute' },
                { value: '5', label: 'Every 5 minutes' },
                { value: '15', label: 'Every 15 minutes' },
                { value: '30', label: 'Every 30 minutes' },
              ]}
            />
            <ToggleSetting
              label="Market Hours Only"
              description="Only auto-refresh during regular market trading hours (Mon–Fri 9:30–16:00 ET)."
              checked={marketHoursOnly === 'true'}
              onChange={(checked) => setMarketHoursOnly(checked ? 'true' : 'false')}
            />
          </Section>

          {/* ── Portfolio ─────────────────────────────────────────── */}
          <Section title="Portfolio">
            <SelectSetting
              label="Cost Basis Method"
              description="How to calculate the average cost of your holdings."
              value={costBasisMethod}
              onChange={setCostBasisMethod}
              options={[
                { value: 'avco', label: 'AVCO — Average Cost' },
                { value: 'fifo', label: 'FIFO — First In, First Out', disabled: true, badge: 'coming soon' },
              ]}
            />
            <SelectSetting
              label="Default Date Range"
              description="Default time range shown on the Performance chart."
              value={defaultRange}
              onChange={setDefaultRange}
              options={[
                { value: '1M', label: '1 Month' },
                { value: '3M', label: '3 Months' },
                { value: '6M', label: '6 Months' },
                { value: '1Y', label: '1 Year' },
                { value: 'All', label: 'All Time' },
              ]}
            />
          </Section>

          {/* ── Display ───────────────────────────────────────────── */}
          <Section title="Display">
            <ToggleSetting
              label="Compact Number Formatting"
              description='Show large numbers in compact form (e.g. "1.2M" instead of "1,200,000").'
              checked={compactNumbers === 'true'}
              onChange={(checked) => setCompactNumbers(checked ? 'true' : 'false')}
            />
          </Section>
        </>
      )}
    </div>
  );
}
