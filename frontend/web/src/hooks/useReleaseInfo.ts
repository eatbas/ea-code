import { useState, useEffect } from "react";

const GITHUB_API =
  "https://api.github.com/repos/eatbas/maestro/releases/latest";

interface ReleaseAsset {
  filename: string;
  download_url: string;
  size_bytes: number;
}

export interface ReleaseInfo {
  version: string;
  tag: string;
  published_at: string;
  release_notes: string;
  assets: {
    windows?: ReleaseAsset;
    macos?: ReleaseAsset;
  };
}

interface GitHubAsset {
  name: string;
  browser_download_url: string;
  size: number;
}

interface GitHubRelease {
  tag_name: string;
  name: string;
  published_at: string;
  body: string;
  assets: GitHubAsset[];
}

function findAsset(
  assets: GitHubAsset[],
  test: (name: string) => boolean,
): ReleaseAsset | undefined {
  const match = assets.find((a) => test(a.name));
  if (!match) return undefined;
  return {
    filename: match.name,
    download_url: match.browser_download_url,
    size_bytes: match.size,
  };
}

function parseRelease(gh: GitHubRelease): ReleaseInfo {
  const version = gh.tag_name.replace(/^v/, "");
  return {
    version,
    tag: gh.tag_name,
    published_at: gh.published_at,
    release_notes: gh.body ?? "",
    assets: {
      windows: findAsset(
        gh.assets,
        (n) => n.endsWith("-setup.exe") || n.endsWith("_x64-setup.exe"),
      ),
      macos: findAsset(gh.assets, (n) => n.endsWith(".dmg")),
    },
  };
}

export function useReleaseInfo(): { release: ReleaseInfo | null; loading: boolean } {
  const [release, setRelease] = useState<ReleaseInfo | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch(GITHUB_API, {
      headers: { Accept: "application/vnd.github+json" },
    })
      .then((r) => (r.ok ? r.json() : null))
      .then((data: GitHubRelease | null) => {
        if (data?.tag_name) setRelease(parseRelease(data));
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  return { release, loading };
}
