// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import type { ReactNode } from "react";
import { t } from "../../../i18n";

type Props = {
  title: string;
  /** Бренд-акцент плашки: WB magenta / Ozon blue. */
  tone?: "wb" | "ozon";
  count: number;
  open: boolean;
  onToggle: () => void;
  refreshLabel: string;
  refreshBusy: boolean;
  refreshDisabled: boolean;
  refreshTitle: string;
  onRefresh: () => void;
  showRefresh?: boolean;
  hint: string;
  children: ReactNode;
};

export function CalculatorTablesPlaque({
  title,
  tone = "wb",
  count,
  open,
  onToggle,
  refreshLabel,
  refreshBusy,
  refreshDisabled,
  refreshTitle,
  onRefresh,
  showRefresh = true,
  hint,
  children,
}: Props) {
  const toneClass = tone === "ozon" ? " wb-tables-plaque--ozon" : " wb-tables-plaque--wb";
  return (
    <section
      className={
        `wb-tables-plaque${toneClass}` +
        (open ? "" : " wb-tables-plaque--collapsed")
      }
    >
      <div className="wb-tables-plaque-bar">
        <button
          type="button"
          className="input-trigger input-trigger--plaque wb-tables-plaque-head"
          aria-expanded={open}
          onClick={onToggle}
        >
          <span className="wb-tables-plaque-title">{title}</span>
          <span className="wb-tables-plaque-meta">{count}</span>
          <span className="wb-tables-plaque-chevron" aria-hidden>
            {open ? "▾" : "▸"}
          </span>
        </button>
        {showRefresh && (
          <button
            type="button"
            className="btn btn-primary btn-sm wb-tables-plaque-refresh"
            disabled={refreshDisabled}
            title={refreshTitle}
            onClick={(e) => {
              e.stopPropagation();
              onRefresh();
            }}
          >
            {refreshBusy ? t("common.updating") : refreshLabel}
          </button>
        )}
      </div>
      {open && (
        <>
          <p className="muted calc-hint">{hint}</p>
          {children}
        </>
      )}
    </section>
  );
}
