// ─── Preset scenario selector for stress test ─────────────────────────────────
import { HelpCircle } from 'lucide-react';
import { Select } from './ui/Select';
import type { StressScenarioInfo } from '../types/portfolio';

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

// ─── Props ────────────────────────────────────────────────────────────────────
export interface PresetScenarioSelectorProps {
  presetName: string;
  presetNames: string[];
  scenarioInfo: StressScenarioInfo[];
  onSelect: (name: string) => void;
  onInfoOpen: () => void;
}

// ─── PresetScenarioSelector component ────────────────────────────────────────
export function PresetScenarioSelector({
  presetName,
  presetNames,
  scenarioInfo,
  onSelect,
  onInfoOpen,
}: PresetScenarioSelectorProps) {
  const activePresetInfo =
    presetName !== 'Custom' ? (scenarioInfo.find((s) => s.name === presetName) ?? null) : null;

  return (
    <div style={{ marginBottom: 20 }}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: 12,
          marginBottom: 10,
        }}
      >
        <div style={{ ...SECTION_TITLE, flex: 1, marginBottom: 0 }}>Preset Scenario</div>
        {activePresetInfo && (
          <button
            onClick={onInfoOpen}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: 6,
              background: 'transparent',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-secondary)',
              padding: '5px 10px',
              cursor: 'pointer',
              fontFamily: 'var(--font-mono)',
              fontSize: 10,
              textTransform: 'uppercase',
              letterSpacing: '0.06em',
            }}
          >
            <HelpCircle size={12} />
            Info
          </button>
        )}
      </div>
      <Select
        value={presetName}
        onChange={onSelect}
        options={presetNames.map((n) => ({ value: n, label: n }))}
      />
      {/* Scenario description */}
      {activePresetInfo && (
        <div
          style={{
            marginTop: 8,
            fontSize: 10,
            color: 'var(--text-muted)',
            fontFamily: 'var(--font-sans)',
            lineHeight: 1.5,
          }}
        >
          {activePresetInfo.description}
        </div>
      )}
    </div>
  );
}
