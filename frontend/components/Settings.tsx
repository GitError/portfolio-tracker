import { useState, useRef } from 'react';
import { Download, Upload } from 'lucide-react';
import { isTauri, tauriInvoke } from '../lib/tauri';
import { useConfig } from '../hooks/useConfig';
import { useAutoRefresh } from '../hooks/useAutoRefresh';
import { useTheme, type ThemeMode } from '../hooks/useTheme';
import { AccountsModal } from './AccountsModal';
import { Select } from './ui/Select';

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

type BackupStatus =
  | { kind: 'idle' }
  | { kind: 'loading' }
  | { kind: 'success'; path: string }
  | { kind: 'error'; message: string };

type RestoreStatus =
  | { kind: 'idle' }
  | { kind: 'confirm'; filePath: string }
  | { kind: 'loading' }
  | { kind: 'success'; message: string }
  | { kind: 'error'; message: string };

function buildBackupFilename(): string {
  const now = new Date();
  const yyyy = now.getFullYear();
  const mm = String(now.getMonth() + 1).padStart(2, '0');
  const dd = String(now.getDate()).padStart(2, '0');
  return `portfolio-backup-${yyyy}-${mm}-${dd}.db`;
}

function ActionButton({
  onClick,
  disabled,
  icon,
  label,
  variant,
}: {
  onClick: () => void;
  disabled: boolean;
  icon: React.ReactNode;
  label: string;
  variant: 'primary' | 'warning';
}) {
  const bgColor = variant === 'primary' ? 'var(--color-accent)' : 'transparent';
  const borderColor = variant === 'primary' ? 'var(--color-accent)' : 'var(--color-warning)';
  const textColor = variant === 'primary' ? '#fff' : 'var(--color-warning)';

  return (
    <button
      onClick={onClick}
      disabled={disabled}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 6,
        padding: '7px 14px',
        fontSize: 12,
        fontWeight: 500,
        fontFamily: 'var(--font-sans)',
        background: disabled ? 'var(--bg-surface-alt)' : bgColor,
        border: `1px solid ${disabled ? 'var(--border-primary)' : borderColor}`,
        color: disabled ? 'var(--text-muted)' : textColor,
        borderRadius: 2,
        cursor: disabled ? 'not-allowed' : 'pointer',
        transition: 'opacity 150ms',
        opacity: disabled ? 0.6 : 1,
      }}
    >
      {icon}
      {label}
    </button>
  );
}

const THEME_OPTIONS: { label: string; value: ThemeMode }[] = [
  { label: 'Dark', value: 'dark' },
  { label: 'Light', value: 'light' },
  { label: 'System', value: 'system' },
];

function ThemePicker({
  theme,
  onSelect,
}: {
  theme: ThemeMode;
  onSelect: (mode: ThemeMode) => void;
}) {
  return (
    <div style={{ display: 'flex', gap: 1 }}>
      {THEME_OPTIONS.map((opt) => {
        const isActive = theme === opt.value;
        return (
          <button
            key={opt.value}
            onClick={() => onSelect(opt.value)}
            style={{
              padding: '5px 16px',
              fontSize: 12,
              fontWeight: 500,
              fontFamily: 'var(--font-sans)',
              cursor: 'pointer',
              background: isActive ? 'var(--color-accent)' : 'var(--bg-surface-alt)',
              color: isActive ? '#fff' : 'var(--text-secondary)',
              border: `1px solid ${isActive ? 'var(--color-accent)' : 'var(--border-primary)'}`,
              borderRadius: 2,
              transition: 'background 150ms, border-color 150ms, color 150ms',
            }}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}

function DataManagementSection() {
  const [backupStatus, setBackupStatus] = useState<BackupStatus>({ kind: 'idle' });
  const [restoreStatus, setRestoreStatus] = useState<RestoreStatus>({ kind: 'idle' });
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleBackup = async () => {
    setBackupStatus({ kind: 'loading' });
    const filename = buildBackupFilename();
    try {
      if (!isTauri()) {
        setBackupStatus({ kind: 'error', message: 'Backup is only available in the desktop app.' });
        return;
      }
      // The dialog plugin is not available in this build. We pass a bare
      // filename; the backend resolves it to the user's Desktop so it is
      // easy to find.
      const savedPath = await tauriInvoke<string>('backup_database', {
        destinationPath: filename,
      });
      setBackupStatus({ kind: 'success', path: savedPath });
    } catch (err) {
      setBackupStatus({ kind: 'error', message: String(err) });
    }
  };

  const handleRestoreFileSelected = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    // Reset so the same file can be re-selected after cancelling.
    e.target.value = '';
    // In Tauri v2, the File object from a file input includes a non-standard
    // `path` property with the real filesystem path.
    const filePath = (file as File & { path?: string }).path ?? file.name;
    setRestoreStatus({ kind: 'confirm', filePath });
  };

  const handleRestoreConfirm = async () => {
    if (restoreStatus.kind !== 'confirm') return;
    const { filePath } = restoreStatus;
    setRestoreStatus({ kind: 'loading' });
    try {
      if (!isTauri()) {
        setRestoreStatus({
          kind: 'error',
          message: 'Restore is only available in the desktop app.',
        });
        return;
      }
      const message = await tauriInvoke<string>('restore_database', { sourcePath: filePath });
      setRestoreStatus({ kind: 'success', message });
    } catch (err) {
      setRestoreStatus({ kind: 'error', message: String(err) });
    }
  };

  const handleRestoreCancel = () => {
    setRestoreStatus({ kind: 'idle' });
  };

  const isBackupLoading = backupStatus.kind === 'loading';
  const isRestoreLoading = restoreStatus.kind === 'loading';

  return (
    <div
      style={{
        background: 'var(--bg-surface)',
        border: '1px solid var(--border-primary)',
        borderRadius: 2,
        padding: '0 16px',
      }}
    >
      {/* Backup row */}
      <div
        style={{
          padding: '16px 0',
          borderBottom: '1px solid var(--border-subtle)',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-start',
            justifyContent: 'space-between',
            gap: 24,
          }}
        >
          <div style={{ flex: 1 }}>
            <div style={{ fontSize: 13, fontWeight: 500, color: 'var(--text-primary)' }}>
              Backup Database
            </div>
            <div style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 3 }}>
              Save a complete copy of your portfolio database to a file.
            </div>
            {backupStatus.kind === 'success' && (
              <div
                style={{
                  fontSize: 12,
                  color: 'var(--color-gain)',
                  marginTop: 6,
                  fontFamily: 'var(--font-mono)',
                  wordBreak: 'break-all',
                }}
              >
                Saved: {backupStatus.path}
              </div>
            )}
            {backupStatus.kind === 'error' && (
              <div
                style={{
                  fontSize: 12,
                  color: 'var(--color-loss)',
                  marginTop: 6,
                }}
              >
                {backupStatus.message}
              </div>
            )}
          </div>
          <div style={{ flexShrink: 0, paddingTop: 2 }}>
            <ActionButton
              onClick={handleBackup}
              disabled={isBackupLoading}
              icon={<Download size={13} />}
              label={isBackupLoading ? 'Saving…' : 'Backup Database'}
              variant="primary"
            />
          </div>
        </div>
      </div>

      {/* Restore row */}
      <div style={{ padding: '16px 0' }}>
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-start',
            justifyContent: 'space-between',
            gap: 24,
          }}
        >
          <div style={{ flex: 1 }}>
            <div style={{ fontSize: 13, fontWeight: 500, color: 'var(--text-primary)' }}>
              Restore from Backup
            </div>
            <div style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 3 }}>
              Replace all current data with a previously saved backup file.
            </div>

            {/* Persistent warning */}
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                marginTop: 8,
                padding: '7px 10px',
                background: 'rgba(251,191,36,0.07)',
                border: '1px solid rgba(251,191,36,0.25)',
                borderRadius: 2,
                fontSize: 12,
                color: 'var(--color-warning)',
              }}
            >
              This will replace all current data. The app will need to restart after restoring.
            </div>

            {/* Inline confirmation */}
            {restoreStatus.kind === 'confirm' && (
              <div
                style={{
                  marginTop: 10,
                  padding: '12px 14px',
                  background: 'rgba(255,71,87,0.07)',
                  border: '1px solid rgba(255,71,87,0.3)',
                  borderRadius: 2,
                }}
              >
                <div style={{ fontSize: 12, color: 'var(--text-primary)', marginBottom: 4 }}>
                  Restore from:
                </div>
                <div
                  style={{
                    fontSize: 11,
                    fontFamily: 'var(--font-mono)',
                    color: 'var(--text-secondary)',
                    marginBottom: 10,
                    wordBreak: 'break-all',
                  }}
                >
                  {restoreStatus.filePath}
                </div>
                <div style={{ fontSize: 12, color: 'var(--color-loss)', marginBottom: 12 }}>
                  Are you sure? This cannot be undone.
                </div>
                <div style={{ display: 'flex', gap: 8 }}>
                  <button
                    onClick={handleRestoreConfirm}
                    style={{
                      padding: '6px 14px',
                      fontSize: 12,
                      fontWeight: 500,
                      fontFamily: 'var(--font-sans)',
                      background: 'var(--color-loss)',
                      border: '1px solid var(--color-loss)',
                      color: '#fff',
                      borderRadius: 2,
                      cursor: 'pointer',
                    }}
                  >
                    Confirm Restore
                  </button>
                  <button
                    onClick={handleRestoreCancel}
                    style={{
                      padding: '6px 14px',
                      fontSize: 12,
                      fontWeight: 500,
                      fontFamily: 'var(--font-sans)',
                      background: 'transparent',
                      border: '1px solid var(--border-primary)',
                      color: 'var(--text-secondary)',
                      borderRadius: 2,
                      cursor: 'pointer',
                    }}
                  >
                    Cancel
                  </button>
                </div>
              </div>
            )}

            {restoreStatus.kind === 'success' && (
              <div
                style={{
                  fontSize: 12,
                  color: 'var(--color-gain)',
                  marginTop: 8,
                }}
              >
                {restoreStatus.message}
              </div>
            )}

            {restoreStatus.kind === 'error' && (
              <div
                style={{
                  fontSize: 12,
                  color: 'var(--color-loss)',
                  marginTop: 8,
                }}
              >
                {restoreStatus.message}
              </div>
            )}
          </div>

          <div style={{ flexShrink: 0, paddingTop: 2 }}>
            {/* Hidden native file picker */}
            <input
              ref={fileInputRef}
              type="file"
              accept=".db"
              style={{ display: 'none' }}
              onChange={handleRestoreFileSelected}
            />
            <ActionButton
              onClick={() => fileInputRef.current?.click()}
              disabled={isRestoreLoading || restoreStatus.kind === 'confirm'}
              icon={<Upload size={13} />}
              label={isRestoreLoading ? 'Restoring…' : 'Choose Backup File'}
              variant="warning"
            />
          </div>
        </div>
      </div>
    </div>
  );
}

// Noop used as onRefresh in settings — Settings does not trigger live refreshes.
async function noopRefresh(): Promise<void> {}

export function Settings() {
  const { value: baseCurrency, setValue: setBaseCurrency } = useConfig('base_currency', 'CAD');
  const { value: costBasisMethod, setValue: setCostBasisMethod } = useConfig(
    'cost_basis_method',
    'AVCO'
  );
  const { theme, setTheme } = useTheme();
  const [accountsOpen, setAccountsOpen] = useState(false);

  // Auto-refresh controls — reads/writes the same config keys as useAutoRefresh
  // in AppRoutes, so changes take effect immediately app-wide.
  const {
    intervalMinutes,
    marketHoursOnly,
    setInterval: setAutoRefreshInterval,
    setMarketHoursOnly,
  } = useAutoRefresh({ onRefresh: noopRefresh });

  // Derive the ms string for the select control
  const autoRefreshMsStr = String(intervalMinutes * 60_000);

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

      {/* General */}
      <SectionHeader title="General" />
      <div
        style={{
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          borderRadius: 2,
          padding: '0 16px',
        }}
      >
        <SettingRow
          label="Accounts"
          description="Create and manage named accounts (TFSA, RRSP, etc.) to group your holdings."
        >
          <button
            onClick={() => setAccountsOpen(true)}
            style={{
              padding: '6px 16px',
              background: 'var(--color-accent)',
              border: 'none',
              color: '#fff',
              borderRadius: 2,
              fontFamily: 'var(--font-sans)',
              fontSize: 12,
              fontWeight: 600,
              cursor: 'pointer',
            }}
          >
            Manage Accounts
          </button>
        </SettingRow>
      </div>

      <AccountsModal isOpen={accountsOpen} onClose={() => setAccountsOpen(false)} />

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
            style={{ minWidth: 160 }}
          />
        </SettingRow>
      </div>

      {/* Appearance */}
      <SectionHeader title="Appearance" />
      <div
        style={{
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          borderRadius: 2,
          padding: '0 16px',
        }}
      >
        <SettingRow
          label="Theme"
          description="Choose between dark, light, or match your system preference."
        >
          <ThemePicker theme={theme} onSelect={setTheme} />
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
          <Select
            value={autoRefreshMsStr}
            onChange={(v) => setAutoRefreshInterval(Math.round(Number(v) / 60_000))}
            options={REFRESH_OPTIONS}
            style={{ minWidth: 160 }}
          />
        </SettingRow>
        <SettingRow
          label="Market Hours Only"
          description="Only auto-refresh during NYSE trading hours (Mon–Fri 09:30–16:00 ET)."
        >
          <button
            onClick={() => setMarketHoursOnly(!marketHoursOnly)}
            aria-pressed={marketHoursOnly}
            style={{
              display: 'flex',
              alignItems: 'center',
              padding: '5px 16px',
              background: marketHoursOnly ? 'var(--color-accent)' : 'var(--bg-surface-alt)',
              border: `1px solid ${marketHoursOnly ? 'var(--color-accent)' : 'var(--border-primary)'}`,
              color: marketHoursOnly ? '#fff' : 'var(--text-secondary)',
              borderRadius: 2,
              cursor: 'pointer',
              fontSize: 12,
              fontFamily: 'var(--font-sans)',
              transition: 'background 150ms, border-color 150ms, color 150ms',
            }}
          >
            {marketHoursOnly ? 'On' : 'Off'}
          </button>
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

      {/* Data Management */}
      <SectionHeader title="Data Management" />
      <DataManagementSection />

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
