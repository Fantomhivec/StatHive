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

/**
 * Session group-run wave (row tints). Lessons 115–121, 123.
 * Paint CSS only from tones; drive tones via progress events and/or hydrate cursor.
 */

import type { TemplateGroupDto } from "../../bridge/hostApi";
import {
  getTemplatesSnapCache,
  type TemplatesMarket,
} from "../../bridge/snapCache";

export type TemplatesRunProgress = {
  entityType: string;
  id: string;
  lastRunStatus?: string;
  isRunning?: boolean;
  lastRunAt?: string | null;
  activeTemplateId?: string | null;
  groupId?: string | null;
};

/** Accept camelCase (emit) and snake_case if any layer renames. */
export function normalizeRunProgress(
  raw: TemplatesRunProgress | Record<string, unknown>,
): TemplatesRunProgress {
  const p = raw as Record<string, unknown>;
  const bool = (v: unknown): boolean | undefined => {
    if (typeof v === "boolean") return v;
    if (v === "true" || v === 1) return true;
    if (v === "false" || v === 0) return false;
    return undefined;
  };
  const str = (v: unknown): string | undefined =>
    typeof v === "string" ? v : v == null ? undefined : String(v);
  return {
    entityType: str(p.entityType ?? p.entity_type) ?? "",
    id: str(p.id) ?? "",
    lastRunStatus: str(p.lastRunStatus ?? p.last_run_status),
    isRunning: bool(p.isRunning ?? p.is_running),
    lastRunAt: (p.lastRunAt ?? p.last_run_at) as string | null | undefined,
    activeTemplateId: str(p.activeTemplateId ?? p.active_template_id) ?? null,
    groupId: str(p.groupId ?? p.group_id) ?? null,
  };
}

export type WaveTone = "run" | "ok" | "err";
export type GroupWave = {
  inFlight: boolean;
  memberIds: string[];
  /** Sole source for plate--wave-* classes. */
  tones: Record<string, WaveTone>;
};

const groupWaves = new Map<string, GroupWave>();
type WaveListener = () => void;
const waveListeners = new Set<WaveListener>();

function notifyGroupWaves() {
  for (const l of waveListeners) l();
}

/** Read-only access for snap bridge mirroring. */
export function getGroupWave(groupId: string): GroupWave | undefined {
  return groupWaves.get(groupId);
}

/** Immutable snapshot for React state. */
export function snapshotGroupWaves(): Record<string, GroupWave> {
  const out: Record<string, GroupWave> = {};
  for (const [id, w] of groupWaves) {
    out[id] = {
      inFlight: w.inFlight,
      memberIds: [...w.memberIds],
      tones: { ...w.tones },
    };
  }
  return out;
}

export function subscribeGroupWaves(listener: WaveListener): () => void {
  waveListeners.add(listener);
  return () => {
    waveListeners.delete(listener);
  };
}

function setToneRun(w: GroupWave, templateId: string) {
  const prev = w.tones[templateId];
  if (prev === "ok" || prev === "err") return;
  if (prev === "run") return;
  for (const id of Object.keys(w.tones)) {
    if (w.tones[id] === "run" && id !== templateId) w.tones[id] = "ok";
  }
  w.tones[templateId] = "run";
}

/** Clear tints and mark first nested as warning (optimistic Run). */
export function beginGroupWave(groupId: string, memberIds?: string[]) {
  const members = memberIds ? [...memberIds] : [];
  const first = members[0];
  const tones: Record<string, WaveTone> = {};
  if (first) tones[first] = "run";
  groupWaves.set(groupId, {
    inFlight: true,
    memberIds: members,
    tones,
  });
  notifyGroupWaves();
}

/**
 * Drive tones from Rust cursor (`active_template_id` / hydrate).
 * Lesson 121: same paint pipeline as progress — not a second CSS source.
 */
export function advanceGroupWaveFromActive(
  groupId: string,
  activeTemplateId: string | null | undefined,
): boolean {
  const tid = (activeTemplateId ?? "").trim();
  if (!tid) return false;
  const w = groupWaves.get(groupId);
  if (!w?.inFlight) return false;
  const before = JSON.stringify(w.tones);
  const members =
    w.memberIds.length > 0
      ? w.memberIds
      : (() => {
          for (const market of ["wb", "ozon"] as TemplatesMarket[]) {
            const g = getTemplatesSnapCache(market)?.groups?.find((x) => x.id === groupId);
            if (g?.template_ids?.length) return [...g.template_ids];
          }
          return [] as string[];
        })();
  if (members.length && !w.memberIds.length) w.memberIds = members;
  const idx = members.indexOf(tid);
  if (idx > 0) {
    for (let i = 0; i < idx; i++) {
      const id = members[i];
      if (w.tones[id] !== "err") w.tones[id] = "ok";
    }
  }
  setToneRun(w, tid);
  if (JSON.stringify(w.tones) === before) return false;
  notifyGroupWaves();
  return true;
}

/** Optimistic Stop: end wave; current run → err (lesson 118). */
export function cancelGroupWave(groupId: string) {
  const w = groupWaves.get(groupId);
  if (!w) return;
  w.inFlight = false;
  for (const id of Object.keys(w.tones)) {
    if (w.tones[id] === "run") w.tones[id] = "err";
  }
  notifyGroupWaves();
}

export function waveRowClass(
  wave: GroupWave | undefined,
  templateId: string,
): "" | "plate--wave-run" | "plate--wave-ok" | "plate--wave-err" {
  const tone = wave?.tones[templateId];
  if (tone === "run") return "plate--wave-run";
  if (tone === "ok") return "plate--wave-ok";
  if (tone === "err") return "plate--wave-err";
  return "";
}

/** Prefer live wave Map; fall back to snap-mirrored tones (lesson 119). */
export function waveFromGroup(
  wave: GroupWave | undefined,
  group: TemplateGroupDto,
): GroupWave | undefined {
  if (wave) return wave;
  if (!group.wave_tones && !group.wave_in_flight) return undefined;
  return {
    inFlight: !!group.wave_in_flight,
    memberIds: [...(group.template_ids ?? [])],
    tones: { ...(group.wave_tones ?? {}) },
  };
}

function ensureWave(groupId: string): GroupWave {
  let w = groupWaves.get(groupId);
  if (!w) {
    w = { inFlight: false, memberIds: [], tones: {} };
    groupWaves.set(groupId, w);
  }
  return w;
}

function toneFromStatus(status: string | undefined): WaveTone {
  const s = (status ?? "").trim();
  if (s.startsWith("error") || s === "cancelled") return "err";
  return "ok";
}

function wavesForTemplateEvent(
  templateId: string,
  groupId: string | null | undefined,
  markRunning: boolean,
): GroupWave[] {
  const gid = (groupId ?? "").trim();
  if (gid) {
    const w = ensureWave(gid);
    // Lesson 123: only arm inFlight on start — finish must not revive «выполняется».
    if (markRunning) w.inFlight = true;
    return [w];
  }
  const out: GroupWave[] = [];
  for (const w of groupWaves.values()) {
    if (!w.inFlight) continue;
    if (
      w.memberIds.includes(templateId) ||
      w.tones[templateId] === "run" ||
      w.memberIds.length === 0
    ) {
      out.push(w);
    }
  }
  if (out.length === 0) {
    for (const w of groupWaves.values()) {
      if (w.inFlight) out.push(w);
    }
  }
  return out;
}

/** Apply group-end tones: remaining run → ok/err, clear inFlight. */
function endGroupWaveSession(w: GroupWave, lastRunStatus: string | undefined) {
  w.inFlight = false;
  const endTone = toneFromStatus(lastRunStatus);
  for (const id of Object.keys(w.tones)) {
    if (w.tones[id] === "run") w.tones[id] = endTone;
  }
}

/**
 * Lesson 123 hydrate end: cancel_map empty (`!is_running`) but session wave still
 * inFlight — same finalize as group-end progress (events may have been lost).
 * Returns true when a flying wave was closed.
 */
export function finalizeGroupWaveFromHydrate(
  groupId: string,
  lastRunStatus?: string | null,
): boolean {
  const w = groupWaves.get(groupId);
  if (!w) return false;
  const hasRun = Object.values(w.tones).some((t) => t === "run");
  if (!w.inFlight && !hasRun) return false;
  const st = (lastRunStatus ?? "").trim() || "ok";
  endGroupWaveSession(w, st);
  notifyGroupWaves();
  return true;
}

/** Last nested (or sole member) finished — no next cursor to advance tones. */
function finalizeWaveIfLastMember(w: GroupWave, templateId: string, endTone: WaveTone) {
  const members = w.memberIds;
  // Empty roster: still finalize once no `run` tones remain (beginWave may omit ids).
  if (members.length > 0 && members[members.length - 1] !== templateId) return;
  for (const id of Object.keys(w.tones)) {
    if (w.tones[id] === "run") w.tones[id] = endTone;
  }
  if (!Object.values(w.tones).some((t) => t === "run")) {
    w.inFlight = false;
  }
}

/**
 * Session summary after a finished group wave (lesson 123).
 * null while inFlight or when no terminal tones yet.
 */
export function groupWaveSuccessCount(
  wave: GroupWave | undefined,
  fallbackMemberCount: number,
): { ok: number; total: number } | null {
  if (!wave || wave.inFlight) return null;
  let ok = 0;
  let terminal = 0;
  for (const tone of Object.values(wave.tones)) {
    if (tone === "ok") {
      ok += 1;
      terminal += 1;
    } else if (tone === "err") {
      terminal += 1;
    }
  }
  if (terminal === 0) return null;
  const total =
    wave.memberIds.length > 0 ? wave.memberIds.length : fallbackMemberCount;
  if (total <= 0) return null;
  return { ok, total };
}

/** Update session wave from templates.runProgress (live, no tab switch). */
export function applyGroupWaveFromProgress(
  raw: TemplatesRunProgress | Record<string, unknown>,
) {
  const p = normalizeRunProgress(raw);
  if (p.entityType === "group") {
    const w = ensureWave(p.id);
    if (p.isRunning === false) {
      // Lesson 123: last nested has no "next" cursor — finalize run, do not delete.
      endGroupWaveSession(w, p.lastRunStatus);
    } else {
      w.inFlight = true;
      if (p.activeTemplateId) setToneRun(w, p.activeTemplateId);
      if (w.memberIds.length === 0) {
        for (const market of ["wb", "ozon"] as TemplatesMarket[]) {
          const snap = getTemplatesSnapCache(market);
          const g = snap?.groups?.find((x) => x.id === p.id);
          if (g?.template_ids?.length) {
            w.memberIds = [...g.template_ids];
            break;
          }
        }
      }
    }
    notifyGroupWaves();
    return;
  }
  if (p.entityType !== "template" || !p.id) return;

  const running = p.isRunning === true;
  const targets = wavesForTemplateEvent(p.id, p.groupId, running);
  if (targets.length === 0) return;

  for (const w of targets) {
    if (running) {
      setToneRun(w, p.id);
      continue;
    }
    const st = (p.lastRunStatus ?? "").trim();
    const finished =
      p.isRunning === false ||
      st === "ok" ||
      st.startsWith("ok") ||
      st.startsWith("error") ||
      st === "cancelled";
    if (!finished) continue;
    const endTone = toneFromStatus(p.lastRunStatus);
    w.tones[p.id] = endTone;
    // Sole / last nested: paint ok now — do not wait for group-end or a next cursor.
    finalizeWaveIfLastMember(w, p.id, endTone);
  }
  notifyGroupWaves();
}
