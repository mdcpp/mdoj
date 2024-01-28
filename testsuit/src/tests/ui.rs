use indicatif::*;

pub struct UI {
    pb: ProgressBar,
    len: u64,
    progress: u64,
}

impl Drop for UI {
    fn drop(&mut self) {
        self.pb.finish();
    }
}

impl UI {
    pub fn new(m: &MultiProgress, len: u64) -> Self {
        let pb = m.add(ProgressBar::new_spinner());

        pb.set_message("");
        pb.set_prefix(format!("[0/{}]", len));

        let style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} Running {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");

        pb.set_style(style);

        Self {
            pb,
            len,
            progress: 0,
        }
    }
    pub fn inc(&mut self, msg: &'static str) {
        log::warn!("ui inc");
        self.progress += 1;
        self.pb.set_message(msg);
        self.pb
            .set_prefix(format!("[{}/{}]", self.progress, self.len));
    }
}
