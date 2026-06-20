import { ProgressEvent } from "../types";
import { Loader2 } from "lucide-react";

interface SwitchProgressProps {
  progress: ProgressEvent;
}

export function SwitchProgress({ progress }: SwitchProgressProps) {
  const percentage =
    progress.total > 0
      ? Math.round((progress.current / progress.total) * 100)
      : 0;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-bg-secondary border border-bg-tertiary rounded-2xl w-80 p-6 shadow-xl">
        <div className="flex items-center gap-3 mb-4">
          <Loader2 size={20} className="text-accent animate-spin" />
          <span className="text-sm font-medium text-slate-200">
            正在操作...
          </span>
        </div>

        <div className="space-y-3">
          <div className="flex items-center justify-between text-xs text-slate-400">
            <span>{progress.step}</span>
            <span>
              [{progress.current}/{progress.total}]
            </span>
          </div>

          <div className="w-full h-2 rounded-full bg-bg-primary overflow-hidden">
            <div
              className="h-full rounded-full bg-accent transition-all duration-300"
              style={{ width: `${percentage}%` }}
            />
          </div>

          <p className="text-xs text-slate-500 text-center">
            请稍候，操作正在进行中
          </p>
        </div>
      </div>
    </div>
  );
}
