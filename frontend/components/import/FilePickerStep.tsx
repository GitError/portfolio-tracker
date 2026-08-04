import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Account } from '../../types/portfolio';
import { ACCOUNT_TYPE_CONFIG } from '../../lib/constants';
import { Select, type SelectOption } from '../ui/Select';
import { Spinner } from '../ui/Spinner';
import { MONO, formatFileSize } from './constants';

interface Props {
  accounts: Account[];
  accountsLoading: boolean;
  accountsError: string | null;
  selectedAccountId: string;
  onSelectAccount: (id: string) => void;
  fileName: string;
  fileSize: number | null;
  parsing: boolean;
  parseError: string | null;
  onFileSelected: (file: File) => void;
}

function accountTypeLabel(type: string): string {
  return ACCOUNT_TYPE_CONFIG[type]?.label ?? type.toUpperCase();
}

export function FilePickerStep({
  accounts,
  accountsLoading,
  accountsError,
  selectedAccountId,
  onSelectAccount,
  fileName,
  fileSize,
  parsing,
  parseError,
  onFileSelected,
}: Props) {
  const { t } = useTranslation();
  const [dragActive, setDragActive] = useState(false);
  const [extensionError, setExtensionError] = useState<string | null>(null);

  const accountOptions: SelectOption[] = accounts.map((a) => ({
    value: a.id,
    label: `${a.name} (${accountTypeLabel(a.accountType)})`,
  }));

  const canPickFile = !!selectedAccountId && !parsing;

  function acceptFile(file: File) {
    if (!file.name.toLowerCase().endsWith('.csv')) {
      setExtensionError(t('importWizard.file.onlyCsv'));
      return;
    }
    setExtensionError(null);
    onFileSelected(file);
  }

  return (
    <div>
      <div style={{ marginBottom: 14 }}>
        <label
          style={{
            ...MONO,
            fontSize: 11,
            color: 'var(--text-muted)',
            textTransform: 'uppercase',
            letterSpacing: '0.06em',
            display: 'block',
            marginBottom: 6,
          }}
        >
          {t('importWizard.file.accountLabel')}
        </label>
        {accountsLoading ? (
          <Spinner size="sm" />
        ) : accountsError ? (
          <div style={{ ...MONO, fontSize: 12, color: 'var(--color-loss)' }}>{accountsError}</div>
        ) : accounts.length === 0 ? (
          <div style={{ ...MONO, fontSize: 12, color: 'var(--text-secondary)' }}>
            {t('importWizard.file.noAccounts')}
          </div>
        ) : (
          <Select value={selectedAccountId} onChange={onSelectAccount} options={accountOptions} />
        )}
      </div>

      <label style={{ display: 'block', marginBottom: 12 }}>
        <div
          onDragOver={(e) => {
            e.preventDefault();
            if (canPickFile) setDragActive(true);
          }}
          onDragLeave={() => setDragActive(false)}
          onDrop={(e) => {
            e.preventDefault();
            setDragActive(false);
            if (!canPickFile) return;
            const file = e.dataTransfer.files?.[0];
            if (file) acceptFile(file);
          }}
          style={{
            border: `1px dashed ${dragActive ? 'var(--color-accent)' : 'var(--border-primary)'}`,
            background: dragActive ? 'var(--bg-surface-hover)' : 'var(--bg-primary)',
            padding: '28px 20px',
            color: canPickFile ? 'var(--text-secondary)' : 'var(--text-muted)',
            ...MONO,
            fontSize: 12,
            textAlign: 'center',
            cursor: canPickFile ? 'pointer' : 'not-allowed',
            opacity: canPickFile ? 1 : 0.6,
          }}
        >
          <div>{fileName || t('importWizard.file.dropzone')}</div>
          {fileName && fileSize != null ? (
            <div style={{ marginTop: 4, fontSize: 11, color: 'var(--text-muted)' }}>
              {formatFileSize(fileSize)}
            </div>
          ) : null}
          <div style={{ marginTop: 6, fontSize: 11, color: 'var(--text-muted)' }}>
            {t('importWizard.file.hint', { maxRows: 500 })}
          </div>
        </div>
        <input
          type="file"
          accept=".csv,text/csv"
          disabled={!canPickFile}
          style={{ display: 'none' }}
          onChange={(e) => {
            const file = e.target.files?.[0];
            e.target.value = '';
            if (file) acceptFile(file);
          }}
        />
      </label>

      {extensionError ? (
        <div style={{ ...MONO, fontSize: 12, color: 'var(--color-loss)', marginBottom: 8 }}>
          {extensionError}
        </div>
      ) : null}

      {parsing ? (
        <div
          style={{
            ...MONO,
            fontSize: 12,
            color: 'var(--text-muted)',
            display: 'flex',
            alignItems: 'center',
            gap: 8,
          }}
        >
          <Spinner size="sm" />
          {t('importWizard.file.parsing')}
        </div>
      ) : null}

      {parseError ? (
        <div
          style={{
            marginTop: 8,
            border: '1px solid var(--color-loss)',
            color: 'var(--color-loss)',
            background: 'rgba(255,71,87,0.08)',
            padding: '10px 12px',
            fontSize: 12,
            ...MONO,
          }}
        >
          {parseError}
        </div>
      ) : null}
    </div>
  );
}
