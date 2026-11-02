/** Builtin main Admin (AccessLevel Admin / Админ) — паритет Rust `is_admin_role`. */
export function isMainAdminRole(level: string | undefined | null): boolean {
  const compact = (level || "")
    .toLowerCase()
    .replace(/[\s_\-\u00a0]/g, "");
  return compact === "admin" || compact === "админ";
}
