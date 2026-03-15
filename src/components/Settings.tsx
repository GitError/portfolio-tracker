import { useConfig } from '../hooks/useConfig';

const CURRENCIES = ['CAD', 'USD', 'EUR', 'GBP', 'AUD', 'CHF', 'JPY'];

const REFRESH_OPTIONS: { label: string; value: string }[] = [
  { label: 'Disabled', value: '0' },
  { label: '1 minute', value: '60000' },
  { label: '5 minutes', value: '300000' },
  { label: '15 minutes', value: '900000' },
  { label: '30 minutes', value: '1800000' },
  { label: '1 hour', value: '3600000' },
];

const COST_BASIS_OPTIONS: { label: string; value: string; description: string }[] = [
  {
    label: 'Average Cost (AVCO)',
    value: 'AVCO',
    description: 'Uses the average purchase price across all lots.',
  },
  {
    label: 'First In, First Out (FIFO)',
    value: 'FIFO',
    description: 'Oldest shares are considered sold first.',
  },
];

function SettingRow({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: 24,
        padding: '16px 0',
        borderBottom: '1px solid var(--border-subtle)',
      }}
    >
      <div style={{ flex: 1 }}>
        <div style={{ fontSize: 13, fontWeight: 500, color: 'var(--text-primary)' }}>{label}</div>
        {description && (
          <div style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 3 }}>
            {description}
          </div>
        )}
      </div>
      <div style={{ flexShrink: 0 }}>{children}</div>
    </div>
  );
}

function Select({
  value,
  onChange,
  options,
}: {
  value: string;
  onChange: (v: string) => void;
  options: { label: string; value: string }[];
}) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      style={{
        background: 'var(--bg-surface-alt)',
        border: '1px solid var(--border-primary)',
        color: 'var(--text-primary)',
        fontSize: 13,
        padding: '6px 10px',
        borderRadius: 2,
        cursor: 'pointer',
        outline: 'none',
        minWidth: 160,
        fontFamily: 'var(--font-sans)',
      }}
    >
      {options.map((opt) => (
        <option key={opt.value} value={opt.value}>
          {opt.label}
        </option>
      ))}
    </select>
  );
}

function SectionHeader({ title }: { title: string }) {
  return (
    <div
      style={{
        fontSize: 11,
        fontWeight: 600,
        textTransform: 'uppercase',
        letterSpacing: '0.08em',
        color: 'var(--text-muted)',
        padding: '24px 0 8px',
      }}
    >
      {title}
    </div>
  );
}

export function Settings() {
  const { value: baseCurrency, setValue: setBaseCurrency } = useConfig('base_currency', 'CAD');
  const { value: autoRefreshStr, setValue: setAutoRefresh } = useConfig(
    'auto_refresh_interval_ms',
    '0'
  );
  const { value: costBasisMethod, setValue: setCostBasisMethod } = useConfig(
    'cost_basis_method',
    'AVCO'
  );

  return (
    <div
      style={{
        flex: 1,
        overflow: 'auto',
        padding: '24px 32px',
        maxWidth: 640,
      }}
    >
      <h1
        style={{
          fontSize: 18,
          fontWeight: 600,
          color: 'var(--text-primary)',
          margin: 0,
          marginBottom: 4,
        }}
      >
        Settings
      </h1>
      <p style={{ fontSize: 13, color: 'var(--text-secondary)', margin: 0 }}>
        Configure display, refresh, and calculation preferences.
      </p>

      {/* Display */}
      <SectionHeader title="Display" />
      <div
        style={{
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          borderRadius: 2,
          padding: '0 16px',
        }}
      >
        <SettingRow
          label="Base Currency"
          description="All portfolio values are converted and displayed in this currency."
        >
          <Select
            value={baseCurrency}
            onChange={setBaseCurrency}
            options={CURRENCIES.map((c) => ({ label: c, value: c }))}
          />
        </SettingRow>
      </div>

      {/* Data */}
      <SectionHeader title="Data" />
      <div
        style={{
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          borderRadius: 2,
          padding: '0 16px',
        }}
      >
        <SettingRow
          label="Auto-Refresh Interval"
          description="Automatically refresh prices in the background at this interval."
        >
          <Select value={autoRefreshStr} onChange={setAutoRefresh} options={REFRESH_OPTIONS} />
        </SettingRow>
      </div>

      {/* Calculations */}
      <SectionHeader title="Calculations" />
      <div
        style={{
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          borderRadius: 2,
          padding: '0 16px',
        }}
      >
        {COST_BASIS_OPTIONS.map((opt, i) => (
          <div
            key={opt.value}
            onClick={() => setCostBasisMethod(opt.value)}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 12,
              padding: '14px 0',
              borderBottom:
                i < COST_BASIS_OPTIONS.length - 1 ? '1px solid var(--border-subtle)' : 'none',
              cursor: 'pointer',
            }}
          >
            <div
              style={{
                width: 16,
                height: 16,
                borderRadius: '50%',
                border: `2px solid ${costBasisMethod === opt.value ? 'var(--color-accent)' : 'var(--border-primary)'}`,
                background: costBasisMethod === opt.value ? 'var(--color-accent)' : 'transparent',
                flexShrink: 0,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition: 'border-color 150ms, background 150ms',
              }}
            >
              {costBasisMethod === opt.value && (
                <div style={{ width: 6, height: 6, borderRadius: '50%', background: '#fff' }} />
              )}
            </div>
            <div>
              <div style={{ fontSize: 13, fontWeight: 500, color: 'var(--text-primary)' }}>
                {opt.label}
              </div>
              <div style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 2 }}>
                {opt.description}
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* About */}
      <SectionHeader title="About" />
      <div
        style={{
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          borderRadius: 2,
          padding: '0 16px',
        }}
      >
        <SettingRow label="Version">
          <span
            style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 12,
              color: 'var(--text-secondary)',
            }}
          >
            0.1.0
          </span>
        </SettingRow>
      </div>
    </div>
  );
}
