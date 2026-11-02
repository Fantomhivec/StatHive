import { invoke } from "@tauri-apps/api/core";
import type {
  TemplateDto,
  TemplateField,
  TemplateGroupDto,
  TemplatesSnapshot,
} from "./hostApi.types";

/** Templates invoke helpers for hostApi barrel. */
export const hostApiTemplates = {
  templatesSnapshot: (marketplace: "wb" | "ozon") =>
    invoke<TemplatesSnapshot>("templates_snapshot", { args: { marketplace } }),
  templatesGet: (
    id?: string,
    scriptKey?: string,
    marketplace: "wb" | "ozon" = "wb",
    tabName = "",
  ) =>
    invoke<{ template: TemplateDto }>("templates_get", {
      args: {
        id: id ?? "",
        script_key: scriptKey ?? "",
        marketplace,
        tab_name: tabName,
      },
    }),
  templatesSave: (template: TemplateDto) =>
    invoke<{ template: TemplateDto }>("templates_save", { template }),
  templatesSetSpreadsheetTitle: (id: string, spreadsheetTitle: string) =>
    invoke<{ ok: boolean }>("templates_set_spreadsheet_title", {
      args: { id, spreadsheet_title: spreadsheetTitle },
    }),
  templatesSetSpreadsheetTitles: (
    items: { id: string; spreadsheetTitle: string }[],
  ) =>
    invoke<{ ok: boolean }>("templates_set_spreadsheet_titles", {
      args: {
        items: items.map((i) => ({
          id: i.id,
          spreadsheet_title: i.spreadsheetTitle,
        })),
      },
    }),
  templatesDelete: (id: string) =>
    invoke<{ ok: boolean }>("templates_delete", { args: { id } }),
  templatesListFields: (
    scriptKey: string,
    selectedFields: TemplateField[] = [],
  ) =>
    invoke<{ fields: TemplateField[] }>("templates_list_fields", {
      args: { script_key: scriptKey, selected_fields: selectedFields },
    }),
  templatesRun: (id: string, dryRun = false) =>
    invoke<{ ok: boolean; rows_written: number; message: string; dry_run?: boolean }>(
      "templates_run",
      {
        args: { id, dry_run: dryRun },
      },
    ),
  templatesRunGroup: (id: string, dryRun = false) =>
    invoke<{ ok: boolean; rows_written: number; message: string; dry_run?: boolean }>(
      "templates_run_group",
      {
        args: { id, dry_run: dryRun },
      },
    ),
  templatesCancel: (
    id: string,
    entityType: "template" | "group" = "template",
  ) =>
    invoke<{ ok: boolean }>("templates_cancel", {
      args: { id, entity_type: entityType },
    }),
  templatesSelectTab: (marketplace: "wb" | "ozon", name: string) =>
    invoke<TemplatesSnapshot>("templates_select_tab", {
      args: { marketplace, name },
    }),
  templatesAddTab: (marketplace: "wb" | "ozon", name: string) =>
    invoke<TemplatesSnapshot>("templates_add_tab", {
      args: { marketplace, name },
    }),
  templatesRenameTab: (
    marketplace: "wb" | "ozon",
    from: string,
    to: string,
  ) =>
    invoke<TemplatesSnapshot>("templates_rename_tab", {
      args: { marketplace, from, to },
    }),
  templatesDeleteTab: (marketplace: "wb" | "ozon", name: string) =>
    invoke<TemplatesSnapshot>("templates_delete_tab", {
      args: { marketplace, name },
    }),
  templatesSaveGroup: (group: TemplateGroupDto) =>
    invoke<{ group: TemplateGroupDto }>("templates_save_group", { group }),
  templatesDeleteGroup: (id: string) =>
    invoke<{ ok: boolean }>("templates_delete_group", { args: { id } }),
  templatesSetGroupExpanded: (id: string, expanded: boolean) =>
    invoke<{ ok: boolean }>("templates_set_group_expanded", {
      args: { id, expanded },
    }),
  templatesAddToGroup: (templateId: string, groupId: string) =>
    invoke<{ ok: boolean }>("templates_add_to_group", {
      args: { template_id: templateId, group_id: groupId },
    }),
  templatesMove: (
    templateId: string,
    direction: "up" | "down",
    groupId?: string,
  ) =>
    invoke<{ ok: boolean }>("templates_move", {
      args: {
        template_id: templateId,
        direction,
        group_id: groupId ?? "",
      },
    }),
  templatesMoveGroup: (groupId: string, direction: "up" | "down") =>
    invoke<{ ok: boolean }>("templates_move_group", {
      args: { group_id: groupId, direction },
    }),
  templatesRemoveFromGroup: (templateId: string) =>
    invoke<{ ok: boolean }>("templates_remove_from_group", {
      args: { id: templateId },
    }),
};
