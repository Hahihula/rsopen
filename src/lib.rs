use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use strsim::levenshtein;
use walkdir::WalkDir;

#[derive(Debug)]
struct SearchResult {
    path: PathBuf,
    score: usize, // Lower is better (0 = exact)
    exec: Option<String>,
    is_desktop: bool,
}

/// Attempts to launch an application by its name.
pub fn launch_app(app_name: &str, verbose: bool) -> Result<()> {
    if verbose {
        println!("Attempting to launch '{}'...", app_name);
    }

    // 1. Fast Path (Native)
    if let Ok(()) = launch_app_native(app_name) {
        if verbose {
            println!("Successfully launched '{}' using native command.", app_name);
        }
        return Ok(());
    }

    let mut best_candidate: Option<SearchResult> = None;
    let query = app_name.to_lowercase();

    // Helper to update best candidate
    let mut update_best = |candidate: SearchResult| match &best_candidate {
        Some(current) => {
            if candidate.score < current.score {
                best_candidate = Some(candidate);
            }
        }
        None => best_candidate = Some(candidate),
    };

    // 2. Desktop File Search (Linux only)
    #[cfg(target_os = "linux")]
    {
        if verbose {
            println!("Native launch failed. Searching desktop entries...");
        }
        if let Some(res) = search_desktop_entries(app_name, &query, verbose) {
            if res.score == 0 {
                if verbose {
                    println!("Found exact desktop entry match.");
                }
                return launch_search_result(res);
            }
            update_best(res);
        }
    }

    if verbose {
        println!("Searching common paths...");
    }

    // 3. Common Paths Search
    let common_paths = get_common_paths();
    if let Some(res) = search_paths(&common_paths, app_name, &query) {
        if res.score == 0 {
            if verbose {
                println!("Found exact common path match.");
            }
            return launch_search_result(res);
        }
        update_best(res);
    }

    if verbose {
        println!("Searching full filesystem...");
    }

    // 4. Full Search
    let root = get_root_path();
    if let Some(res) = search_recursive(root, app_name, &query, verbose) {
        if res.score == 0 {
            if verbose {
                println!("Found exact filesystem match.");
            }
            return launch_search_result(res);
        }
        update_best(res);
    }

    // If we are here, no exact match. Check fuzzy.
    if let Some(res) = best_candidate {
        if verbose {
            println!(
                "No exact match found. Using closest fuzzy match (score: {}).",
                res.score
            );
        }
        return launch_search_result(res);
    }

    bail!("Could not find or launch application: {}", app_name);
}

fn launch_search_result(res: SearchResult) -> Result<()> {
    #[cfg(target_os = "linux")]
    if res.is_desktop {
        if let Some(exec) = res.exec {
            println!("Launching desktop entry: {:?} (Exec={})", res.path, exec);
            return execute_desktop_entry(&exec);
        }
    }

    println!("Launching: {:?}", res.path);
    launch_executable(&res.path)
}

#[cfg(target_os = "linux")]
fn execute_desktop_entry(exec: &str) -> Result<()> {
    let parts: Vec<&str> = exec
        .split_whitespace()
        .filter(|p| !p.starts_with('%'))
        .collect();
    if parts.is_empty() {
        bail!("Empty Exec line");
    }

    let cmd = parts[0];
    let args = &parts[1..];

    Command::new(cmd)
        .args(args)
        .spawn()
        .context("Failed to spawn desktop entry command")?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn search_desktop_entries(
    _original_name: &str,
    query: &str,
    verbose: bool,
) -> Option<SearchResult> {
    use std::io::BufRead;

    let dirs = [
        PathBuf::from("/usr/share/applications"),
        dirs::data_local_dir()
            .map(|p| p.join("applications"))
            .unwrap_or_else(|| PathBuf::from("~/.local/share/applications")),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        PathBuf::from("/snap/gui"),
    ];

    let mut best_res: Option<SearchResult> = None;

    for dir in &dirs {
        if !dir.exists() {
            continue;
        }

        let walker = WalkDir::new(dir).max_depth(2).follow_links(true);
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "desktop") {
                if let Ok(file) = std::fs::File::open(path) {
                    let reader = std::io::BufReader::new(file);
                    let mut name_found: Option<String> = None;
                    let mut exec_found: Option<String> = None;

                    for line in reader.lines().map_while(Result::ok) {
                        let line = line.trim();
                        if line.starts_with("Name=") {
                            name_found = Some(line.trim_start_matches("Name=").to_string());
                        } else if line.starts_with("Exec=") {
                            exec_found = Some(line.trim_start_matches("Exec=").to_string());
                        }
                    }

                    if let (Some(name), Some(exec)) = (name_found, exec_found) {
                        let name_lower = name.to_lowercase();

                        let score = if name_lower == query {
                            0
                        } else if name_lower.contains(query) {
                            levenshtein(&name_lower, query)
                        } else {
                            // Not a substring match, ignore
                            continue;
                        };

                        let candidate = SearchResult {
                            path: path.to_path_buf(),
                            score,
                            exec: Some(exec),
                            is_desktop: true,
                        };

                        if verbose {
                            println!("Desktop entry found: {:?}", candidate);
                        }

                        match best_res {
                            Some(ref current) => {
                                if score < current.score {
                                    best_res = Some(candidate);
                                }
                            }
                            None => best_res = Some(candidate),
                        }

                        if score == 0 {
                            return best_res;
                        }
                    }
                }
            }
        }
    }
    best_res
}

#[cfg(target_os = "windows")]
fn launch_app_native(app_name: &str) -> Result<()> {
    let output = Command::new("cmd")
        .args(["/C", "start", "", app_name])
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        bail!("Failed to launch on Windows")
    }
}

#[cfg(target_os = "macos")]
fn launch_app_native(app_name: &str) -> Result<()> {
    let output = Command::new("open").args(["-a", app_name]).output()?;
    if output.status.success() {
        Ok(())
    } else {
        bail!("Failed to launch on macOS")
    }
}

#[cfg(target_os = "linux")]
fn launch_app_native(app_name: &str) -> Result<()> {
    let output = Command::new(app_name).output();
    match output {
        Ok(o) if o.status.success() => Ok(()),
        _ => bail!("Failed to launch on Linux"),
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn launch_app_native(_: &str) -> Result<()> {
    bail!("Unsupported platform")
}

fn launch_executable(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        if path.extension().map_or(false, |ext| ext == "app") {
            let output = Command::new("open").arg(path).output()?;
            if output.status.success() {
                return Ok(());
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .output()?;
        if output.status.success() {
            return Ok(());
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = Command::new("xdg-open").arg(path).output();
        if let Ok(o) = output {
            if o.status.success() {
                return Ok(());
            }
        }

        if Command::new(path).spawn().is_ok() {
            return Ok(());
        }

        // Fallback to sh
        Command::new("sh")
            .arg(path)
            .spawn()
            .context("Failed to spawn executable (tried xdg-open, direct execution, and sh)")?;
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        Command::new(path)
            .spawn()
            .context("Failed to spawn executable")?;
        Ok(())
    }
}

fn get_common_paths() -> Vec<&'static str> {
    #[cfg(target_os = "windows")]
    {
        vec!["C:\\Program Files", "C:\\Program Files (x86)"]
    }
    #[cfg(target_os = "macos")]
    {
        vec![
            "/Applications",
            "/System/Applications",
            "/Users/Shared/Applications",
        ]
    }
    #[cfg(target_os = "linux")]
    {
        vec![
            "/usr/bin",
            "/usr/local/bin",
            "/opt",
            "/snap/bin",
            "/var/lib/flatpak/exports/bin",
        ]
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        vec![]
    }
}

fn get_root_path() -> &'static str {
    if cfg!(target_os = "windows") {
        "C:\\"
    } else {
        "/"
    }
}

fn search_paths(paths: &[&str], _original_name: &str, query: &str) -> Option<SearchResult> {
    let mut best_res: Option<SearchResult> = None;

    for base in paths {
        let path = Path::new(base);
        if !path.exists() {
            continue;
        }

        let walker = WalkDir::new(path).max_depth(3);

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            if let Some(score) = check_match_score(&entry, query) {
                let candidate = SearchResult {
                    path: entry.into_path(),
                    score,
                    exec: None,
                    is_desktop: false,
                };
                match best_res {
                    Some(ref current) => {
                        if score < current.score {
                            best_res = Some(candidate);
                        }
                    }
                    None => best_res = Some(candidate),
                }
                if score == 0 {
                    return best_res;
                }
            }
        }
    }
    best_res
}

fn search_recursive(
    root: &str,
    _original_name: &str,
    query: &str,
    _verbose: bool,
) -> Option<SearchResult> {
    let walker = WalkDir::new(root)
        .follow_links(false)
        .same_file_system(true)
        .into_iter();

    let walker = walker.filter_entry(|e| {
        #[cfg(target_os = "linux")]
        {
            let p = e.path();
            if p.starts_with("/proc")
                || p.starts_with("/sys")
                || p.starts_with("/dev")
                || p.starts_with("/run")
            {
                return false;
            }
        }
        true
    });

    let mut best_res: Option<SearchResult> = None;

    for entry in walker {
        match entry {
            Ok(e) => {
                if let Some(score) = check_match_score(&e, query) {
                    let candidate = SearchResult {
                        path: e.into_path(),
                        score,
                        exec: None,
                        is_desktop: false,
                    };
                    match best_res {
                        Some(ref current) => {
                            if score < current.score {
                                best_res = Some(candidate);
                            }
                        }
                        None => best_res = Some(candidate),
                    }
                    if score == 0 {
                        return best_res;
                    }
                }
            }
            Err(_err) => {}
        }
    }
    best_res
}

fn check_match_score(entry: &walkdir::DirEntry, query: &str) -> Option<usize> {
    #[cfg(target_os = "linux")]
    if entry.file_type().is_dir() {
        return None;
    }
    #[cfg(target_os = "windows")]
    if entry.file_type().is_dir() {
        return None;
    }

    let file_name = entry.file_name().to_string_lossy();
    let name_lower = file_name.to_lowercase();

    // Check match
    if name_lower == query {
        return Some(0);
    }

    #[cfg(target_os = "windows")]
    if name_lower == format!("{}.exe", query) {
        return Some(0);
    }

    #[cfg(target_os = "macos")]
    if name_lower == format!("{}.app", query) {
        return Some(0);
    }

    if name_lower.contains(query) {
        // Calculate score
        return Some(levenshtein(&name_lower, query));
    }

    None
}
