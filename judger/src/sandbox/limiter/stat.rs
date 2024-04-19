pub struct Memory {
    pub kernel: u64,
    pub user: u64,
    pub total: u64,
}

pub struct Cpu {
    pub kernel: u64,
    pub user: u64,
    pub total: u64,
}

impl Cpu {
    pub(super) fn out_of_resources(resource: &Self, stat: Self) -> bool {
        stat.kernel > resource.kernel || stat.user > resource.user || stat.total > resource.total
    }
}
