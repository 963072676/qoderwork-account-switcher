import { useState } from "react";
import { X, UserPlus } from "lucide-react";

interface AccountFormProps {
  accountCount: number;
  onAdd: (phone: string, label: string, userId?: string) => Promise<void>;
  onClose: () => void;
}

export function AccountForm({ accountCount, onAdd, onClose }: AccountFormProps) {
  const [phone, setPhone] = useState("");
  const [label, setLabel] = useState(`账号${accountCount + 1}`);
  const [userId, setUserId] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!phone.trim()) {
      setError("请输入手机号");
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      await onAdd(phone.trim(), label.trim() || `账号${accountCount + 1}`, userId.trim() || undefined);
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-bg-secondary border border-bg-tertiary rounded-2xl w-96 shadow-xl">
        <div className="flex items-center justify-between px-5 py-4 border-b border-bg-tertiary">
          <div className="flex items-center gap-2">
            <UserPlus size={18} className="text-accent" />
            <h2 className="text-base font-semibold text-slate-100">
              添加账号
            </h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-bg-tertiary transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-5 space-y-4">
          <div>
            <label className="block text-sm text-slate-400 mb-1.5">
              手机号 <span className="text-red-400">*</span>
            </label>
            <input
              type="tel"
              value={phone}
              onChange={(e) => setPhone(e.target.value)}
              placeholder="请输入手机号"
              className="w-full px-3 py-2.5 rounded-lg bg-bg-primary border border-bg-tertiary text-slate-100 placeholder-slate-600 focus:outline-none focus:border-accent/50 transition-colors"
              autoFocus
            />
          </div>

          <div>
            <label className="block text-sm text-slate-400 mb-1.5">
              备注名称
            </label>
            <input
              type="text"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              placeholder="例如：工作号"
              className="w-full px-3 py-2.5 rounded-lg bg-bg-primary border border-bg-tertiary text-slate-100 placeholder-slate-600 focus:outline-none focus:border-accent/50 transition-colors"
            />
          </div>

          <div>
            <label className="block text-sm text-slate-400 mb-1.5">
              用户 ID{" "}
              <span className="text-slate-600">(可选)</span>
            </label>
            <input
              type="text"
              value={userId}
              onChange={(e) => setUserId(e.target.value)}
              placeholder="自动检测或手动输入"
              className="w-full px-3 py-2.5 rounded-lg bg-bg-primary border border-bg-tertiary text-slate-100 placeholder-slate-600 focus:outline-none focus:border-accent/50 transition-colors"
            />
          </div>

          {error && (
            <p className="text-sm text-red-400">{error}</p>
          )}

          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 px-4 py-2.5 rounded-lg border border-bg-tertiary text-slate-300 hover:bg-bg-tertiary transition-colors"
            >
              取消
            </button>
            <button
              type="submit"
              disabled={submitting}
              className="flex-1 px-4 py-2.5 rounded-lg bg-accent text-white font-medium hover:bg-accent-dark transition-colors disabled:opacity-50"
            >
              {submitting ? "添加中..." : "添加"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
