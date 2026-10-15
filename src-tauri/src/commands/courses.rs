use std::path::Path;

use crate::db::{self, DbState, SaveCourseInput};
use crate::parser;

#[tauri::command]
pub async fn parse_course_folder(folder_path: String) -> Result<parser::ParsedCourse, String> {
    let path = Path::new(&folder_path);
    parser::parse_folder(path)
}

#[tauri::command]
pub async fn import_course(
    state: tauri::State<'_, DbState>,
    parsed: parser::ParsedCourse,
    config: SaveCourseInput,
) -> Result<i64, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::save_parsed_course(&conn, &parsed, &config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_courses(state: tauri::State<'_, DbState>) -> Result<Vec<db::Course>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::get_all_courses(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_course(
    state: tauri::State<'_, DbState>,
    course_id: i64,
) -> Result<Option<db::Course>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::get_course_by_id(&conn, course_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_course_detail(
    state: tauri::State<'_, DbState>,
    course_id: i64,
) -> Result<Option<db::CourseDetail>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::get_course_detail(&conn, course_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_course(
    state: tauri::State<'_, DbState>,
    course_id: i64,
    title: String,
    author: String,
    accent_color: String,
    category: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::update_course(&conn, course_id, &title, &author, &accent_color, &category)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_course_progress(
    state: tauri::State<'_, DbState>,
    course_id: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::reset_course_progress(&conn, course_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_course(state: tauri::State<'_, DbState>, course_id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::delete_course(&conn, course_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_bookmarked_courses(state: tauri::State<'_, DbState>) -> Result<Vec<db::Course>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::get_bookmarked_courses(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_bookmark(state: tauri::State<'_, DbState>, course_id: i64) -> Result<bool, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::toggle_bookmark(&conn, course_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_dashboard_stats(state: tauri::State<'_, DbState>) -> Result<db::DashboardStats, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::get_dashboard_stats(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_progress_data(state: tauri::State<'_, DbState>) -> Result<db::ProgressData, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::get_progress_data(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_all_data(state: tauri::State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::delete_all_data(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_custom_categories(state: tauri::State<'_, DbState>) -> Result<Vec<String>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::get_custom_categories(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_custom_category(state: tauri::State<'_, DbState>, name: String) -> Result<(), String> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err("Category name cannot be empty".to_string());
    }
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::add_custom_category(&conn, &trimmed).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_custom_category(
    state: tauri::State<'_, DbState>,
    name: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::delete_custom_category(&conn, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_content(
    state: tauri::State<'_, DbState>,
    query: String,
) -> Result<Vec<db::SearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    db::search_content(&conn, &query).map_err(|e| e.to_string())
}

// --- File rescan (added/removed files vs. DB) ---

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddedFile {
    pub path: String,
    pub title: String,
    pub section_title: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingFile {
    pub lesson_id: i64,
    pub title: String,
    pub video_path: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileScanResult {
    pub added: Vec<AddedFile>,
    pub missing: Vec<MissingFile>,
    pub unchanged_count: usize,
}

/// Compare a local course's folder on disk against its lessons in the DB.
/// Returns which video files were added and which disappeared. Non-local
/// courses are rejected: only a filesystem path can be walked.
fn scan_files_impl(state: &DbState, course_id: i64) -> Result<FileScanResult, String> {
    let (source_type, folder_path, lessons) = {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        let course = db::get_course_by_id(&conn, course_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Course not found".to_string())?;
        let source_type: String = conn
            .query_row(
                "SELECT source_type FROM courses WHERE id = ?1",
                [course_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let lessons = db::get_course_lesson_files(&conn, course_id).map_err(|e| e.to_string())?;
        (source_type, course.folder_path, lessons)
    };

    if source_type != "local" {
        return Err("Only local courses can be scanned for file changes".to_string());
    }
    let root = Path::new(&folder_path);
    if !root.is_dir() {
        return Err(format!("Course folder not found: {}", folder_path));
    }

    let on_disk = parser::collect_all_videos(root)?;

    let mut disk_set: std::collections::HashSet<String> = on_disk
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let mut db_set: std::collections::HashSet<String> =
        lessons.iter().map(|l| l.video_path.clone()).collect();

    let missing: Vec<MissingFile> = lessons
        .iter()
        .filter(|l| !disk_set.contains(&l.video_path))
        .map(|l| MissingFile {
            lesson_id: l.id,
            title: l.title.clone(),
            video_path: l.video_path.clone(),
        })
        .collect();

    let added: Vec<AddedFile> = on_disk
        .iter()
        .filter(|p| !db_set.contains(p.to_string_lossy().as_ref()))
        .map(|p| {
            let path_str = p.to_string_lossy().into_owned();
            let title = p
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| path_str.clone());
            let section_title = p
                .parent()
                .and_then(|dir| dir.strip_prefix(root).ok())
                .map(|rel| rel.to_string_lossy().replace('/', " — "))
                .unwrap_or_default();
            AddedFile {
                path: path_str,
                title,
                section_title,
            }
        })
        .collect();

    let unchanged_count = db_set.intersection(&disk_set).count();
    disk_set.clear();
    db_set.clear();

    Ok(FileScanResult {
        added,
        missing,
        unchanged_count,
    })
}

#[tauri::command]
pub async fn scan_course_files(
    state: tauri::State<'_, DbState>,
    course_id: i64,
) -> Result<FileScanResult, String> {
    scan_files_impl(state.inner(), course_id)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RescanSummary {
    pub lessons_added: usize,
    pub lessons_removed: usize,
}

/// Apply file changes: remove lessons whose files are gone, add lessons for
/// new files (with duration/subtitle probing, as at import).
#[tauri::command]
pub async fn rescan_course_files(
    state: tauri::State<'_, DbState>,
    course_id: i64,
) -> Result<RescanSummary, String> {
    // Scan first (no lock held while walking the filesystem / probing).
    let scan = scan_files_impl(state.inner(), course_id)?;

    if scan.added.is_empty() && scan.missing.is_empty() {
        return Ok(RescanSummary { lessons_added: 0, lessons_removed: 0 });
    }

    // Build sections (duration + subtitle probing) without holding the DB
    // lock, then apply in one transaction.
    let (title, folder_path) = {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        let course = db::get_course_by_id(&conn, course_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Course not found".to_string())?;
        (course.title, course.folder_path)
    };
    let root = Path::new(&folder_path);
    let added_paths: Vec<std::path::PathBuf> =
        scan.added.iter().map(|a| std::path::PathBuf::from(&a.path)).collect();
    let sections = parser::build_rescan_sections(root, &title, &added_paths)?;

    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let missing_ids: Vec<i64> = scan.missing.iter().map(|m| m.lesson_id).collect();
    let (added, removed) = db::apply_course_rescan(&conn, course_id, &sections, &missing_ids)
        .map_err(|e| e.to_string())?;

    Ok(RescanSummary { lessons_added: added, lessons_removed: removed })
}
