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

export interface ReportByCategoryItem {
  category_id: string;
  total_amount_minor: number;
}

export interface MonthlyReportResponse {
  total_amount_minor: number;
  by_category: ReportByCategoryItem[];
}
