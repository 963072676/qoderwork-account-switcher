import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AccountWithStatus, ProgressEvent, QuotaMap, AllQuotasResult, ClaimAllResult } from "../types";

function parseError(e: unknown): string {
  const raw = String(e);
  try {
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed.message === "string") {
      return parsed.message;
    }
  } catch {
    // not JSON
  }
  return raw;
}

/** Fetch quota/credit usage for the currently active account. */
export async function getQuotaUsage() {
  try {
    return await invoke("get_quota_usage");
  } catch {
    return null;
  }
}

interface UseAccountsReturn {
  accounts: AccountWithStatus[];
  currentUserId: string | null;
  loading: boolean;
  progress: ProgressEvent | null;
  error: string | null;
  quotas: QuotaMap;
  quotaErrors: Record<string, string>;
  addAccount: (phone: string, label: string, userId?: string) => Promise<void>;
  deleteAccount: (id: string) => Promise<void>;
  switchAccount: (id: string) => Promise<void>;
  saveAccount: () => Promise<void>;
  detectCurrent: () => Promise<void>;
  refreshQuotas: () => Promise<void>;
  claimCheckinAll: () => Promise<ClaimAllResult>;
  setError: (msg: string | null) => void;
  clearError: () => void;
  clearProgress: () => void;
}

export function useAccounts(): UseAccountsReturn {
  const [accounts, setAccounts] = useState<AccountWithStatus[]>([]);
  const [currentUserId, setCurrentUserId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [quotas, setQuotas] = useState<QuotaMap>({});
  const [quotaErrors, setQuotaErrors] = useState<Record<string, string>>({});

  const fetchAccounts = useCallback(async () => {
    try {
      const result = await invoke<{
        accounts: AccountWithStatus[];
        current_user_id: string | null;
      }>("list_accounts");
      setAccounts(result.accounts);
      setCurrentUserId(result.current_user_id);
      setError(null);
    } catch (e) {
      setError(parseError(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchAccounts();
  }, [fetchAccounts]);

  const refreshQuotas = useCallback(async () => {
    try {
      const result = await invoke<AllQuotasResult>("refresh_all_quotas");
      console.log("[quota] refresh_all_quotas result:", JSON.stringify(result, null, 2));
      setQuotas(result.quotas);
      setQuotaErrors(result.errors);
      // Surface a summary error if any account failed
      const errCount = Object.keys(result.errors).length;
      if (errCount > 0) {
        const msgs = Object.values(result.errors);
        console.warn(`额度获取失败 (${errCount} 个账号):`, msgs);
      }
    } catch (e) {
      console.warn("refresh_all_quotas failed:", e);
    }
  }, []);

  // Fetch quotas in background after accounts are loaded
  useEffect(() => {
    if (!loading && accounts.length > 0) {
      refreshQuotas();
    }
  }, [loading, accounts.length, refreshQuotas]);

  useEffect(() => {
    const unlisten = listen<ProgressEvent>("switch-progress", (event) => {
      setProgress(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const addAccount = useCallback(
    async (phone: string, label: string, userId?: string) => {
      try {
        setError(null);
        await invoke("add_account", { phone, label, userId });
        await fetchAccounts();
      } catch (e) {
        setError(parseError(e));
        throw e;
      }
    },
    [fetchAccounts],
  );

  const deleteAccount = useCallback(
    async (id: string) => {
      try {
        setError(null);
        await invoke("delete_account", { id });
        await fetchAccounts();
      } catch (e) {
        setError(parseError(e));
        throw e;
      }
    },
    [fetchAccounts],
  );

  const switchAccount = useCallback(
    async (id: string) => {
      try {
        setError(null);
        setProgress({ step: "准备切换...", current: 0, total: 4 });
        await invoke("switch_account", { id });
        setProgress(null);
        // Wait for Electron app to start and write .status.json
        await new Promise((r) => setTimeout(r, 3000));
        await fetchAccounts();
        // Retry once if current account hasn't updated yet
        setTimeout(async () => {
          await fetchAccounts();
          await refreshQuotas();
        }, 3000);
      } catch (e) {
        setProgress(null);
        setError(parseError(e));
        throw e;
      }
    },
    [fetchAccounts],
  );

  const saveAccount = useCallback(async () => {
    try {
      setError(null);
      setProgress({ step: "正在保存...", current: 0, total: 3 });
      await invoke("save_current_account");
      setProgress(null);
      // Wait for Electron app to start and write .status.json
      await new Promise((r) => setTimeout(r, 3000));
      await fetchAccounts();
    } catch (e) {
      setProgress(null);
      setError(parseError(e));
      throw e;
    }
  }, [fetchAccounts]);

  const detectCurrent = useCallback(async () => {
    try {
      setError(null);
      await invoke("detect_current_account");
      await fetchAccounts();
    } catch (e) {
      setError(parseError(e));
      throw e;
    }
  }, [fetchAccounts]);

  const claimCheckinAll = useCallback(async () => {
    try {
      setError(null);
      const result = await invoke<ClaimAllResult>("claim_checkin_all");
      console.log("[checkin] claim_checkin_all result:", JSON.stringify(result, null, 2));
      // Refresh quotas after claiming to update check-in status
      await refreshQuotas();
      return result;
    } catch (e) {
      setError(parseError(e));
      throw e;
    }
  }, [refreshQuotas]);

  const clearError = useCallback(() => setError(null), []);
  const clearProgress = useCallback(() => setProgress(null), []);

  return {
    accounts,
    currentUserId,
    loading,
    progress,
    error,
    quotas,
    quotaErrors,
    addAccount,
    deleteAccount,
    switchAccount,
    saveAccount,
    detectCurrent,
    refreshQuotas,
    claimCheckinAll,
    setError,
    clearError,
    clearProgress,
  };
}
