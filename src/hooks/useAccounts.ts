import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AccountWithStatus, ProgressEvent } from "../types";

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

interface UseAccountsReturn {
  accounts: AccountWithStatus[];
  currentUserId: string | null;
  loading: boolean;
  progress: ProgressEvent | null;
  error: string | null;
  addAccount: (phone: string, label: string, userId?: string) => Promise<void>;
  deleteAccount: (id: string) => Promise<void>;
  switchAccount: (id: string) => Promise<void>;
  saveAccount: () => Promise<void>;
  detectCurrent: () => Promise<void>;
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

  const clearError = useCallback(() => setError(null), []);
  const clearProgress = useCallback(() => setProgress(null), []);

  return {
    accounts,
    currentUserId,
    loading,
    progress,
    error,
    addAccount,
    deleteAccount,
    switchAccount,
    saveAccount,
    detectCurrent,
    setError,
    clearError,
    clearProgress,
  };
}
