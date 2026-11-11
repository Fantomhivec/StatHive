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

import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";

type ShellNavApi = {
  open: boolean;
  setOpen: (open: boolean) => void;
  toggle: () => void;
};

const ShellNavContext = createContext<ShellNavApi | null>(null);

export function ShellNavProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const toggle = useCallback(() => setOpen((v) => !v), []);
  const value = useMemo(() => ({ open, setOpen, toggle }), [open, toggle]);
  return (
    <ShellNavContext.Provider value={value}>{children}</ShellNavContext.Provider>
  );
}

export function useShellNav(): ShellNavApi {
  const ctx = useContext(ShellNavContext);
  if (!ctx) {
    throw new Error("useShellNav must be used within ShellNavProvider");
  }
  return ctx;
}

/** True when Tab should not toggle the sections drawer (forms / dialogs). */
export function shellNavTabBlocked(): boolean {
  if (document.querySelector(".overlay--win, .overlay--modal, .overlay--nested")) {
    return true;
  }
  const el = document.activeElement as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  if (el.isContentEditable) return true;
  if (el.closest(".app-combo, .dialog-panel, .popover")) return true;
  return false;
}
