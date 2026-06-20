import { Trash2, Phone, AlertCircle, CheckCircle, XCircle, Gift, CreditCard, Calendar } from "lucide-react";
import { AccountWithStatus, QuotaMap } from "../types";

interface AccountListProps {
  accounts: AccountWithStatus[];
  onSwitch: (id: string) => void;
  onDelete: (id: string) => void;
  quotas: QuotaMap;
  quotaErrors?: Record<string, string>;
  disabled?: boolean;
}

function formatNum(n?: number): string {
  if (n == null) return "--";
  if (n >= 10000) return (n / 10000).toFixed(1) + "w";
  if (n >= 1000) return (n / 1000).toFixed(1) + "k";
  return n.toLocaleString("zh-CN");
}

function formatSubDays(days?: number): string {
  if (days == null) return "--";
  if (days <= 0) return "已过期";
  if (days > 365) return `${Math.floor(days / 365)}年${days % 365}天`;
  return `${days}天`;
}

function maskPhone(phone: string): string {
  if (phone.length <= 7) return phone;
  return phone.slice(0, 3) + "****" + phone.slice(-4);
}

export function AccountList({
  accounts,
  onSwitch,
  onDelete,
  quotas,
  quotaErrors,
  disabled,
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
      {accounts.map((account) => {
        const quota = quotas[account.id];
        const quotaErr = quotaErrors?.[account.id];
        return (
          <div
            key={account.id}
            className={`group flex items-center justify-between p-4 rounded-xl border transition-all ${
              disabled
                ? "opacity-50 pointer-events-none"
                : "cursor-pointer"
            } ${
              account.is_current
                ? "bg-accent/10 border-accent/30"
                : "bg-bg-secondary border-bg-tertiary hover:border-accent/20 hover:bg-bg-secondary/80"
            }`}
            onClick={() => !disabled && !account.is_current && onSwitch(account.id)}
            onContextMenu={(e) => {
              if (disabled) return;
              e.preventDefault();
              if (window.confirm(`确定要删除账号「${account.label}」吗？`)) {
                onDelete(account.id);
              }
            }}
          >
            <div className="flex items-center gap-3 min-w-0 flex-1">
              <div
                className={`w-10 h-10 rounded-full flex items-center justify-center text-sm font-semibold flex-shrink-0 ${
                  account.is_current
                    ? "bg-accent text-white"
                    : "bg-bg-tertiary text-slate-300"
                }`}
              >
                {account.label.charAt(0).toUpperCase()}
              </div>
              <div className="min-w-0 flex-1">
                <p className="font-medium text-slate-100 truncate">
                  {account.label}
                </p>
                <p className="text-sm text-slate-400 truncate">
                  {maskPhone(account.phone)}
                </p>
                {quota && (
                  <div className="flex flex-wrap items-center gap-x-3 gap-y-1 mt-1">
                    {/* Column 1: Daily free 3.7MAX credits */}
                    <span
                      className="inline-flex items-center gap-1 text-xs"
                      title="每日免费模型剩余额度"
                    >
                      <Gift size={11} className="text-emerald-400" />
                      <span className={quota.exceeded ? "text-red-400" : "text-slate-300"}>
                        {formatNum(quota.dailyFree)}
                      </span>
                    </span>

                    {/* Column 2: Other credits (plan + org) */}
                    <span
                      className="inline-flex items-center gap-1 text-xs"
                      title="其他额度（订阅计划 + 组织包）"
                    >
                      <CreditCard size={11} className="text-blue-400" />
                      <span className={quota.exceeded ? "text-red-400" : "text-slate-300"}>
                        {formatNum(quota.otherCredits)}
                      </span>
                    </span>

                    {/* Column 3: Check-in status */}
                    <span
                      className="inline-flex items-center gap-1 text-xs"
                      title={quota.checkedIn ? "今日已签到" : "今日未签到"}
                    >
                      {quota.checkedIn ? (
                        <>
                          <CheckCircle size={11} className="text-emerald-400" />
                          <span className="text-emerald-400">已签到</span>
                        </>
                      ) : (
                        <>
                          <XCircle size={11} className="text-slate-500" />
                          <span className="text-slate-500">未签到</span>
                        </>
                      )}
                    </span>

                    {/* Column 4: Subscription days remaining */}
                    <span
                      className="inline-flex items-center gap-1 text-xs"
                      title="订阅剩余时间"
                    >
                      <Calendar size={11} className={
                        quota.subDaysRemaining != null && quota.subDaysRemaining <= 7
                          ? "text-red-400"
                          : "text-amber-400"
                      } />
                      <span className={
                        quota.subDaysRemaining != null && quota.subDaysRemaining <= 0
                          ? "text-red-400"
                          : quota.subDaysRemaining == null
                            ? "text-slate-500"
                            : "text-slate-300"
                      }>
                        {formatSubDays(quota.subDaysRemaining)}
                      </span>
                    </span>
                  </div>
                )}
                {quotaErr && (
                  <p className="text-xs text-red-400/70 mt-0.5 flex items-center gap-1" title={quotaErr}>
                    <AlertCircle size={10} />
                    <span className="truncate">额度获取失败: {quotaErr}</span>
                  </p>
                )}
              </div>
            </div>

            <div className="flex items-center gap-2 flex-shrink-0 ml-2">
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
                    if (window.confirm(`确定要删除账号「${account.label}」吗？`)) {
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
        );
      })}
    </div>
  );
}
