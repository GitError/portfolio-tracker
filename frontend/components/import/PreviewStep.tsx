import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { ImportPlan, RowAction } from '../../types/portfolio';
import { useFormatNumber } from '../../hooks/useFormatters';
import {
  computeStatusCounts,
  countCommittable,
  filterRowsByStatus,
  rowFixHint,
  ROW_ACTIONS,
} from '../../lib/importWizard';
import { MONO, STATUS_META, TD } from './constants';
import { StatusBadge } from './shared';

interface Props {
  plan: ImportPlan;
  statusFilter: RowAction | 'all';
  onChangeFilter: (filter: RowAction | 'all') => void;
  includeCash: boolean;
  onToggleIncludeCash: (value: boolean) => void;
  onBack: () => void;
  onCommit: () => void;
  committing: boolean;
}

export function PreviewStep({
  plan,
  statusFilter,
  onChangeFilter,
  includeCash,
  onToggleIncludeCash,
  onBack,
  onCommit,
  committing,
}: Props) {
  const { t } = useTranslation();
  const formatNumber = useFormatNumber();

  const allRows = useMemo(() => [...plan.rows, ...plan.cashRows], [plan]);
  const filteredRows = useMemo(
    () => filterRowsByStatus(allRows, statusFilter),
    [allRows, statusFilter]
  );
  const counts = useMemo(() => computeStatusCounts(allRows), [allRows]);
  const committableRows = includeCash ? allRows : plan.rows;
  const importableCount = countCommittable(committableRows);

  return (
    <div>
      <div
        style={{
          display: 'flex',
          gap: 8,
          flexWrap: 'wrap',
          marginBottom: 12,
        }}
      >
        <FilterButton
          active={statusFilter === 'all'}
          label={t('importWizard.preview.all', { count: allRows.length })}
          onClick={() => onChangeFilter('all')}
        />
        {ROW_ACTIONS.map((action) => (
          <FilterButton
            key={action}
            active={statusFilter === action}
            label={`${t(STATUS_META[action].i18nKey)} (${counts[action]})`}
            color={STATUS_META[action].color}
            onClick={() => onChangeFilter(action)}
          />
        ))}
      </div>

      {plan.cashRows.length > 0 ? (
        <label
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            marginBottom: 12,
            ...MONO,
            fontSize: 12,
            color: 'var(--text-secondary)',
            cursor: 'pointer',
          }}
        >
          <input
            type="checkbox"
            checked={includeCash}
            onChange={(e) => onToggleIncludeCash(e.target.checked)}
          />
          {t('importWizard.preview.includeCash', { count: plan.cashRows.length })}
        </label>
      ) : null}

      <div
        style={{ border: '1px solid var(--border-primary)', overflowX: 'auto', marginBottom: 16 }}
      >
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 11 }}>
          <thead>
            <tr style={{ background: 'var(--bg-surface-alt)' }}>
              {[
                t('importWizard.preview.columns.row'),
                t('importWizard.preview.columns.symbol'),
                t('importWizard.preview.columns.name'),
                t('importWizard.preview.columns.type'),
                t('importWizard.preview.columns.quantity'),
                t('importWizard.preview.columns.cost'),
                t('importWizard.preview.columns.currency'),
                t('importWizard.preview.columns.status'),
              ].map((h) => (
                <th
                  key={h}
                  style={{
                    ...TD,
                    ...MONO,
                    textAlign: 'left',
                    color: 'var(--text-muted)',
                    textTransform: 'uppercase',
                    letterSpacing: '0.06em',
                    fontWeight: 400,
                  }}
                >
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {filteredRows.map((row) => {
              const hint = rowFixHint(row);
              return (
                <tr key={row.rowNumber}>
                  <td style={{ ...TD, ...MONO, color: 'var(--text-muted)' }}>{row.rowNumber}</td>
                  <td style={{ ...TD, ...MONO, color: 'var(--text-primary)', fontWeight: 600 }}>
                    {row.resolvedSymbol || row.symbol || '—'}
                  </td>
                  <td
                    style={{
                      ...TD,
                      fontFamily: 'var(--font-sans)',
                      color: 'var(--text-secondary)',
                      maxWidth: 160,
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {row.name || '—'}
                  </td>
                  <td style={{ ...TD, ...MONO, color: 'var(--text-muted)' }}>
                    {row.assetType || '—'}
                  </td>
                  <td
                    style={{ ...TD, ...MONO, color: 'var(--text-secondary)', textAlign: 'right' }}
                  >
                    {row.quantity != null ? formatNumber(row.quantity, 4) : '—'}
                  </td>
                  <td
                    style={{ ...TD, ...MONO, color: 'var(--text-secondary)', textAlign: 'right' }}
                  >
                    {row.costBasis != null ? formatNumber(row.costBasis, 2) : '—'}
                  </td>
                  <td style={{ ...TD, ...MONO, color: 'var(--text-muted)' }}>
                    {row.currency || '—'}
                  </td>
                  <td style={TD}>
                    <StatusBadge action={row.action} t={t} />
                    {hint ? (
                      <div
                        style={{
                          ...MONO,
                          fontSize: 10,
                          color: 'var(--text-muted)',
                          marginTop: 4,
                          maxWidth: 260,
                        }}
                      >
                        {hint}
                      </div>
                    ) : null}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between' }}>
        <button
          onClick={onBack}
          disabled={committing}
          style={{
            background: 'none',
            border: '1px solid var(--border-primary)',
            color: 'var(--text-secondary)',
            ...MONO,
            fontSize: 11,
            padding: '6px 10px',
            cursor: committing ? 'not-allowed' : 'pointer',
          }}
        >
          {t('importWizard.back')}
        </button>
        <button
          onClick={onCommit}
          disabled={committing || importableCount === 0}
          style={{
            background: importableCount > 0 ? 'var(--color-accent)' : 'var(--border-primary)',
            border: 'none',
            color: importableCount > 0 ? '#fff' : 'var(--text-muted)',
            fontFamily: 'var(--font-sans)',
            fontSize: 12,
            fontWeight: 600,
            padding: '8px 16px',
            cursor: committing || importableCount === 0 ? 'not-allowed' : 'pointer',
          }}
        >
          {committing
            ? t('importWizard.preview.importing')
            : t('importWizard.preview.importButton', { count: importableCount })}
        </button>
      </div>
    </div>
  );
}

function FilterButton({
  active,
  label,
  color,
  onClick,
}: {
  active: boolean;
  label: string;
  color?: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      style={{
        background: active ? 'var(--bg-surface-hover)' : 'none',
        border: `1px solid ${active ? (color ?? 'var(--color-accent)') : 'var(--border-primary)'}`,
        color: active ? (color ?? 'var(--text-primary)') : 'var(--text-secondary)',
        ...MONO,
        fontSize: 11,
        padding: '4px 8px',
        borderRadius: 2,
        cursor: 'pointer',
      }}
    >
      {label}
    </button>
  );
}
