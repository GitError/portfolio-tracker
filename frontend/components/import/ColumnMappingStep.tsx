import { useTranslation } from 'react-i18next';
import type { ColumnMapping } from '../../types/portfolio';
import { Select } from '../ui/Select';
import { CANONICAL_FIELDS, MONO, TD } from './constants';

interface Props {
  mappings: ColumnMapping[];
  overrides: Record<string, string>;
  onChangeOverride: (sourceHeader: string, canonicalField: string) => void;
  onBack: () => void;
  onContinue: () => void;
  loading: boolean;
}

const IGNORE_VALUE = '__ignore__';

export function ColumnMappingStep({
  mappings,
  overrides,
  onChangeOverride,
  onBack,
  onContinue,
  loading,
}: Props) {
  const { t } = useTranslation();

  const fieldOptions = [
    { value: IGNORE_VALUE, label: t('importWizard.mapping.ignoreColumn') },
    ...CANONICAL_FIELDS.map((f) => ({ value: f.value, label: t(f.i18nKey) })),
  ];

  return (
    <div>
      <div style={{ ...MONO, fontSize: 12, color: 'var(--text-secondary)', marginBottom: 12 }}>
        {t('importWizard.mapping.description')}
      </div>
      <div
        style={{ border: '1px solid var(--border-primary)', overflowX: 'auto', marginBottom: 16 }}
      >
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 11 }}>
          <thead>
            <tr style={{ background: 'var(--bg-surface-alt)' }}>
              {[
                t('importWizard.mapping.columns.source'),
                t('importWizard.mapping.columns.detected'),
                t('importWizard.mapping.columns.mapTo'),
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
            {mappings.map((mapping) => {
              const isUnmapped = mapping.canonicalField === null;
              const currentOverride = overrides[mapping.sourceHeader];
              const detectedLabel = mapping.canonicalField
                ? (CANONICAL_FIELDS.find((f) => f.value === mapping.canonicalField)?.i18nKey &&
                    t(CANONICAL_FIELDS.find((f) => f.value === mapping.canonicalField)!.i18nKey)) ||
                  mapping.canonicalField
                : t('importWizard.mapping.unmapped');
              return (
                <tr key={mapping.sourceHeader}>
                  <td style={{ ...TD, ...MONO, color: 'var(--text-primary)', fontWeight: 600 }}>
                    {mapping.sourceHeader}
                  </td>
                  <td
                    style={{
                      ...TD,
                      ...MONO,
                      color: isUnmapped ? 'var(--color-warning)' : 'var(--text-secondary)',
                    }}
                  >
                    {detectedLabel}
                  </td>
                  <td style={{ ...TD, minWidth: 200 }}>
                    {isUnmapped ? (
                      <Select
                        value={currentOverride ?? IGNORE_VALUE}
                        onChange={(value) =>
                          onChangeOverride(
                            mapping.sourceHeader,
                            value === IGNORE_VALUE ? '' : value
                          )
                        }
                        options={fieldOptions}
                      />
                    ) : (
                      <span style={{ ...MONO, fontSize: 11, color: 'var(--text-muted)' }}>—</span>
                    )}
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
          disabled={loading}
          style={{
            background: 'none',
            border: '1px solid var(--border-primary)',
            color: 'var(--text-secondary)',
            ...MONO,
            fontSize: 11,
            padding: '6px 10px',
            cursor: loading ? 'not-allowed' : 'pointer',
          }}
        >
          {t('importWizard.back')}
        </button>
        <button
          onClick={onContinue}
          disabled={loading}
          style={{
            background: 'var(--color-accent)',
            border: 'none',
            color: '#fff',
            fontFamily: 'var(--font-sans)',
            fontSize: 12,
            fontWeight: 600,
            padding: '8px 16px',
            cursor: loading ? 'not-allowed' : 'pointer',
          }}
        >
          {t('importWizard.continue')}
        </button>
      </div>
    </div>
  );
}
