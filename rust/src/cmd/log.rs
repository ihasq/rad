use crate::store::RadStore;

pub struct LogOptions {
    pub participant: Option<String>,
    pub file: Option<String>,
    pub status: Option<String>,
}

pub fn run_log(store: &RadStore, opts: &LogOptions) -> Result<String, String> {
    let oplog = store.load_oplog()?;
    let mut ops = oplog.get_all_operations();

    // Sort by timestamp
    ops.sort_by_key(|o| o.timestamp);

    // Apply filters
    if let Some(ref p) = opts.participant {
        ops.retain(|o| &o.participant_id == p);
    }

    if let Some(ref f) = opts.file {
        ops.retain(|o| o.region_id.starts_with(f));
    }

    if let Some(ref s) = opts.status {
        let status_lower = s.to_lowercase();
        ops.retain(|o| {
            format!("{:?}", o.status).to_lowercase() == status_lower
        });
    }

    // Build output
    let mut output = String::new();
    for op in ops {
        // Extract content preview (first 50 chars, escape newlines)
        let content_clean = op.content.replace('\n', "\\n").replace('\r', "");
        let content_preview = if content_clean.len() > 50 {
            format!("\"{}...\"", &content_clean[..47])
        } else {
            format!("\"{}\"", content_clean)
        };

        let status_str = format!("{:?}", op.status).to_lowercase();

        output.push_str(&format!("{} [{}]  {}  {}  {}\n",
            op.id,
            status_str,
            op.participant_id,
            op.region_id,
            content_preview));
    }

    Ok(output)
}
