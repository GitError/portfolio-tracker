// ─── Shock slider controls for stress test scenarios ─────────────────────────

type ShockMap = Record<string, number>; // values are decimals e.g. -0.20

const ASSET_SLIDERS: { key: string; label: string }[] = [
  { key: 'stock', label: 'Stocks' },
  { key: 'etf', label: 'ETFs' },
  { key: 'crypto', label: 'Crypto' },
];

const SECTION_TITLE: React.CSSProperties = {
  fontSize: 10,
  color: 'var(--text-muted)',
  fontFamily: 'var(--font-mono)',
  textTransform: 'uppercase',
  letterSpacing: '0.1em',
  marginBottom: 12,
  paddingBottom: 6,
  borderBottom: '1px solid var(--border-subtle)',
};

// ─── Single slider ────────────────────────────────────────────────────────────
function ShockSlider({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (v: number) => void;
}) {
  const pct = Math.round(value * 100);
  const color =
    value > 0 ? 'var(--color-gain)' : value < 0 ? 'var(--color-loss)' : 'var(--color-accent)';
  // Fill gradient position: 0 = -50%, 50 = 0%, 100 = +50%
  const pos = ((value * 100 + 50) / 100) * 100; // 0-100
  const gradStop =
    value < 0
      ? `var(--color-loss) 0%, var(--color-loss) ${pos}%, var(--border-primary) ${pos}%`
      : value > 0
        ? `var(--border-primary) 0%, var(--border-primary) ${pos}%, var(--color-gain) ${pos}%`
        : `var(--border-primary) 0%`;

  return (
    <div style={{ marginBottom: 14 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 5 }}>
        <span
          style={{ fontSize: 12, color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)' }}
        >
          {label}
        </span>
        <span
          style={{
            fontSize: 12,
            fontFamily: 'var(--font-mono)',
            fontWeight: 600,
            color,
            minWidth: 50,
            textAlign: 'right',
          }}
        >
          {pct >= 0 ? '+' : ''}
          {pct}%
        </span>
      </div>
      <input
        type="range"
        min={-50}
        max={50}
        step={1}
        value={pct}
        onChange={(e) => onChange(parseInt(e.target.value) / 100)}
        style={{
          width: '100%',
          height: 4,
          appearance: 'none',
          WebkitAppearance: 'none',
          background: `linear-gradient(to right, ${gradStop})`,
          outline: 'none',
          cursor: 'pointer',
          borderRadius: 0,
        }}
      />
    </div>
  );
}

// ─── Props ────────────────────────────────────────────────────────────────────
export interface ShockSlidersProps {
  shocks: ShockMap;
  onChange: (key: string, value: number) => void;
  activeFxSliders: { key: string; label: string }[];
  baseCurrency: string;
}

// ─── ShockSliders component ───────────────────────────────────────────────────
export function ShockSliders({
  shocks,
  onChange,
  activeFxSliders,
  baseCurrency,
}: ShockSlidersProps) {
  return (
    <>
      {/* Asset class shocks */}
      <div style={{ marginBottom: 20 }}>
        <div style={SECTION_TITLE}>Asset Class Shocks</div>
        {ASSET_SLIDERS.map(({ key, label }) => (
          <ShockSlider
            key={key}
            label={label}
            value={shocks[key] ?? 0}
            onChange={(v) => onChange(key, v)}
          />
        ))}
      </div>

      {/* FX shocks */}
      {activeFxSliders.length > 0 && (
        <div>
          <div style={SECTION_TITLE}>Currency Shocks</div>
          <div
            style={{
              fontSize: 10,
              color: 'var(--text-muted)',
              fontFamily: 'var(--font-mono)',
              marginBottom: 10,
            }}
          >
            Positive = {baseCurrency} weakens. Foreign holdings convert into more {baseCurrency}.
            Negative = {baseCurrency} strengthens.
          </div>
          {activeFxSliders.map(({ key, label }) => (
            <ShockSlider
              key={key}
              label={label}
              value={shocks[key] ?? 0}
              onChange={(v) => onChange(key, v)}
            />
          ))}
        </div>
      )}

      {/* Slider thumb global style injection */}
      <style>{`
        input[type='range']::-webkit-slider-thumb {
          -webkit-appearance: none;
          appearance: none;
          width: 12px;
          height: 12px;
          background: var(--color-accent);
          border-radius: 0;
          cursor: pointer;
        }
        input[type='range']::-moz-range-thumb {
          width: 12px;
          height: 12px;
          background: var(--color-accent);
          border-radius: 0;
          border: none;
          cursor: pointer;
        }
      `}</style>
    </>
  );
}
