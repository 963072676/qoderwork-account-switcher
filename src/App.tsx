import { useState } from "react";
import { UserPlus, Save, CheckCircle } from "lucide-react";
import { useAccounts } from "./hooks/useAccounts";
import { Header } from "./components/Header";
import { AccountList } from "./components/AccountList";
import { AccountForm } from "./components/AccountForm";
import { SwitchProgress } from "./components/SwitchProgress";
import { SettingsModal } from "./components/SettingsModal";

export default function App() {
  const {
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
    claimCheckinAll,
    setError,
    clearError,
  } = useAccounts();

  const [showForm, setShowForm] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [checkinBusy, setCheckinBusy] = useState(false);
  const [checkinMsg, setCheckinMsg] = useState<string | null>(null);

  const isBusy = loading || !!progress;

  const handleSwitch = async (id: string) => {
    if (isBusy) return;
    const account = accounts.find((a) => a.id === id);
    if (!account) return;
    if (!account.saved) {
      setError(`账号「${account.label}」尚未保存会话数据。请先登录该账号，然后点击"保存当前"。`);
      return;
    }
    if (window.confirm(`确定要切换到账号「${account.label}」吗？\n当前未保存的更改将会丢失。`)) {
      try {
        await switchAccount(id);
      } catch {
        // error is set by the hook
      }
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteAccount(id);
    } catch {
      // error is set by the hook
    }
  };

  const handleSave = async () => {
    try {
      await saveAccount();
    } catch {
      // error is set by the hook
    }
  };

  const handleCheckinAll = async () => {
    if (checkinBusy) return;
    setCheckinBusy(true);
    setCheckinMsg("正在签到...");
    try {
      const result = await claimCheckinAll();
      const claimed = Object.values(result.results).filter(
        (r) => r === "CLAIMED"
      ).length;
      const already = Object.values(result.results).filter(
        (r) => r === "ALREADY_CLAIMED"
      ).length;
      const errCount = Object.keys(result.errors).length;
      const parts: string[] = [];
      if (claimed > 0) parts.push(`${claimed} 个签到成功`);
      if (already > 0) parts.push(`${already} 个已签到`);
      if (errCount > 0) parts.push(`${errCount} 个失败`);
      setCheckinMsg(parts.join("，") || "签到完成");
    } catch {
      setCheckinMsg("签到失败");
    } finally {
      setCheckinBusy(false);
      // Clear the message after 5 seconds
      setTimeout(() => setCheckinMsg(null), 5000);
    }
  };

  return (
    <div className="flex flex-col h-screen bg-bg-primary">
      <Header
        currentUserId={currentUserId}
        onSettings={() => setShowSettings(true)}
        onDetect={detectCurrent}
        loading={loading}
      />

      {error && (
        <div
          className="mx-4 mt-3 px-4 py-2.5 rounded-lg bg-red-900/30 border border-red-800/50 text-red-300 text-sm flex items-center justify-between cursor-pointer"
          onClick={clearError}
        >
          <span className="truncate">{error}</span>
          <span className="text-xs text-red-400/60 flex-shrink-0 ml-2">点击关闭</span>
        </div>
      )}

      <AccountList
        accounts={accounts}
        onSwitch={handleSwitch}
        onDelete={handleDelete}
        quotas={quotas}
        quotaErrors={quotaErrors}
        disabled={isBusy}
      />

      <div className="px-4 py-3 bg-bg-secondary border-t border-bg-tertiary flex gap-3">
        <button
          onClick={handleSave}
          disabled={isBusy}
          className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl bg-bg-tertiary text-slate-200 font-medium hover:bg-slate-600 transition-colors disabled:opacity-50"
        >
          <Save size={16} />
          <span className="text-sm">保存当前</span>
        </button>
        <button
          onClick={() => setShowForm(true)}
          disabled={isBusy}
          className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl bg-accent text-white font-medium hover:bg-accent-dark transition-colors disabled:opacity-50"
        >
          <UserPlus size={16} />
          <span className="text-sm">添加账号</span>
        </button>
        <button
          onClick={handleCheckinAll}
          disabled={isBusy || checkinBusy}
          className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl bg-emerald-600 text-white font-medium hover:bg-emerald-700 transition-colors disabled:opacity-50"
          title={checkinMsg || "一键签到所有账号"}
        >
          <CheckCircle size={16} />
          <span className="text-sm">{checkinBusy ? "签到中..." : checkinMsg ? checkinMsg : "一键签到"}</span>
        </button>
      </div>

      {showForm && (
        <AccountForm
          accountCount={accounts.length}
          onAdd={addAccount}
          onClose={() => setShowForm(false)}
        />
      )}

      {showSettings && (
        <SettingsModal onClose={() => setShowSettings(false)} />
      )}

      {progress && <SwitchProgress progress={progress} />}
    </div>
  );
}
