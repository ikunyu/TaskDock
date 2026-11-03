use crate::{
    db::{Database, SettingsSnapshot},
    error::ServiceError,
};
use rusqlite::{params, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri_plugin_global_shortcut::Shortcut;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopSettings {
    pub widget_opacity: u8,
    pub widget_font_size: u8,
    pub widget_compact_font_size: u8,
    pub widget_date_font_size: u8,
    pub widget_week_font_size: u8,
    pub widget_week_date_font_size: u8,
    pub widget_topmost: bool,
    #[serde(default)]
    pub widget_locked: bool,
    pub widget_visible: bool,
    pub shortcut: String,
    pub autostart: bool,
    pub close_behavior: String,
}
impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            widget_opacity: 100,
            widget_font_size: 13,
            widget_compact_font_size: 14,
            widget_date_font_size: 10,
            widget_week_font_size: 15,
            widget_week_date_font_size: 10,
            widget_topmost: false,
            widget_locked: false,
            widget_visible: false,
            shortcut: "Ctrl+Alt+T".into(),
            autostart: false,
            close_behavior: "hide".into(),
        }
    }
}
impl DesktopSettings {
    pub fn validate(&mut self) -> Result<(), ServiceError> {
        if !(10..=20).contains(&self.widget_font_size) {
            return Err(ServiceError::Validation("悬浮窗字号需为 10–20。"));
        }
        if !(10..=24).contains(&self.widget_compact_font_size) {
            return Err(ServiceError::Validation("收起任务字号需为 10–24。"));
        }
        if !(8..=18).contains(&self.widget_date_font_size) {
            return Err(ServiceError::Validation("任务日期 / 时间字号需为 8–18。"));
        }
        if !(10..=26).contains(&self.widget_week_font_size) {
            return Err(ServiceError::Validation(
                "周标题（本周 / 下周）字号需为 10–26。",
            ));
        }
        if !(8..=18).contains(&self.widget_week_date_font_size) {
            return Err(ServiceError::Validation("周日期范围字号需为 8–18。"));
        }
        if !(20..=100).contains(&self.widget_opacity)
            || !matches!(self.close_behavior.as_str(), "hide" | "exit")
        {
            return Err(ServiceError::Validation("窗口设置无效。"));
        }
        self.shortcut = normalize_shortcut(&self.shortcut)?;
        Ok(())
    }
}
pub fn normalize_shortcut(value: &str) -> Result<String, ServiceError> {
    if value.len() > 100 {
        return Err(ServiceError::Validation("快捷键过长。"));
    }
    let parts: Vec<_> = value.split('+').map(str::trim).collect();
    if parts.len() < 2 || parts.iter().any(|p| p.is_empty()) {
        return Err(ServiceError::Validation(
            "快捷键需包含 Ctrl 或 Alt 以及一个字母、数字或 F1–F12。",
        ));
    }
    let mut modifiers = Vec::new();
    for modifier in &parts[..parts.len() - 1] {
        let m = match modifier.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => "Ctrl",
            "alt" => "Alt",
            "shift" => "Shift",
            _ => return Err(ServiceError::Validation("只支持 Ctrl、Alt、Shift 修饰键。")),
        };
        if modifiers.contains(&m) {
            return Err(ServiceError::Validation("快捷键修饰键重复。"));
        }
        modifiers.push(m);
    }
    if !modifiers.contains(&"Ctrl") && !modifiers.contains(&"Alt") {
        return Err(ServiceError::Validation("快捷键必须包含 Ctrl 或 Alt。"));
    }
    let key = parts.last().unwrap().to_ascii_uppercase();
    let valid = key.len() == 1 && key.as_bytes()[0].is_ascii_alphanumeric()
        || key
            .strip_prefix('F')
            .and_then(|s| s.parse::<u8>().ok())
            .is_some_and(|n| (1..=12).contains(&n));
    if !valid {
        return Err(ServiceError::Validation(
            "快捷键主键只支持字母、数字或 F1–F12。",
        ));
    }
    let mut ordered = Vec::new();
    for m in ["Ctrl", "Alt", "Shift"] {
        if modifiers.contains(&m) {
            ordered.push(m);
        }
    }
    let canonical = format!("{}+{}", ordered.join("+"), key);
    Shortcut::from_str(&canonical).map_err(|_| ServiceError::Validation("快捷键格式无效。"))?;
    Ok(canonical)
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub label: String,
    pub x: i32,
    pub y: i32,
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}
impl Placement {
    pub fn valid(&self) -> bool {
        matches!(self.label.as_str(), "main" | "widget" | "quick-entry")
            && self.x.unsigned_abs() <= 1_000_000
            && self.y.unsigned_abs() <= 1_000_000
            && self.width.is_finite()
            && self.height.is_finite()
            && self.scale.is_finite()
            && (100.0..=10000.0).contains(&self.width)
            && (100.0..=10000.0).contains(&self.height)
            && (0.25..=8.0).contains(&self.scale)
    }
}
impl Database {
    #[cfg(test)]
    pub fn desktop_settings(&self) -> Result<DesktopSettings, ServiceError> {
        let c = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        read_desktop(&c)
    }
    pub fn save_desktop(
        &self,
        mut settings: DesktopSettings,
        expected: i64,
    ) -> Result<SettingsSnapshot, ServiceError> {
        settings.validate()?;
        let mut c = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute("UPDATE app_settings SET revision=revision+1,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=1 AND revision=?1",[expected])?!=1{return Err(ServiceError::Conflict);}
        tx.execute("UPDATE desktop_settings SET widget_opacity=?1,widget_topmost=?2,widget_visible=?3,shortcut=?4,autostart=?5,close_behavior=?6,widget_font_size=?7,widget_compact_font_size=?8,widget_date_font_size=?9,widget_week_font_size=?10,widget_week_date_font_size=?11,widget_locked=?12 WHERE id=1",params![settings.widget_opacity,settings.widget_topmost,settings.widget_visible,settings.shortcut,settings.autostart,settings.close_behavior,settings.widget_font_size,settings.widget_compact_font_size,settings.widget_date_font_size,settings.widget_week_font_size,settings.widget_week_date_font_size,settings.widget_locked])?;
        let result = crate::db::read_settings(&tx)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn set_widget_visibility(&self, visible: bool) -> Result<SettingsSnapshot, ServiceError> {
        let mut c = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: bool = tx.query_row(
            "SELECT widget_visible FROM desktop_settings WHERE id=1",
            [],
            |r| r.get(0),
        )?;
        if current != visible {
            tx.execute(
                "UPDATE desktop_settings SET widget_visible=?1 WHERE id=1",
                [visible],
            )?;
            if tx.execute("UPDATE app_settings SET revision=revision+1,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=1", [])? != 1 {
                return Err(ServiceError::InvalidSettings);
            }
        }
        let result = crate::db::read_settings(&tx)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn set_widget_locked(&self, locked: bool) -> Result<SettingsSnapshot, ServiceError> {
        let mut c = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: bool = tx.query_row(
            "SELECT widget_locked FROM desktop_settings WHERE id=1",
            [],
            |r| r.get(0),
        )?;
        if current != locked {
            tx.execute(
                "UPDATE desktop_settings SET widget_locked=?1 WHERE id=1",
                [locked],
            )?;
            if tx.execute("UPDATE app_settings SET revision=revision+1,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=1", [])? != 1 {
                return Err(ServiceError::InvalidSettings);
            }
        }
        let result = crate::db::read_settings(&tx)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn placements(&self) -> Result<Vec<Placement>, ServiceError> {
        let c = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        let mut s = c.prepare("SELECT label,x,y,width,height,scale FROM window_state")?;
        let rows = s.query_map([], |r| {
            Ok(Placement {
                label: r.get(0)?,
                x: r.get(1)?,
                y: r.get(2)?,
                width: r.get(3)?,
                height: r.get(4)?,
                scale: r.get(5)?,
            })
        })?;
        Ok(rows
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(Placement::valid)
            .collect())
    }
    pub fn save_placements(&self, placements: &[Placement]) -> Result<(), ServiceError> {
        if placements.iter().any(|p| !p.valid()) {
            return Err(ServiceError::Validation("窗口位置无效。"));
        }
        let mut c = self.connection.lock().map_err(|_| ServiceError::Lock)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Immediate)?;
        for p in placements {
            tx.execute("INSERT INTO window_state(label,x,y,width,height,scale) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(label) DO UPDATE SET x=excluded.x,y=excluded.y,width=excluded.width,height=excluded.height,scale=excluded.scale",params![p.label,p.x,p.y,p.width,p.height,p.scale])?;
        }
        tx.commit()?;
        Ok(())
    }
}
pub fn read_desktop(c: &rusqlite::Connection) -> Result<DesktopSettings, ServiceError> {
    let raw=c.query_row("SELECT widget_opacity,widget_topmost,widget_visible,shortcut,autostart,close_behavior,widget_font_size,widget_compact_font_size,widget_date_font_size,widget_week_font_size,widget_week_date_font_size,widget_locked FROM desktop_settings WHERE id=1",[],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?,r.get::<_,i64>(7)?,r.get::<_,i64>(8)?,r.get::<_,i64>(9)?,r.get::<_,i64>(10)?,r.get::<_,i64>(11)?)))?;
    let mut value = DesktopSettings {
        widget_opacity: u8::try_from(raw.0).unwrap_or(0),
        widget_topmost: raw.1 == 1,
        widget_visible: raw.2 == 1,
        shortcut: raw.3,
        autostart: raw.4 == 1,
        close_behavior: raw.5,
        widget_font_size: u8::try_from(raw.6).unwrap_or(13),
        widget_compact_font_size: u8::try_from(raw.7).unwrap_or(14),
        widget_date_font_size: u8::try_from(raw.8).unwrap_or(10),
        widget_week_font_size: u8::try_from(raw.9).unwrap_or(15),
        widget_week_date_font_size: u8::try_from(raw.10).unwrap_or(10),
        widget_locked: raw.11 == 1,
    };
    if ![raw.1, raw.2, raw.4, raw.11]
        .iter()
        .all(|v| [0, 1].contains(v))
        || value.validate().is_err()
    {
        log::warn!("invalid desktop configuration; safe defaults used");
        value = Default::default();
    }
    Ok(value)
}
