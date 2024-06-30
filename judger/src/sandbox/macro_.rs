#[macro_export]
macro_rules! async_loop {
    ($e:expr) => {
        async move {
            loop {
                $e
                tokio::time::sleep($crate::sandbox::monitor::mem_cpu::MONITOR_ACCURACY).await;
            }
        }
    };
}
