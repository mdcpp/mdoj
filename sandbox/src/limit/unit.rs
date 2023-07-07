use std::{path::PathBuf, sync::Arc};

use tokio::fs;

use crate::limit::utils::{limiter::Limiter, nsjail::NsJail};

use super::{prison::Prison, proc::RunningProc, Error, Limit};

pub struct Unit<'a> {
    pub(super) id: String,
    pub(super) controller: &'a Prison,
    pub(super) root: PathBuf,
}

impl<'a> Drop for Unit<'a> {
    fn drop(&mut self) {
        let tmp_path = self.controller.tmp.as_path().clone().join(self.id.clone());
        tokio::spawn(async {
            log::trace!("removing Cell's presistent storage");
            fs::remove_dir_all(tmp_path).await
        });
    }
}

impl<'a> Unit<'a> {
    pub async fn execute(&self, args: &Vec<&str>, limit: Limit) -> Result<RunningProc, Error> {
        log::debug!("preparing Cell");

        let cg_name = format!("mdoj/{}", self.id);

        let reversed_memory = limit.user_mem + limit.kernel_mem;

        let memory_holder = self
            .controller
            .memory_counter
            .allocate(reversed_memory)
            .await?;

        let nsjail = NsJail::new(&self.root)
            .cgroup(&cg_name)
            .done()
            .presist_vol(&self.id)
            .mount("src", limit.lockdown)
            .done()
            .common().cmds(args)
            .build()?;

        let limiter = Limiter::new(&cg_name, limit)?;

        Ok(RunningProc {
            limiter,
            nsjail,
            _memory_holder: memory_holder,
        })
    }
}
