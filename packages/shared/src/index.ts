export type Role = "user" | "admin" | "auditor";
export type ExpenseStatus = "pending" | "approved" | "rejected";

export interface AuthChallengeRequest {
  wallet_address: string;
}

export interface AuthChallengeResponse {
  challenge_id: string;
  wallet_address: string;
  message: string;
  nonce: string;
  expires_at: string;
}

export interface AuthVerifyRequest {
  challenge_id: string;
  wallet_address: string;
  signature: string;
}

export interface AuthVerifyResponse {
  access_token: string;
  token_type: "Bearer";
  expires_in: number;
  user_id: string;
  role: Role;
}

export interface UserMeResponse {
  id: string;
  wallet_address: string;
  role: Role;
  created_at: string;
}

export interface CategoryDto {
  id: string;
  owner_user_id: string;
  name: string;
  created_at: string;
}

export interface CreateCategoryInput {
  name: string;
}

export interface CommitCategoryOnchainInput {
  tx_hash: string;
  category_name: string;
  client_ref_id?: string;
  rpc_url_override?: string;
}

export interface OnchainCommitResponse {
  ok: boolean;
  tx_hash: string;
  commitment: string;
  slot: number;
  program_id: string;
  action: string;
  target_id: string;
  audit_log_id: string;
  metadata: Record<string, unknown>;
}

export interface ExpenseDto {
  id: string;
  owner_user_id: string;
  category_id: string;
  amount_minor: number;
  currency: string;
  status: ExpenseStatus;
  tx_hash?: string | null;
  occurred_at: string;
  created_at: string;
}

export interface CreateExpenseInput {
  category_id: string;
  amount_minor: number;
  currency: string;
  occurred_at?: string;
}

export interface UpdateExpenseStatusInput {
  status: Exclude<ExpenseStatus, "pending">;
  reason?: string;
}

export interface CommitExpenseCreateOnchainInput {
  tx_hash: string;
  expense_id_onchain: number;
  category_pda: string;
  amount_minor: number;
  currency: string;
  occurred_at?: string;
  client_ref_id?: string;
  rpc_url_override?: string;
}

export interface CommitExpenseStatusOnchainInput {
  tx_hash: string;
  to_status: Exclude<ExpenseStatus, "pending">;
  reason?: string;
  client_ref_id?: string;
  rpc_url_override?: string;
}

export interface ListAuditLogsQuery {
  action?: string;
  actor_wallet?: string;
  target_id?: string;
  from?: string;
  to?: string;
  limit?: number;
}

export interface AuditLogDto {
  id: string;
  actor_wallet: string;
  action: string;
  target_id?: string | null;
  tx_hash?: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface ExpenseHistoryQuery {
  from?: string;
  to?: string;
  limit?: number;
}

export type ExpenseHistoryItem = AuditLogDto;

export interface SearchExpensesQuery {
  status?: ExpenseStatus;
  category_id?: string;
  currency?: string;
  from?: string;
  to?: string;
  q?: string;
  limit?: number;
  offset?: number;
}

export interface SearchExpensesResponse {
  items: ExpenseDto[];
  total: number;
  limit: number;
  offset: number;
  has_more: boolean;
}

export interface MonthlyReportQuery {
  month?: number;
  year?: number;
  timezone?: string;
  top_n?: number;
}

export interface ReportByCategoryItem {
  category_id: string;
  total_amount_minor: number;
}

export interface ReportByDayItem {
  day: string;
  total_amount_minor: number;
}

export interface TopSpendingItem {
  id: string;
  category_id: string;
  amount_minor: number;
  currency: string;
  status: string;
  occurred_at: string;
}

export interface MonthlyReportPeriod {
  month: number;
  year: number;
  timezone: string;
  from_utc: string;
  to_utc: string;
}

export interface MonthlyReportResponse {
  total_amount_minor: number;
  by_category: ReportByCategoryItem[];
  by_day: ReportByDayItem[];
  top_spending: TopSpendingItem[];
  period: MonthlyReportPeriod;
}
