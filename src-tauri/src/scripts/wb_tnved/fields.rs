use crate::core::script_field::ScriptField;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("subjectID", crate::core::i18n::t("fields.wb_tnved_codes.subjectID"), true),
        ScriptField::new("subjectName", crate::core::i18n::t("fields.wb_tnved_codes.subjectName"), true),
        ScriptField::new("tnved", crate::core::i18n::t("fields.wb_tnved_codes.tnved"), true),
        ScriptField::new("isKiz", crate::core::i18n::t("fields.wb_tnved_codes.isKiz"), true),
    ]
}
