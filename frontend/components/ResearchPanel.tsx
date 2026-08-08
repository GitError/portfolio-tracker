import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { X } from 'lucide-react';
import type { WatchlistItemWithSnapshot } from '../types/portfolio';

export interface ResearchPanelFields {
  thesis: string | null;
  catalysts: string | null;
  risks: string | null;
  entryPriceLow: number | null;
  entryPriceHigh: number | null;
}

interface ResearchPanelProps {
  item: WatchlistItemWithSnapshot;
  onSave: (fields: ResearchPanelFields) => void;
  onClose: () => void;
}

const LABEL_STYLE: React.CSSProperties = {
  display: 'block',
  fontSize: 10,
  color: 'var(--text-muted)',
  fontFamily: 'var(--font-mono)',
  textTransform: 'uppercase',
  letterSpacing: '0.08em',
  marginBottom: 4,
};

const TEXTAREA_STYLE: React.CSSProperties = {
  width: '100%',
  background: 'var(--bg-primary)',
  border: '1px solid var(--border-primary)',
  color: 'var(--text-primary)',
  padding: '8px 10px',
  fontSize: 13,
  fontFamily: 'var(--font-sans)',
  borderRadius: 2,
  outline: 'none',
  resize: 'vertical',
  minHeight: 64,
  boxSizing: 'border-box',
};

const INPUT_STYLE: React.CSSProperties = {
  width: '100%',
  background: 'var(--bg-primary)',
  border: '1px solid var(--border-primary)',
  color: 'var(--text-primary)',
  padding: '7px 10px',
  fontSize: 13,
  fontFamily: 'var(--font-mono)',
  borderRadius: 2,
  outline: 'none',
  boxSizing: 'border-box',
};

/** Editable research fields for one watchlist item. Auto-saves the full field
 * set on blur — see `update_watchlist_item`, which overwrites all research
 * fields together rather than patching individual ones. */
export function ResearchPanel({ item, onSave, onClose }: ResearchPanelProps) {
  const { t } = useTranslation();
  const [thesis, setThesis] = useState(item.thesis ?? '');
  const [catalysts, setCatalysts] = useState(item.catalysts ?? '');
  const [risks, setRisks] = useState(item.risks ?? '');
  const [entryLow, setEntryLow] = useState(
    item.entryPriceLow != null ? String(item.entryPriceLow) : ''
  );
  const [entryHigh, setEntryHigh] = useState(
    item.entryPriceHigh != null ? String(item.entryPriceHigh) : ''
  );

  useEffect(() => {
    setThesis(item.thesis ?? '');
    setCatalysts(item.catalysts ?? '');
    setRisks(item.risks ?? '');
    setEntryLow(item.entryPriceLow != null ? String(item.entryPriceLow) : '');
    setEntryHigh(item.entryPriceHigh != null ? String(item.entryPriceHigh) : '');
  }, [item.id, item.thesis, item.catalysts, item.risks, item.entryPriceLow, item.entryPriceHigh]);

  function handleBlur() {
    const low = entryLow.trim() === '' ? null : parseFloat(entryLow);
    const high = entryHigh.trim() === '' ? null : parseFloat(entryHigh);
    onSave({
      thesis: thesis.trim() === '' ? null : thesis,
      catalysts: catalysts.trim() === '' ? null : catalysts,
      risks: risks.trim() === '' ? null : risks,
      entryPriceLow: low != null && !isNaN(low) ? low : null,
      entryPriceHigh: high != null && !isNaN(high) ? high : null,
    });
  }

  return (
    <div
      style={{
        background: 'var(--bg-surface-alt)',
        border: '1px solid var(--border-primary)',
        borderTop: 'none',
        padding: 16,
        display: 'flex',
        flexDirection: 'column',
        gap: 14,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <span
          style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 12,
            fontWeight: 600,
            color: 'var(--text-primary)',
          }}
        >
          {item.symbol}
        </span>
        <button
          onClick={onClose}
          aria-label={t('common.close')}
          style={{
            background: 'transparent',
            border: 'none',
            color: 'var(--text-secondary)',
            cursor: 'pointer',
            display: 'flex',
          }}
        >
          <X size={14} />
        </button>
      </div>

      <div>
        <label style={LABEL_STYLE}>{t('research.panel.thesis')}</label>
        <textarea
          value={thesis}
          onChange={(e) => setThesis(e.target.value)}
          onBlur={handleBlur}
          style={TEXTAREA_STYLE}
        />
      </div>

      <div>
        <label style={LABEL_STYLE}>{t('research.panel.catalysts')}</label>
        <textarea
          value={catalysts}
          onChange={(e) => setCatalysts(e.target.value)}
          onBlur={handleBlur}
          style={TEXTAREA_STYLE}
        />
      </div>

      <div>
        <label style={LABEL_STYLE}>{t('research.panel.risks')}</label>
        <textarea
          value={risks}
          onChange={(e) => setRisks(e.target.value)}
          onBlur={handleBlur}
          style={TEXTAREA_STYLE}
        />
      </div>

      <div>
        <label style={LABEL_STYLE}>{t('research.panel.entryRange')}</label>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          <input
            type="number"
            step="0.01"
            min="0"
            placeholder={t('research.panel.entryLow')}
            value={entryLow}
            onChange={(e) => setEntryLow(e.target.value)}
            onBlur={handleBlur}
            style={INPUT_STYLE}
          />
          <input
            type="number"
            step="0.01"
            min="0"
            placeholder={t('research.panel.entryHigh')}
            value={entryHigh}
            onChange={(e) => setEntryHigh(e.target.value)}
            onBlur={handleBlur}
            style={INPUT_STYLE}
          />
        </div>
      </div>
    </div>
  );
}
