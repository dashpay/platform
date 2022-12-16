use super::SystemIDs;

pub mod types {
    pub const WITHDRAWAL: &str = "withdrawal";
}

pub mod statuses {
    pub const QUEUED: u8 = 0;
    pub const POOLED: u8 = 1;
    pub const BROADCASTED: u8 = 2;
    pub const COMPLETE: u8 = 3;
    pub const EXPIRED: u8 = 4;
}

pub fn system_ids() -> SystemIDs {
    SystemIDs {
        contract_id: "4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6".to_string(),
        owner_id: "CUjAw7eD64wmaznNrfC5sKdn4Lpr1wBvWKMjGLrmEs5h".to_string(),
    }
}
