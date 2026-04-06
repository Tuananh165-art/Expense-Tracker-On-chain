import type {
  AuthChallengeRequest,
  AuthChallengeResponse,
  AuthVerifyRequest,
  AuthVerifyResponse,
  CategoryDto,
  CreateCategoryInput,
  CreateExpenseInput,
  ExpenseDto,
  MonthlyReportResponse,
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
    if (!res.ok) throw new Error(`create expense failed: ${res.status}`);
    return res.json();
  }

  async monthlyReport(): Promise<MonthlyReportResponse> {
    const res = await fetch(`${this.baseUrl}/api/v1/reports/monthly`, {
      headers: this.authHeaders(),
    });
    if (!res.ok) throw new Error(`monthly report failed: ${res.status}`);
    return res.json();
  }
}
