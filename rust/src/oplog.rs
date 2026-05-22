use crate::types::Operation;

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
}
