use crate::types::{Operation, OpStatus};

pub struct OpLog {
    ops: Vec<Operation>,
}

impl OpLog {
    pub fn new() -> Self {
        Self { ops: vec![] }
    }

    pub fn append(&mut self, op: Operation) {
        self.ops.push(op);
    }

    pub fn get_chain(&self, file: &str, start: u32, end: u32) -> Vec<&Operation> {
        let region_id = format!("{}:{}-{}", file, start, end);
        let mut chain: Vec<&Operation> = self.ops.iter()
            .filter(|op| op.region_id == region_id)
            .collect();
        chain.sort_by_key(|op| op.timestamp);
        chain
    }

    #[allow(dead_code)]
    pub fn all(&self) -> &[Operation] {
        &self.ops
    }

    pub fn get_by_id(&self, id: &str) -> Option<&Operation> {
        self.ops.iter().find(|op| op.id == id)
    }

    pub fn set_status(&mut self, id: &str, status: OpStatus) {
        if let Some(op) = self.ops.iter_mut().find(|op| op.id == id) {
            op.status = status;
        }
    }

    pub fn get_chain_by_region_id(&self, region_id: &str) -> Vec<&Operation> {
        let mut chain: Vec<&Operation> = self.ops.iter()
            .filter(|op| op.region_id == region_id)
            .collect();
        chain.sort_by_key(|op| op.timestamp);
        chain
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }
}
