import type {
  AuditLogDto,
  AuthChallengeRequest,
  AuthChallengeResponse,
  AuthVerifyRequest,
  AuthVerifyResponse,
  CategoryDto,
  CommitCategoryOnchainInput,
  CommitExpenseCreateOnchainInput,
  CommitExpenseStatusOnchainInput,
  CreateCategoryInput,
  CreateExpenseInput,
  ExpenseDto,
  ExpenseHistoryItem,
  ExpenseHistoryQuery,
  ListAuditLogsQuery,
  MonthlyReportQuery,
  MonthlyReportResponse,
  OnchainCommitResponse,
  SearchExpensesQuery,
  SearchExpensesResponse,
  UpdateExpenseStatusInput,
  UserMeResponse,
} from "@expense/shared";

export class ExpenseApiClient {
  constructor(
    private readonly baseUrl: string,
    private readonly getToken?: () => string | null
  ) {}

  private authHeaders(extra?: Record<string, string>): Record<string, string> {
    const token = this.getToken?.();
    return {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(extra ?? {}),
    };
  }

  async health(): Promise<{ status: string; service: string }> {
    const res = await fetch(`${this.baseUrl}/health`);
    if (!res.ok) throw new Error(`health check failed: ${res.status}`);
    return res.json();
  }

  async challenge(input: AuthChallengeRequest): Promise<AuthChallengeResponse> {
    const res = await fetch(`${this.baseUrl}/api/v1/auth/challenge`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    if (!res.ok) throw new Error(`challenge failed: ${res.status}`);
    return res.json();
  }

  async verify(input: AuthVerifyRequest): Promise<AuthVerifyResponse> {
    const res = await fetch(`${this.baseUrl}/api/v1/auth/verify`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    if (!res.ok) throw new Error(`verify failed: ${res.status}`);
    return res.json();
  }

  async me(): Promise<UserMeResponse> {
    const res = await fetch(`${this.baseUrl}/api/v1/users/me`, {
      headers: this.authHeaders(),
    });
    if (!res.ok) throw new Error(`me failed: ${res.status}`);
    return res.json();
  }

  async listCategories(): Promise<CategoryDto[]> {
    const res = await fetch(`${this.baseUrl}/api/v1/categories`, {
      headers: this.authHeaders(),
    });
    if (!res.ok) throw new Error(`list categories failed: ${res.status}`);
    return res.json();
  }

  async createCategory(input: CreateCategoryInput): Promise<CategoryDto> {
    const res = await fetch(`${this.baseUrl}/api/v1/categories`, {
      method: "POST",
      headers: this.authHeaders(),
      body: JSON.stringify(input),
    });
    if (!res.ok) throw new Error(`create category failed: ${res.status}`);
    return res.json();
  }

  async commitCategoryOnchain(input: CommitCategoryOnchainInput): Promise<OnchainCommitResponse> {
    const res = await fetch(`${this.baseUrl}/api/v1/onchain/categories/commit`, {
      method: "POST",
      headers: this.authHeaders(),
      body: JSON.stringify(input),
    });
    if (!res.ok) throw new Error(`commit category onchain failed: ${res.status}`);
    return res.json();
  }

  async listExpenses(): Promise<ExpenseDto[]> {
    const res = await fetch(`${this.baseUrl}/api/v1/expenses`, {
      headers: this.authHeaders(),
    });
    if (!res.ok) throw new Error(`list expenses failed: ${res.status}`);
    return res.json();
  }

  async createExpense(input: CreateExpenseInput, idempotencyKey: string): Promise<ExpenseDto> {
    const res = await fetch(`${this.baseUrl}/api/v1/expenses`, {
      method: "POST",
      headers: this.authHeaders({ "x-idempotency-key": idempotencyKey }),
      body: JSON.stringify(input),
    });
    if (!res.ok) {
      let detail = "";
      try {
        const body = await res.json();
        if (body?.message && typeof body.message === "string") {
          detail = ` - ${body.message}`;
        }
      } catch {
        // ignore non-json error body
      }
      throw new Error(`create expense failed: ${res.status}${detail}`);
    }
    return res.json();
  }

  async updateExpenseStatus(
    expenseId: string,
    input: UpdateExpenseStatusInput,
    idempotencyKey: string
  ): Promise<ExpenseDto> {
    const res = await fetch(`${this.baseUrl}/api/v1/expenses/${expenseId}/status`, {
      method: "POST",
      headers: this.authHeaders({ "x-idempotency-key": idempotencyKey }),
      body: JSON.stringify(input),
    });
    if (!res.ok) throw new Error(`update expense status failed: ${res.status}`);
    return res.json();
  }

  async commitExpenseCreateOnchain(input: CommitExpenseCreateOnchainInput): Promise<OnchainCommitResponse> {
    const res = await fetch(`${this.baseUrl}/api/v1/onchain/expenses/commit-create`, {
      method: "POST",
      headers: this.authHeaders(),
      body: JSON.stringify(input),
    });
    if (!res.ok) {
      let detail = "";
      try {
        const body = await res.json();
        if (body?.message && typeof body.message === "string") {
          detail = ` - ${body.message}`;
        }
      } catch {
        // ignore non-json error body
      }
      throw new Error(`commit expense create onchain failed: ${res.status}${detail}`);
    }
    return res.json();
  }

  async commitExpenseStatusOnchain(
    expenseId: string,
    input: CommitExpenseStatusOnchainInput
  ): Promise<OnchainCommitResponse> {
    const res = await fetch(`${this.baseUrl}/api/v1/onchain/expenses/${expenseId}/commit-status`, {
      method: "POST",
      headers: this.authHeaders(),
      body: JSON.stringify(input),
    });
    if (!res.ok) {
      let detail = "";
      try {
        const body = await res.json();
        if (body?.message && typeof body.message === "string") {
          detail = ` - ${body.message}`;
        }
      } catch {
        // ignore non-json error body
      }
      throw new Error(`commit expense status onchain failed: ${res.status}${detail}`);
    }
    return res.json();
  }

  async listAuditLogs(query: ListAuditLogsQuery = {}): Promise<AuditLogDto[]> {
    const params = new URLSearchParams();
    if (query.action) params.set("action", query.action);
    if (query.actor_wallet) params.set("actor_wallet", query.actor_wallet);
    if (query.target_id) params.set("target_id", query.target_id);
    if (query.from) params.set("from", query.from);
    if (query.to) params.set("to", query.to);
    if (query.limit !== undefined) params.set("limit", String(query.limit));

    const suffix = params.toString() ? `?${params.toString()}` : "";
    const res = await fetch(`${this.baseUrl}/api/v1/audit/logs${suffix}`, {
      headers: this.authHeaders(),
    });
    if (!res.ok) throw new Error(`list audit logs failed: ${res.status}`);
    return res.json();
  }

  async listExpenseHistory(expenseId: string, query: ExpenseHistoryQuery = {}): Promise<ExpenseHistoryItem[]> {
    const params = new URLSearchParams();
    if (query.from) params.set("from", query.from);
    if (query.to) params.set("to", query.to);
    if (query.limit !== undefined) params.set("limit", String(query.limit));

    const suffix = params.toString() ? `?${params.toString()}` : "";
    const res = await fetch(`${this.baseUrl}/api/v1/expenses/${expenseId}/history${suffix}`, {
      headers: this.authHeaders(),
    });
    if (!res.ok) throw new Error(`expense history failed: ${res.status}`);
    return res.json();
  }

  async searchExpenses(query: SearchExpensesQuery = {}): Promise<SearchExpensesResponse> {
    const params = new URLSearchParams();
    if (query.status) params.set("status", query.status);
    if (query.category_id) params.set("category_id", query.category_id);
    if (query.currency) params.set("currency", query.currency);
    if (query.from) params.set("from", query.from);
    if (query.to) params.set("to", query.to);
    if (query.q) params.set("q", query.q);
    if (query.limit !== undefined) params.set("limit", String(query.limit));
    if (query.offset !== undefined) params.set("offset", String(query.offset));

    const suffix = params.toString() ? `?${params.toString()}` : "";
    const res = await fetch(`${this.baseUrl}/api/v1/expenses/search${suffix}`, {
      headers: this.authHeaders(),
    });
    if (!res.ok) throw new Error(`search expenses failed: ${res.status}`);
    return res.json();
  }

  async monthlyReport(query: MonthlyReportQuery = {}): Promise<MonthlyReportResponse> {
    const params = new URLSearchParams();
    if (query.month !== undefined) params.set("month", String(query.month));
    if (query.year !== undefined) params.set("year", String(query.year));
    if (query.timezone) params.set("timezone", query.timezone);
    if (query.top_n !== undefined) params.set("top_n", String(query.top_n));

    const suffix = params.toString() ? `?${params.toString()}` : "";
    const res = await fetch(`${this.baseUrl}/api/v1/reports/monthly${suffix}`, {
      headers: this.authHeaders(),
    });
    if (!res.ok) throw new Error(`monthly report failed: ${res.status}`);
    return res.json();
  }
}
