import type {
  ApiMessage,
  Application,
  ApplicationUpdate,
  ApplyKit,
  AutoUpdateSettings,
  Client,
  ContractRow,
  KeywordSetting,
  Lead,
  NewApplication,
  NewClient,
  NewContract,
  NewLead,
  OutreachDraft,
  Profile,
  ScrapeResponse,
  Stats,
} from "./types";

const BASE = (import.meta.env.VITE_API_URL as string | undefined) ?? "";

const TOKEN_KEY = "leadgen_token";

function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

/** Notifies the app (e.g. to show the login screen) when auth expires. */
function emitUnauthorized() {
  window.dispatchEvent(new Event("leadgen:unauthorized"));
}

export const auth = {
  getToken,
  setToken(token: string) {
    localStorage.setItem(TOKEN_KEY, token);
  },
  clearToken() {
    localStorage.removeItem(TOKEN_KEY);
  },
};

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(init?.headers as Record<string, string> | undefined),
  };
  const token = getToken();
  if (token) headers["Authorization"] = `Bearer ${token}`;

  let res: Response;
  try {
    res = await fetch(`${BASE}${path}`, { ...init, headers });
  } catch {
    throw new Error("Cannot reach the backend. Is `cargo run` server running on :8080?");
  }
  if (res.status === 401 && !path.startsWith("/login") && !path.startsWith("/register")) {
    auth.clearToken();
    emitUnauthorized();
  }
  if (!res.ok) {
    let message = `${res.status} ${res.statusText}`;
    try {
      const body = await res.json();
      if (typeof body === "string" && body) message = body;
      else if (body && typeof body.message === "string") message = body.message;
    } catch {
      /* keep fallback message */
    }
    throw new Error(message);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

export const api = {
  login: (username: string, password: string) =>
    request<{ token: string; username: string }>("/login", { method: "POST", body: JSON.stringify({ username, password }) }),
  register: (username: string, password: string) =>
    request<{ token: string; username: string; message?: string }>("/register", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    }),
  logout: () => request<{ message?: string }>("/auth/logout", { method: "POST" }),
  authStatus: () => request<{ authenticated: boolean; username: string | null }>("/auth/status"),

  stats: () => request<Stats>("/stats"),

  listLeads(params?: { source?: string; status?: string; q?: string; limit?: number }) {
    const qs = new URLSearchParams();
    if (params?.source) qs.set("source", params.source);
    if (params?.status) qs.set("status", params.status);
    if (params?.q) qs.set("q", params.q);
    if (params?.limit) qs.set("limit", String(params.limit));
    const suffix = qs.size ? `?${qs.toString()}` : "";
    return request<Lead[]>(`/leads${suffix}`);
  },
  addLead: (lead: NewLead) =>
    request<ApiMessage>("/leads", { method: "POST", body: JSON.stringify(lead) }),
  deleteLead: (id: number) =>
    request<ApiMessage>(`/leads/${id}`, { method: "DELETE" }),
  updateLeadStatus: (id: number, status: string) =>
    request<ApiMessage>(`/leads/${id}/status`, {
      method: "PATCH",
      body: JSON.stringify({ status }),
    }),
  updateLeadNotes: (id: number, notes: string) =>
    request<ApiMessage>(`/leads/${id}/notes`, {
      method: "PATCH",
      body: JSON.stringify({ notes }),
    }),
  convertLead: (id: number) =>
    request<ApiMessage>(`/leads/${id}/to-client`, { method: "POST" }),
  importLeadUrl: (url: string) =>
    request<ApiMessage>("/leads/import", { method: "POST", body: JSON.stringify({ url }) }),
  rescoreLeads: () => request<{ message: string; updated: number }>("/leads/rescore", { method: "POST" }),
  leadOutreach: (id: number) => request<OutreachDraft[]>(`/leads/${id}/outreach`),

  queuedLeads: () => request<Lead[]>("/leads/queue"),
  applyKit: (id: number) => request<ApplyKit>(`/leads/${id}/apply`),
  applicationsDue: () => request<Application[]>("/applications/due"),

  getAutoUpdateSettings: () => request<AutoUpdateSettings>("/settings/auto-update"),
  saveAutoUpdateSettings: (s: {
    enabled?: boolean;
    interval_mins?: number;
    threshold?: number;
  }) => request<ApiMessage>("/settings/auto-update", {
    method: "PUT",
    body: JSON.stringify(s),
  }),

  listClients: (status?: string) =>
    request<Client[]>(`/clients${status ? `?status=${status}` : ""}`),
  addClient: (client: NewClient) =>
    request<ApiMessage>("/clients", { method: "POST", body: JSON.stringify(client) }),
  updateClient: (client: Client) =>
    request<ApiMessage>(`/clients/${client.id}`, {
      method: "PUT",
      body: JSON.stringify(client),
    }),
  deleteClient: (id: number) =>
    request<ApiMessage>(`/clients/${id}`, { method: "DELETE" }),

  listContracts: () => request<ContractRow[]>("/contracts"),
  addContract: (contract: NewContract) =>
    request<ApiMessage>("/contracts", { method: "POST", body: JSON.stringify(contract) }),
  deployContract: (id: number, result: { tx_hash: string; contract_address: string }) =>
    request<ApiMessage>(`/contracts/${id}/deploy`, {
      method: "POST",
      body: JSON.stringify(result),
    }),
  updateContractStatus: (id: number, status: string) =>
    request<ApiMessage>(`/contracts/${id}/status`, {
      method: "PATCH",
      body: JSON.stringify({ status }),
    }),

  getKeywords: () => request<KeywordSetting>("/settings/keywords"),
  saveKeywords: (keywords: string[]) =>
    request<ApiMessage>("/settings/keywords", {
      method: "PUT",
      body: JSON.stringify({ keywords }),
    }),
  getSources: () => request<KeywordSetting>("/settings/sources"),
  saveSources: (sources: string[]) =>
    request<ApiMessage>("/settings/sources", {
      method: "PUT",
      body: JSON.stringify({ keywords: sources }),
    }),

  scrape: (body: { sources?: string[]; keywords?: string[] } = {}) =>
    request<ScrapeResponse>("/scrape", { method: "POST", body: JSON.stringify(body) }),

  listApplications: () => request<Application[]>("/applications"),
  addApplication: (app: NewApplication) =>
    request<ApiMessage>("/applications", { method: "POST", body: JSON.stringify(app) }),
  updateApplication: (id: number, update: ApplicationUpdate) =>
    request<ApiMessage>(`/applications/${id}`, { method: "PATCH", body: JSON.stringify(update) }),
  deleteApplication: (id: number) =>
    request<ApiMessage>(`/applications/${id}`, { method: "DELETE" }),

  getProfile: () => request<Profile>("/profile"),
  saveProfile: (profile: Profile) =>
    request<ApiMessage>("/profile", { method: "PUT", body: JSON.stringify(profile) }),

  getLinkedinSettings: () =>
    request<{ client_id: string; client_secret_set: boolean; redirect_uri: string }>("/settings/linkedin"),
  saveLinkedinSettings: (s: { client_id?: string; client_secret?: string; redirect_uri?: string }) =>
    request<ApiMessage>("/settings/linkedin", { method: "PUT", body: JSON.stringify(s) }),
  linkedinAuthUrl: (redirectUri?: string) =>
    request<{ url: string; state: string }>(
      `/linkedin/auth-url${redirectUri ? `?redirect_uri=${encodeURIComponent(redirectUri)}` : ""}`,
    ),
  linkedinCallback: (body: { code: string; state?: string; redirect_uri?: string }) =>
    request<unknown>("/linkedin/callback", { method: "POST", body: JSON.stringify(body) }),
  linkedinStatus: () =>
    request<{ connected: boolean; configured: boolean; member_name: string; client_id: string }>(
      "/linkedin/status",
    ),
};