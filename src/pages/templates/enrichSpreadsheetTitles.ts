import { hostApi, type TemplateDto, type TemplatesSnapshot } from "../../bridge/hostApi";

/** Session cache: spreadsheet_id → title (avoid re-hitting Google on every tab/market switch). */
const titleCache = new Map<string, string>();

/**
 * Для шаблонов с spreadsheet_id без названия — запросить title у Google
 * и сохранить в шаблон (карточки показывают имя документа, не ID).
 * Не блокирует первый рендер: вызывающий показывает snap сразу, затем патчит.
 */
export async function enrichSpreadsheetTitles(
  snap: TemplatesSnapshot,
): Promise<TemplatesSnapshot> {
  const list = snap.templates ?? [];
  const need = list.filter(
    (t) => (t.spreadsheet_id ?? "").trim() && !(t.spreadsheet_title ?? "").trim(),
  );
  if (need.length === 0) return snap;

  const bySid = new Map<string, TemplateDto[]>();
  for (const t of need) {
    const sid = t.spreadsheet_id.trim();
    const arr = bySid.get(sid) ?? [];
    arr.push(t);
    bySid.set(sid, arr);
  }

  const titleBySid = new Map<string, string>();
  const missingSids: string[] = [];
  for (const sid of bySid.keys()) {
    const cached = titleCache.get(sid);
    if (cached) titleBySid.set(sid, cached);
    else missingSids.push(sid);
  }

  // Cap parallel Google calls (same idea as CABINET_PARALLELISM) to avoid 429.
  const TITLE_CHUNK = 3;
  for (let i = 0; i < missingSids.length; i += TITLE_CHUNK) {
    const chunk = missingSids.slice(i, i + TITLE_CHUNK);
    await Promise.all(
      chunk.map(async (sid) => {
        try {
          const res = await hostApi.googleListSheets(sid);
          const title = (res.title ?? "").trim();
          if (title) {
            titleCache.set(sid, title);
            titleBySid.set(sid, title);
          }
        } catch {
          /* OAuth / сеть — карточка останется без названия */
        }
      }),
    );
  }
  if (titleBySid.size === 0) return snap;

  const persistItems: { id: string; spreadsheetTitle: string }[] = [];
  for (const t of need) {
    const title = titleBySid.get(t.spreadsheet_id.trim());
    if (!title) continue;
    persistItems.push({ id: t.id, spreadsheetTitle: title });
  }
  if (persistItems.length > 0) {
    try {
      await hostApi.templatesSetSpreadsheetTitles(persistItems);
    } catch {
      /* ignore — UI всё равно получит title из патча */
    }
  }

  const patch = (t: TemplateDto): TemplateDto => {
    if ((t.spreadsheet_title ?? "").trim()) return t;
    const title = titleBySid.get((t.spreadsheet_id ?? "").trim());
    return title ? { ...t, spreadsheet_title: title } : t;
  };

  return {
    ...snap,
    templates: (snap.templates ?? []).map(patch),
    standalone: (snap.standalone ?? []).map(patch),
  };
}
