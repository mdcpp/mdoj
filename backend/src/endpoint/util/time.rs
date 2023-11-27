pub fn into_prost(time: chrono::NaiveDateTime) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: time.timestamp(),
        nanos: time.timestamp_subsec_nanos() as i32,
    }
}

pub fn into_chrono(time: prost_types::Timestamp) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::from_timestamp_opt(time.seconds, time.nanos as u32).unwrap_or_default()
}
