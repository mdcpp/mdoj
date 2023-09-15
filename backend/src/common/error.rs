pub fn result_into<T>(result: Result<T, sea_orm::DbErr>) -> Result<T, tonic::Status> {
    result.map_err(|e| {
        log::error!("{}", e);
        tonic::Status::internal("DbErr")
    })
}
