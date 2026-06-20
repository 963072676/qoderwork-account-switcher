import { Trash2, Phone } from "lucide-react";
import { AccountWithStatus } from "../types";

interface AccountListProps {
  accounts: AccountWithStatus[];
  onSwitch: (id: string) => void;
  onDelete: (id: string) => void;
}

export function AccountList({
  accounts,
  onSwitch,
  onDelete,
}: AccountListProps) {
  if (accounts.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center text-slate-500">
          <Phone size={48} className="mx-auto mb-3 opacity-30" />
          <p className="text-sm">还没有账号，点击下方添加</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto px-4 py-3 space-y-2">
      {accounts.map((account) => (
        <div
          key={account.id}
          className={`group flex items-center justify-between p-4 rounded-xl border transition-all cursor-pointer ${
            account.is_current
              ? "bg-accent/10 border-accent/30"
              : "bg-bg-secondary border-bg-tertiary hover:border-accent/20 hover:bg-bg-secondary/80"
          }`}
          onClick={() => !account.is_current && onSwitch(account.id)}
          onContextMenu={(e) => {
            e.preventDefault();
            if (
              window.confirm(`确定要删除账号「${account.label}」吗？`)
            ) {
              onDelete(account.id);
            }
          }}
        >
          <div className="flex items-center gap-3 min-w-0">
            <div
              className={`w-10 h-10 rounded-full flex items-center justify-center text-sm font-semibold ${
                account.is_current
                  ? "bg-accent text-white"
                  : "bg-bg-tertiary text-slate-300"
              }`}
            >
              {account.label.charAt(0).toUpperCase()}
            </div>
            <div className="min-w-0">
              <p className="font-medium text-slate-100 truncate">
                {account.label}
              </p>
              <p className="text-sm text-slate-400 truncate">
                {account.phone}
                {account.user_id && (
                  <span className="text-slate-500 ml-2">
                    ID: {account.user_id}
                  </span>
                )}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2 flex-shrink-0">
            {account.is_current ? (
              <span className="text-xs px-2 py-1 rounded-md bg-slate-700 text-slate-300">
                (当前)
              </span>
            ) : account.saved ? (
              <span className="text-xs px-2 py-1 rounded-md bg-emerald-900/50 text-emerald-400">
                [已就绪]
              </span>
            ) : (
              <span className="text-xs px-2 py-1 rounded-md bg-red-900/50 text-red-400">
                [未保存]
              </span>
            )}

            {!account.is_current && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  if (
                    window.confirm(
                      `确定要删除账号「${account.label}」吗？`,
                    )
                  ) {
                    onDelete(account.id);
                  }
                }}
                className="p-1.5 rounded-lg text-slate-500 opacity-0 group-hover:opacity-100 hover:text-red-400 hover:bg-red-900/20 transition-all"
                title="删除账号"
              >
                <Trash2 size={14} />
              </button>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
