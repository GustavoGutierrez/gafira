// The download links of the latest Gafira release, read from GitHub when the
// site is built (static output: no request at visit time). Without network
// or with the GitHub API unavailable, the pinned release in
// src/data/release-fallback.json is used, so a build never fails for it.
import fallback from '../data/release-fallback.json';

const REPO = 'GustavoGutierrez/gafira';

export interface Asset {
  name: string;
  size: number;
  url: string;
}

export interface Release {
  tag: string;
  version: string;
  published: string;
  url: string;
  assets: Asset[];
}

/** The packages of one system, by the file names release.yml publishes. */
export interface Downloads {
  windowsInstaller?: Asset;
  windowsZip?: Asset;
  macos?: Asset;
  deb?: Asset;
  appImage?: Asset;
  checksums?: Asset;
}

interface GitHubRelease {
  tag_name: string;
  published_at: string;
  html_url: string;
  draft: boolean;
  prerelease: boolean;
  assets: { name: string; size: number; browser_download_url: string }[];
}

function fromFallback(): Release {
  return { ...fallback, version: fallback.tag.replace(/^v/, '') };
}

let cached: Promise<Release> | undefined;

/** The latest published release (not a draft or a pre-release). */
export function latestRelease(): Promise<Release> {
  cached ??= fetchLatest();
  return cached;
}

async function fetchLatest(): Promise<Release> {
  const headers: Record<string, string> = {
    Accept: 'application/vnd.github+json',
    'User-Agent': 'gafira-website-build',
  };
  const token = process.env.GITHUB_TOKEN;
  if (token) headers.Authorization = `Bearer ${token}`;
  try {
    const response = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers,
      signal: AbortSignal.timeout(10_000),
    });
    if (!response.ok) throw new Error(`GitHub answered ${response.status}`);
    const data = (await response.json()) as GitHubRelease;
    if (data.draft || data.prerelease || data.assets.length === 0) {
      throw new Error('latest release has no packages');
    }
    return {
      tag: data.tag_name,
      version: data.tag_name.replace(/^v/, ''),
      published: data.published_at,
      url: data.html_url,
      assets: data.assets.map((a) => ({ name: a.name, size: a.size, url: a.browser_download_url })),
    };
  } catch (error) {
    console.warn(`[release] using the pinned release: ${(error as Error).message}`);
    return fromFallback();
  }
}

/** Sorts the assets of `release` by system. */
export function downloads(release: Release): Downloads {
  const find = (test: (name: string) => boolean) => release.assets.find((a) => test(a.name));
  return {
    windowsInstaller: find((n) => /-Setup-x64\.exe$/i.test(n)),
    windowsZip: find((n) => /-windows-x64\.zip$/i.test(n)),
    macos: find((n) => /\.dmg$/i.test(n)),
    deb: find((n) => /\.deb$/i.test(n)),
    appImage: find((n) => /\.AppImage$/i.test(n)),
    checksums: find((n) => n === 'SHA256SUMS'),
  };
}

/** "82 MB", with the decimal units the systems show. */
export function megabytes(bytes: number): string {
  return `${Math.round(bytes / 1_000_000)} MB`;
}
