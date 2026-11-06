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

import { memo } from "react";
import type { TemplateDto, TemplateGroupDto } from "../../bridge/hostApi";
import { useT } from "../../i18n";
import {
  formatPeriodLabel,
  formatRunAtDisplay,
  formatTemplateMeta,
} from "./templateCardMeta";
import {
  groupWaveSuccessCount,
  waveFromGroup,
  waveRowClass,
  type GroupWave,
} from "./templatesPageCache";

function ReorderBtns({
  busy,
  onMove,
}: {
  busy: boolean;
  onMove: (dir: "up" | "down") => void;
}) {
  const t = useT();
  return (
    <div className="tpl-reorder">
      <button
        type="button"
        className="reorder-arrow"
        title={t("common.moveUp")}
        disabled={busy}
        onClick={() => onMove("up")}
      >
        ▲
      </button>
      <button
        type="button"
        className="reorder-arrow"
        title={t("common.moveDown")}
        disabled={busy}
        onClick={() => onMove("down")}
      >
        ▼
      </button>
    </div>
  );
}

function TemplateTitleBlock({ template: tpl, running }: { template: TemplateDto; running: boolean }) {
  const period = formatPeriodLabel(tpl);
  const meta = formatTemplateMeta(tpl, running);
  const fullMeta = [meta.lead, meta.status].filter(Boolean).join(" · ");
  return (
    <div className="tpl-card-text">
      <div className="tpl-card-title-row">
        <h2 className="tpl-card-name">{tpl.name}</h2>
        {period ? <span className="tpl-period">{period}</span> : null}
      </div>
      <p className="tpl-card-meta" title={fullMeta || undefined}>
        {meta.lead}
        {meta.lead && meta.status ? " · " : null}
        {meta.status ? (
          <span
            className={"tpl-status" + (meta.statusClass ? ` ${meta.statusClass}` : "")}
            title={meta.status}
          >
            {meta.status}
          </span>
        ) : null}
      </p>
    </div>
  );
}

export type GroupCardHandlers = {
  onExpandGroup: (id: string, expanded: boolean) => void;
  onRunGroup: (id: string, dryRun?: boolean) => void;
  onCancelGroup: (id: string) => void;
  onEditGroup: (id: string) => void;
  onDeleteGroup: (id: string) => void;
  onMoveGroup: (id: string, dir: "up" | "down") => void;
  onEditTemplate: (id: string) => void;
  onDuplicateTemplate: (id: string) => void;
  onRunTemplate: (id: string, dryRun?: boolean) => void;
  onCancelTemplate: (id: string) => void;
  onMoveTemplate: (groupId: string, id: string, dir: "up" | "down") => void;
  onRemoveFromGroup: (id: string) => void;
};

type GroupProps = {
  group: TemplateGroupDto;
  byId: Map<string, TemplateDto>;
  busyId: string | null;
  wave?: GroupWave;
} & GroupCardHandlers;

function GroupCardInner({
  group: g,
  byId,
  busyId,
  wave,
  onExpandGroup,
  onRunGroup,
  onCancelGroup,
  onEditGroup,
  onDeleteGroup,
  onMoveGroup,
  onEditTemplate,
  onDuplicateTemplate,
  onRunTemplate,
  onCancelTemplate,
  onMoveTemplate,
  onRemoveFromGroup,
}: GroupProps) {
  const t = useT();
  const liveWave = waveFromGroup(wave, g);
  const nestedRunning = (g.template_ids ?? []).some((tid) => !!byId.get(tid)?.is_running);
  // Lesson 123: session wave already terminal → do not keep Stop via stale nested is_running.
  const waveSessionDone =
    !!liveWave &&
    !liveWave.inFlight &&
    Object.keys(liveWave.tones).length > 0 &&
    !Object.values(liveWave.tones).some((tone) => tone === "run");
  const groupRunning = waveSessionDone
    ? false
    : !!g.is_running || busyId === g.id || nestedRunning || !!liveWave?.inFlight;
  const running = groupRunning;
  const when = formatRunAtDisplay(g.last_run_at);
  const status = (g.last_run_status ?? "").trim();
  const groupMetaText = running
    ? `${t("templates.card.templatesShort", { count: g.template_ids?.length ?? 0 })} · ${t("templates.card.running")}`
    : when || status
      ? `${t("templates.card.templatesShort", { count: g.template_ids?.length ?? 0 })} · ${[when, status || t("templates.card.ready")].filter(Boolean).join(" · ")}`
      : `${t("templates.card.templatesShort", { count: g.template_ids?.length ?? 0 })} · ${t("templates.card.ready")}`;
  const successCount = !running
    ? groupWaveSuccessCount(liveWave, g.template_ids?.length ?? 0)
    : null;
  return (
    <div className="card card--md card--pad tpl-card">
      <div className="tpl-card-row tpl-card-row--4">
        <button
          type="button"
          className="btn-expand"
          aria-label={g.is_expanded ? t("common.collapse") : t("common.expand")}
          onClick={() => onExpandGroup(g.id, !g.is_expanded)}
        >
          {g.is_expanded ? "−" : "+"}
        </button>
        <div className="tpl-card-text">
          <div className="tpl-card-title-row">
            <h2 className="tpl-card-name">{g.name}</h2>
            {successCount ? (
              <span className="tpl-card-group-success">
                {t("templates.card.groupSuccessCount", {
                  ok: successCount.ok,
                  total: successCount.total,
                })}
              </span>
            ) : null}
          </div>
          <p
            className={
              "tpl-card-meta" +
              (status.startsWith("error") || status === "cancelled"
                ? " is-err"
                : status
                  ? " is-ok"
                  : "")
            }
            title={groupMetaText}
          >
            {groupMetaText}
          </p>
        </div>
        <ReorderBtns busy={!!busyId} onMove={(dir) => onMoveGroup(g.id, dir)} />
        <div className="form-actions form-actions--end">
          {running ? (
            <button
              className="btn btn-danger btn-card"
              type="button"
              onClick={() => onCancelGroup(g.id)}
            >
              {t("templates.card.stopFull")}
            </button>
          ) : (
            <button
              className="btn btn-primary btn-card"
              type="button"
              disabled={!!busyId || !(g.template_ids?.length > 0)}
              title={t("templates.card.runDryHint")}
              onClick={(e) => onRunGroup(g.id, e.shiftKey)}
            >
              {t("templates.card.run")}
            </button>
          )}
          <button
            className="btn btn-outline btn-card"
            type="button"
            disabled={!!busyId}
            onClick={() => onEditGroup(g.id)}
          >
            {t("common.edit")}
          </button>
          <button
            className="btn btn-danger btn-card"
            type="button"
            disabled={!!busyId}
            onClick={() => onDeleteGroup(g.id)}
          >
            {t("common.delete")}
          </button>
        </div>
      </div>
      {g.is_expanded ? (
        <div className="tpl-nested">
          {(g.template_ids ?? []).map((tid) => {
            const tpl = byId.get(tid);
            if (!tpl) return null;
            const tplSoloRunning =
              !groupRunning && (!!tpl.is_running || busyId === tpl.id);
            const tint = waveRowClass(liveWave, tpl.id);
            return (
              <div
                key={tid}
                className={
                  "plate plate--pad tpl-nested-row" + (tint ? ` ${tint}` : "")
                }
              >
                <ReorderBtns
                  busy={!!busyId}
                  onMove={(dir) => onMoveTemplate(g.id, tpl.id, dir)}
                />
                <TemplateTitleBlock
                  template={tpl}
                  running={groupRunning ? false : tplSoloRunning}
                />
                <div className="form-actions form-actions--end">
                  {groupRunning ? (
                    <button
                      className="btn btn-outline btn-card"
                      type="button"
                      disabled
                    >
                      {t("templates.card.run")}
                    </button>
                  ) : tplSoloRunning ? (
                    <button
                      className="btn btn-danger btn-card"
                      type="button"
                      onClick={() => onCancelTemplate(tpl.id)}
                    >
                      {t("templates.card.stop")}
                    </button>
                  ) : (
                    <button
                      className="btn btn-primary btn-card"
                      type="button"
                      disabled={!!busyId}
                      title={t("templates.card.runDryHint")}
                      onClick={(e) => onRunTemplate(tpl.id, e.shiftKey)}
                    >
                      {t("templates.card.run")}
                    </button>
                  )}
                  <button
                    className="btn btn-outline btn-card"
                    type="button"
                    disabled={!!busyId}
                    onClick={() => onEditTemplate(tpl.id)}
                  >
                    {t("common.edit")}
                  </button>
                  <button
                    className="btn btn-outline btn-card"
                    type="button"
                    disabled={!!busyId}
                    onClick={() => onDuplicateTemplate(tpl.id)}
                  >
                    {t("common.copy")}
                  </button>
                  <button
                    className="btn btn-outline btn-card"
                    type="button"
                    disabled={!!busyId}
                    onClick={() => onRemoveFromGroup(tpl.id)}
                  >
                    {t("templates.card.removeFromGroup")}
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}

export const GroupCard = memo(GroupCardInner);

export type StandaloneCardHandlers = {
  onRunTemplate: (id: string, dryRun?: boolean) => void;
  onCancelTemplate: (id: string) => void;
  onEditTemplate: (id: string) => void;
  onDuplicateTemplate: (id: string) => void;
  onDeleteTemplate: (id: string) => void;
  onAddToGroup: (id: string) => void;
  onMoveTemplate: (id: string, dir: "up" | "down") => void;
};

type StandaloneProps = {
  template: TemplateDto;
  busyId: string | null;
  canAddToGroup: boolean;
} & StandaloneCardHandlers;

function StandaloneCardInner({
  template: tpl,
  busyId,
  canAddToGroup,
  onRunTemplate,
  onCancelTemplate,
  onEditTemplate,
  onDuplicateTemplate,
  onDeleteTemplate,
  onAddToGroup,
  onMoveTemplate,
}: StandaloneProps) {
  const t = useT();
  const running = !!tpl.is_running || busyId === tpl.id;
  return (
    <div className="card card--md card--pad tpl-card">
      <div className="tpl-card-row tpl-card-row--3">
        <ReorderBtns busy={!!busyId} onMove={(dir) => onMoveTemplate(tpl.id, dir)} />
        <TemplateTitleBlock template={tpl} running={running} />
        <div className="form-actions form-actions--end">
          {running ? (
            <button
              className="btn btn-danger btn-card"
              type="button"
              onClick={() => onCancelTemplate(tpl.id)}
            >
              {t("templates.card.stopFull")}
            </button>
          ) : (
            <button
              className="btn btn-primary btn-card"
              type="button"
              disabled={!!busyId}
              title={t("templates.card.runDryHint")}
              onClick={(e) => onRunTemplate(tpl.id, e.shiftKey)}
            >
              {t("templates.card.run")}
            </button>
          )}
          <button
            className="btn btn-outline btn-card"
            type="button"
            disabled={!!busyId}
            onClick={() => onEditTemplate(tpl.id)}
          >
            {t("common.edit")}
          </button>
          <button
            className="btn btn-outline btn-card"
            type="button"
            disabled={!!busyId}
            onClick={() => onDuplicateTemplate(tpl.id)}
          >
            {t("common.copy")}
          </button>
          <button
            className="btn btn-outline btn-card"
            type="button"
            disabled={!!busyId || !canAddToGroup}
            onClick={() => onAddToGroup(tpl.id)}
          >
            {t("templates.card.addToGroup")}
          </button>
          <button
            className="btn btn-danger btn-card"
            type="button"
            disabled={!!busyId}
            onClick={() => onDeleteTemplate(tpl.id)}
          >
            {t("common.delete")}
          </button>
        </div>
      </div>
    </div>
  );
}

export const StandaloneCard = memo(StandaloneCardInner);
