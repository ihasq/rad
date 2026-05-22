use crate::types::CodeRegion;

pub struct RegionMap {
    regions: Vec<CodeRegion>,
}

impl RegionMap {
    pub fn new() -> Self {
        Self { regions: vec![] }
    }

    pub fn register(&mut self, region: CodeRegion) -> bool {
        // 重複チェック: 同一ファイル・行範囲の先着優先
        if self.get_owner(&region.file_path, region.start_line).is_some() {
            return false; // 既に登録済み
        }
        self.regions.push(region);
        true
    }

    pub fn get_owner(&self, file: &str, line: u32) -> Option<&str> {
        for r in &self.regions {
            if r.file_path == file && line >= r.start_line && line <= r.end_line {
                return Some(&r.owner_id);
            }
        }
        None
    }

    pub fn list(&self, file: &str) -> Vec<&CodeRegion> {
        self.regions.iter().filter(|r| r.file_path == file).collect()
    }

    pub fn get_role(&self, file: &str, line: u32, participant: &str) -> &str {
        match self.get_owner(file, line) {
            Some(owner) if owner == participant => "leader",
            Some(_) => "follower",
            None => "unowned",
        }
    }
}
