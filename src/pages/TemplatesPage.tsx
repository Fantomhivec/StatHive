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

import { lazy, Suspense } from "react";
import { hostApi } from "../bridge/hostApi";
import { TPL_TAB_COLOR_IDS } from "../bridge/hostApi.types";
import { GroupCard, StandaloneCard } from "./templates/TemplateCards";
import { useTemplatesActions } from "./templates/useTemplatesActions";
import "./TemplatesPage.css";

const TemplateEditorDialog = lazy(() => import("../components/templates/dialogs/TemplateEditorDialog"));
const TemplateGroupEditorDialog = lazy(
  () => import("../components/templates/dialogs/TemplateGroupEditorDialog"),
);
const GroupPickDialog = lazy(() => import("../components/templates/dialogs/GroupPickDialog"));

function isSystemTab(name: string, t: (key: string) => string): boolean {
  const n = name.trim();
  return n === t("templates.defaultTab") || n === t("templates.calculatorTab");
}

function tabColorLabel(id: string, t: (key: string) => string): string {
  switch (id) {
    case "teal":
      return t("templates.tabColorTeal");
    case "blue":
      return t("templates.tabColorBlue");
    case "violet":
      return t("templates.tabColorViolet");
    case "coral":
      return t("templates.tabColorCoral");
    case "amber":
      return t("templates.tabColorAmber");
    case "green":
      return t("templates.tabColorGreen");
    case "rose":
      return t("templates.tabColorRose");
    default:
      return id;
  }
}

export default function TemplatesPage() {
  const a = useTemplatesActions();
  const { t, snap, market } = a;
  const title = market === "wb" ? t("templates.titleWb") : t("templates.titleOzon");

  return (
    <div>
      <div className="tpl-page-head">
        <h1 className="page-title u-m-0">{title}</h1>
        <div className="form-actions tpl-toolbar">
          <button
            className="btn"
            type="button"
            disabled={!!a.busyId}
            onClick={() => a.setGroupEditor({ groupId: null })}
          >
            {t("templates.addGroup")}
          </button>
          <button className="btn btn-primary" type="button" onClick={() => a.setEditorId(null)}>
            {t("templates.addTemplate")}
          </button>
        </div>
      </div>

      <div className="tpl-market-strip" role="tablist">
        <button
          type="button"
          className={"btn-market" + (market === "wb" ? " is-active" : "")}
          onClick={() => a.setMarket("wb")}
        >
          WB
        </button>
        <button
          type="button"
          className={"btn-market" + (market === "ozon" ? " is-active" : "")}
          onClick={() => a.setMarket("ozon")}
        >
          OZON
        </button>
      </div>

      <div className="tpl-tabs-strip">
        {snap.tabs.map((tabName) => {
          const color = snap.tab_colors?.[tabName]?.trim() || undefined;
          return (
          <button
            key={tabName}
            type="button"
            className={"btn-tab" + (tabName === snap.selected_tab ? " is-active" : "")}
            data-tab-color={color}
            onClick={() => void a.withReload(() => hostApi.templatesSelectTab(market, tabName))}
            onContextMenu={(e) => {
              e.preventDefault();
              a.setTabMenu({ name: tabName, x: e.clientX, y: e.clientY });
            }}
          >
            {tabName}
          </button>
          );
        })}
        <button
          type="button"
          className="btn-tab btn-tab-add"
          onClick={() => {
            void (async () => {
              const name = await a.prompt(
                t("templates.tabNamePrompt"),
                t("templates.newTabPromptTitle"),
              );
              if (!name?.trim()) return;
              await a.withReload(() => hostApi.templatesAddTab(market, name.trim()));
            })();
          }}
        >
          +
        </button>
      </div>

      {a.error ? <p className="error">{a.error}</p> : null}
      {a.msg ? <p className="ok-text">{a.msg}</p> : null}

      {market === "ozon" && snap.templates.length === 0 && snap.groups.length === 0 ? (
        <div className="card card--pad stub-card stub-card--text u-mt-8">
          {t("templates.noOzonTemplates")}
        </div>
      ) : null}

      <section className="section-tint tpl-groups-block" aria-label={t("templates.groupsAria")}>
        <div className="tpl-section-label">{t("templates.groupsAria")}</div>
        <div className="tpl-groups">
          {snap.groups.length === 0 ? <p className="page-sub u-m-0">{t("templates.noGroups")}</p> : null}
          {snap.groups.map((g) => (
            <GroupCard
              key={g.id}
              group={g}
              byId={a.byId}
              busyId={a.busyId}
              wave={a.groupWaves[g.id]}
              onExpandGroup={a.onExpandGroup}
              onRunGroup={a.runGroup}
              onCancelGroup={a.onCancelGroup}
              onEditGroup={a.onEditGroup}
              onDeleteGroup={a.removeGroup}
              onMoveGroup={a.onMoveGroup}
              onEditTemplate={a.onEditTemplate}
              onDuplicateTemplate={a.duplicateTemplate}
              onRunTemplate={a.run}
              onCancelTemplate={a.onCancelTemplate}
              onMoveTemplate={a.onMoveTemplateInGroup}
              onRemoveFromGroup={a.onRemoveFromGroup}
            />
          ))}
        </div>
      </section>

      <div className="tpl-section-label">{t("nav.templates")}</div>
      <div className="tpl-standalone">
        {snap.standalone.length === 0 ? (
          <p className="page-sub">{t("templates.noUngrouped")}</p>
        ) : (
          snap.standalone.map((tpl) => (
            <StandaloneCard
              key={tpl.id}
              template={tpl}
              busyId={a.busyId}
              canAddToGroup={snap.groups.length > 0}
              onRunTemplate={a.run}
              onCancelTemplate={a.onCancelTemplate}
              onEditTemplate={a.onEditTemplate}
              onDuplicateTemplate={a.duplicateTemplate}
              onDeleteTemplate={a.remove}
              onAddToGroup={a.onAddToGroup}
              onMoveTemplate={a.onMoveStandalone}
            />
          ))
        )}
      </div>

      {a.editorOpen ? (
        <Suspense fallback={null}>
          <TemplateEditorDialog
            templateId={a.editorId}
            marketplace={market}
            tabName={snap.selected_tab}
            tabs={snap.tabs}
            onClose={() => a.setEditorId(undefined)}
            onSaved={() => void a.reloadSnap()}
          />
        </Suspense>
      ) : null}

      {a.groupEditor ? (
        <Suspense fallback={null}>
          <TemplateGroupEditorDialog
            marketplace={market}
            tabName={snap.selected_tab}
            group={a.editingGroup}
            tabTemplates={snap.templates}
            onClose={() => a.setGroupEditor(null)}
            onSaved={() => void a.reloadSnap()}
          />
        </Suspense>
      ) : null}

      {a.groupPick ? (
        <Suspense fallback={null}>
          <GroupPickDialog
            groups={snap.groups}
            onCancel={() => a.setGroupPick(null)}
            onSelect={(groupId) => {
              const tid = a.groupPick!.templateId;
              a.setGroupPick(null);
              void a.withReload(async () => {
                await hostApi.templatesAddToGroup(tid, groupId);
                return hostApi.templatesSnapshot(market);
              });
            }}
          />
        </Suspense>
      ) : null}

      {a.tabMenu ? (
        <div
          className="popover tpl-ctx-menu"
          style={{ left: a.tabMenu.x, top: a.tabMenu.y }}
          onClick={(e) => e.stopPropagation()}
        >
          <div className="tpl-tab-color-block">
            <div className="tpl-tab-color-label">{t("templates.tabColor")}</div>
            <div className="tpl-tab-swatches" role="group" aria-label={t("templates.tabColor")}>
              <button
                type="button"
                className={
                  "tpl-tab-swatch tpl-tab-swatch--none" +
                  (!(snap.tab_colors?.[a.tabMenu.name]?.trim())
                    ? " is-selected"
                    : "")
                }
                title={t("templates.tabColorNone")}
                aria-label={t("templates.tabColorNone")}
                onClick={() => {
                  const name = a.tabMenu!.name;
                  a.setTabMenu(null);
                  void a.withReload(() =>
                    hostApi.templatesSetTabColor(market, name, ""),
                  );
                }}
              />
              {TPL_TAB_COLOR_IDS.map((id) => {
                const selected =
                  (snap.tab_colors?.[a.tabMenu!.name] ?? "").trim() === id;
                return (
                  <button
                    key={id}
                    type="button"
                    className={"tpl-tab-swatch" + (selected ? " is-selected" : "")}
                    data-tab-color={id}
                    title={tabColorLabel(id, t)}
                    aria-label={tabColorLabel(id, t)}
                    onClick={() => {
                      const name = a.tabMenu!.name;
                      a.setTabMenu(null);
                      void a.withReload(() =>
                        hostApi.templatesSetTabColor(market, name, id),
                      );
                    }}
                  />
                );
              })}
            </div>
          </div>
          {!isSystemTab(a.tabMenu.name, t) ? (
            <>
              <button
                type="button"
                className="menu-item"
                onClick={() => {
                  const from = a.tabMenu!.name;
                  a.setTabMenu(null);
                  void (async () => {
                    const name = await a.prompt(
                      t("templates.renameTabPrompt"),
                      t("templates.renameTabTitle"),
                      from,
                    );
                    if (!name?.trim() || name.trim() === from) return;
                    await a.withReload(() =>
                      hostApi.templatesRenameTab(market, from, name.trim()),
                    );
                  })();
                }}
              >
                {t("common.rename")}
              </button>
              <button
                type="button"
                className="menu-item"
                onClick={() => {
                  const name = a.tabMenu!.name;
                  a.setTabMenu(null);
                  void (async () => {
                    if (name === snap.default_tab || snap.tabs.length <= 1) {
                      await a.alert(
                        t("templates.deleteMainTabError"),
                        t("common.error"),
                        "error",
                      );
                      return;
                    }
                    if (
                      !(await a.confirm(
                        t("templates.deleteTabConfirm", { name }),
                        t("dialogs.confirm"),
                      ))
                    )
                      return;
                    await a.withReload(() =>
                      hostApi.templatesDeleteTab(market, name),
                    );
                  })();
                }}
              >
                {t("common.delete")}
              </button>
            </>
          ) : null}
        </div>
      ) : null}

      {a.dialogs}
    </div>
  );
}
