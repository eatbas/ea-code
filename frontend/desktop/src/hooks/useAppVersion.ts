import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";

/** Fetches the Tauri app version once on mount. */
export function useAppVersion(): string | null {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    void getVersion()
      .then((v) => { if (mounted) setVersion(v); })
      .catch(() => { if (mounted) setVersion(null); });
    return () => { mounted = false; };
  }, []);

  return version;
}
