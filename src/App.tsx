import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { UserPlus, Save, CheckCircle, DownloadCloud, X } from "lucide-react";
import { useAccounts } from "./hooks/useAccounts";
import { useUpdate } from "./hooks/useUpdate";
import { Header } from "./components/Header";
import { AccountList } from "./components/AccountList";
import { AccountForm } from "./components/AccountForm";
import { SwitchProgress } from "./components/SwitchProgress";
import { SettingsModal } from "./components/SettingsModal";

interface DebugInfo {
  statusFileExists: boolean;
  statusFilePath: string;
  loggedIn: boolean | null;
  username: string | null;
  avatarUrl: string | null;
  detectedUserId: string | null;
  appDataDir: string;
  appDataDirExists: boolean;
  partitionsDirExists: boolean;
}

export default function App() {
  const {
    accounts,
    currentUserId,
    loading,
    progress,
    error,
    quotas,
    quotaErrors,
    quotasLoading,
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
  const [debugInfo, setDebugInfo] = useState<DebugInfo | null>(null);

  const {
    updateInfo,
    showUpdateBanner,
    dismissUpdate,
    openDownload,
  } = useUpdate();

  const handleShowDebug = async () => {
    try {
      const info = await invoke<DebugInfo>("get_debug_info");
      setDebugInfo(info);
    } catch (e) {
      setError("获取诊断信息失败: " + String(e));
    }
  };

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
        onDebug={handleShowDebug}
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

      {showUpdateBanner && updateInfo && (
        <div className="mx-4 mt-3 px-4 py-2.5 rounded-lg bg-cyan-900/30 border border-cyan-700/50 text-cyan-200 text-sm flex items-center justify-between">
          <div className="flex items-center gap-2">
            <DownloadCloud size={16} className="text-cyan-400 flex-shrink-0" />
            <span className="truncate">
              发现新版本 <span className="font-semibold text-cyan-300">v{updateInfo.version}</span>
            </span>
          </div>
          <div className="flex items-center gap-2 flex-shrink-0 ml-2">
            <button
              onClick={openDownload}
              className="px-3 py-1 rounded-md bg-cyan-600 text-white text-xs font-medium hover:bg-cyan-500 transition-colors"
            >
              更新
            </button>
            <button
              onClick={dismissUpdate}
              className="p-1 rounded-md text-cyan-400/60 hover:text-cyan-200 hover:bg-cyan-800/40 transition-colors"
            >
              <X size={14} />
            </button>
          </div>
        </div>
      )}

      <AccountList
        accounts={accounts}
        onSwitch={handleSwitch}
        onDelete={handleDelete}
        quotas={quotas}
        quotaErrors={quotaErrors}
        quotasLoading={quotasLoading}
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

      {debugInfo && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
          <div className="bg-bg-secondary border border-bg-tertiary rounded-2xl w-[480px] max-h-[80vh] overflow-y-auto shadow-xl">
            <div className="flex items-center justify-between px-5 py-4 border-b border-bg-tertiary sticky top-0 bg-bg-secondary">
              <h2 className="text-base font-semibold text-slate-100">诊断信息</h2>
              <button
                onClick={() => setDebugInfo(null)}
                className="p-1 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-bg-tertiary transition-colors"
              >
                ✕
              </button>
            </div>
            <div className="p-5 space-y-3 text-sm font-mono">
              <div>
                <span className="text-slate-400">状态文件：</span>
                <span className={debugInfo.statusFileExists ? "text-green-400" : "text-red-400"}>
                  {debugInfo.statusFileExists ? "存在" : "不存在"}
                </span>
              </div>
              <div>
                <span className="text-slate-400">路径：</span>
                <span className="text-slate-300 break-all">{debugInfo.statusFilePath}</span>
              </div>
              <div>
                <span className="text-slate-400">已登录：</span>
                <span className={debugInfo.loggedIn ? "text-green-400" : "text-red-400"}>
                  {debugInfo.loggedIn === null ? "未知" : debugInfo.loggedIn ? "是" : "否"}
                </span>
              </div>
              <div>
                <span className="text-slate-400">username：</span>
                <span className="text-slate-300">{debugInfo.username || "无"}</span>
              </div>
              <div>
                <span className="text-slate-400">avatar_url：</span>
                <span className="text-slate-300 break-all">{debugInfo.avatarUrl || "无"}</span>
              </div>
              <div>
                <span className="text-slate-400">检测到的 userId：</span>
                <span className={debugInfo.detectedUserId ? "text-green-400" : "text-red-400"}>
                  {debugInfo.detectedUserId || "未检测到"}
                </span>
              </div>
              <hr className="border-bg-tertiary" />
              <div>
                <span className="text-slate-400">App Data 目录：</span>
                <span className={debugInfo.appDataDirExists ? "text-green-400" : "text-red-400"}>
                  {debugInfo.appDataDirExists ? "存在" : "不存在"}
                </span>
              </div>
              <div>
                <span className="text-slate-400">路径：</span>
                <span className="text-slate-300 break-all">{debugInfo.appDataDir}</span>
              </div>
              <div>
                <span className="text-slate-400">Partitions 目录：</span>
                <span className={debugInfo.partitionsDirExists ? "text-green-400" : "text-red-400"}>
                  {debugInfo.partitionsDirExists ? "存在" : "不存在"}
                </span>
              </div>
            </div>
          </div>
        </div>
      )}

      {progress && <SwitchProgress progress={progress} />}
    </div>
  );
}
