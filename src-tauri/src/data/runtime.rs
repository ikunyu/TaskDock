use crate::{desktop::DesktopRuntime, service::LocalService};
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let service = app.state::<Arc<LocalService>>().inner().clone();
        let mut ticks = 0u32;
        while !app
            .state::<DesktopRuntime>()
            .quitting
            .load(Ordering::SeqCst)
        {
            if ticks % 30 == 0 && !service.maintenance.load(Ordering::SeqCst) {
                match service.with_database(|db| db.backup_due()) {
                    Ok(true) => {
                        if let Err(e) = service.create_backup("auto") {
                            log::warn!("automatic backup failed; tasks unaffected: {e}");
                        }
                        let _ = app.emit("taskdock://data-changed", ());
                    }
                    Ok(false) => {}
                    Err(e) => log::warn!("automatic backup deferred: {e}"),
                }
            }
            ticks = ticks.wrapping_add(1);
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}
