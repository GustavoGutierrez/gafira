# Gafira website

The landing page of Gafira at <https://gafira.com>: Spanish at `/`, English at `/en/`, light and dark mode (the system's, or the one chosen with the header toggle).

- **Astro 7, static output.** Only two small scripts reach the browser: the reveal and theme script in `Base.astro` and the system detection of the download button in `Hero.astro`. Screenshots are served as AVIF/WebP in several widths.
- **Brand from the source.** Colours and radii are read from `assets/theme/gafira.tokens.json` (the file the app loads), so the site follows the app. The logos and favicon come from `docs/manuals/marca/assets/`, and the touch and manifest icons from `assets/app-icon/`.
- **Downloads from the release.** At build time `src/lib/release.ts` reads the latest GitHub release of `GustavoGutierrez/gafira` and sorts its packages by system. If GitHub cannot be reached, the build uses `src/data/release-fallback.json` and logs a warning. **Rebuild and deploy after each release** so the links point to the new version, and update the fallback file then.
- **Cloudflare Workers static assets.** `wrangler.jsonc` serves `dist/` with the custom domain `gafira.com`. No Worker script runs.

## Layout

| Path | What it is |
|---|---|
| `src/pages/index.astro`, `src/pages/en/index.astro` | The landing page in each language (`src/components/Landing.astro`) |
| `src/pages/404.astro` | The page for any missing path, in both languages |
| `src/i18n/ui.ts` | Every text, in Spanish and English, with one shared type |
| `src/components/` | Header, Hero, How (with `MotionDesign`, the looping motion graphic), Features, Showcase, Local, Download, Cta, Footer |
| `src/assets/screens/` | Screenshots of Gafira (optimised at build) |
| `public/` | Favicon, logos, icons, `site.webmanifest`, `robots.txt` |

## Commands

Run them from `website/`. Node 22.12 or later is required.

| Command | Does |
|---|---|
| `npm install` | Install the dependencies |
| `npm run dev` | Local server with hot reload (<http://localhost:4321>) |
| `npm run build` | `astro check` and the production build into `dist/` |
| `npm run preview` | Build, then serve it with the Workers runtime (`wrangler dev`) |
| `npm run deploy:dry-run` | Build and validate the upload without publishing |
| `npm run deploy` | Build and publish to Cloudflare |

Optional: set `GITHUB_TOKEN` to raise the GitHub API rate limit of the build.

## Cloudflare setup (once)

Prerequisite: the `gafira.com` zone is active in the Cloudflare account (its nameservers point to Cloudflare).

1. **Sign in.** Either run `npx wrangler login` (browser), or copy `.env.example` to `.env` and fill in `CLOUDFLARE_ACCOUNT_ID` and an API token with "Workers Scripts: Edit" on the account plus "Workers Routes: Edit" and "DNS: Edit" on the zone. `.env` is never committed.
2. **Deploy.** `npm run deploy`. The first deploy creates the Worker `gafira-website` and the custom domain `gafira.com`, including its DNS record and certificate.
3. **Redirect `www` to the apex.** Use one canonical host, `gafira.com`, and send `www` there with a permanent redirect at the edge, before any Worker runs:
   - DNS → add a record `www`, type `AAAA`, content `100::`, **Proxied**. The record only lets Cloudflare receive the traffic; the rule below answers it.
   - Rules → Redirect Rules → create a rule from the template **"Redirect from WWW to root"**, or by hand:
     - When: `Hostname` equals `www.gafira.com`
     - Then: Dynamic redirect, expression `concat("https://gafira.com", http.request.uri.path)`, status **301**, **Preserve query string** on.
4. **HTTPS.** SSL/TLS → Edge Certificates → turn on **Always Use HTTPS**. Turn on HSTS only once everything works over HTTPS: it is hard to undo.

## Checking a deploy

```sh
for u in / /en/ /favicon.svg /sitemap-index.xml /no-such-page; do
  printf "%s " "$u"; curl -s -o /dev/null -w "%{http_code}\n" "https://gafira.com$u"
done
# 200 for pages and files, 404 for the missing page.
curl -sI https://www.gafira.com/en/?a=1 | grep -i -E "^(HTTP|location)"
# 301 to https://gafira.com/en/?a=1
```
