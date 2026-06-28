import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface UpdateInfo {
  available: boolean;
  version: string;
  releaseNotes: string;
  downloadUrl: string;
}

export function useUpdate() {
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateChecking, setUpdateChecking] = useState(false);
  const [updateDismissed, setUpdateDismissed] = useState(false);
  const [currentVersion, setCurrentVersion] = useState<string | null>(null);
  const [upToDateMessage, setUpToDateMessage] = useState<string | null>(null);

  const checkForUpdate = useCallback(async () => {
    try {
      setUpdateChecking(true);
      setUpToDateMessage(null);
      const info = await invoke<UpdateInfo>("check_update");
      setUpdateInfo(info);
      if (!info.available) {
        setUpToDateMessage("已是最新版本");
        setTimeout(() => setUpToDateMessage(null), 3000);
      }
    } catch (e) {
      console.warn("Failed to check for updates:", e);
    } finally {
      setUpdateChecking(false);
    }
  }, []);

  const dismissUpdate = useCallback(() => {
    setUpdateDismissed(true);
  }, []);

  const openDownload = useCallback(async () => {
    if (!updateInfo?.downloadUrl) return;
    try {
      await invoke("open_url", { url: updateInfo.downloadUrl });
    } catch (e) {
      console.error("Failed to open download URL:", e);
    }
  }, [updateInfo]);

  // Check for updates on mount
  useEffect(() => {
    checkForUpdate();
  }, [checkForUpdate]);

  // Fetch current app version on mount
  useEffect(() => {
    invoke<string>("get_app_version")
      .then(setCurrentVersion)
      .catch(() => {});
  }, []);

  const showUpdateBanner =
    updateInfo?.available === true && !updateDismissed;

  return {
    updateInfo,
    updateChecking,
    showUpdateBanner,
    dismissUpdate,
    openDownload,
    checkForUpdate,
    currentVersion,
    upToDateMessage,
  };
}
