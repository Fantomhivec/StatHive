/** Имя файла .json при копировании конфигурации (из названия калькулятора). */
export function configExportFileName(calculatorName: string): string {
  const cleaned = calculatorName
    .trim()
    .replace(/[<>:"/\\|?*\u0000-\u001f]/g, "_")
    .replace(/\s+/g, " ")
    .replace(/[. ]+$/g, "")
    .trim();
  const base = cleaned || "calculator-config";
  return base.toLowerCase().endsWith(".json") ? base : `${base}.json`;
}

export function defaultConfigExportPath(calculatorName: string): string {
  return `C:\\Users\\Public\\${configExportFileName(calculatorName)}`;
}
