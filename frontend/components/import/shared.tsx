import type { TFunction } from 'i18next';
import type { RowAction } from '../../types/portfolio';
import { MONO, STATUS_META } from './constants';

export function StatusBadge({ action, t }: { action: RowAction; t: TFunction }) {
  const meta = STATUS_META[action];
  return (
    <span
      style={{
        ...MONO,
        fontSize: 10,
        fontWeight: 600,
        textTransform: 'uppercase',
        letterSpacing: '0.06em',
        padding: '2px 6px',
        borderRadius: 2,
        background: `${meta.color}18`,
        color: meta.color,
        border: `1px solid ${meta.color}55`,
        whiteSpace: 'nowrap',
      }}
    >
      {t(meta.i18nKey)}
    </span>
  );
}
