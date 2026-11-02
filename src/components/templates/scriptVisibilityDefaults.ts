import { t } from "../../i18n";

/** Visibility редактора шаблонов по registry. */


export type ScriptVisibility = {
  showSecondSpreadsheet: boolean;
  /** Унифицированные диапазоны (UnifiedSourcesCard). */
  showCabinetsSourceRange: boolean;
  showNmIdSourceRange: boolean;
  /** chrtId (`StocksSkusSourceRange`). */
  showStocksSkusSourceRange: boolean;
  /** ID РК (`IdAdvCampaignSourceRange`). */
  showIdAdvCampaignSourceRange: boolean;
  showOfferIdSourceRange: boolean;
  showProductIdSourceRange: boolean;
  showSkuSourceRange: boolean;
  showContentMediaLinksSourceRange: boolean;
  showSubjectIdSourceRange: boolean;
  showSubjectNameSourceRange: boolean;
  showCardsUpdateRanges: boolean;
  showCardsMoveTrashHint: boolean;
  showCardsUpdateModes: boolean;
  showSubjectsFilters: boolean;
  showTnvedCustomIds: boolean;
  showObjectCharcsCustomIds: boolean;
  showPromotionsAllPromo: boolean;
  showPricesUploadRanges: boolean;
  showPriceSetRanges: boolean;
  showPricesHistoryUploadIdRange: boolean;
  showPaidStorageStatusRanges: boolean;
  showPaidStorageDownloadTaskIdRange: boolean;
  showSymbolsCheckCard: boolean;
  showCellsCountCard: boolean;
  showReportPeriod: boolean;
  /**
   * Ozon: показать диапазон под выбранный `id_mode`
   * (offer/product/sku), если статические Show* выключены.
   */
  showOzonIdRangeByMode: boolean;
  /** Карточка выбора sku / offer_id / product_id. */
  showOzonIdMode: boolean;
  /** Период DateFrom–DateTo + пресеты. */
  requiresDateRange: boolean;
  /** Одна дата (tariffs_box → date_from). */
  requiresSingleDate: boolean;
  hideCabinets: boolean;
  hideFields: boolean;
  hideWriteSettings: boolean;
  /** Скрыть карточку «Создать Excel файл» (мутации без табличной выгрузки). */
  hideExportToExcelCard: boolean;
  /** Заголовок блока spreadsheet: источник вместо «куда писать». */
  sheetAsSource: boolean;
  secondHint?: string;
};

export const DEFAULT: ScriptVisibility = {
  showSecondSpreadsheet: false,
  showCabinetsSourceRange: false,
  showNmIdSourceRange: false,
  showStocksSkusSourceRange: false,
  showIdAdvCampaignSourceRange: false,
  showOfferIdSourceRange: false,
  showProductIdSourceRange: false,
  showSkuSourceRange: false,
  showContentMediaLinksSourceRange: false,
  showSubjectIdSourceRange: false,
  showSubjectNameSourceRange: false,
  showCardsUpdateRanges: false,
  showCardsMoveTrashHint: false,
  showCardsUpdateModes: false,
  showSubjectsFilters: false,
  showTnvedCustomIds: false,
  showObjectCharcsCustomIds: false,
  showPromotionsAllPromo: false,
  showPricesUploadRanges: false,
  showPriceSetRanges: false,
  showPricesHistoryUploadIdRange: false,
  showPaidStorageStatusRanges: false,
  showPaidStorageDownloadTaskIdRange: false,
  showSymbolsCheckCard: false,
  showCellsCountCard: false,
  showReportPeriod: false,
  showOzonIdRangeByMode: false,
  showOzonIdMode: false,
  requiresDateRange: false,
  requiresSingleDate: false,
  hideCabinets: false,
  hideFields: false,
  hideWriteSettings: false,
  hideExportToExcelCard: false,
  sheetAsSource: false,
};

/** Ozon ID-источник: Second + режим ID + диапазон под режим (Offer/Product/Sku). */
export const OZON_ID_SOURCE: ScriptVisibility = {
  ...DEFAULT,
  showSecondSpreadsheet: true,
  showOzonIdMode: true,
  showOzonIdRangeByMode: true,
  secondHint:
    t("templates.scriptHints.ozonIdSource"),
};

export const MUTATE_FROM_SHEET: ScriptVisibility = {
  ...DEFAULT,
  hideCabinets: true,
  hideFields: true,
  hideWriteSettings: true,
  hideExportToExcelCard: true,
  sheetAsSource: true,
  showCabinetsSourceRange: true,
  showNmIdSourceRange: true,
};
