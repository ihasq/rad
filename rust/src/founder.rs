use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct FounderTree {
    founders: HashMap<String, String>, // dir_path → participant_id
    file_founders: HashMap<String, String>, // file_path → participant_id (file creator)
    root_founder: String,
}

impl FounderTree {
    pub fn new(root_founder: &str) -> Self {
        let mut founders = HashMap::new();
        founders.insert(".".to_string(), root_founder.to_string());
        Self {
            founders,
            file_founders: HashMap::new(),
            root_founder: root_founder.to_string(),
        }
    }

    /// write 時にファイルのディレクトリ階層を走査し Founder を自動登録
    pub fn register_from_write(&mut self, file_path: &str, participant: &str) {
        // ファイルパスからディレクトリを抽出
        let dir = match file_path.rfind('/') {
            Some(pos) => &file_path[..pos],
            None => return, // ディレクトリなし（root直下）
        };

        // 各親ディレクトリを走査して未登録なら participant を登録
        let mut current = String::new();
        for segment in dir.split('/') {
            if segment.is_empty() {
                continue;
            }
            if !current.is_empty() {
                current.push('/');
            }
            current.push_str(segment);

            self.founders
                .entry(current.clone())
                .or_insert_with(|| participant.to_string());
        }
    }

    pub fn get_founder(&self, dir: &str) -> Option<&str> {
        self.founders.get(dir).map(|s| s.as_str())
    }

    /// 上位 Founder が下位 Founder の Leader かどうか
    /// upper_dir が lower_dir の親ディレクトリであればtrue
    pub fn is_ancestor_founder(&self, upper_dir: &str, lower_dir: &str) -> bool {
        if upper_dir == "." {
            // root は全ディレクトリの親
            return lower_dir != ".";
        }
        lower_dir.starts_with(upper_dir) && lower_dir != upper_dir
    }

    pub fn list_all(&self) -> Vec<(&str, &str)> {
        let mut entries: Vec<_> = self
            .founders
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        entries.sort_by_key(|(k, _)| k.to_string());
        entries
    }

    pub fn get_root_founder(&self) -> &str {
        &self.root_founder
    }

    /// ファイルが属するディレクトリの Founder を取得
    pub fn get_dir_founder_for_file(&self, file_path: &str) -> Option<&str> {
        let dir = match file_path.rfind('/') {
            Some(pos) => &file_path[..pos],
            None => ".", // root直下
        };
        self.get_founder(dir)
    }

    /// ファイルを最初に create した参加者（file Founder）を登録
    pub fn register_file_founder(&mut self, file_path: &str, participant: &str) {
        self.file_founders
            .entry(file_path.to_string())
            .or_insert_with(|| participant.to_string());
    }

    /// ファイルの Founder（最初に create した参加者）を取得
    pub fn get_file_founder(&self, file_path: &str) -> Option<&str> {
        self.file_founders.get(file_path).map(|s| s.as_str())
    }

    /// delete 操作を accept できるかチェック
    /// file founder 自身の delete → ディレクトリ founder が accept 必要
    /// 他者の delete → file founder が accept
    pub fn can_accept_delete(&self, file_path: &str, participant: &str) -> bool {
        match self.get_file_founder(file_path) {
            Some(founder) => founder == participant,
            None => false,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_json(json: &str, root_founder: &str) -> Self {
        serde_json::from_str(json).unwrap_or_else(|_| Self::new(root_founder))
    }
}
