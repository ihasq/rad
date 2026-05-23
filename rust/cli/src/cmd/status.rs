use crate::store::RadStore;
use crate::types::{OpStatus, OpType};

pub fn run_status(store: &RadStore) -> Result<String, String> {
    // Load all state
    let oplog = store.load_oplog()?;
    let participants = store.load_participants();
    let region_map = store.load_regions();
    let founders = store.load_founders();

    // Get founder
    let founder = founders.get_founder(".").unwrap_or("(unknown)").to_string();

    // Count operations by status
    let ops = oplog.get_all_operations();
    let total = ops.len();
    let accepted = ops.iter().filter(|o| o.status == OpStatus::Accepted).count();
    let visible = ops.iter().filter(|o| o.status == OpStatus::Visible).count();
    let rejected = ops.iter().filter(|o| o.status == OpStatus::Rejected).count();
    let discarded = ops.iter().filter(|o| o.status == OpStatus::Discarded).count();

    // Count regions (unique file paths)
    let regions = region_map.get_all_regions();
    let region_count = regions.len();

    // Count files (unique file paths from regions)
    let mut files = std::collections::HashSet::new();
    for region in &regions {
        files.insert(&region.file_path);
    }
    let file_count = files.len();

    // Get visible writes awaiting review
    let visible_writes: Vec<_> = ops.iter()
        .filter(|o| o.status == OpStatus::Visible && o.op_type == OpType::Write)
        .collect();

    // Build output
    let mut output = String::new();
    output.push_str(&format!("rad project: . (founder: {})\n", founder));
    output.push_str(&format!("participants: {}\n", participants.len()));
    output.push_str(&format!("operations: {} ({} accepted, {} visible, {} rejected, {} discarded)\n",
        total, accepted, visible, rejected, discarded));
    output.push_str(&format!("regions: {}\n", region_count));
    output.push_str(&format!("files: {}\n", file_count));

    if !visible_writes.is_empty() {
        output.push_str("\nvisible writes awaiting review:\n");
        for op in visible_writes {
            // Extract content preview (first 50 chars, escape newlines)
            let content_clean = op.content.replace('\n', "\\n").replace('\r', "");
            let content_preview = if content_clean.len() > 50 {
                format!("\"{}...\"", &content_clean[..47])
            } else {
                format!("\"{}\"", content_clean)
            };
            output.push_str(&format!("  {} [visible] {}  {}  {}\n",
                op.id, op.participant_id, op.region_id, content_preview));
        }
    }

    Ok(output)
}
