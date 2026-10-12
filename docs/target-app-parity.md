# Target-app parity

> **Last updated:** 2026-10-12 · **Change:** major (first assessment) · **Target:** Adobe Express (web, desktop and mobile), feature set as publicly documented in October 2026

The feature inventory of the target app, compiled from **public sources only** (help pages, product
and pricing pages, press releases, developer docs; URLs per area below), in our own words, feature
names only. Next to each feature: what GenHa does today. Scores are self-assessed and err low.

Legend: ✅ works (basic form) · 🟡 partial · ⬜ not yet (milestone in [roadmap.md](roadmap.md)) · ➖ out of scope / replaced by an open alternative

## Summary by area

| Area | Score | Notes |
|---|---|---|
| 1. Editor & canvas | ~50% (estimated) | pages of any size, select/move/resize/rotate, groups, order, lock/hide, undo/redo, snapping, version history, crop page, Data Recovery; no comments or co-editing yet |
| 2. Templates & content | ~15% (estimated) | 17 original templates, shape & icon library in code; no stock library (by design: open content later) |
| 3. Text | ~35% (estimated) | text boxes, styles, presets, shadow/outline/highlight effects, system fonts; no curved text, spellcheck, translate |
| 4. Pictures & AI | ~25% (estimated) | place, crop, fit, crop-to-shape, adjustments, filter presets, classic background removal, convert/resize quick actions; no generative AI yet |
| 5. Video | 0% | model has page durations and animations; no timeline or video export yet |
| 6. Documents & PDF | ~15% (estimated) | multi-page PDF export (raster), presentations as pages, QR codes; no PDF editing/import, charts, web pages |
| 7. Brand kits | ~35% (estimated) | colours, heading/body fonts, logos slot, apply to page; no locking/template controls |
| 8. Social | 0% | size presets only; no scheduler (planned as an optional, self-hostable add-on) |
| 9. Export & share | ~40% (estimated) | PNG (transparent), JPEG, SVG, PDF; no MP4/GIF, share links, print ordering |
| 10. Platform | ~30% (estimated) | MCP server, control channel, CLI, every action a command; no add-on SDK yet |
| 11. Accessibility & localisation | ~5% (estimated) | keyboard shortcuts; no screen-reader support or translations yet |

## 1. Editor and canvas

Sources: https://helpx.adobe.com/express/web/organize-your-designs/arrange-layers-and-pages/change-page-size.html · https://helpx.adobe.com/express/web/arrange-layers-and-pages/version-history.html · https://www.adobe.com/express/learn/blog/collaboration-share

| Feature | Status | GenHa |
|---|---|---|
| Multi-page designs, pages of different sizes | ✅ | `page.add/duplicate/delete/move/goto`, per-page size |
| Resize page (presets or custom) | ✅ | `page.setSize`, presets (`presets.list`) |
| Smart resize / duplicate & resize to other sizes | ✅ | `page.resize` (smart re-anchoring), `page.resize {copy: true}` |
| Crop page to content | ✅ | `page.cropToContent {padding}` |
| Layers panel, reorder, lock, hide | ✅ | `object.bringForward…`, `object.lock/hide`, Layers panel |
| Group / ungroup | ✅ | `object.group/ungroup` |
| Align & distribute, snapping, guides | ✅ | `object.align/distribute`, smart snapping in the Select tool |
| Undo/redo | ✅ | unlimited-ish (200 steps), structural sharing |
| Copy/paste style | ✅ | `style.copy/paste` |
| Version history | ✅ | kept in the file: `version.save/list/restore/copy/rename/delete`; automatic version on save (20 kept), bookmarks kept |
| Comments, @mentions, review/approval | ⬜ | M7 (needs a sync service) |
| Real-time co-editing, share links | ⬜ | M7 (optional self-hostable server) |

## 2. Templates and content

Sources: https://www.adobe.com/express/feature · https://www.adobe.com/express/pricing

| Feature | Status | GenHa |
|---|---|---|
| Templates by category (social, marketing, print, documents) | 🟡 | 17 original templates generated in code (`template.list/use`) |
| Template search / filters | 🟡 | category + tags |
| Stock photos, video, audio | ➖/⬜ | no proprietary stock; M6: optional open-content sources (CC0 libraries) and local folders |
| Icons, shapes, design assets | 🟡 | 10 basic shapes + 25 original icons as paths (`library.list/addShape`) |
| Backgrounds (solid, gradient) | ✅ | page background fill |
| Photo grids / collages, frames | 🟡 | `layout.collage` (layouts for 1–9 pictures); crop-to-shape frames |
| Text-to-template, generate similar | ⬜ | M8 (pluggable local/remote AI providers) |

## 3. Text

Sources: https://www.adobe.com/express/feature/design/text-effects/curved · https://helpx.adobe.com/express/web/create-and-customize-text/translate-technical-requirements.html

| Feature | Status | GenHa |
|---|---|---|
| Add text, font, size, colour, alignment, spacing, line height | ✅ | `text.add`, `text.setStyle` |
| Text presets (heading/subheading/body…) | ✅ | 11 presets, `text.addPreset` |
| Shadow, outline, shape-behind-text (highlight) | ✅ | `text.setEffect` |
| Curved / grid text | ⬜ | M3 |
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
| Remove / change background | 🟡 | classic colour-based remover (no ML yet); ML model: M8 |
| Quick actions: resize, crop, convert JPG/PNG/WebP | ✅ | `genha-cli quick …` |
| Convert to SVG (vectorize) | ⬜ | M4 (SVG files can be imported as editable shapes: `image.place`, `file.open`) |
| Generative fill, expand, text-to-image | ⬜ | M8 |
| AI assistant (chat editing) | 🟡 | any MCP-capable assistant can drive every command today |

## 5. Video

Sources: https://helpx.adobe.com/express/create-and-edit-videos/create-videos/add-scenes.html · https://news.adobe.com/news/2025/04/adobe-introduces-new-ai-powered-video-tools-adobe-express

| Feature | Status | GenHa |
|---|---|---|
| Scenes/timeline, trim, split, speed | ⬜ | M6 |
| Animations (in/out/loop) | 🟡 | stored in the model (`Element::animation`); playback/export M6 |
| Audio, voice-over | ⬜ | M6 |
| Captions | ⬜ | M8 |
| MP4 / GIF export | ⬜ | M6 (GIF first: pure Rust) |

## 6. Documents and PDF

Sources: https://helpx.adobe.com/express/using/charts.html · https://helpx.adobe.com/express/using/hosting-projects.html

| Feature | Status | GenHa |
|---|---|---|
| Multi-page PDF export | ✅ | `export.pdf` (raster pages); vector PDF M4 |
| Presentations (pages + notes, present mode) | 🟡 | pages + notes; present mode M5 |
| PDF edit / combine / import | ⬜ | M9 (shared with the PDFCraft sibling) |
| Charts, tables | ⬜ | M5 |
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
| PDF (standard), print PDF with bleed/CMYK | 🟡 | standard; print options M4 |
| MP4 / GIF | ⬜ | M6 |
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
| Desktop apps | ✅ | native Linux/macOS/Windows (egui) |
| Web app | 🟡 | `apps/genha-web`: the same UI in the browser (WASM, WebGL2), verified in Chromium; open/place/drop through the browser, Save and Download as downloads; text only in a built-in fallback face, no Data Recovery or recent designs yet |
| Mobile apps | ⬜ | later |
| Offline use | ✅ | everything runs locally; no account needed; Data Recovery copies unsaved designs every minute (`recovery.*`) |

## 11. Accessibility and localisation

Source: https://www.adobe.com/accessibility/compliance/adobe-express-android-2025-acr.html

| Feature | Status | GenHa |
|---|---|---|
| Keyboard shortcuts | ✅ | per command |
| Screen reader support | ⬜ | M5 (egui AccessKit) |
| UI translations | ⬜ | M5 (German first) |

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-12 | minor | Version history, crop page to content, Data Recovery, recent files, SVG import |
| 2026-10-12 | major | First inventory from public sources and first self-assessment |
