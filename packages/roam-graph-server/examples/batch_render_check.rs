//! Renders every node's .org file via the production render pipeline,
//! prints any panics or empty-output cases. Used to validate the spec's
//! "≥ 95% pass rate" acceptance criterion against the user's real data.
//!
//! Run:
//!     cargo run --release --example batch_render_check -- /path/to/org-roam.db /path/to/org-roam-directory

use std::path::PathBuf;

use ortk_roam_graph::{db::Db, render};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let db_path = PathBuf::from(args.next().expect("usage: <db> <org_dir>"));
    let org_root = PathBuf::from(args.next().expect("usage: <db> <org_dir>"));

    let db = Db::open(&db_path)?;
    let g = db.graph()?;

    let mut total = 0;
    let mut empty = 0;
    let mut panicked: Vec<String> = vec![];

    for n in &g.nodes {
        total += 1;
        let detail = match db.node(&n.id) {
            Ok(d) => d,
            Err(_) => {
                panicked.push(format!("{} (lookup failed)", n.title));
                continue;
            }
        };
        let source = match std::fs::read_to_string(&detail.file) {
            Ok(s) => s,
            Err(_) => {
                panicked.push(format!("{} (file unreadable)", n.title));
                continue;
            }
        };
        let result = std::panic::catch_unwind(|| {
            render::render(&source, &detail.file, &org_root)
        });
        match result {
            Ok(html) if html.trim().is_empty() => {
                empty += 1;
                println!("EMPTY  {}: {}", n.title, detail.file.display());
            }
            Ok(_) => {}
            Err(_) => {
                panicked.push(format!("{}: {}", n.title, detail.file.display()));
            }
        }
    }

    let bad = empty + panicked.len();
    let pass_rate = (total - bad) as f64 / total.max(1) as f64;
    println!(
        "\nrendered {total} nodes; {bad} bad (empty: {empty}, panicked: {})",
        panicked.len()
    );
    for p in &panicked {
        println!("  PANIC  {p}");
    }
    println!("pass rate: {:.1}%", pass_rate * 100.0);
    if pass_rate < 0.95 {
        std::process::exit(1);
    }
    Ok(())
}
