use crate::db::connect::ConnectionConfig;
use crate::db::driver::Driver;
use crate::error::{AppError, AppResult};

// Redis (key-value) and MongoDB (documents) do NOT fit the
// schema -> table -> rows -> SQL model the `Driver` trait assumes.
// Forcing them into the SQL grid produces a bad experience (this is
// exactly the weakest part of clients like TablePlus).
//
// Recommended approach when you tackle these:
//   - Introduce a separate concept (e.g. `enum Backend { Sql(Box<dyn Driver>), Redis(..), Mongo(..) }`)
//     and a dedicated UI mode per backend instead of overloading the grid.
//   - Redis (crate `redis`): browse keys by pattern (SCAN), show value by type
//     (string / hash / list / set / zset). No "tables".
//   - MongoDB (crate `mongodb`): databases -> collections -> documents (JSON
//     tree view), query with filter documents, not SQL.
//
// Until then, connecting with these kinds returns this explicit error.
pub async fn connect(_cfg: &ConnectionConfig) -> AppResult<Box<dyn Driver>> {
    Err(AppError::Other(
        "Redis/MongoDB need their own non-tabular UI and are not implemented. \
         See src-tauri/src/db/nosql.rs for the recommended design."
            .into(),
    ))
}
