import type {
  ColumnMapping,
  ImportCommitRequest,
  ImportPlan,
  NormalizedImportRow,
  RowAction,
} from '../types/portfolio';

export const ROW_ACTIONS: RowAction[] = ['create', 'update', 'skip', 'needs_fix', 'warning'];

/** Row actions that commit_import will actually write to the DB. */
const COMMITTABLE_ACTIONS: RowAction[] = ['create', 'update', 'warning'];

export function computeStatusCounts(rows: NormalizedImportRow[]): Record<RowAction, number> {
  const counts: Record<RowAction, number> = {
    create: 0,
    update: 0,
    skip: 0,
    needs_fix: 0,
    warning: 0,
  };
  for (const row of rows) {
    counts[row.action] += 1;
  }
  return counts;
}

export function filterRowsByStatus(
  rows: NormalizedImportRow[],
  status: RowAction | 'all'
): NormalizedImportRow[] {
  if (status === 'all') return rows;
  return rows.filter((row) => row.action === status);
}

/** True when at least one column mapping has no canonical field and needs a user decision. */
export function needsManualMapping(mappings: ColumnMapping[]): boolean {
  return mappings.some((mapping) => mapping.canonicalField === null);
}

export function countCommittable(rows: NormalizedImportRow[]): number {
  return rows.filter((row) => COMMITTABLE_ACTIONS.includes(row.action)).length;
}

/** Text shown under a row's status badge: errors for needs_fix, warnings otherwise. */
export function rowFixHint(row: NormalizedImportRow): string | null {
  const messages = row.action === 'needs_fix' ? row.errors : row.warnings;
  return messages.length > 0 ? messages.join('; ') : null;
}

/**
 * cashRows are always included in planRows — commit_import decides per-row whether to
 * write them based on `includeCash` and each row's asset type, so leaving them out here
 * would make the includeCash toggle a no-op.
 */
export function buildCommitRequest(
  plan: ImportPlan,
  accountId: string,
  includeCash: boolean
): ImportCommitRequest {
  return {
    planRows: [...plan.rows, ...plan.cashRows],
    accountId,
    includeCash,
  };
}
