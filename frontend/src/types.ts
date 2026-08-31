export const LEAD_STATUSES = [
  "new",
  "shortlisted",
  "applied",
  "responded",
  "won",
  "lost",
  "archived",
] as const;

export const CLIENT_STATUSES = ["active", "inactive", "blacklisted"] as const;

export const SOURCES = ["upwork", "freelancer", "fiverr", "indeed", "linkedin", "facebook", "manual"] as const;

export const APPLICATION_STATUSES = [
  "saved",
  "applied",
  "replied",
  "interviewed",
  "offered",
  "hired",
  "rejected",
  "closed",
] as const;

export interface Lead {
  id: number;
  source: string;
  title: string;
  description: string;
  url: string;
  budget: string | null;
  budget_min: number | null;
  budget_max: number | null;
  currency: string | null;
  location: string | null;
  technologies: string | null;
  client_name: string | null;
  posted_date: string | null;
  status: string;
  score: number;
  notes: string | null;
  created_at: string;
}

export interface NewLead {
  source: string;
  title: string;
  description: string;
  url: string;
  budget: string | null;
  budget_min: number | null;
  budget_max: number | null;
  currency: string | null;
  location: string | null;
  technologies: string | null;
  client_name: string | null;
  posted_date: string | null;
}

export interface Client {
  id: number;
  lead_id: number | null;
  name: string;
  email: string | null;
  company: string | null;
  country: string | null;
  website: string | null;
  whatsapp: string | null;
  source: string | null;
  linkedin: string | null;
  past_work: string | null;
  preferences: string | null;
  status: string;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface NewClient {
  lead_id: number | null;
  name: string;
  email: string | null;
  company: string | null;
  country: string | null;
  website: string | null;
  whatsapp: string | null;
  source: string | null;
  linkedin: string | null;
  past_work: string | null;
  preferences: string | null;
}

export interface ContractRow {
  id: number;
  client_id: number;
  client_address: string | null;
  freelancer_address: string | null;
  contract_address: string | null;
  title: string;
  amount_wei: string | null;
  currency: string;
  status: string;
  tx_hash: string | null;
  deployed_at: string | null;
  notes: string | null;
  created_at: string;
}

export interface NewContract {
  client_id: number;
  client_address: string | null;
  freelancer_address: string | null;
  contract_address: string | null;
  title: string;
  amount_wei: string | null;
  currency: string;
  notes: string | null;
}

export interface Stats {
  total_leads: number;
  new_leads: number;
  applied_leads: number;
  won_leads: number;
  total_clients: number;
  active_clients: number;
  total_contracts: number;
  total_applications: number;
  interviewed: number;
  hired: number;
  by_source: { source: string; count: number }[];
  top_technologies: { tech: string; count: number }[];
}

export interface ScrapeResponse {
  inserted: number;
  total_found: number;
  errors: string[];
}

export interface ApiMessage {
  message: string;
}

export type KeywordSetting = { keywords: string[] };

export interface Application {
  id: number;
  lead_id: number;
  client_id: number | null;
  status: string;
  applied_at: string | null;
  replied_at: string | null;
  interviewed_at: string | null;
  offered_at: string | null;
  hired_at: string | null;
  company: string | null;
  contact: string | null;
  next_scheduled: string | null;
  follow_up_count: number;
  last_follow_up: string | null;
  notes: string | null;
  created_at: string;
  lead_title: string | null;
  lead_url: string | null;
  lead_source: string | null;
}

export interface NewApplication {
  lead_id: number;
  client_id?: number | null;
  company?: string | null;
  contact?: string | null;
  notes?: string | null;
}

export interface ApplicationUpdate {
  status?: string;
  applied_at?: string | null;
  replied_at?: string | null;
  interviewed_at?: string | null;
  offered_at?: string | null;
  hired_at?: string | null;
  company?: string | null;
  contact?: string | null;
  next_scheduled?: string | null;
  notes?: string | null;
  follow_up: boolean;
}

export interface Profile {
  name: string | null;
  title: string | null;
  email: string | null;
  location: string | null;
  rate: string | null;
  skills: string[];
  experience: string | null;
  availability: string | null;
  bio: string | null;
  portfolio: string | null;
  linkedin: string | null;
  github: string | null;
}

export interface OutreachDraft {
  medium: string;
  subject: string | null;
  body: string;
}