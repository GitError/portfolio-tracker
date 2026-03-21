import { useState, useEffect } from 'react';
import { tauriInvoke } from '../lib/tauri';
import { Pencil, Trash2, X, Plus } from 'lucide-react';
import { ACCOUNT_TYPE_CONFIG } from '../lib/constants';
import type {
  Account,
  CreateAccountRequest,
  HoldingWithPrice,
  PortfolioSnapshot,
} from '../types/portfolio';
import { Select } from './ui/Select';

interface AccountsModalProps {
  isOpen: boolean;
  onClose: () => void;
  portfolio?: PortfolioSnapshot | null;
}

const VALID_ACCOUNT_TYPES = ['tfsa', 'rrsp', 'fhsa', 'taxable', 'crypto', 'other'] as const;
type ValidAccountType = (typeof VALID_ACCOUNT_TYPES)[number];

function accountTypeLabel(type: string): string {
  return ACCOUNT_TYPE_CONFIG[type]?.label ?? type.toUpperCase();
}

function accountTypeColor(type: string): string {
  return ACCOUNT_TYPE_CONFIG[type]?.color ?? 'var(--text-muted)';
}

function AccountTypeBadge({ type }: { type: string }) {
  return (
    <span
      style={{
        fontSize: 10,
        fontFamily: 'var(--font-mono)',
        fontWeight: 600,
        textTransform: 'uppercase',
        letterSpacing: '0.08em',
        padding: '2px 6px',
        borderRadius: 2,
        background: `${accountTypeColor(type)}22`,
        color: accountTypeColor(type),
        border: `1px solid ${accountTypeColor(type)}55`,
      }}
    >
      {accountTypeLabel(type)}
    </span>
  );
}

function holdingAccountNames(portfolio: PortfolioSnapshot | null | undefined): Set<string> {
  if (!portfolio) return new Set();
  return new Set(portfolio.holdings.map((h: HoldingWithPrice) => h.account));
}

const EMPTY_FORM: CreateAccountRequest = {
  name: '',
  accountType: 'taxable',
  institution: '',
};

export function AccountsModal({ isOpen, onClose, portfolio }: AccountsModalProps) {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [form, setForm] = useState<CreateAccountRequest>(EMPTY_FORM);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const referencedNames = holdingAccountNames(portfolio);

  const accountTypeOptions = VALID_ACCOUNT_TYPES.map((t) => ({
    value: t,
    label: accountTypeLabel(t),
  }));

  async function loadAccounts() {
    setLoading(true);
    setError(null);
    try {
      const result = await tauriInvoke<Account[]>('get_accounts');
      setAccounts(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (isOpen) {
      void loadAccounts();
    }
  }, [isOpen]);

  function startEdit(account: Account) {
    setEditingId(account.id);
    setForm({
      name: account.name,
      accountType: account.accountType,
      institution: account.institution ?? '',
    });
  }

  function cancelEdit() {
    setEditingId(null);
    setForm(EMPTY_FORM);
  }

  async function handleSave() {
    const name = form.name.trim();
    if (!name) {
      setError('Account name is required.');
      return;
    }
    if (!VALID_ACCOUNT_TYPES.includes(form.accountType as ValidAccountType)) {
      setError('Invalid account type.');
      return;
    }

    setSaving(true);
    setError(null);
    try {
      const payload: CreateAccountRequest = {
        name,
        accountType: form.accountType,
        institution: form.institution?.trim() || undefined,
      };
      if (editingId) {
        await tauriInvoke<Account>('update_account', {
          id: editingId,
          account: payload,
        });
      } else {
        await tauriInvoke<Account>('add_account', { account: payload });
      }
      await loadAccounts();
      setForm(EMPTY_FORM);
      setEditingId(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete(id: string) {
    setDeletingId(id);
    setError(null);
    try {
      await tauriInvoke('delete_account', { id });
      await loadAccounts();
    } catch (e) {
      setError(String(e));
    } finally {
      setDeletingId(null);
    }
  }

  if (!isOpen) return null;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(0,0,0,0.6)',
        backdropFilter: 'blur(4px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        overflowY: 'auto',
        padding: 24,
        zIndex: 1000,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        style={{
          background: 'var(--bg-surface)',
          border: '1px solid var(--border-primary)',
          borderRadius: 2,
          width: '100%',
          maxWidth: 720,
          // Let the overlay scroll if content exceeds viewport height.
          maxHeight: 'none',
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        {/* Header */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '16px 20px',
            borderBottom: '1px solid var(--border-primary)',
          }}
        >
          <span
            style={{
              fontFamily: 'var(--font-sans)',
              fontWeight: 600,
              fontSize: 15,
              color: 'var(--text-primary)',
            }}
          >
            Manage Accounts
          </span>
          <button
            onClick={onClose}
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--text-secondary)',
              cursor: 'pointer',
              padding: 4,
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <X size={16} />
          </button>
        </div>

        {/* Content */}
        <div style={{ padding: '16px 20px' }}>
          {error && (
            <div
              style={{
                background: 'rgba(255,71,87,0.08)',
                border: '1px solid var(--color-loss)',
                color: 'var(--color-loss)',
                fontFamily: 'var(--font-mono)',
                fontSize: 12,
                padding: '8px 12px',
                marginBottom: 12,
                borderRadius: 2,
              }}
            >
              {error}
            </div>
          )}

          {/* Account list */}
          {loading ? (
            <div
              style={{
                color: 'var(--text-muted)',
                fontFamily: 'var(--font-mono)',
                fontSize: 12,
                textAlign: 'center',
                padding: '20px 0',
              }}
            >
              Loading…
            </div>
          ) : accounts.length === 0 ? (
            <div
              style={{
                color: 'var(--text-muted)',
                fontFamily: 'var(--font-mono)',
                fontSize: 12,
                textAlign: 'center',
                padding: '16px 0',
              }}
            >
              No accounts yet. Add one below.
            </div>
          ) : (
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                gap: 1,
                marginBottom: 20,
                border: '1px solid var(--border-primary)',
              }}
            >
              {accounts.map((acct) => {
                // Holdings reference account *types* (e.g. "taxable"), not the user-visible account name.
                const hasHoldings = referencedNames.has(acct.accountType);
                const isDeleting = deletingId === acct.id;
                const isEditing = editingId === acct.id;
                return (
                  <div
                    key={acct.id}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 10,
                      padding: '10px 12px',
                      background: isEditing ? 'var(--bg-surface-hover)' : 'var(--bg-surface-alt)',
                      borderBottom: '1px solid var(--border-subtle)',
                    }}
                  >
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          gap: 8,
                          marginBottom: acct.institution ? 3 : 0,
                        }}
                      >
                        <span
                          style={{
                            fontFamily: 'var(--font-sans)',
                            fontSize: 13,
                            fontWeight: 500,
                            color: 'var(--text-primary)',
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                          }}
                        >
                          {acct.name}
                        </span>
                        <AccountTypeBadge type={acct.accountType} />
                      </div>
                      {acct.institution && (
                        <div
                          style={{
                            fontFamily: 'var(--font-mono)',
                            fontSize: 11,
                            color: 'var(--text-muted)',
                          }}
                        >
                          {acct.institution}
                        </div>
                      )}
                    </div>
                    <button
                      onClick={() => startEdit(acct)}
                      style={{
                        background: 'transparent',
                        border: '1px solid var(--border-primary)',
                        color: 'var(--text-secondary)',
                        cursor: 'pointer',
                        padding: '4px 8px',
                        borderRadius: 2,
                        display: 'flex',
                        alignItems: 'center',
                        gap: 4,
                        fontSize: 11,
                        fontFamily: 'var(--font-mono)',
                      }}
                    >
                      <Pencil size={11} />
                      Edit
                    </button>
                    <div
                      title={
                        hasHoldings ? 'Cannot delete: holdings reference this account' : undefined
                      }
                    >
                      <button
                        onClick={() => void handleDelete(acct.id)}
                        disabled={hasHoldings || isDeleting}
                        style={{
                          background: 'transparent',
                          border: `1px solid ${hasHoldings ? 'var(--border-primary)' : 'var(--color-loss)'}`,
                          color: hasHoldings ? 'var(--text-muted)' : 'var(--color-loss)',
                          cursor: hasHoldings ? 'not-allowed' : 'pointer',
                          padding: '4px 8px',
                          borderRadius: 2,
                          display: 'flex',
                          alignItems: 'center',
                          gap: 4,
                          fontSize: 11,
                          fontFamily: 'var(--font-mono)',
                          opacity: hasHoldings ? 0.4 : 1,
                        }}
                      >
                        <Trash2 size={11} />
                        {isDeleting ? '…' : 'Delete'}
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {/* Add / Edit Form */}
          <div
            style={{
              background: 'var(--bg-surface-alt)',
              border: '1px solid var(--border-primary)',
              padding: '14px 16px',
              borderRadius: 2,
            }}
          >
            <div
              style={{
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                textTransform: 'uppercase',
                letterSpacing: '0.08em',
                color: 'var(--text-muted)',
                marginBottom: 12,
              }}
            >
              {editingId ? 'Edit Account' : 'Add Account'}
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              <input
                type="text"
                placeholder="Account name (e.g. My TFSA)"
                value={form.name}
                onChange={(e) => setForm((prev) => ({ ...prev, name: e.target.value }))}
                style={{
                  background: 'var(--bg-surface)',
                  border: '1px solid var(--border-primary)',
                  color: 'var(--text-primary)',
                  padding: '7px 10px',
                  fontSize: 12,
                  fontFamily: 'var(--font-sans)',
                  borderRadius: 2,
                  outline: 'none',
                  width: '100%',
                  boxSizing: 'border-box',
                }}
              />
              <Select
                value={form.accountType}
                onChange={(value) =>
                  setForm((prev) => ({ ...prev, accountType: value as ValidAccountType }))
                }
                options={accountTypeOptions}
              />
              <input
                type="text"
                placeholder="Institution (optional, e.g. Questrade)"
                value={form.institution ?? ''}
                onChange={(e) => setForm((prev) => ({ ...prev, institution: e.target.value }))}
                style={{
                  background: 'var(--bg-surface)',
                  border: '1px solid var(--border-primary)',
                  color: 'var(--text-primary)',
                  padding: '7px 10px',
                  fontSize: 12,
                  fontFamily: 'var(--font-sans)',
                  borderRadius: 2,
                  outline: 'none',
                  width: '100%',
                  boxSizing: 'border-box',
                }}
              />
              <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                {editingId && (
                  <button
                    onClick={cancelEdit}
                    disabled={saving}
                    style={{
                      padding: '7px 16px',
                      background: 'transparent',
                      border: '1px solid var(--border-primary)',
                      color: 'var(--text-secondary)',
                      borderRadius: 2,
                      fontFamily: 'var(--font-sans)',
                      fontSize: 12,
                      cursor: 'pointer',
                    }}
                  >
                    Cancel
                  </button>
                )}
                <button
                  onClick={() => void handleSave()}
                  disabled={saving || !form.name.trim()}
                  style={{
                    padding: '7px 16px',
                    background:
                      saving || !form.name.trim() ? 'var(--bg-surface)' : 'var(--color-accent)',
                    border:
                      saving || !form.name.trim() ? '1px solid var(--border-primary)' : 'none',
                    color: saving || !form.name.trim() ? 'var(--text-muted)' : '#fff',
                    borderRadius: 2,
                    fontFamily: 'var(--font-sans)',
                    fontSize: 12,
                    fontWeight: 600,
                    cursor: saving || !form.name.trim() ? 'not-allowed' : 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                  }}
                >
                  <Plus size={13} />
                  {saving ? 'Saving…' : editingId ? 'Update Account' : 'Add Account'}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
