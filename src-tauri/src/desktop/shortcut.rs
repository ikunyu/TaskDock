use crate::error::AppError;
/// Registration lease: the old key is untouched until the caller commits its
/// database transaction. Failure automatically releases only the candidate key.
type Rollback<'a> = Box<dyn Fn(&str) -> Result<(), AppError> + 'a>;
pub struct ShortcutLease<'a> {
    key: &'a str,
    rollback: Rollback<'a>,
    armed: bool,
}
impl<'a> ShortcutLease<'a> {
    /// Unrelated settings must not retry an unavailable, unchanged shortcut.
    pub fn for_settings(
        saved: &str,
        effective: Option<&str>,
        key: &'a str,
        register: impl FnOnce(&str) -> Result<(), AppError>,
        rollback: impl Fn(&str) -> Result<(), AppError> + 'a,
    ) -> Result<Option<Self>, AppError> {
        if saved == key {
            Ok(None)
        } else {
            Self::prepare(effective, key, register, rollback).map(Some)
        }
    }
    pub fn prepare(
        old: Option<&str>,
        key: &'a str,
        register: impl FnOnce(&str) -> Result<(), AppError>,
        rollback: impl Fn(&str) -> Result<(), AppError> + 'a,
    ) -> Result<Self, AppError> {
        let armed = old != Some(key);
        if armed {
            register(key)?;
        }
        Ok(Self {
            key,
            rollback: Box::new(rollback),
            armed,
        })
    }
    pub fn commit(mut self) {
        self.armed = false;
    }
}
impl Drop for ShortcutLease<'_> {
    fn drop(&mut self) {
        if self.armed {
            if let Err(e) = (self.rollback)(self.key) {
                log::warn!("candidate shortcut cleanup failed: {}", e.code);
            }
        }
    }
}
