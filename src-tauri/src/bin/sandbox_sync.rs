use std::path::PathBuf;
use ufora_sync_lib::{api, sync};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Ufora Sync Sandbox Rust Runner ===");

    let mut course_id: Option<String> = None;
    let mut dest_dir = PathBuf::from("test_rust_sync_dir");
    let mut conflict_strategy = "duplicate".to_string();

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--course-id" => {
                if i + 1 < args.len() {
                    course_id = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--dest-dir" => {
                if i + 1 < args.len() {
                    dest_dir = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--conflict" => {
                if i + 1 < args.len() {
                    conflict_strategy = args[i + 1].clone();
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    println!("Ensuring authentication...");
    let token = api::ensure_authenticated(180.0)?;
    println!("Authenticated successfully.");

    println!("Fetching courses from Brightspace API...");
    let courses = api::fetch_enrolled_courses(&token).await?;
    println!("Fetched {} enrolled courses.", courses.len());

    let target_courses = if let Some(cid) = course_id {
        let filtered: Vec<_> = courses.into_iter().filter(|c| c.id == cid).collect();
        if filtered.is_empty() {
            eprintln!("Error: Course ID {cid} not found in enrolled courses.");
            std::process::exit(1);
        }
        filtered
    } else {
        courses
    };

    println!(
        "Targeting {} course(s) into destination: {}",
        target_courses.len(),
        dest_dir.display()
    );

    let mut total_downloaded = 0;
    let mut total_up_to_date = 0;
    let mut total_errors = 0;

    for c in target_courses {
        println!("\n--- Syncing Course: {} ({}) ---", c.name, c.id);
        let stats = sync::sync_course_files(
            &c.id,
            &c.name,
            &c.folder_name,
            &dest_dir,
            &token,
            &conflict_strategy,
            "_edited",
            |msg| println!("{msg}"),
        )
        .await?;

        total_downloaded += stats.downloaded;
        total_up_to_date += stats.up_to_date;
        total_errors += stats.errors;
    }

    println!("\n=== Sandbox Sync Complete ===");
    println!("Downloaded: {total_downloaded}");
    println!("Up to date: {total_up_to_date}");
    println!("Errors:     {total_errors}");

    if total_errors > 0 {
        std::process::exit(2);
    }

    Ok(())
}
