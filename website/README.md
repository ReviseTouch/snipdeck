# Snipdeck → revisetouch.com Integration Guide

> ## ⚠️ CAVEAT — READ FIRST
>
> **The revisetouch.com site is mid-migration to Astro 6 + a Tailwind upgrade.**
> Every path, import alias, schema API, and CSS convention below was read from the
> site **as it is today (pre-upgrade)** and may shift during/after the upgrade.
>
> **Treat this document as a checklist to *adapt and re-verify*, not a blind copy-paste.**
> After the upgrade, re-open the real Çolpan files referenced in each step and diff
> them against the snippets here before applying anything. In particular, watch for:
> - The content-collections API (`glob()` loader, `defineCollection`, `render`,
>   `getCollection`) — Astro 6 may rename or relocate these.
> - The content-config **filename/location** (today it is `src/content.config.ts`,
>   *not* `src/content/config.ts`).
> - Path aliases (`@layouts`, `@i18n`) and Tailwind utility class names
>   (`bg-bg`, `text-fg-soft`, `card`, `lift`, `container-x`, …).
>
> Wherever this guide and the upgraded code disagree, **the upgraded code wins** —
> mirror whatever the post-upgrade Çolpan files actually do.

---

## What is being integrated

All Snipdeck content is already staged in this `website/` folder:

| Staged here | Goes into the site as |
| --- | --- |
| `website/content/snipdeck/en/*.mdx` (18 docs) | `src/content/snipdeck/en/*.mdx` |
| `website/content/snipdeck/tr/*.mdx` (18 docs) | `src/content/snipdeck/tr/*.mdx` |
| `website/page/snipdeck-landing.en.md` (copy spec) | `src/pages/en/snipdeck.astro` |
| `website/page/snipdeck-landing.tr.md` (copy spec) | `src/pages/snipdeck.astro` |

Doc frontmatter is `title`, `description`, `order`, `category`. The `category`
values used across all Snipdeck docs are exactly:
**`getting-started`, `features`, `reference`, `legal`** (no `cql`).

The model the site uses for Çolpan — which we mirror exactly — is:

- One **content collection** per product (`colpan`, `evreka`), both sharing one
  schema, registered in `src/content.config.ts`.
- **Four routing files** per product: a `docs/<product>/index.astro` and
  `docs/<product>/[...slug].astro` for TR, mirrored under `en/`.
- **One shared layout** `src/layouts/DocsLayout.astro` that renders the sidebar,
  search, TOC, prev/next, and category labels.
- A **docs hub card** in `src/pages/docs/index.astro` (TR) and
  `src/pages/en/docs/index.astro` (EN).
- **i18n keys** in `src/i18n/translations.ts`.

> **NOTE — these site paths from the task brief were wrong/renamed; use the real ones:**
> - `src/content/config.ts` → **actually `src/content.config.ts`** (single file at the root of `src/`).
> - `src/components/docs/DocsSidebar.astro` → **does not exist**; the sidebar and all
>   category labels are **inlined inside `src/layouts/DocsLayout.astro`** (see Step 6).

Step paths below are relative to the **site repo root**, `D:/PROJELER/revisetouch.com`.

---

## Step 0 — Copy the prepared content into the site

Copy the staged MDX into the site's content tree (collection folder name must be
`snipdeck` to match the loader `base` in Step 1):

```powershell
# from D:/PROJELER/revisetouch.com
New-Item -ItemType Directory -Force src/content/snipdeck | Out-Null
Copy-Item -Recurse -Force `
  D:/PROJELER/SnipperSlint/website/content/snipdeck/en `
  src/content/snipdeck/en
Copy-Item -Recurse -Force `
  D:/PROJELER/SnipperSlint/website/content/snipdeck/tr `
  src/content/snipdeck/tr
```

Result must be `src/content/snipdeck/en/*.mdx` and `src/content/snipdeck/tr/*.mdx`,
exactly mirroring `src/content/colpan/{en,tr}/`.

---

## Step 1 — Add the `snipdeck` collection to `src/content.config.ts`

Open **`src/content.config.ts`**. Today it looks like this (note the shared
`docsSchema`, and the `evreka` block as the closest template):

```ts
const colpanCollection = defineCollection({
  loader: glob({ pattern: '**/*.{md,mdx}', base: './src/content/colpan' }),
  schema: docsSchema,
});

// Gelecek ürünler için hazır
const evrekaCollection = defineCollection({
  loader: glob({ pattern: '**/*.{md,mdx}', base: './src/content/evreka' }),
  schema: docsSchema,
});

export const collections = {
  colpan: colpanCollection,
  evreka: evrekaCollection,
};
```

**1a.** Add a `snipdeck` collection alongside the others (copy the `evreka` block,
swap the name + base path). Reuse the existing `docsSchema` unchanged — the
Snipdeck frontmatter (`title`, `description`, `order`, `category`) already fits it.

```ts
const snipdeckCollection = defineCollection({
  loader: glob({ pattern: '**/*.{md,mdx}', base: './src/content/snipdeck' }),
  schema: docsSchema,
});
```

**1b.** Register it in the `collections` export:

```ts
export const collections = {
  colpan: colpanCollection,
  evreka: evrekaCollection,
  snipdeck: snipdeckCollection,   // ← add
};
```

> **Re-verify after Astro 6:** confirm `glob()` is still imported from
> `astro/loaders` and `defineCollection`/`z` from `astro:content`, and that the
> file is still named `content.config.ts`. If the upgrade moved to per-collection
> config or a new loader API, mirror whatever the upgraded `colpan`/`evreka`
> definitions do.

---

## Step 2 — Create the 4 routing files (copy Çolpan, swap the collection name)

The Çolpan routes are tiny wrappers that read the collection and hand off to
`DocsLayout`. Create the Snipdeck equivalents by **copying each Çolpan file and
changing two things**: the collection name `'colpan'` → `'snipdeck'`, and the
`CollectionEntry<'colpan'>` type → `CollectionEntry<'snipdeck'>`. Everything else
(the `tr/`/`en/` id filtering, slug stripping, ordering) is identical.

| # | Copy from (source) | Create at (destination) |
| --- | --- | --- |
| 2a | `src/pages/docs/colpan/index.astro` | `src/pages/docs/snipdeck/index.astro` |
| 2b | `src/pages/docs/colpan/[...slug].astro` | `src/pages/docs/snipdeck/[...slug].astro` |
| 2c | `src/pages/en/docs/colpan/index.astro` | `src/pages/en/docs/snipdeck/index.astro` |
| 2d | `src/pages/en/docs/colpan/[...slug].astro` | `src/pages/en/docs/snipdeck/[...slug].astro` |

```powershell
# from D:/PROJELER/revisetouch.com
New-Item -ItemType Directory -Force src/pages/docs/snipdeck | Out-Null
New-Item -ItemType Directory -Force src/pages/en/docs/snipdeck | Out-Null
Copy-Item src/pages/docs/colpan/index.astro        src/pages/docs/snipdeck/index.astro
Copy-Item src/pages/docs/colpan/'[...slug].astro'  src/pages/docs/snipdeck/'[...slug].astro'
Copy-Item src/pages/en/docs/colpan/index.astro        src/pages/en/docs/snipdeck/index.astro
Copy-Item src/pages/en/docs/colpan/'[...slug].astro'  src/pages/en/docs/snipdeck/'[...slug].astro'
```

**Then edit all four** — the only change in each is the collection string (and the
type in the two `[...slug]` files):

- In `index.astro` (both langs):
  `getCollection('colpan', …)` → `getCollection('snipdeck', …)`
- In `[...slug].astro` (both langs):
  - `getCollection('colpan', …)` → `getCollection('snipdeck', …)`
  - `interface Props { doc: CollectionEntry<'colpan'>; }`
    → `interface Props { doc: CollectionEntry<'snipdeck'>; }`

For reference, the **TR `index.astro`** after editing reads:

```astro
---
import { getCollection, render } from 'astro:content';
import DocsLayout from '@layouts/DocsLayout.astro';

const docs = await getCollection('snipdeck', ({ id }) => id.startsWith('tr/'));

const byOrder = (a: (typeof docs)[number], b: (typeof docs)[number]) => a.data.order - b.data.order;
const firstGettingStarted = docs.filter(d => d.data.category === 'getting-started').sort(byOrder)[0];
const firstDoc = firstGettingStarted ?? [...docs].sort(byOrder)[0];

const { Content, headings } = await render(firstDoc);
const slug = firstDoc.id.replace('tr/', '').replace('.mdx', '').replace('.md', '');
---

<DocsLayout
  title={firstDoc.data.title}
  description={firstDoc.data.description}
  category={firstDoc.data.category}
  currentSlug={slug}
  headings={headings}
>
  <Content />
</DocsLayout>
```

The EN variants are identical except `id.startsWith('en/')` and `replace('en/', '')`.

> ### ⚠️ Important gap — `DocsLayout` is hardwired to Çolpan
>
> `src/layouts/DocsLayout.astro` is **not parameterized by product**. As written it
> hardcodes Çolpan branding and URLs, so the four routing files above will render
> Snipdeck content **inside a Çolpan-branded shell**. Specifically, inside
> `DocsLayout.astro`:
> - `const fullTitle = \`${title} - Çolpan Docs\`;` → page `<title>` says "Çolpan Docs".
> - `const docsBasePath = lang === 'tr' ? '/docs/colpan' : '/en/docs/colpan';`
>   → **every sidebar link, search result, prev/next link, and the logo link point
>     back to `/docs/colpan`** — Snipdeck nav will be broken.
> - `getCollection('colpan', …)` is called **twice** (for `allDocs` and
>   `otherDocs`) to build the sidebar/search — so the sidebar lists **Çolpan**
>   pages, not Snipdeck pages.
> - The sidebar logo is hardcoded `Ç` / "Çolpan" / "Documentation".
> - The favicon SVG draws a `Ç`.
>
> **You must make `DocsLayout` product-aware before Snipdeck docs work.** Recommended
> minimal approach (mirror however the upgrade restructures it):
> 1. Add a `product` prop (e.g. `'colpan' | 'snipdeck'`) to `DocsLayout`'s `Props`,
>    defaulting to `'colpan'` for backward compatibility.
> 2. Derive from it: the collection name passed to both `getCollection(...)` calls,
>    `docsBasePath` (`/docs/${product}` and `/en/docs/${product}`), `fullTitle`
>    suffix, `langSwitchHref` base, the logo mark/name, and the favicon letter.
> 3. Pass `product="snipdeck"` from all four Snipdeck routing files.
>
> If the Astro 6 upgrade has already generalized `DocsLayout` (e.g. via a
> `collection`/`product` prop), just pass the Snipdeck product value and skip the
> refactor.

---

## Step 3 — Add a Snipdeck card to the docs hub (TR + EN)

The docs hub lists product cards. Add a **Snipdeck** card next to the Çolpan one.

- **TR file:** `src/pages/docs/index.astro` — link target `/docs/snipdeck`
- **EN file:** `src/pages/en/docs/index.astro` — link target `/en/docs/snipdeck`

**Where:** in each file, inside the
`<div class="grid md:grid-cols-2 gap-8 max-w-4xl mx-auto">` wrapper, the first
child is the **active Çolpan card** (an `<a href="/docs/colpan" …>` / `/en/docs/colpan`)
and the second is the **disabled Evreka "SOON" card**. Insert the Snipdeck card as a
**new active `<a>` card, copied from the Çolpan card** and re-pointed. (The grid is
`md:grid-cols-2`; adding a third active card means you'll likely want
`md:grid-cols-3`, or keep 2-up and drop/relegate the Evreka placeholder — match
whatever layout the upgraded hub uses.)

Copy the Çolpan `<a>` card block and change:
- `href="/docs/colpan"` → `href="/docs/snipdeck"` (EN: `/en/docs/colpan` → `/en/docs/snipdeck`)
- the avatar tile letter `Ç` → `S`
- the three `<span>` bullet labels (TR `Kurulum Rehberi`, `ÇQL Sorgu Dili`,
  `Özellikler & Kısayollar`) → Snipdeck ones, e.g. **TR:** `Kurulum & Hızlı Başlangıç`,
  `Yakalama Kısayolları`, `OCR & Galeri`; **EN:** `Installation & Quick Start`,
  `Capture Shortcuts`, `OCR & Gallery`
- the three i18n token references `t('docs.colpan.title')`, `t('docs.colpan.desc')`,
  `t('docs.colpan.cta')` → `t('docs.snipdeck.title')`, `t('docs.snipdeck.desc')`,
  `t('docs.snipdeck.cta')` (keys added in Step 4)

TR card skeleton to insert (EN: swap the `href` to `/en/docs/snipdeck` and the
bullet labels to English):

```astro
<!-- Snipdeck Docs -->
<a href="/docs/snipdeck" class="group card lift hover:border-accent transition-colors duration-300">
  <div class="flex items-start gap-4 mb-6">
    <div class="w-14 h-14 bg-inverse rounded-xl grid place-items-center text-inverse-fg font-bold text-xl flex-shrink-0">
      S
    </div>
    <div>
      <h2 class="text-xl font-bold text-fg group-hover:text-accent transition-colors">
        {t('docs.snipdeck.title')}
      </h2>
      <p class="text-sm text-fg-muted mt-1">{t('docs.snipdeck.desc')}</p>
    </div>
  </div>

  <div class="space-y-3 mb-6">
    <div class="flex items-center gap-3 text-sm text-fg-soft">
      <svg class="w-5 h-5 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
      </svg>
      <span>Kurulum &amp; Hızlı Başlangıç</span>
    </div>
    <div class="flex items-center gap-3 text-sm text-fg-soft">
      <svg class="w-5 h-5 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
      </svg>
      <span>Yakalama Kısayolları</span>
    </div>
    <div class="flex items-center gap-3 text-sm text-fg-soft">
      <svg class="w-5 h-5 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
      </svg>
      <span>OCR &amp; Galeri</span>
    </div>
  </div>

  <div class="flex items-center gap-2 text-accent font-semibold group-hover:gap-3 transition-all">
    {t('docs.snipdeck.cta')}
    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
    </svg>
  </div>
</a>
```

---

## Step 4 — Add i18n keys to `src/i18n/translations.ts`

The hub card and any nav entry use `t(...)` keys. Add the following keys to
**both** the `tr` block and the `en` block of the `translations` object (mirror the
existing `docs.colpan.*` and `nav.docs.colpan*` keys). The `TranslationKey` type is
derived from `translations.tr`, so **every key must exist in both languages**.

**Add to the `tr:` block** (next to the existing `docs.colpan.*` keys, ~line 103–105):

```ts
// Docs Hub — Snipdeck
'docs.snipdeck.title': 'Snipdeck Dökümanları',
'docs.snipdeck.desc': "Windows için hızlı, klavye odaklı ekran görüntüsü uygulaması: kurulum, yakalama kısayolları, OCR ve galeri.",
'docs.snipdeck.cta': 'Dökümanlara Git',
```

**Add to the `en:` block** (next to the existing `docs.colpan.*` keys, ~line 344–346):

```ts
// Docs Hub — Snipdeck
'docs.snipdeck.title': 'Snipdeck Documentation',
'docs.snipdeck.desc': 'Fast, keyboard-driven screen-snipping app for Windows: installation, capture shortcuts, OCR, and gallery.',
'docs.snipdeck.cta': 'Go to Docs',
```

**Optional — nav dropdown entries** (only if you add Snipdeck to the site header's
"Docs"/"Products" menu; mirror the existing `nav.docs.colpan` / `nav.docs.colpan.desc`):

```ts
// tr:
'nav.docs.snipdeck': 'Snipdeck Dökümanları',
'nav.docs.snipdeck.desc': 'Yakalama, OCR ve galeri rehberi',
// en:
'nav.docs.snipdeck': 'Snipdeck Docs',
'nav.docs.snipdeck.desc': 'Capture, OCR and gallery guide',
```

> **Re-verify after Astro 6:** the i18n file may have moved or the
> `translations`/`useTranslations` shape may have changed. Confirm the keys still
> live in `src/i18n/translations.ts` under `tr`/`en` and that `useTranslations`
> is still imported as `import { getLangFromUrl, useTranslations } from '@i18n/index'`.

---

## Step 5 — Assemble the landing page → `src/pages/snipdeck.astro` (TR) + `src/pages/en/snipdeck.astro` (EN)

The landing copy spec is `website/page/snipdeck-landing.en.md`. It is a **section
spec**, not a finished `.astro` file — each section gives the eyebrow, heading, body
copy, cards/tables, button labels, link targets, and a layout note. Build the page
by translating each numbered section into the site's existing section/markup
patterns.

> **Both language specs exist.** `snipdeck-landing.en.md` (EN source) and
> `snipdeck-landing.tr.md` (TR translation) are both staged. Build the TR page
> (`src/pages/snipdeck.astro`) from the TR spec and the EN page
> (`src/pages/en/snipdeck.astro`) from the EN spec. Both are explicitly grounded in
> the Snipdeck README — keep every hedge ("on the way", "planned"); do not invent claims.

**Template to mirror:** these are standalone marketing pages that use the base
`Layout.astro` (NOT `DocsLayout`). The closest existing single-page examples are
`src/pages/mcp.astro` (simple) and `src/pages/evreka.astro` (full marketing rhythm —
hero badge → "what is it" → feature cards → tech/spec tables → audience → final CTA).
The landing spec was written to mirror the **Evreka** page section-for-section, so
**copy `src/pages/evreka.astro` as the structural starting point** and replace its
content with the Snipdeck sections.

Minimal page shell (matches `mcp.astro`):

```astro
---
import Layout from '@layouts/Layout.astro';
import { getLangFromUrl, useTranslations } from '@i18n/index';

const lang = getLangFromUrl(Astro.url);
const t = useTranslations(lang);
---

<Layout title="Snipdeck — Fast screen-snipping for Windows" description="...">
  <!-- sections 1–8 from the copy spec, reusing Evreka section markup/classes -->
</Layout>
```

Map the spec's 8 sections to existing Evreka section blocks:

| Spec section | Reuse Evreka pattern |
| --- | --- |
| 1. Hero (badge, 2-line heading, lede, trust chips, 2 CTAs + GitHub link) | Evreka hero / grid-bg + dot matrix |
| 2. What is Snipdeck (2-col split + highlight card) | Evreka `what` |
| 3. Key features (8 cards) | Evreka `features` 3-col card grid |
| 4. Capture-shortcut matrix (table) | full-width table on `bg-soft` |
| 5. Privacy & open source (trust cards + GitHub button) | Evreka security/feature cards |
| 6. Made by ReviseTouch (slim band) | `bg-soft` border-y band |
| 7. Platforms & roadmap (status table + settings note) | Evreka tech-spec table |
| 8. Final download CTA (inverse bg + glow) | Evreka `notify` CTA |

**Link tokens** from the spec (use literally):

| Token | URL |
| --- | --- |
| `DOWNLOAD` | `https://github.com/hasantr/snipdeck/releases` |
| `DOCS` | `/en/docs/snipdeck` (TR page: use `/docs/snipdeck`) |
| `GITHUB` | `https://github.com/hasantr/snipdeck` |
| `REVISETOUCH` | `https://revisetouch.com` |

**i18n note:** the spec copy is English prose. To keep TR/EN parity you can either
hardcode strings per-file (simplest, like `mcp.astro` does) or lift the copy into
`translations.ts` under a `snipdeck.*` namespace (consistent with `evreka.*`). For a
single product page, per-file hardcoding matches the existing `mcp.astro` precedent.

> **Optional — link the landing page from the header/nav** (`nav.colpan` /
> `nav.evreka` pattern) so `/snipdeck` is discoverable. This is not required for the
> page to work and depends on the post-upgrade header component.

---

## Step 6 — Category labels: confirm coverage in `DocsLayout.astro`

There is **no `DocsSidebar.astro`** — the sidebar, category grouping, `catOrder`,
and the `categoryLabels` map all live **inline in `src/layouts/DocsLayout.astro`**.

The Snipdeck docs use exactly these four categories:
**`getting-started`, `features`, `reference`, `legal`**.

`DocsLayout.astro` already defines labels and ordering for all four (today around
lines 70 and 81–87):

```ts
const catOrder = { 'getting-started': 1, 'cql': 2, 'features': 3, 'reference': 4, 'legal': 5 };

const categoryLabels: Record<string, Record<string, string>> = {
  'getting-started': { tr: 'Başlangıç',      en: 'Getting Started' },
  'cql':             { tr: 'CQL Sorgu Dili',  en: 'CQL Query Language' },
  'features':        { tr: 'Özellikler',      en: 'Features' },
  'reference':       { tr: 'Referans',        en: 'Reference' },
  'legal':           { tr: 'Yasal',           en: 'Legal' },
};
```

**Result: ✅ all four Snipdeck categories are already covered — no missing labels.**
(`cql` is Çolpan-only and simply won't appear for Snipdeck, which is harmless: any
unknown category falls back to its raw key via `categoryLabels[cat]?.[lang] ?? cat`
and sorts last via `catOrder[...] ?? 99`.)

Caveats to flag:
- The category **order** for Snipdeck will be Getting Started → Features → Reference
  → Legal (positions 1, 3, 4, 5). That's correct; the gap at 2 (`cql`) is fine. If you
  want a tighter product-specific order, give `DocsLayout` a per-product `catOrder`
  when you make it product-aware in Step 2.
- If a future Snipdeck doc introduces a **new** category not in the map, its raw key
  shows in the sidebar — add a `categoryLabels` entry then.
- These labels are duplicated logic that the Astro 6 refactor may relocate; **re-check
  the upgraded `DocsLayout` (or its replacement) still contains a `categoryLabels`
  map covering these four keys.**

---

## Final verification checklist

After applying all steps (and the post-upgrade re-verification), build the site and
confirm:

- [ ] `src/content/snipdeck/{en,tr}/*.mdx` present; `npm run build` type-checks the
      `snipdeck` collection against `docsSchema`.
- [ ] `/docs/snipdeck` and `/en/docs/snipdeck` load the first getting-started doc.
- [ ] `/docs/snipdeck/installation` (and a few others) render via `[...slug]`.
- [ ] The docs **sidebar, search, prev/next, logo, and `<title>`** all say
      **Snipdeck** and link within `/docs/snipdeck` — i.e. `DocsLayout` was made
      product-aware (Step 2 gap), not still Çolpan.
- [ ] Sidebar groups read Getting Started / Features / Reference / Legal (or TR
      equivalents) in that order.
- [ ] TR + EN docs-hub pages show the Snipdeck card linking to the right base path.
- [ ] `/snipdeck` (and `/en/snipdeck`) landing page renders all 8 sections with
      correct CTA targets; TR page exists only if TR copy was authored.
- [ ] Language switcher on Snipdeck docs flips TR↔EN correctly (depends on the
      product-aware `langSwitchHref` from Step 2).
- [ ] No leftover `Ç`/Çolpan branding or `/docs/colpan` links on Snipdeck pages.
```
