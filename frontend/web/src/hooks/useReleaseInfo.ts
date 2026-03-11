import { useState, useEffect } from "react";

interface ReleaseAsset {
  filename: string;
  download_url: string;
  size_bytes: number;
}

interface ReleaseInfo {
  version: string;
  tag: string;
  published_at: string;
  release_notes: string;
  assets: {
    windows?: ReleaseAsset;
    macos?: ReleaseAsset;
  };
}

export function useReleaseInfo(): ReleaseInfo | null {
  const [info, setInfo] = useState<ReleaseInfo | null>(null);

  useEffect(() => {
    fetch("/api/v1/updates/release-info")
      .then((r) => (r.ok ? r.json() : null))
      .then((data) => {
        if (data?.version) setInfo(data);
      })
      .catch(() => {});
  }, []);

  return info;
}
