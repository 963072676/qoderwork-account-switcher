import { useState } from "react";
import { Settings, RefreshCw, User, Bug, Loader2, Copy, Check } from "lucide-react";

function maskPhone(phone: string): string {
  if (phone.length <= 7) return phone;
  return phone.slice(0, 3) + "****" + phone.slice(-4);
}

interface HeaderProps {
  currentUserId: string | null;
  currentPhone: string | null;
  onSettings: () => void;
  onDetect: () => void;
  onDebug: () => void;
  loading: boolean;
  version: string | null;
  onCheckUpdate: () => void;
  updateChecking: boolean;
  upToDateMessage: string | null;
}

export function Header({
  currentUserId,
  currentPhone,
  onSettings,
  onDetect,
  onDebug,
  loading,
  version,
  onCheckUpdate,
  updateChecking,
  upToDateMessage,
}: HeaderProps) {
  const [copied, setCopied] = useState(false);

  const handleCopyId = async () => {
    if (!currentUserId) return;
    try {
      await navigator.clipboard.writeText(currentUserId);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // fallback
      const ta = document.createElement("textarea");
      ta.value = currentUserId;
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

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
          <div className="flex items-center gap-2 text-xs text-slate-400">
            <span>
              当前账号：
              {currentPhone || currentUserId ? (
                <button
                  onClick={handleCopyId}
                  disabled={!currentUserId}
                  className="inline-flex items-center gap-1 text-accent-light hover:text-accent transition-colors group/id relative"
                  title={currentUserId || undefined}
                >
                  <span>{currentPhone ? maskPhone(currentPhone) : currentUserId}</span>
                  {copied ? (
                    <Check size={10} className="text-emerald-400" />
                  ) : currentUserId ? (
                    <Copy size={10} className="opacity-0 group-hover/id:opacity-60 transition-opacity" />
                  ) : null}
                  {copied && (
                    <span className="absolute -top-6 left-1/2 -translate-x-1/2 text-[10px] bg-bg-tertiary text-emerald-400 px-1.5 py-0.5 rounded whitespace-nowrap">
                      已复制
                    </span>
                  )}
                  {!copied && currentUserId && (
                    <span className="absolute -top-6 left-1/2 -translate-x-1/2 text-[10px] bg-bg-tertiary text-slate-300 px-1.5 py-0.5 rounded whitespace-nowrap opacity-0 group-hover/id:opacity-100 transition-opacity pointer-events-none">
                      {currentUserId}
                    </span>
                  )}
                </button>
              ) : (
                <span className="text-slate-500">未检测到</span>
              )}
            </span>
            {version && (
              <>
                <span className="text-slate-600">|</span>
                <button
                  onClick={onCheckUpdate}
                  disabled={updateChecking}
                  className="inline-flex items-center gap-1 text-slate-500 hover:text-accent transition-colors disabled:cursor-wait"
                  title="点击检查更新"
                >
                  {updateChecking ? (
                    <Loader2 size={10} className="animate-spin" />
                  ) : null}
                  <span>v{version}</span>
                </button>
              </>
            )}
            {upToDateMessage && !updateChecking && (
              <span className="text-emerald-400 text-xs animate-pulse">
                {upToDateMessage}
              </span>
            )}
          </div>
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
