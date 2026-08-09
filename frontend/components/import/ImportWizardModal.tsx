import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type {
  Account,
  ImportCommitResult,
  ImportContext,
  ImportPlan,
  RowAction,
} from '../../types/portfolio';
import { buildCommitRequest, needsManualMapping } from '../../lib/importWizard';
import { getErrorMessage, isTauri, tauriInvoke } from '../../lib/tauri';
import { ColumnMappingStep } from './ColumnMappingStep';
import { FilePickerStep } from './FilePickerStep';
import { PreviewStep } from './PreviewStep';
import { ResultStep } from './ResultStep';
import { fileNameFromPath, MONO } from './constants';

type Step = 'file' | 'mapping' | 'preview' | 'result';

interface Props {
  isOpen: boolean;
  onClose: () => void;
  /** Called after a successful commit so the caller can refresh portfolio state. */
  onImported: () => void;
}

const INITIAL_STATE = {
  step: 'file' as Step,
  filePath: '',
  fileName: '',
  fileSize: null as number | null,
  parsing: false,
  parseError: null as string | null,
  plan: null as ImportPlan | null,
  columnOverrides: {} as Record<string, string>,
  statusFilter: 'all' as RowAction | 'all',
  includeCash: false,
  committing: false,
  commitError: null as string | null,
  commitResult: null as ImportCommitResult | null,
};

export function ImportWizardModal({ isOpen, onClose, onImported }: Props) {
  const { t } = useTranslation();
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [accountsLoading, setAccountsLoading] = useState(false);
  const [accountsError, setAccountsError] = useState<string | null>(null);
  const [selectedAccountId, setSelectedAccountId] = useState('');

  const [step, setStep] = useState<Step>(INITIAL_STATE.step);
  const [filePath, setFilePath] = useState(INITIAL_STATE.filePath);
  const [fileName, setFileName] = useState(INITIAL_STATE.fileName);
  const [fileSize, setFileSize] = useState<number | null>(INITIAL_STATE.fileSize);
  const [parsing, setParsing] = useState(INITIAL_STATE.parsing);
  const [parseError, setParseError] = useState<string | null>(INITIAL_STATE.parseError);
  const [plan, setPlan] = useState<ImportPlan | null>(INITIAL_STATE.plan);
  const [columnOverrides, setColumnOverrides] = useState<Record<string, string>>(
    INITIAL_STATE.columnOverrides
  );
  const [statusFilter, setStatusFilter] = useState<RowAction | 'all'>(INITIAL_STATE.statusFilter);
  const [includeCash, setIncludeCash] = useState(INITIAL_STATE.includeCash);
  const [committing, setCommitting] = useState(INITIAL_STATE.committing);
  const [commitError, setCommitError] = useState<string | null>(INITIAL_STATE.commitError);
  const [commitResult, setCommitResult] = useState<ImportCommitResult | null>(
    INITIAL_STATE.commitResult
  );

  const busy = parsing || committing;

  useEffect(() => {
    if (!isOpen || !isTauri()) return;
    let cancelled = false;
    setAccountsLoading(true);
    setAccountsError(null);
    tauriInvoke<Account[]>('get_accounts')
      .then((result) => {
        if (!cancelled) setAccounts(result);
      })
      .catch((e) => {
        if (!cancelled) setAccountsError(getErrorMessage(e));
      })
      .finally(() => {
        if (!cancelled) setAccountsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [isOpen]);

  function resetWizard() {
    setStep(INITIAL_STATE.step);
    setFilePath(INITIAL_STATE.filePath);
    setFileName(INITIAL_STATE.fileName);
    setFileSize(INITIAL_STATE.fileSize);
    setParsing(INITIAL_STATE.parsing);
    setParseError(INITIAL_STATE.parseError);
    setPlan(INITIAL_STATE.plan);
    setColumnOverrides(INITIAL_STATE.columnOverrides);
    setStatusFilter(INITIAL_STATE.statusFilter);
    setIncludeCash(INITIAL_STATE.includeCash);
    setCommitting(INITIAL_STATE.committing);
    setCommitError(INITIAL_STATE.commitError);
    setCommitResult(INITIAL_STATE.commitResult);
    setSelectedAccountId('');
  }

  function handleClose() {
    if (busy) return;
    resetWizard();
    onClose();
  }

  async function runParse(
    path: string,
    overrides: Record<string, string>,
    opts: { forcePreview?: boolean } = {}
  ) {
    const account = accounts.find((a) => a.id === selectedAccountId);
    const context: ImportContext = {
      accountType: account?.accountType ?? 'other',
      accountName: account?.name ?? null,
      accountId: account?.id ?? null,
      sourceProfile: null,
      columnOverrides: overrides,
    };
    setParsing(true);
    setParseError(null);
    try {
      const result = await tauriInvoke<ImportPlan>('parse_import_file', {
        filePath: path,
        context,
      });
      setPlan(result);
      setColumnOverrides(overrides);
      setIncludeCash(result.cashRows.length > 0);
      setStatusFilter('all');
      if (opts.forcePreview || !needsManualMapping(result.columnMappings)) {
        setStep('preview');
      } else {
        setStep('mapping');
      }
    } catch (e) {
      setParseError(getErrorMessage(e));
    } finally {
      setParsing(false);
    }
  }

  function handleFileSelected(path: string) {
    setFileName(fileNameFromPath(path));
    // The dialog/drag-drop path only gives us a filesystem path, not a File
    // object, so the size shown in Step 1 is unknown until the plan comes back.
    setFileSize(null);
    setFilePath(path);
    void runParse(path, {});
  }

  function handleMappingOverrideChange(sourceHeader: string, canonicalField: string) {
    setColumnOverrides((prev) => {
      if (!canonicalField) {
        const next = { ...prev };
        delete next[sourceHeader];
        return next;
      }
      return { ...prev, [sourceHeader]: canonicalField };
    });
  }

  function handleMappingContinue() {
    void runParse(filePath, columnOverrides, { forcePreview: true });
  }

  async function handleCommit() {
    if (!plan) return;
    setCommitting(true);
    setCommitError(null);
    try {
      const request = buildCommitRequest(plan, selectedAccountId, includeCash);
      const result = await tauriInvoke<ImportCommitResult>('commit_import', { request });
      setCommitResult(result);
      setStep('result');
      onImported();
    } catch (e) {
      setCommitError(getErrorMessage(e));
      setStep('result');
    } finally {
      setCommitting(false);
    }
  }

  if (!isOpen) return null;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(0,0,0,0.65)',
        backdropFilter: 'blur(4px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1100,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
    >
      <div
        style={{
          width: '90vw',
          maxWidth: 880,
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          padding: 20,
          maxHeight: '88vh',
          overflow: 'auto',
        }}
      >
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'flex-start',
            marginBottom: 16,
          }}
        >
          <div>
            <div
              style={{
                fontFamily: 'var(--font-sans)',
                fontSize: 16,
                fontWeight: 600,
                color: 'var(--text-primary)',
              }}
            >
              {t('importWizard.title')}
            </div>
            <div style={{ ...MONO, fontSize: 11, color: 'var(--text-muted)', marginTop: 4 }}>
              {t(`importWizard.step.${step}`)}
            </div>
          </div>
          <button
            onClick={handleClose}
            disabled={busy}
            style={{
              background: 'none',
              border: '1px solid var(--border-primary)',
              color: 'var(--text-secondary)',
              ...MONO,
              fontSize: 11,
              padding: '6px 10px',
              cursor: busy ? 'not-allowed' : 'pointer',
            }}
          >
            {t('common.close')}
          </button>
        </div>

        {!isTauri() ? (
          <div style={{ ...MONO, fontSize: 12, color: 'var(--text-muted)' }}>
            {t('importWizard.desktopOnly')}
          </div>
        ) : (
          <>
            {step === 'file' ? (
              <FilePickerStep
                accounts={accounts}
                accountsLoading={accountsLoading}
                accountsError={accountsError}
                selectedAccountId={selectedAccountId}
                onSelectAccount={setSelectedAccountId}
                fileName={fileName}
                fileSize={fileSize}
                parsing={parsing}
                parseError={parseError}
                onFileSelected={handleFileSelected}
              />
            ) : null}

            {step === 'mapping' && plan ? (
              <ColumnMappingStep
                mappings={plan.columnMappings}
                overrides={columnOverrides}
                onChangeOverride={handleMappingOverrideChange}
                onBack={() => setStep('file')}
                onContinue={handleMappingContinue}
                loading={parsing}
              />
            ) : null}

            {step === 'preview' && plan ? (
              <PreviewStep
                plan={plan}
                statusFilter={statusFilter}
                onChangeFilter={setStatusFilter}
                includeCash={includeCash}
                onToggleIncludeCash={setIncludeCash}
                onBack={() => setStep('file')}
                onCommit={() => void handleCommit()}
                committing={committing}
              />
            ) : null}

            {step === 'result' ? (
              <ResultStep
                result={commitResult}
                error={commitError}
                onDone={handleClose}
                onBack={() => {
                  setCommitError(null);
                  setStep('preview');
                }}
              />
            ) : null}
          </>
        )}
      </div>
    </div>
  );
}
