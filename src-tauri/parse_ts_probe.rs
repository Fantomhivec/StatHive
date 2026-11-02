#[cfg(test)]
mod t {
  #[test]
  fn p() {
    use chrono::NaiveDateTime;
    let f = \"%Y-%m-%d %H:%M:%S\";
    eprintln!(\"pad {:?}\", NaiveDateTime::parse_from_str(\"2026-10-26 08:58:39\", f));
    eprintln!(\"one {:?}\", NaiveDateTime::parse_from_str(\"2026-10-26 8:58:39\", f));
  }
}
