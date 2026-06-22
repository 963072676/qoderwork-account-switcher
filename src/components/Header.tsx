import { Settings, RefreshCw, User, Bug } from "lucide-react";

interface HeaderProps {
  currentUserId: string | null;
  onSettings: () => void;
  onDetect: () => void;
  onDebug: () => void;
  loading: boolean;
}

export function Header({
  currentUserId,
  onSettings,
  onDetect,
  onDebug,
  loading,
}: HeaderProps) {
  return (
    <header className="flex items-center justify-between px-6 py-4 bg-bg-secondary border-b border-bg-tertiary">
      <div className="flex items-center gap-3">
        <div className="w-8 h-8 rounded-lg bg-accent flex items-center justify-center">
          <User size={18} className="text-white" />
        </div>
        <div>
          <h1 className="text-lg font-semibold text-slate-100">
            QoderWork CN 账号切换器
          </h1>
          <p className="text-xs text-slate-400">
            当前账号：
            {currentUserId ? (
              <span className="text-accent-light">{currentUserId}</span>
            ) : (
              <span className="text-slate-500">未检测到</span>
            )}
          </p>
        </div>
      </div>

      <div className="flex items-center gap-2">
        <button
          onClick={onDetect}
          disabled={loading}
          className="p-2 rounded-lg text-slate-400 hover:text-accent hover:bg-bg-tertiary transition-colors disabled:opacity-50"
          title="检测当前账号"
        >
          <RefreshCw
            size={18}
            className={loading ? "animate-spin" : ""}
          />
        </button>
        <button
          onClick={onDebug}
          className="p-2 rounded-lg text-slate-400 hover:text-accent hover:bg-bg-tertiary transition-colors"
          title="诊断信息"
        >
          <Bug size={18} />
        </button>
        <button
          onClick={onSettings}
          className="p-2 rounded-lg text-slate-400 hover:text-accent hover:bg-bg-tertiary transition-colors"
          title="设置"
        >
          <Settings size={18} />
        </button>
      </div>
    </header>
  );
}
