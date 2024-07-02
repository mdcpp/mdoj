pub fn into_prost(time: chrono::NaiveDateTime) -> prost_wkt_types::Timestamp {
    prost_wkt_types::Timestamp {
        seconds: time.and_utc().timestamp(),
        nanos: time.and_utc().timestamp_subsec_nanos() as i32,
    }
}
/// convert prost_types's time to chrono's
pub fn into_chrono(time: prost_wkt_types::Timestamp) -> chrono::NaiveDateTime {
    match chrono::DateTime::from_timestamp(time.seconds, time.nanos as u32) {
        Some(x) => x.naive_utc(),
        None => chrono::NaiveDateTime::default(),
    }
}
