use crate::db::Database;
use crate::library::service::SharedProgress;
use crate::paths::AppPaths;

pub struct AppState {
    pub paths: AppPaths,
    pub database: Database,
    pub library_progress: SharedProgress,
}
