import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { X, FolderOpen, Search, Settings as SettingsIcon } from "lucide-react";

interface SettingsModalProps {
  onClose: () => void;
}

export function SettingsModal({ onClose }: SettingsModalProps) {
  const [exePath, setExePath] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      try {
        const path = await invoke<string>("get_exe_path");
        setExePath(path);
      } catch {
        setExePath("");
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  const handleBrowse = async () => {
    try {
      const selected = await open({
        filters: [{ name: "可执行文件", extensions: ["exe"] }],
        multiple: false,
        directory: false,
      });
      if (selected) {
        setExePath(selected as string);
        await invoke("set_exe_path", { path: selected });
      }
    } catch (e) {
      console.error("选择文件失败:", e);
    }
  };

  const handleAutoDetect = async () => {
    try {
      setLoading(true);
      const path = await invoke<string>("auto_detect_exe");
      setExePath(path);
    } catch (e) {
      alert(`自动检测失败: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-bg-secondary border border-bg-tertiary rounded-2xl w-[28rem] shadow-xl">
        <div className="flex items-center justify-between px-5 py-4 border-b border-bg-tertiary">
          <div className="flex items-center gap-2">
            <SettingsIcon size={18} className="text-accent" />
            <h2 className="text-base font-semibold text-slate-100">设置</h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-bg-tertiary transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        <div className="p-5 space-y-5">
          <div>
            <label className="block text-sm text-slate-400 mb-2">
              QoderWork 程序路径
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={loading ? "加载中..." : exePath || "未设置"}
                readOnly
                className="flex-1 px-3 py-2.5 rounded-lg bg-bg-primary border border-bg-tertiary text-slate-300 text-sm truncate"
              />
              <button
                onClick={handleBrowse}
                className="px-3 py-2.5 rounded-lg border border-bg-tertiary text-slate-300 hover:bg-bg-tertiary hover:text-accent transition-colors flex items-center gap-1.5"
                title="浏览文件"
              >
                <FolderOpen size={15} />
                <span className="text-sm">浏览</span>
              </button>
            </div>
          </div>

          <button
            onClick={handleAutoDetect}
            disabled={loading}
            className="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg border border-bg-tertiary text-slate-300 hover:bg-bg-tertiary hover:text-accent transition-colors disabled:opacity-50"
          >
            <Search size={15} />
            <span className="text-sm">自动检测路径</span>
          </button>

          <div className="pt-2">
            <button
              onClick={onClose}
              className="w-full px-4 py-2.5 rounded-lg bg-accent text-white font-medium hover:bg-accent-dark transition-colors"
            >
              关闭
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
