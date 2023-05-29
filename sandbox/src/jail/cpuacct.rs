use cgroups_rs::cpu::CpuController;

pub enum CpuStatKey {
    UsageUsec = 0,
    UserUsec = 1,
    SystemUsec = 2,
    NrPeriods = 3,
    NrThrottled = 4,
    ThrottledUsec = 5,
    NrBursts = 6,
    BurstUsec = 7,
}

pub struct CpuAcct([i64; 8]);

impl CpuAcct {
    pub fn get(&self, stat: CpuStatKey) -> Option<i64> {
        match self.0[stat as usize] {
            i64::MIN => None,
            x => Some(x),
        }
    }
    pub fn from_controller(cpu: &CpuController) -> Self {
        Self::from_raw(&cpu.cpu().stat)
    }
    fn from_raw(raw: &str) -> Self {
        let mut cpuacct = [i64::MIN; 8];
        for (key, value) in raw
            .split("\n")
            .filter_map(|stmt| match stmt.split_once(" ") {
                Some(a) => Some(a),
                None => None,
            })
        {
            let value: i64 = value.parse().unwrap();
            match key {
                "usage_usec" => cpuacct[0] = value,
                "user_usec" => cpuacct[1] = value,
                "system_usec" => cpuacct[2] = value,
                "nr_periods" => cpuacct[3] = value,
                "nr_throttled" => cpuacct[4] = value,
                "throttled_usec" => cpuacct[5] = value,
                "nr_bursts" => cpuacct[6] = value,
                "burst_usec" => cpuacct[7] = value,
                _ => {}
            };
        }
        Self(cpuacct)
    }
}

// an example of /sys/fs/cgroup/mdoj0/cpu.stat
// usage_usec 0
// user_usec 0
// system_usec 0
// core_sched.force_idle_usec 0
// nr_periods 2
// nr_throttled 0
// throttled_usec 0
// nr_bursts 0
// burst_usec 0
