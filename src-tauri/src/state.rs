use crate::db::Database;
use crate::paths::AppPaths;

pub struct AppState {
    pub paths: AppPaths,
    pub database: Database,
}
