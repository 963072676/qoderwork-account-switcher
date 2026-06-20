import { useState } from "react";
import { UserPlus, Save, Settings } from "lucide-react";
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
    addAccount,
    deleteAccount,
    switchAccount,
    saveAccount,
    detectCurrent,
    setError,
    clearError,
  } = useAccounts();

  const [showForm, setShowForm] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

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
          onClick={() => setShowSettings(true)}
          disabled={isBusy}
          className="flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl bg-bg-tertiary text-slate-200 font-medium hover:bg-slate-600 transition-colors disabled:opacity-50"
        >
          <Settings size={16} />
          <span className="text-sm">设置</span>
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
