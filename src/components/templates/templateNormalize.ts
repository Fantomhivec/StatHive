import type { TemplateDto } from "../../bridge/hostApi";
import { calculateDateRangeFromTemplate } from "./dateTemplates";
import { visibilityForScript } from "./scriptVisibility";

export type FieldRow = { name: string; title: string; selected: boolean };

/** Normalize PascalCase / camelCase field rows from host JSON. */
export function normalizeFieldRow(raw: unknown): FieldRow | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const name = String(o.name ?? o.Name ?? "").trim();
  if (!name) return null;
  const title = String(o.title ?? o.Title ?? name);
  const selectedRaw = o.selected ?? o.Selected;
  const selected = selectedRaw !== false && selectedRaw !== "false";
  return { name, title, selected };
}

export function normalizeFieldRows(list: unknown): FieldRow[] {
  if (!Array.isArray(list)) return [];
  return list.map(normalizeFieldRow).filter((x): x is FieldRow => !!x);
}

export function normalizeTpl(
  t: TemplateDto,
  marketplace: string,
  tabName: string,
): TemplateDto {
  const next: TemplateDto = {
    ...t,
    selected_fields: normalizeFieldRows(t.selected_fields),
    marketplace,
    tab_name: tabName,
    time_triggers: t.time_triggers ?? [],
    spreadsheet_id: t.spreadsheet_id ?? "",
    sheet_name: t.sheet_name ?? "",
    export_to_excel: !!t.export_to_excel,
    spreadsheet_title: t.spreadsheet_title ?? "",
    second_spreadsheet_id: t.second_spreadsheet_id ?? "",
    second_sheet_name: t.second_sheet_name ?? "",
    second_spreadsheet_title: t.second_spreadsheet_title ?? "",
    id_mode: t.id_mode ?? "",
    id_source_range: t.id_source_range ?? "",
    cabinets_source_range: t.cabinets_source_range ?? "",
    nm_id_source_range: t.nm_id_source_range ?? "",
    stocks_skus_source_range: t.stocks_skus_source_range ?? "",
    id_adv_campaign_source_range: t.id_adv_campaign_source_range ?? "",
    offer_id_source_range: t.offer_id_source_range ?? "",
    product_id_source_range: t.product_id_source_range ?? "",
    sku_source_range: t.sku_source_range ?? "",
    content_media_links_source_range: t.content_media_links_source_range ?? "",
    subject_id_source_range: t.subject_id_source_range ?? "",
    subject_name_source_range: t.subject_name_source_range ?? "",
    use_custom_subject_ids: !!t.use_custom_subject_ids,
    promotions_all_promo: !!t.promotions_all_promo,
    cards_update_dimensions_mode: t.cards_update_dimensions_mode !== false,
    cards_update_vendor_code_mode: !!t.cards_update_vendor_code_mode,
    cards_update_vat_mode: !!t.cards_update_vat_mode,
    cards_update_brand_mode: !!t.cards_update_brand_mode,
    cards_update_length_range: t.cards_update_length_range ?? "",
    cards_update_width_range: t.cards_update_width_range ?? "",
    cards_update_height_range: t.cards_update_height_range ?? "",
    cards_update_vendor_code_range: t.cards_update_vendor_code_range ?? "",
    cards_update_vat_range: t.cards_update_vat_range ?? "",
    cards_update_brand_range: t.cards_update_brand_range ?? "",
    subjects_filters: t.subjects_filters ?? {
      locale: "ru",
      name: "",
      limit: 30,
      offset: 0,
      parent_id: null,
    },
    date_from: t.date_from ?? "",
    date_to: t.date_to ?? "",
    date_template_type: t.date_template_type ?? "",
    sync_settings: t.sync_settings ?? {
      enable_change_sync: false,
      key_field_1: "",
      key_field_2: "",
      do_not_delete_old_rows: false,
    },
    write_sort_field_name: t.write_sort_field_name ?? "",
    write_sort_direction:
      (t.write_sort_direction ?? "asc").toLowerCase() === "desc" ? "desc" : "asc",
    clear_only_written_range: !!t.clear_only_written_range,
  };
  const mode = (next.id_mode ?? "").trim().toLowerCase() || "sku";
  const legacy = (next.id_source_range ?? "").trim();
  if (legacy) {
    if (mode === "offer_id" && !(next.offer_id_source_range ?? "").trim()) {
      next.offer_id_source_range = legacy;
    } else if (mode === "product_id" && !(next.product_id_source_range ?? "").trim()) {
      next.product_id_source_range = legacy;
    } else if (!(next.sku_source_range ?? "").trim()) {
      next.sku_source_range = legacy;
    }
  }
  const tag = (next.date_template_type ?? "").trim();
  if (tag) {
    const range = calculateDateRangeFromTemplate(tag);
    const vis = visibilityForScript(next.script_key);
    if (vis.requiresSingleDate) {
      next.date_from = range.to;
      next.date_to = "";
    } else if (vis.requiresDateRange) {
      next.date_from = range.from;
      next.date_to = range.to;
    }
  }
  return next;
}
