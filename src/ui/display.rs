use super::logger::{LogType, log};
use comfy_table::{Cell, Color, ContentArrangement, Table, presets::UTF8_FULL};
use std::collections::HashMap;

pub fn print_table(projects: &HashMap<String, String>) {
    if projects.is_empty() {
        log(
            "No projects found. Add one with: vcode add <name> <path>",
            LogType::Info,
        );
        return;
    }

    let mut sorted: Vec<_> = projects.iter().collect();
    sorted.sort_by_key(|(name, _)| name.to_lowercase());

    const PAGE_SIZE: usize = 20;
    let total_projects = sorted.len();
    let total_pages = (total_projects + PAGE_SIZE - 1) / PAGE_SIZE;

    if total_projects <= PAGE_SIZE {
        display_project_page(&sorted, 0, total_projects, 1, 1);
    } else {
        let mut current_page = 0;

        loop {
            let start_idx = current_page * PAGE_SIZE;
            let end_idx = ((current_page + 1) * PAGE_SIZE).min(total_projects);

            display_project_page(
                &sorted[start_idx..end_idx],
                start_idx,
                total_projects,
                current_page + 1,
                total_pages,
            );

            if current_page < total_pages - 1 {
                use inquire::Select;
                let options = vec!["Next page", "Exit"];
                match Select::new("", options).without_help_message().prompt() {
                    Ok("Next page") => current_page += 1,
                    _ => break,
                }
            } else {
                break;
            }
        }
    }
}

fn display_project_page(
    projects: &[(&String, &String)],
    start_idx: usize,
    total: usize,
    current_page: usize,
    total_pages: usize,
) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("#").fg(Color::Cyan),
            Cell::new("Name").fg(Color::Cyan),
            Cell::new("Path").fg(Color::Cyan),
        ]);

    for (idx, (name, path)) in projects.iter().enumerate() {
        table.add_row(vec![
            Cell::new(start_idx + idx + 1).fg(Color::DarkGrey),
            Cell::new(name).fg(Color::Green),
            Cell::new(path).fg(Color::White),
        ]);
    }

    println!("\n{}", table);

    if total_pages > 1 {
        println!(
            "\nShowing {} projects (Page {}/{}) | Total: {}\n",
            projects.len(),
            current_page,
            total_pages,
            total
        );
    } else {
        println!(
            "\nTotal: {} project{}\n",
            total,
            if total == 1 { "" } else { "s" }
        );
    }
}
