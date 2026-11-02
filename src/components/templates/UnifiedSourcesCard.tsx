import { useEffect, useState } from "react";
import type { TemplateDto } from "../../bridge/hostApi";
import SpreadsheetIdField from "./SpreadsheetIdField";
import { useT } from "../../i18n";
import {
  hasUnifiedSources,
  type ScriptVisibility,
} from "./scriptVisibility";

type Props = {
  visibility: ScriptVisibility;
  tpl: TemplateDto;
  disabled?: boolean;
  onPatch: (partial: Partial<TemplateDto>) => void;
  onMessage?: (msg: string | null) => void;
};

type RangeDef = {
  field: keyof TemplateDto;
  label: string;
  placeholder: string;
};

/** Второй spreadsheet + унифицированные диапазоны. */
export default function UnifiedSourcesCard({
  visibility,
  tpl,
  disabled,
  onPatch,
  onMessage,
}: Props) {
  const t = useT();
  const [sheetOptions, setSheetOptions] = useState<string[]>([]);

  useEffect(() => {
    setSheetOptions([]);
  }, [tpl.id, tpl.second_spreadsheet_id]);

  if (!hasUnifiedSources(visibility)) {
    return null;
  }

  const mode = ((tpl.id_mode ?? "").trim() || "sku").toLowerCase();
  const ranges: RangeDef[] = [];

  if (visibility.showCabinetsSourceRange) {
    ranges.push({
      field: "cabinets_source_range",
      label: t("templates.unifiedSources.ranges.cabinets"),
      placeholder: "A2:A",
    });
  }
  if (visibility.showNmIdSourceRange) {
    ranges.push({
      field: "nm_id_source_range",
      label: t("templates.unifiedSources.ranges.nmId"),
      placeholder: "B2:B",
    });
  }
  if (visibility.showStocksSkusSourceRange) {
    ranges.push({
      field: "stocks_skus_source_range",
      label: t("templates.unifiedSources.ranges.chrtId"),
      placeholder: "C2:C",
    });
  }
  if (visibility.showIdAdvCampaignSourceRange) {
    ranges.push({
      field: "id_adv_campaign_source_range",
      label: t("templates.unifiedSources.ranges.advCampaignId"),
      placeholder: "D2:D",
    });
  }
  if (
    visibility.showOfferIdSourceRange ||
    (visibility.showOzonIdRangeByMode && mode === "offer_id")
  ) {
    ranges.push({
      field: "offer_id_source_range",
      label: t("templates.unifiedSources.ranges.offerId"),
      placeholder: "A2:A",
    });
  }
  if (
    visibility.showProductIdSourceRange ||
    (visibility.showOzonIdRangeByMode && mode === "product_id")
  ) {
    ranges.push({
      field: "product_id_source_range",
      label: t("templates.unifiedSources.ranges.productId"),
      placeholder: "B2:B",
    });
  }
  if (
    visibility.showSkuSourceRange ||
    (visibility.showOzonIdRangeByMode && mode === "sku")
  ) {
    ranges.push({
      field: "sku_source_range",
      label: t("templates.unifiedSources.ranges.sku"),
      placeholder: "B2:B",
    });
  }
  if (visibility.showContentMediaLinksSourceRange) {
    ranges.push({
      field: "content_media_links_source_range",
      label: t("templates.unifiedSources.ranges.mediaLinks"),
      placeholder: "D2:D",
    });
  }
  if (visibility.showSubjectIdSourceRange) {
    ranges.push({
      field: "subject_id_source_range",
      label: t("templates.unifiedSources.ranges.subjectId"),
      placeholder: "A2:A",
    });
  }
  if (visibility.showSubjectNameSourceRange) {
    ranges.push({
      field: "subject_name_source_range",
      label: t("templates.unifiedSources.ranges.subjectName"),
      placeholder: "B2:B",
    });
  }
  // Диапазоны правки карточек — только для включённых режимов (тумблеры).
  if (visibility.showCardsUpdateRanges) {
    if (tpl.cards_update_dimensions_mode) {
      ranges.push(
        {
          field: "cards_update_length_range",
          label: t("templates.unifiedSources.ranges.length"),
          placeholder: "C2:C",
        },
        {
          field: "cards_update_width_range",
          label: t("templates.unifiedSources.ranges.width"),
          placeholder: "D2:D",
        },
        {
          field: "cards_update_height_range",
          label: t("templates.unifiedSources.ranges.height"),
          placeholder: "E2:E",
        },
      );
    }
    if (tpl.cards_update_vendor_code_mode) {
      ranges.push({
        field: "cards_update_vendor_code_range",
        label: t("templates.unifiedSources.ranges.vendorCode"),
        placeholder: "C2:C",
      });
    }
    if (tpl.cards_update_vat_mode) {
      ranges.push({
        field: "cards_update_vat_range",
        label: t("templates.unifiedSources.ranges.vat"),
        placeholder: "F2:F",
      });
    }
    if (tpl.cards_update_brand_mode) {
      ranges.push({
        field: "cards_update_brand_range",
        label: t("templates.unifiedSources.ranges.brand"),
        placeholder: "G2:G",
      });
    }
  }
  if (visibility.showPricesUploadRanges) {
    ranges.push(
      {
        field: "prices_price_range",
        label: t("templates.unifiedSources.ranges.prices"),
        placeholder: "C2:C",
      },
      {
        field: "prices_discount_range",
        label: t("templates.unifiedSources.ranges.discounts"),
        placeholder: "D2:D",
      },
      {
        field: "prices_exclude_range",
        label: t("templates.unifiedSources.ranges.nmExclude"),
        placeholder: "E2:E",
      },
    );
  }
  if (visibility.showPriceSetRanges) {
    ranges.push(
      {
        field: "price_set_price_range",
        label: t("templates.unifiedSources.ranges.pricesOptional"),
        placeholder: "C2:C",
      },
      {
        field: "price_set_discount_range",
        label: t("templates.unifiedSources.ranges.discountsOptional"),
        placeholder: "D2:D",
      },
    );
  }
  if (visibility.showPricesHistoryUploadIdRange) {
    ranges.push({
      field: "prices_history_task_upload_id_range",
      label: t("templates.unifiedSources.ranges.uploadId"),
      placeholder: "B2:B",
    });
  }
  if (visibility.showPaidStorageStatusRanges) {
    ranges.push(
      {
        field: "paid_storage_status_task_id_range",
        label: t("templates.unifiedSources.ranges.taskIdSource"),
        placeholder: "B2:B",
      },
      {
        field: "paid_storage_status_output_range",
        label: t("templates.unifiedSources.ranges.statusWriteRange"),
        placeholder: "C2:C",
      },
    );
  }
  if (visibility.showPaidStorageDownloadTaskIdRange) {
    ranges.push({
      field: "paid_storage_download_task_id_range",
      label: t("templates.unifiedSources.ranges.taskId"),
      placeholder: "B2:B",
    });
  }

  const patchRange = (field: keyof TemplateDto, value: string) => {
    const partial: Partial<TemplateDto> = { [field]: value };
    if (
      field === "offer_id_source_range" ||
      field === "product_id_source_range" ||
      field === "sku_source_range"
    ) {
      partial.id_source_range = value;
    }
    onPatch(partial);
  };

  return (
    <div className="tpl-dialog-card">
      <h3 className="tpl-dialog-card-header">{t("templates.unifiedSources.title")}</h3>
      {visibility.secondHint ? <p className="tpl-info">{visibility.secondHint}</p> : null}

      {visibility.showSecondSpreadsheet ? (
        <SpreadsheetIdField
          idPrefix="tpl-src"
          spreadsheetIdLabel={t("templates.spreadsheet.sourceTableIdOrUrl")}
          sheetNameLabel={t("templates.spreadsheet.sourceSheet")}
          spreadsheetId={tpl.second_spreadsheet_id ?? ""}
          sheetName={tpl.second_sheet_name ?? ""}
          spreadsheetTitle={tpl.second_spreadsheet_title ?? ""}
          sheetOptions={sheetOptions}
          disabled={disabled}
          onSpreadsheetIdChange={(id) => {
            onPatch({ second_spreadsheet_id: id });
            setSheetOptions([]);
          }}
          onSheetNameChange={(name) => onPatch({ second_sheet_name: name })}
          onSheetOptionsChange={setSheetOptions}
          onSpreadsheetTitleChange={(title) => onPatch({ second_spreadsheet_title: title })}
          onMessage={onMessage}
        />
      ) : null}

      {ranges.length > 0 ? (
        <>
          <div className="tpl-subtitle tpl-subtitle--ranges">{t("templates.unifiedSources.rangesTitle")}</div>
          {ranges.map((r) => (
            <div key={String(r.field)} className="field">
              <label htmlFor={`tpl-range-${String(r.field)}`}>{r.label}</label>
              <input
                id={`tpl-range-${String(r.field)}`}
                className="input"
                value={String(tpl[r.field] ?? "")}
                disabled={disabled}
                placeholder={r.placeholder}
                onChange={(e) => patchRange(r.field, e.target.value)}
              />
            </div>
          ))}
        </>
      ) : null}
    </div>
  );
}
