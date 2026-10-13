# Target-app parity

> **Last updated:** 2026-10-13 · **Change:** minor (home screen with My designs and project folders, text shaping, text edited on the canvas, vector PDF, multi-series charts and richer tables, formatting part of a text box, MP4 video export, German UI, AI background removal, text that shrinks to fit its box) · **Target:** Adobe Express (web, desktop and mobile), feature set as publicly documented in October 2026

The feature inventory of the target app, compiled from **public sources only** (help pages, product
and pricing pages, press releases, developer docs; URLs per area below), in our own words, feature
names only. Next to each feature: what GenHa does today. Scores are self-assessed and err low.

Legend: ✅ works (basic form) · 🟡 partial · ⬜ not yet (milestone in [roadmap.md](roadmap.md)) · ➖ out of scope / replaced by an open alternative

## Summary by area

| Area | Score | Notes |
|---|---|---|
| 1. Editor & canvas | ~50% (estimated) | pages of any size, select/move/resize/rotate, groups, order, lock/hide, undo/redo, snapping, version history, crop page, Data Recovery; no comments or co-editing yet |
| 2. Templates & content | ~30% (estimated) | 121 original templates in 6 categories (animated ones, charts, tables, curved text, QR codes among them), 114 icons, 40 palettes, all generated in code; no stock library (by design: open content later) |
| 3. Text | ~55% (estimated) | text boxes edited in place on the canvas, styles, formatting part of a box (font, size, bold, italic, underline, strikethrough, colour per word), presets, shadow/outline/highlight effects, curved text, system fonts, real shaping (kerning, ligatures, right-to-left and complex scripts, font fallback); no bulleted lists, grid text, colour emoji, spellcheck, translate |
| 4. Pictures & AI | ~30% (estimated) | place, crop, fit, crop-to-shape, adjustments, filter presets, background removal with an optional local AI model (or colour-based), convert/resize quick actions; no generative AI yet |
| 5. Video | ~15% (estimated) | element animations (12 entrances, 4 loops, exits) with canvas playback and present mode; MP4 (H.264), GIF and animated PNG export; no timeline, scenes or audio yet |
| 6. Documents & PDF | ~35% (estimated) | multi-page vector PDF export (real text with embedded fonts), presentations as pages, QR codes, charts (bar, line, area, pie, donut, progress; several series, stacked, axis titles) and tables (merged cells, cell styles); no PDF editing/import, web pages |
| 7. Brand kits | ~35% (estimated) | colours, heading/body fonts, logos slot, apply to page; no locking/template controls |
| 8. Social | 0% | size presets only; no scheduler (planned as an optional, self-hostable add-on) |
| 9. Export & share | ~55% (estimated) | PNG (transparent), JPEG, SVG, vector and print PDF, MP4 video, animated GIF and PNG; no share links, print ordering |
| 10. Platform | ~30% (estimated) | MCP server, control channel, CLI, every action a command; no add-on SDK yet |
| 11. Accessibility & localisation | ~25% (estimated) | keyboard shortcuts; the whole UI in English or German (follows the system, or chosen; switches live); UI fonts for every script the system has fonts for; no screen-reader support or other languages yet |

## 1. Editor and canvas

Sources: https://helpx.adobe.com/express/web/organize-your-designs/arrange-layers-and-pages/change-page-size.html · https://helpx.adobe.com/express/web/arrange-layers-and-pages/version-history.html · https://www.adobe.com/express/learn/blog/collaboration-share

| Feature | Status | GenHa |
|---|---|---|
| Multi-page designs, pages of different sizes | ✅ | `page.add/duplicate/delete/move/goto`, per-page size |
| Resize page (presets or custom) | ✅ | `page.setSize`, presets (`presets.list`) |
| Smart resize / duplicate & resize to other sizes | ✅ | `page.resize {mode: smart}` re-lays out the design (M2.21): text reflowed to the new width with real measurements and readable sizes, stacks/rows/grids detected and re-arranged (rows become stacks on tall pages, grids change column count, tall designs go into columns on wide pages), backgrounds and bands follow the page, cards and buttons scale as units, decorations stay anchored; never leaves text overflowing, off the page or overlapping (checked on every template × story/square/landscape/A4, with the system's fonts, with none and with only DejaVu Sans); text that shrinks to fit is re-laid out at the size it is drawn at and keeps shrinking in its new box; `notes` report text set below the readable size. `mode: scale` (fit and centre), `mode: none`; `copy: true` adds a resized copy. Limits: no focal point for cropped pictures; dense two-column documents re-flow mechanically |
| Crop page to content | ✅ | `page.cropToContent {padding}` |
| Layers panel, reorder, lock, hide | ✅ | `object.bringForward…`, `object.lock/hide`, Layers panel |
| Group / ungroup | ✅ | `object.group/ungroup` |
| Align & distribute, snapping, guides | ✅ | `object.align/distribute`; smart guides in the Select tool while moving and resizing (any handle, several objects, Shift for proportions, Alt from the centre): page edges and centre, other objects' edges and centres, same width/height, equal spacing in a row or column (with markers); distances to the nearest neighbours or page edges while dragging; Alt-hover shows the distances to the hovered object; threshold constant on screen; hold Ctrl/Cmd to skip snapping, `view.snapping` (Snap button) turns it off. Limits: rotated objects do not snap while resizing; no hand-placed ruler guides |
| Undo/redo | ✅ | unlimited-ish (200 steps), structural sharing |
| Copy/paste style | ✅ | `style.copy/paste` |
| Version history | ✅ | kept in the file: `version.save/list/restore/copy/rename/delete`; automatic version on save (20 kept), bookmarks kept |
| Comments, @mentions, review/approval | ⬜ | M7 (needs a sync service) |
| Real-time co-editing, share links | ⬜ | M7 (optional self-hostable server) |
| Home / start page | 🟡 | M2.24: navigation rail (Create, Home, My designs, Templates, Brand, project folders, Settings); greeting by time of day (optional name, `settings.set`), five start tiles (open a file — drop zone, new design — size picker with presets by category and a custom size, edit a photo, create a video — story/square/wide with an animated title or an animated template, create a presentation — blank 16:9 or a deck template), recent designs with thumbnails and "edited … ago", quick actions with illustrations, a template row with category chips; works at 1000 px (rows scroll sideways). Designed from the owner's description; no account, no cloud |
| Your stuff: projects / folders | 🟡 | M2.24: My designs keeps every design made or opened (`design.*`: list with folder, search and sort, open, rename, duplicate, move, remove, sync); designs never saved to a file are kept as copies in the data folder (desktop) or IndexedDB (browser), so they are not lost; project folders with colours and order (`folder.*`), drag a card onto a folder; delete with confirmation. Not yet: shared folders, multi-select, trash/restore |
| Search everything | 🟡 | M2.24: one search over templates (name, tags, category), size presets, quick actions and your designs, in English and German (`home.search`), results grouped under the field, Enter opens the first; the Templates page has its own search and chips. No stock content to search (by design) |

## 2. Templates and content

Sources: https://www.adobe.com/express/feature · https://www.adobe.com/express/pricing

| Feature | Status | GenHa |
|---|---|---|
| Templates by category (social, marketing, print, documents) | 🟡 | 121 original templates generated in code (`template.list/use`): social posts, carousels, animated posts, stories and short videos (entrance animations, page durations), reel covers, thumbnails, banners, pins, flyers, posters, menus, price lists, recipe card, property flyer, business and loyalty cards, invitations, save the date, greeting cards, certificates, vouchers, coupons, tickets, letterhead, invoice, cover letter, newsletter, timetable, worksheet, decks with charts, tables and speaker notes, infographic, landing page, resumes, logos (curved text), web ads, email headers; all take brand colours and fonts; headings and short blocks shrink to fit their box, so designs made for Inter keep their lines with a wider font; tested for text that fits its box (system fonts, no fonts, only DejaVu Sans) and readable contrast. Still far fewer than the target app's library, and none photo-based |
| Template search / filters | 🟡 | category + lower-case search tags on every template (at least 3 each) |
| Stock photos, video, audio | ➖/⬜ | no proprietary stock; M6: optional open-content sources (CC0 libraries) and local folders |
| Icons, shapes, design assets | 🟡 | 10 basic shapes + 114 original filled icons as paths in 11 categories (arrows, interface, people & messages, media, business, food, nature, travel, celebration, badges, decorative: blobs, dividers, frames, brush stroke) (`library.list/addShape`) |
| Backgrounds (solid, gradient) | ✅ | page background fill |
| Photo grids / collages, frames | 🟡 | `layout.collage` (layouts for 1–9 pictures); crop-to-shape frames |
| Text-to-template, generate similar | ⬜ | M8 (pluggable local/remote AI providers) |

## 3. Text

Sources: https://www.adobe.com/express/feature/design/text-effects/curved · https://helpx.adobe.com/express/web/create-and-customize-text/translate-technical-requirements.html · https://www.adobe.com/learn/express/web/edit-text-express

| Feature | Status | GenHa |
|---|---|---|
| Add text, font, size, colour, alignment, spacing, line height | ✅ | `text.add`, `text.setStyle`; a text box can shrink its text to fit (M3.3: "Shrink text to fit" in the panel, `text.setFit {fit: shrink}`): every size of the box scaled together, down to 40% (never below 6 px), to the largest scale at which the lines fit the box without breaking a word; the stored sizes stay, the box keeps its size while typing, and the canvas, the text editor's caret, PNG, SVG, vector PDF, curved text (fitted along its arc) and smart resize all use the same scale (`text.inspect` reports it) |
| Format part of a text box (bold, italic, underline, size, font, colour of selected words) | 🟡 | select words while editing on the canvas and use the panel or Ctrl/Cmd+B/I/U (Strikethrough: Ctrl/Cmd+Shift+X): font, size, bold, italic, underline, strikethrough and colour per character range (`TextEl::spans`); with nothing selected the change applies to the next text typed; typed text takes the style before the caret; mixed values show as mixed in the panel; drawn per run on the canvas and in PNG, SVG (nested `<tspan>`s or coloured outlines) and vector PDF (one text run per style, decorations, synthetic bold/italic); `text.setStyle {range}`, `text.toggleBold`…, `text.inspect` for agents. Not yet: bulleted/numbered lists, formatting kept by copy/paste, Ctrl+Z inside an edit for formatting (Undo after the edit works) |
| Edit text in place on the canvas | ✅ | double-click a text box, press Enter, or draw one with the Text tool; caret and selection placed from the renderer's own shaped layout (wrapping, alignment, spacing, uppercase, ligatures split by character, right-to-left runs, zoom, rotation); click, drag, Shift+click, double-click word, triple-click paragraph, arrows (Ctrl/Alt by word), Home/End, Ctrl+A, Backspace/Delete, Enter, copy/cut/paste, IME composition, undo inside the edit; Esc or a click outside leaves, the whole edit is one undo step and the box height follows the text. `text.beginEdit` / `text.edit` / `text.endEdit` for agents. Limits: curved text is shown straight while edited; arrows move in logical order inside right-to-left runs |
| Typography: kerning, ligatures, right-to-left and complex scripts, mixed scripts | 🟡 | `genha-text` shapes with HarfRust (kerning, standard ligatures, off when letter-spaced; mark positioning; Arabic joining, Hebrew, Devanagari and other Indic scripts as far as HarfRust and the font go), reorders bidi text per line (UAX #9), breaks lines at UAX #14 opportunities (also between ideographs), and sets characters the chosen font lacks in another installed font. Not yet: colour emoji, justified text, user OpenType features, vertical text |
| Text presets (heading/subheading/body…) | ✅ | 25 presets (display, serif, mono, kicker, tracked capitals, quotes, big numbers, price tag, button, outline, shadow, glow, highlight), `text.addPreset` |
| Shadow, outline, shape-behind-text (highlight) | ✅ | `text.setEffect` |
| Curved / grid text | 🟡 | curved text: `text.setCurve` (-360..360 degrees, arch or smile, multi-line on concentric arcs, shadow and outline follow; highlight boxes are left out on curves); grid text M3 |
| Document-wide text styles | ⬜ | M3 |
| Brand fonts (uploaded) | 🟡 | system fonts; adding font files to a design: M3 |
| Spellcheck, translate | ⬜ | M8 |
| Generative text effects | ⬜ | M8 |

## 4. Pictures, quick actions and AI

Sources: https://helpx.adobe.com/express/web/image-creation-and-editing/generate-and-edit-with-ai/resize-with-expand.html · https://news.adobe.com/news/2025/10/adobe-max-2025-express-ai-assistant

| Feature | Status | GenHa |
|---|---|---|
| Place pictures (PNG, JPEG, WebP, GIF, BMP) and SVG | ✅ | `image.place`, drag & drop; SVG becomes editable shapes (text in SVG not yet) |
| HEIC import | ⬜ | M4 |
| Crop, fit, flip, rotate, crop to shape | ✅ | `image.crop/setFit/mask`, `object.flip*` |
| Adjustments (brightness, contrast, saturation, warmth, sharpen, blur) | ✅ | `image.adjust` |
| Filters with intensity | ✅ | `image.filter` (14 original presets) |
| Remove / change background | 🟡 | remove, blur or replace the background: an optional AI model (U²-Net family, downloaded on request, runs locally) finds the subject on busy photos, a colour-based method handles plain backdrops (`method: auto/ml/color`); edges of hair and fur are softer than a matting model's, and there is no brush to refine the cut-out (M4.2) |
| Quick actions: resize, crop, convert JPG/PNG/WebP | ✅ | `genha-cli quick …` |
| Convert to SVG (vectorize) | ⬜ | M4 (SVG files can be imported as editable shapes: `image.place`, `file.open`) |
| Generative fill, expand, text-to-image | ⬜ | M8 |
| AI assistant (chat editing) | 🟡 | any MCP-capable assistant can drive every command today |

## 5. Video

Sources: https://helpx.adobe.com/express/create-and-edit-videos/create-videos/add-scenes.html · https://news.adobe.com/news/2025/04/adobe-introduces-new-ai-powered-video-tools-adobe-express

| Feature | Status | GenHa |
|---|---|---|
| Scenes/timeline, trim, split, speed | ⬜ | M6 |
| Animations (in/out/loop) | ✅ | `animation.set` / `animation.applyToPage` (staggered): 12 entrances (fade, rise, slides in four directions, pop, zoom, bounce, spin, flip, typewriter), 4 loops (pulse, float, sway, blink), exits; per-object duration and delay, page length auto or fixed; Play on the canvas. Groups animate as one. Limits: no motion paths or per-word text animation; typewriter text centred in a narrow box can re-wrap while typing |
| Audio, voice-over | ⬜ | M6 |
| Captions | ⬜ | M8 |
| MP4 / GIF export | ✅ | `export.mp4` (Download › MP4 video): H.264 in MP4, every page for its duration (or chosen pages), 1–60 fps (30 default), quality low/medium/high, frames up to 1920 × 1080 pixels' worth (1080 × 1920 stories exact), ≤ 3600 frames; plays in browsers, QuickTime, Windows and phones (Constrained Baseline). `export.gif` and `export.apng` (one page or all, 5–30 fps, ≤ 600 frames, ≤ 2 MP per frame, still frames merged); `export.frame` for one moment. Limits: no audio track, no 4K, no resolution presets |

## 6. Documents and PDF

Sources: https://helpx.adobe.com/express/using/charts.html · https://helpx.adobe.com/express/using/hosting-projects.html

| Feature | Status | GenHa |
|---|---|---|
| Multi-page PDF export | ✅ | `export.pdf`: vector PDF (krilla) — shapes, icons, borders, gradients, opacity, clipping and crop-to-shape as PDF graphics; text as real text in the layout's font, embedded as a subset (select, search, copy; curved text as glyphs placed along the arc); pictures at native resolution (JPEG as is, crops as clips; adjusted pictures as lossless pixels); charts and tables as paths and text; pages of different sizes, title metadata. Only blurred drop shadows become pictures (300 dpi, that element only). `raster: true` keeps the earlier pages-as-pictures PDF with an invisible text layer |
| Presentations (pages + notes, present mode) | 🟡 | pages + notes; present mode (F5 or Present: full screen, page animations play, arrows/space/click to move, N for notes, Esc); no presenter view on a second screen |
| PDF edit / combine / import | ⬜ | M9 (shared with the PDFCraft sibling) |
| Charts, tables | 🟡 | `chart.add/setData/setStyle/setKind`: bar, horizontal bar, line, area, pie, donut, progress bar and ring; up to 20 named series per chart with shared category labels (bars grouped or stacked, lines and areas overlapping or stacked; pies and progress use the first series), per-series colours, axis titles (the vertical one turned), gridlines on/off, legend none/auto/top/bottom/right, number/percent/currency formats with a symbol and fixed decimals, fixed or automatic min/max. Data typed in a series × categories grid in the panel, as `Label: value` text for one series, or pasted as CSV/TSV (first row series names, first column labels). Old one-series files open as one series. `table.add/setCell/insertRow/deleteRow/insertColumn/deleteColumn/setRows/setStyle/mergeCells/splitCell/setCellStyle/setColumnWidth/setRowHeight`: header row, banded rows, merged cells (row and column spans; inserting and deleting rows and columns keeps them consistent), per-cell fill, text colour, bold, alignment and text size for a picked range, column widths in pixels or fitted to the text, row heights (at least a given height, else fitted), lines all/outside/inside/rows with a different outside line, wrapped cell text, CSV in (quoted cells). Charts and tables draw through one shared scene, so PNG, SVG and vector PDF match. Not yet: cell ranges picked on the canvas (the panel's grid picks them), formulas, combo/scatter charts, data labels inside pie legends |
| QR code generator | ✅ | `element.qrCode` |
| Single-page web pages | ⬜ | M9 (static HTML export) |
| PPTX import/export | ⬜ | M9 |

## 7. Brand kits

Sources: https://www.adobe.com/express/learn/blog/brand-kits · https://helpx.adobe.com/express/using/template-controls.html

| Feature | Status | GenHa |
|---|---|---|
| Brand colours, fonts, logos | ✅ | `brand.*` commands, Brand panel |
| Apply brand to a design | ✅ | `brand.apply`, templates take brand colours |
| Palette from a photo | ✅ | `brand.fromPicture` (dominant colours) |
| Several brands, shared libraries | ⬜ | M5 |
| Template locking / brand controls | ⬜ | M7 |

## 8. Social

Source: https://www.adobe.com/express/learn/blog/content-scheduler

| Feature | Status | GenHa |
|---|---|---|
| Platform size presets | ✅ | `presets.list` |
| Content scheduler, publishing, analytics | ⬜ | M10, optional add-on (platform APIs need user tokens) |

## 9. Export, share, print

Sources: https://www.adobe.com/express/feature · https://helpx.adobe.com/express/mobile/publish-and-share/print-and-order-designs.html

| Feature | Status | GenHa |
|---|---|---|
| PNG (transparent), JPEG | ✅ | `export.png/jpeg`, scale option |
| SVG | ✅ | `export.svg` |
| PDF (standard), print PDF with bleed/CMYK | 🟡 | vector PDF; print (`print: true`: 3 mm bleed, crop marks, Trim/BleedBox); `cmyk: true` writes DeviceCMYK colours and pictures (plain conversion, no ICC profile or output intent, no PDF/X) |
| MP4 / GIF | ✅ | MP4 video (`export.mp4`, H.264, desktop and web), animated GIF and PNG (`export.gif`, `export.apng`); no audio, no 4K |
| Share links, publish to web | ⬜ | M7 / M9 |
| Print ordering | ➖ | not planned (third-party service) |

## 10. Platform and integrations

Sources: https://developer.adobe.com/express/add-ons/docs/references/document-sandbox/document-apis · https://developer.adobe.com/express/embed-sdk/docs/v4/

| Feature | Status | GenHa |
|---|---|---|
| Add-ons / plugins | ⬜ | M9: sandboxed WebAssembly plug-ins calling the command registry |
| Document API | ✅ | the command registry (JSON) via control channel, CLI and MCP |
| MCP / chat assistants | ✅ | `genha-cli mcp` |
| Embed SDK | ⬜ | M9 (web build) |
| Desktop apps | ✅ | native Linux/macOS/Windows (egui); installable downloads from GitHub Releases (M7.1, docs/release.md): a universal macOS DMG signed with a Developer ID and notarized when the signing secrets are set, `.genha` files open from Finder; a Windows zip (icon embedded, not code-signed); a Linux tar.gz with `.desktop` entry, icons and MIME type; an original app icon. The macOS and Windows release jobs are untested until the first tagged release |
| Web app | 🟡 | `apps/genha-web`: the same UI in the browser (WASM, WebGL2), verified in Chromium; open/place/drop through the browser, Save and Download as downloads; fonts added from files (kept in IndexedDB) or the installed fonts (Local Font Access, Chromium only), synthetic bold/italic; Data Recovery and recent designs with thumbnails in IndexedDB; pictures pasted with Ctrl+V. Not yet: control channel, WOFF fonts, coordination between several tabs |
| Mobile apps | ⬜ | later |
| Offline use | ✅ | everything runs locally; no account needed; Data Recovery copies unsaved designs every minute on the desktop, every 15 s in the browser (`recovery.*`, one `Store` abstraction: a folder or IndexedDB) |

## 11. Accessibility and localisation

Source: https://www.adobe.com/accessibility/compliance/adobe-express-android-2025-acr.html

| Feature | Status | GenHa |
|---|---|---|
| Keyboard shortcuts | ✅ | per command |
| Screen reader support | ⬜ | M5 (egui AccessKit) |
| UI translations | 🟡 | M5.2: English and German (`crates/i18n`, gettext PO catalogues compiled in): every panel, menu, dialog, the command palette (labels and menus), status and common error messages, size presets, template categories, shapes, text styles and palettes; `ui.setLanguage {lang: system\|en\|de}` kept between runs, picker on the home screen, switches without restart; "system" follows the OS or browser language. Not yet: other languages, invalid-parameter errors (agent-facing, English), template names |
| Text in every script in the UI | 🟡 | M5.2: system fonts covering Arabic, Hebrew, Indic scripts, Thai, CJK, Hangul and more are added as egui fallbacks (worker thread at start, again when fonts are added); egui does not shape, so Arabic and Indic text in UI fields is unjoined (the canvas shapes it); the browser has only fonts the user adds |

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-13 | minor | Home screen, My designs with project folders, one search (M2.24) |
| 2026-10-13 | minor | Text that shrinks to fit its box (M3.3) |
| 2026-10-13 | minor | German UI, language setting, UI fonts for every script (M5.2) |
| 2026-10-13 | minor | Formatting part of a text box (style runs, M3.2) |
| 2026-10-13 | minor | Text edited in place on the canvas (M2.16) |
| 2026-10-13 | minor | Vector PDF (krilla): real text with embedded fonts, vectors, native-resolution pictures; CMYK option |
| 2026-10-13 | minor | Text shaping (kerning, ligatures, bidi, complex scripts, font fallback) |
| 2026-10-12 | minor | PDF text layer and print PDF (bleed, crop marks) |
| 2026-10-12 | minor | Version history, crop page to content, Data Recovery, recent files, SVG import |
| 2026-10-12 | major | First inventory from public sources and first self-assessment |
