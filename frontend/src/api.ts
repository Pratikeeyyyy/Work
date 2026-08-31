import type {
  ApiMessage,
  Client,
  ContractRow,
  KeywordSetting,
  Lead,
  NewClient,
  NewContract,
  NewLead,
  ScrapeResponse,
  Stats,
} from "./types";

const BASE = (import.meta.env.VITE_API_URL as string | undefined) ?? "";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  let res: Response;
  try {
    res = await fetch(`${BASE}${path}`, {
      headers: { "Content-Type": "application/json" },
      ...init,
    });
  } catch {
    throw new Error("Cannot reach the backend. Is `cargo run` server running on :8080?");
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
};