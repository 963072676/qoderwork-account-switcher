export interface Account {
  id: string;
  phone: string;
  label: string;
  user_id?: string;
  saved: boolean;
}

export interface AccountWithStatus extends Account {
  is_current: boolean;
}

export interface ProgressEvent {
  step: string;
  current: number;
  total: number;
}

export interface QuotaDetail {
  total?: number;
  used?: number;
  remaining?: number;
  percentage?: number;
  unit?: string;
}

export interface OrgQuota {
  used?: number;
  cap?: number;
  remaining?: number;
  percentage?: number;
  available?: boolean;
  unit?: string;
}

export interface QuotaInfo {
  userId?: string;
  userType?: string;
  totalUsagePercentage?: number;
  isQuotaExceeded?: boolean;
  userQuota?: QuotaDetail;
  addOnQuota?: QuotaDetail;
  orgResourcePackage?: OrgQuota;
  email?: string;
  userName?: string;
}

export interface QuotaSummary {
  dailyFree?: number;
  otherCredits?: number;
  checkedIn?: boolean;
  subDaysRemaining?: number;
  userType?: string;
  exceeded?: boolean;
}

export type QuotaMap = Record<string, QuotaSummary | null>;

export interface AllQuotasResult {
  quotas: QuotaMap;
  errors: Record<string, string>;
}

export interface ClaimAllResult {
  results: Record<string, string>;
  errors: Record<string, string>;
}
