import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import uiCatalog from "../../locales/ru/ui.json";
import { getLocale, setLocale as hostSetLocale } from "../bridge/hostApi.i18n";

export type AppLocale = "ru";

type Catalog = Record<string, unknown>;

type I18nContextValue = {
  locale: AppLocale;
  t: (key: string, vars?: Record<string, string | number>) => string;
  setLocale: (locale: AppLocale) => Promise<void>;
};

const I18nContext = createContext<I18nContextValue | null>(null);

function flatten(prefix: string, value: unknown, out: Record<string, string>) {
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    for (const [k, v] of Object.entries(value as Catalog)) {
      const next = prefix ? `${prefix}.${k}` : k;
      flatten(next, v, out);
    }
    return;
  }
  if (typeof value === "string") {
    out[prefix] = value;
    return;
  }
  if (Array.isArray(value) && value.every((x) => typeof x === "string")) {
    out[prefix] = (value as string[]).join("\u0001");
  }
}

const FLAT_RU: Record<string, string> = {};
flatten("", uiCatalog as Catalog, FLAT_RU);

function catalogsFor(_locale: AppLocale): Record<string, string> {
  return FLAT_RU;
}

function format(template: string, vars?: Record<string, string | number>): string {
  if (!vars) return template;
  let s = template;
  for (const [k, v] of Object.entries(vars)) {
    s = s.split(`{${k}}`).join(String(v));
  }
  return s;
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<AppLocale>("ru");

  useEffect(() => {
    void getLocale()
      .then((loc) => {
        if (loc === "ru") setLocaleState("ru");
      })
      .catch(() => undefined);
  }, []);

  const setLocale = useCallback(async (next: AppLocale) => {
    await hostSetLocale(next);
    setLocaleState(next);
  }, []);

  const t = useCallback(
    (key: string, vars?: Record<string, string | number>) => {
      const cat = catalogsFor(locale);
      const raw = cat[key] ?? key;
      return format(raw, vars);
    },
    [locale],
  );

  const value = useMemo(
    () => ({ locale, t, setLocale }),
    [locale, t, setLocale],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useT() {
  const ctx = useContext(I18nContext);
  if (!ctx) {
    throw new Error("useT outside I18nProvider");
  }
  return ctx.t;
}

export function useI18n() {
  const ctx = useContext(I18nContext);
  if (!ctx) {
    throw new Error("useI18n outside I18nProvider");
  }
  return ctx;
}

/** Translate without React (e.g. modules). Uses current Ru catalog. */
export function t(key: string, vars?: Record<string, string | number>): string {
  return format(FLAT_RU[key] ?? key, vars);
}
