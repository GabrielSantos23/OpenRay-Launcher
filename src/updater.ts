import { emit, listen } from "@tauri-apps/api/event";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useEffect, useState } from "react";

export const UPDATE_FOUND_EVENT = "update-found";
const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;

export async function checkForUpdate(): Promise<Update | null> {
  const update = await check();
  if (update) await emit(UPDATE_FOUND_EVENT);
  return update;
}

export function useAvailableUpdate() {
  const [update, setUpdate] = useState<Update | null>(null);
  useEffect(() => {
    const run = () =>
      check()
        .then(setUpdate)
        .catch((e) => console.error("update check failed", e));
    run();
    const timer = window.setInterval(run, CHECK_INTERVAL_MS);
    const unlisten = listen(UPDATE_FOUND_EVENT, run);
    return () => {
      window.clearInterval(timer);
      unlisten.then((off) => off());
    };
  }, []);
  return update;
}
