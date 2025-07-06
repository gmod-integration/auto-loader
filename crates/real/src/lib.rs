use gmod::{gmod13_close, gmod13_open, lua::State};
use serde::{Deserialize, Serialize};
use std::{fs, io::copy, path::{Path, PathBuf}};
use reqwest::blocking::Client;
use zip::ZipArchive;
use chrono::Local;
use std::time::Duration;

#[derive(Deserialize, Debug)]
struct Release {
	tag_name: String,
	assets: Vec<Asset>,
}

#[derive(Deserialize, Debug)]
struct Asset {
	name: String,
	browser_download_url: String,
}

#[derive(Deserialize, Serialize, Default)]
struct VersionCache {
	gmod_integration_loader: Option<String>,
	gmod_integration: Option<String>,
	gwsockets: Option<String>,
	reqwest: Option<String>,
}

const VERSION_FILE: &str = "garrysmod/lua/bin/versions.json";
const BIN_DIR: &str = "garrysmod/lua/bin";
const GWSOCKETS_API: &str = "https://api.github.com/repos/FredyH/GWSockets/releases/latest";
const REQWEST_API: &str = "https://api.github.com/repos/WilliamVenner/gmsv_reqwest/releases/latest";
const TMP_JSON_PATH: &str = "garrysmod/data/gm_integration/tmp.json";

fn print_log(msg: &str) {
	let time = Local::now().format("%Y-%m-%d %H:%M:%S");
	println!(" | {} | Gmod Integration | Auto Updater: {}", time, msg);
}

fn load_version_cache() -> VersionCache {
	fs::read_to_string(VERSION_FILE)
		.ok()
		.and_then(|content| serde_json::from_str(&content).ok())
		.unwrap_or_default()
}

fn save_version_cache(cache: &VersionCache) {
	if let Ok(content) = serde_json::to_string_pretty(cache) {
		let _ = fs::write(VERSION_FILE, content);
	}
}

fn get_platform_suffix() -> &'static str {
	if cfg!(target_os = "windows") {
		if cfg!(target_arch = "x86_64") { "win64" } else { "win32" }
	} else {
		if cfg!(target_arch = "x86_64") { "linux64" } else { "linux" }
	}
}

fn download_dependency_asset(client: &Client, asset: &Asset) -> Result<(), Box<dyn std::error::Error>> {
	// Download dependency binary from GitHub releases
	let mut resp = client
		.get(&asset.browser_download_url)
		.header("User-Agent", "Gmod-Integration-Updater")
		.timeout(Duration::from_secs(30))
		.send()?;

	// Check if response is successful
	if !resp.status().is_success() {
		return Err(format!("HTTP error: {}", resp.status()).into());
	}

	let mut out_path = PathBuf::from(BIN_DIR);
	out_path.push(&asset.name);

	// Ensure bin directory exists
	fs::create_dir_all(BIN_DIR)?;

	// Download to temporary file then rename for atomic operation
	let tmp_path = out_path.with_extension("tmp");
	let mut file = fs::File::create(&tmp_path)?;
	copy(&mut resp, &mut file)?;
	
	// Verify file was written and has content
	let metadata = fs::metadata(&tmp_path)?;
	if metadata.len() == 0 {
		fs::remove_file(&tmp_path)?;
		return Err("Downloaded file is empty".into());
	}

	fs::rename(tmp_path, &out_path)?;
	
	print_log(&format!("Downloaded {}", asset.name));
	Ok(())
}

fn download_dependency(client: &Client, api_url: &str, dep_name: &str, current_version: Option<&String>) -> Result<Option<String>, Box<dyn std::error::Error>> {
	// Fetch latest release information for dependency
	let release: Release = client
		.get(api_url)
		.header("User-Agent", "Gmod-Integration-Updater")
		.timeout(Duration::from_secs(30))
		.send()?
		.error_for_status()?
		.json()?;

	// Validate release data
	if release.tag_name.is_empty() {
		return Err(format!("Invalid release for {}: empty tag name", dep_name).into());
	}

	// Skip if already up to date
	if let Some(current) = current_version {
		if current == &release.tag_name {
			print_log(&format!("{} is up to date ({})", dep_name, release.tag_name));
			return Ok(None);
		}
	}

	// Find and download the correct binary for current platform
	let suffix = get_platform_suffix();
	let target_name = format!("gmsv_{}_{}.dll", dep_name.to_lowercase(), suffix);
	
	for asset in &release.assets {
		if asset.name == target_name {
			if let Err(e) = download_dependency_asset(client, asset) {
				print_log(&format!("Failed to download {}: {}", asset.name, e));
				// Clean up any partial download
				let partial_path = PathBuf::from(BIN_DIR).join(&asset.name);
				let _ = fs::remove_file(partial_path);
				return Err(e);
			}
			return Ok(Some(release.tag_name));
		}
	}
	
	print_log(&format!("No matching asset found for {} on {}", dep_name, suffix));
	Ok(None)
}

fn update_tmp_json() {
	// Create directory if it doesn't exist
	if let Some(parent) = Path::new(TMP_JSON_PATH).parent() {
		let _ = fs::create_dir_all(parent);
	}
	
	// Signal to Lua that gmod integration was updated
	let tmp_content = r#"{
	"gmod_integration_latest_updated": true
}"#;
	
	if let Err(e) = fs::write(TMP_JSON_PATH, tmp_content) {
		print_log(&format!("Failed to update tmp.json: {}", e));
	} else {
		print_log("Updated tmp.json with gmod_integration_latest_updated = true");
	}
}

#[gmod13_open]
fn gmod13_open(_lua: State) -> i32 {
	print_log("Starting auto-updater...");

	let mut version_cache = load_version_cache();
	let client = Client::builder()
		.timeout(Duration::from_secs(30))
		.build()
		.unwrap_or_else(|_| Client::new());

	// Update dependencies first (GWsockets and reqwest)
	print_log("Checking dependencies...");

	// Download GWsockets with error recovery
	match download_dependency(&client, GWSOCKETS_API, "gwsockets", version_cache.gwsockets.as_ref()) {
		Ok(Some(new_version)) => {
			version_cache.gwsockets = Some(new_version);
			print_log("GWsockets updated");
		}
		Ok(None) => {}, // Up to date
		Err(e) => {
			print_log(&format!("Failed to update GWsockets: {}", e));
			// Continue with other dependencies
		}
	}

	// Download reqwest with error recovery
	match download_dependency(&client, REQWEST_API, "reqwest", version_cache.reqwest.as_ref()) {
		Ok(Some(new_version)) => {
			version_cache.reqwest = Some(new_version);
			print_log("reqwest updated");
		}
		Ok(None) => {}, // Up to date
		Err(e) => {
			print_log(&format!("Failed to update reqwest: {}", e));
			// Continue with main integration
		}
	}

	// Save dependency versions (even if some failed)
	save_version_cache(&version_cache);

	// Now update the main gmod integration addon
	print_log("Checking Gmod Integration...");

	// Fetch latest gmod integration release with timeout
	let res = match client
		.get("https://api.github.com/repos/gmod-integration/gmod-integration/releases/latest")
		.header("User-Agent", "Gmod-Integration-Updater")
		.timeout(Duration::from_secs(30))
		.send()
	{
		Ok(r) => r,
		Err(e) => {
			print_log(&format!("Failed to fetch release info: {:?}", e));
			return 1;
		}
	};

	// Check response status
	if !res.status().is_success() {
		print_log(&format!("API request failed with status: {}", res.status()));
		return 1;
	}

	let release: Release = match res.json() {
		Ok(r) => r,
		Err(e) => {
			print_log(&format!("Failed to parse release data: {:?}", e));
			return 1;
		}
	};

	// Validate release data
	if release.tag_name.is_empty() {
		print_log("Invalid release: empty tag name");
		return 1;
	}

	// Check if addon folder exists and version matches
	let addon_exists = Path::new("./garrysmod/addons/_gmod_integration_latest").exists();
	
	if let Some(current) = &version_cache.gmod_integration {
		if current == &release.tag_name && addon_exists {
			print_log(&format!("Gmod integration is up to date ({})", release.tag_name));
			return 0;
		}
	}

	if !addon_exists {
		print_log("Addon folder missing, downloading...");
	} else {
		print_log("Version mismatch, updating...");
	}

	print_log("Downloading latest version...");

	// Download source code archive from GitHub with timeout
	let download_url = format!("https://github.com/gmod-integration/gmod-integration/archive/refs/tags/{}.zip", release.tag_name);

	let response = match client
		.get(&download_url)
		.header("User-Agent", "Gmod-Integration-Updater")
		.timeout(Duration::from_secs(120)) // Longer timeout for large files
		.send()
	{
		Ok(r) => r,
		Err(e) => {
			print_log(&format!("Failed to download release: {:?}", e));
			return 1;
		}
	};

	// Check if response is successful
	if !response.status().is_success() {
		print_log(&format!("Download failed with status: {}", response.status()));
		return 1;
	}

	// Check content type
	if let Some(content_type) = response.headers().get("content-type") {
		print_log(&format!("Content-Type: {:?}", content_type));
	}

	let bytes = match response.bytes() {
		Ok(b) => b,
		Err(e) => {
			print_log(&format!("Failed to read download data: {:?}", e));
			return 1;
		}
	};

	// Verify downloaded content size is reasonable
	if bytes.len() > 100_000_000 { // 100MB limit
		print_log("Downloaded file is suspiciously large");
		return 1;
	}

	if bytes.len() < 1000 { // Minimum reasonable size
		print_log("Downloaded file is too small to be valid");
		return 1;
	}

	// Validate downloaded ZIP file
	if bytes.len() < 4 || &bytes[0..4] != b"PK\x03\x04" {
		print_log("Downloaded file is not a valid ZIP file");
		return 1;
	}

	let zip_path = Path::new("gmod-integration.zip");

	if let Err(e) = fs::write(&zip_path, &bytes) {
		print_log(&format!("Failed to save zip file: {:?}", e));
		return 1;
	}

	print_log("Extracting files...");

	let file = match fs::File::open(&zip_path) {
		Ok(f) => f,
		Err(e) => {
			print_log(&format!("Failed to open zip file: {:?}", e));
			return 1;
		}
	};

	let mut archive = match ZipArchive::new(file) {
		Ok(a) => a,
		Err(e) => {
			print_log(&format!("Failed to read zip archive: {:?}", e));
			// Clean up the invalid zip file
			let _ = fs::remove_file(&zip_path);
			return 1;
		}
	};

	// Extract ZIP archive to addon directory
	let target_dir = PathBuf::from("./garrysmod/addons/_gmod_integration_latest");

	if target_dir.exists() {
		let _ = fs::remove_dir_all(&target_dir);
	}

	if let Err(e) = fs::create_dir_all(&target_dir) {
		print_log(&format!("Failed to create installation directory: {:?}", e));
		return 1;
	}

	// Clean up on extraction failure
	for i in 0..archive.len() {
		let mut file = match archive.by_index(i) {
			Ok(f) => f,
			Err(e) => {
				print_log(&format!("Failed to read zip entry {}: {}", i, e));
				let _ = fs::remove_dir_all(&target_dir);
				let _ = fs::remove_file(&zip_path);
				return 1;
			}
		};
		
		let out_path = target_dir.join(file.name());

		// Prevent directory traversal attacks
		if !out_path.starts_with(&target_dir) {
			print_log(&format!("Suspicious file path in zip: {}", file.name()));
			let _ = fs::remove_dir_all(&target_dir);
			let _ = fs::remove_file(&zip_path);
			return 1;
		}

		if file.is_dir() {
			let _ = fs::create_dir_all(&out_path);
		} else {
			if let Some(parent) = out_path.parent() {
				if let Err(e) = fs::create_dir_all(parent) {
					print_log(&format!("Failed to create directory: {}", e));
					continue;
				}
			}
			
			match fs::File::create(&out_path) {
				Ok(mut out_file) => {
					if let Err(e) = copy(&mut file, &mut out_file) {
						print_log(&format!("Failed to extract file {}: {}", out_path.display(), e));
					}
				}
				Err(e) => {
					print_log(&format!("Failed to create file {}: {}", out_path.display(), e));
				}
			}
		}
	}

	print_log("Installing update...");

	// Verify extraction was successful
	let extracted_root = match fs::read_dir(&target_dir)
		.map_err(|e| format!("Failed to read target dir: {}", e))
		.and_then(|mut entries| entries.next().ok_or("No extracted folder found".to_string()))
		.and_then(|entry| entry.map_err(|e| format!("Failed to read entry: {}", e)))
	{
		Ok(entry) => entry.path(),
		Err(e) => {
			print_log(&format!("Error: {}", e));
			let _ = fs::remove_dir_all(&target_dir);
			let _ = fs::remove_file(&zip_path);
			return 1;
		}
	};

	// Move files from the GitHub-generated folder to the target directory
	for entry in fs::read_dir(&extracted_root).expect("Failed to read extracted content") {
		let entry = entry.expect("Failed to read entry");
		let from = entry.path();
		let to = target_dir.join(entry.file_name());

		let _ = fs::rename(&from, &to);
	}

	let _ = fs::remove_dir_all(&extracted_root);
	let _ = fs::remove_dir_all(target_dir.join(".git"));
	let _ = fs::remove_dir_all(target_dir.join(".github"));
	let _ = fs::remove_file(&zip_path);

	// Rename files and folders to use _gmod_integration_latest prefix
	let old_lua_path = target_dir.join("lua/autorun/gmod_integration.lua");
	let new_lua_path = target_dir.join("lua/autorun/_gmod_integration_latest.lua");
	
	if old_lua_path.exists() {
		if let Err(e) = fs::rename(&old_lua_path, &new_lua_path) {
			print_log(&format!("Failed to rename Lua file: {}", e));
		} else {
			print_log("Renamed gmod_integration.lua to _gmod_integration_latest.lua");
		}
	}

	// Rename the main folder if it exists
	let old_folder_path = target_dir.join("gmod_integration");
	let new_folder_path = target_dir.join("_gmod_integration_latest");
	
	if old_folder_path.exists() {
		if let Err(e) = fs::rename(&old_folder_path, &new_folder_path) {
			print_log(&format!("Failed to rename main folder: {}", e));
		} else {
			print_log("Renamed gmod_integration folder to _gmod_integration_latest");
		}
	}

	// Rename the lua/gmod_integration folder to lua/_gmod_integration_latest
	let old_lua_folder_path = target_dir.join("lua/gmod_integration");
	let new_lua_folder_path = target_dir.join("lua/_gmod_integration_latest");
	
	if old_lua_folder_path.exists() {
		if let Err(e) = fs::rename(&old_lua_folder_path, &new_lua_folder_path) {
			print_log(&format!("Failed to rename lua/gmod_integration folder: {}", e));
		} else {
			print_log("Renamed lua/gmod_integration folder to lua/_gmod_integration_latest");
		}
	}

	// Update version cache and signal completion
	version_cache.gmod_integration = Some(release.tag_name);
	save_version_cache(&version_cache);

	// Signal to Lua that update completed
	update_tmp_json();

	print_log("Update completed successfully!");

	0
}

#[gmod13_close]
fn gmod13_close(_: State) -> i32 {
	0
}
