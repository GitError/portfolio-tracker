import { useTranslation } from 'react-i18next';
import type { ImportCommitResult } from '../../types/portfolio';
import { MONO } from './constants';

interface Props {
  result: ImportCommitResult | null;
  error: string | null;
  onDone: () => void;
  onBack: () => void;
}

function SymbolList({
  title,
  symbols,
  color,
}: {
  title: string;
  symbols: string[];
  color: string;
}) {
  if (symbols.length === 0) return null;
  return (
    <div style={{ marginBottom: 12 }}>
      <div style={{ ...MONO, fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>
        {title}
      </div>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
        {symbols.map((s) => (
          <span
            key={s}
            style={{
              ...MONO,
              fontSize: 11,
              color,
              background: `${color}18`,
              border: `1px solid ${color}55`,
              borderRadius: 2,
              padding: '2px 6px',
            }}
          >
            {s}
          </span>
        ))}
      </div>
    </div>
  );
}

export function ResultStep({ result, error, onDone, onBack }: Props) {
  const { t } = useTranslation();

  if (error) {
    return (
      <div>
        <div
          style={{
            border: '1px solid var(--color-loss)',
            color: 'var(--color-loss)',
            background: 'rgba(255,71,87,0.08)',
            padding: '10px 12px',
            fontSize: 12,
            ...MONO,
            marginBottom: 16,
          }}
        >
          {error}
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <button
            onClick={onBack}
            style={{
              background: 'none',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-secondary)',
              ...MONO,
              fontSize: 11,
              padding: '6px 10px',
              cursor: 'pointer',
            }}
          >
            {t('importWizard.back')}
          </button>
        </div>
      </div>
    );
  }

  if (!result) return null;

  return (
    <div>
      <div
        style={{
          fontFamily: 'var(--font-sans)',
          fontSize: 14,
          fontWeight: 600,
          color: 'var(--text-primary)',
          marginBottom: 4,
        }}
      >
        {t('importWizard.result.summary', { created: result.created, updated: result.updated })}
      </div>
      {result.skipped > 0 ? (
        <div style={{ ...MONO, fontSize: 12, color: 'var(--text-muted)', marginBottom: 12 }}>
          {t('importWizard.result.skipped', { count: result.skipped })}
        </div>
      ) : (
        <div style={{ marginBottom: 12 }} />
      )}

      <SymbolList
        title={t('importWizard.result.newSymbols')}
        symbols={result.newSymbols}
        color="var(--color-gain)"
      />
      <SymbolList
        title={t('importWizard.result.changedSymbols')}
        symbols={result.changedSymbols}
        color="var(--color-accent)"
      />
      <SymbolList
        title={t('importWizard.result.staleSymbols')}
        symbols={result.staleSymbols}
        color="var(--color-warning)"
      />
      <SymbolList
        title={t('importWizard.result.missingFromImport')}
        symbols={result.missingFromImport}
        color="var(--text-muted)"
      />

      {result.errors.length > 0 ? (
        <div style={{ marginBottom: 12 }}>
          <div style={{ ...MONO, fontSize: 11, color: 'var(--color-loss)', marginBottom: 4 }}>
            {t('importWizard.result.errors')}
          </div>
          <ul style={{ margin: 0, paddingLeft: 18 }}>
            {result.errors.map((e, i) => (
              <li key={i} style={{ ...MONO, fontSize: 11, color: 'var(--color-loss)' }}>
                {e}
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <button
          onClick={onDone}
          style={{
            background: 'var(--color-accent)',
            border: 'none',
            color: '#fff',
            fontFamily: 'var(--font-sans)',
            fontSize: 12,
            fontWeight: 600,
            padding: '8px 16px',
            cursor: 'pointer',
          }}
        >
          {t('importWizard.result.done')}
        </button>
      </div>
    </div>
  );
}
