use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TriggerType {
    Once,
    Daily,
    Weekly,
    Interval,
}

impl Default for TriggerType {
    fn default() -> Self {
        Self::Once
    }
}

impl TriggerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Once => "Once",
            Self::Daily => "Daily",
            Self::Weekly => "Weekly",
            Self::Interval => "Interval",
        }
    }

}

fn deserialize_type<'de, D>(deserializer: D) -> Result<TriggerType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        S(String),
        N(u8),
    }
    Ok(match Raw::deserialize(deserializer)? {
        Raw::N(0) => TriggerType::Once,
        Raw::N(1) => TriggerType::Daily,
        Raw::N(2) => TriggerType::Weekly,
        Raw::N(3) => TriggerType::Interval,
        Raw::S(s) => match s.as_str() {
            "Once" | "once" | "0" => TriggerType::Once,
            "Daily" | "daily" | "1" => TriggerType::Daily,
            "Weekly" | "weekly" | "2" => TriggerType::Weekly,
            "Interval" | "interval" | "3" => TriggerType::Interval,
            _ => TriggerType::Once,
        },
        Raw::N(_) => TriggerType::Once,
    })
}

fn serialize_type<S>(t: &TriggerType, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(t.as_str())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeTrigger {
    #[serde(default = "new_id", alias = "Id")]
    pub id: String,
    #[serde(default, alias = "TemplateId")]
    pub template_id: String,
    #[serde(
        default,
        rename = "type",
        alias = "Type",
        alias = "trigger_type",
        deserialize_with = "deserialize_type",
        serialize_with = "serialize_type"
    )]
    pub trigger_type: TriggerType,
    #[serde(default = "default_true", alias = "IsEnabled")]
    pub is_enabled: bool,
    #[serde(default, alias = "ScheduledDateTime")]
    pub scheduled_date_time: Option<String>,
    #[serde(default, alias = "DailyTime")]
    pub daily_time: Option<String>,
    #[serde(default, alias = "WeeklyDay")]
    pub weekly_day: Option<String>,
    #[serde(default, alias = "WeeklyTime")]
    pub weekly_time: Option<String>,
    #[serde(default, alias = "IntervalMinutes")]
    pub interval_minutes: Option<i64>,
    #[serde(default, alias = "CreatedAt")]
    pub created_at: String,
    #[serde(default, alias = "LastExecutedAt")]
    pub last_executed_at: Option<String>,
    #[serde(default, alias = "NextExecutionAt")]
    pub next_execution_at: Option<String>,
    #[serde(default, alias = "LastExecutionStatus")]
    pub last_execution_status: Option<String>,
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn default_true() -> bool {
    true
}

impl Default for TimeTrigger {
    fn default() -> Self {
        Self {
            id: new_id(),
            template_id: String::new(),
            trigger_type: TriggerType::Once,
            is_enabled: true,
            scheduled_date_time: None,
            daily_time: None,
            weekly_day: None,
            weekly_time: None,
            interval_minutes: None,
            created_at: Local::now().to_rfc3339(),
            last_executed_at: None,
            next_execution_at: None,
            last_execution_status: None,
        }
    }
}

impl TimeTrigger {
    pub fn refresh_next(&mut self) {
        self.next_execution_at = self.calculate_next(Local::now().naive_local());
    }

    pub fn calculate_next(&self, reference: NaiveDateTime) -> Option<String> {
        if !self.is_enabled {
            return None;
        }
        let next = match self.trigger_type {
            TriggerType::Once => {
                let dt = parse_datetime(self.scheduled_date_time.as_deref()?)?;
                if dt > reference {
                    Some(dt)
                } else {
                    None
                }
            }
            TriggerType::Daily => {
                let t = parse_time(self.daily_time.as_deref()?)?;
                let mut cand = reference.date().and_time(t);
                if cand <= reference {
                    cand += Duration::days(1);
                }
                Some(cand)
            }
            TriggerType::Weekly => {
                let day = parse_weekday(self.weekly_day.as_deref()?)?;
                let t = parse_time(self.weekly_time.as_deref()?)?;
                Some(next_weekly(reference, day, t))
            }
            TriggerType::Interval => {
                let mins = self.interval_minutes.unwrap_or(60).max(1);
                Some(reference + Duration::minutes(mins))
            }
        }?;
        Some(Local.from_local_datetime(&next).single()?.to_rfc3339())
    }

    pub fn should_execute_now(&self, now: NaiveDateTime) -> bool {
        if !self.is_enabled {
            return false;
        }
        let Some(next_s) = self.next_execution_at.as_deref() else {
            return false;
        };
        let Some(next) = parse_datetime(next_s) else {
            return false;
        };
        now >= next
    }

}

use chrono::TimeZone;

fn parse_time(s: &str) -> Option<NaiveTime> {
    let s = s.trim();
    NaiveTime::parse_from_str(s, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M"))
        .ok()
}

fn parse_datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim();
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Local).naive_local());
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }
    if s.len() >= 10 {
        let d = NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d").ok()?;
        let t = parse_time(s.get(11..).unwrap_or("00:00")).unwrap_or_else(|| {
            NaiveTime::from_hms_opt(0, 0, 0).unwrap()
        });
        return Some(d.and_time(t));
    }
    None
}

fn parse_weekday(s: &str) -> Option<Weekday> {
    match s.trim() {
        "Monday" | "monday" | "1" => Some(Weekday::Mon),
        "Tuesday" | "tuesday" | "2" => Some(Weekday::Tue),
        "Wednesday" | "wednesday" | "3" => Some(Weekday::Wed),
        "Thursday" | "thursday" | "4" => Some(Weekday::Thu),
        "Friday" | "friday" | "5" => Some(Weekday::Fri),
        "Saturday" | "saturday" | "6" => Some(Weekday::Sat),
        "Sunday" | "sunday" | "0" => Some(Weekday::Sun),
        _ => None,
    }
}

fn next_weekly(reference: NaiveDateTime, day: Weekday, time: NaiveTime) -> NaiveDateTime {
    let current = reference.weekday().num_days_from_monday();
    let target = day.num_days_from_monday();
    let mut days = (target + 7 - current) % 7;
    if days == 0 {
        if reference.time() < time {
            return reference.date().and_time(time);
        }
        days = 7;
    }
    reference.date().and_time(time) + Duration::days(days as i64)
}

pub fn normalize_triggers(mut list: Vec<TimeTrigger>) -> Vec<TimeTrigger> {
    for t in &mut list {
        if t.id.trim().is_empty() {
            t.id = new_id();
        }
        if t.created_at.trim().is_empty() {
            t.created_at = Local::now().to_rfc3339();
        }
        t.refresh_next();
    }
    list
}
