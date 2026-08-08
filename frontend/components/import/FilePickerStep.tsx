import { useEffect, useRef, useState } from 'react';
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
  /** Filesystem path of the selected file — Tauri v2 File objects from the webview
   * (input[type=file] or HTML5 drag-drop) never expose a real path, so both entry
   * points below resolve one via the dialog plugin / native drag-drop event instead. */
  onFileSelected: (path: string) => void;
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
  // The drag-drop listener below is registered once; it reads this ref rather than
  // the `canPickFile` closure so it always sees the latest value.
  const canPickFileRef = useRef(canPickFile);
  useEffect(() => {
    canPickFileRef.current = canPickFile;
  }, [canPickFile]);

  function acceptPath(path: string) {
    if (!path.toLowerCase().endsWith('.csv')) {
      setExtensionError(t('importWizard.file.onlyCsv'));
      return;
    }
    setExtensionError(null);
    onFileSelected(path);
  }

  async function browseForFile() {
    if (!canPickFileRef.current) return;
    const { open } = await import('@tauri-apps/plugin-dialog');
    const path = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'CSV', extensions: ['csv'] }],
    });
    if (typeof path === 'string') acceptPath(path);
  }

  // Tauri v2 intercepts native OS drag-and-drop at the webview level, so the
  // browser's HTML5 DataTransfer API never sees real files here — the drop target
  // must listen for the webview's own drag-drop event instead, which carries real
  // filesystem paths.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void (async () => {
      const { getCurrentWebview } = await import('@tauri-apps/api/webview');
      const stopListening = await getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === 'over') {
          if (canPickFileRef.current) setDragActive(true);
        } else if (event.payload.type === 'drop') {
          setDragActive(false);
          if (!canPickFileRef.current) return;
          const path = event.payload.paths[0];
          if (path) acceptPath(path);
        } else {
          setDragActive(false);
        }
      });
      if (cancelled) {
        stopListening();
      } else {
        unlisten = stopListening;
      }
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- registered once; reads canPickFileRef for freshness
  }, []);

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

      <div
        role="button"
        tabIndex={canPickFile ? 0 : -1}
        aria-label={t('importWizard.file.dropzone')}
        data-testid="file-dropzone"
        onClick={() => void browseForFile()}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            void browseForFile();
          }
        }}
        style={{
          display: 'block',
          marginBottom: 12,
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
