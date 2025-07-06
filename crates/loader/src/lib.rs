use chrono::Local;
use gmod::{lua::State, gmod13_close, gmod13_open};
use libloading;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{
	fs,
	io::copy,
	path::PathBuf,
	time::Duration,
};

#[derive(Deserialize)]
struct Release {
	assets: Vec<Asset>,
	tag_name: String,
}

#[derive(Deserialize)]
struct Asset {
	name: String,
	browser_download_url: String,
}

#[derive(Deserialize, Serialize, Default)]
struct LoaderVersionCache {
	gmod_integration_loader: Option<String>,
	gmod_integration: Option<String>,
	gwsockets: Option<String>,
	reqwest: Option<String>,
}

const API_LATEST: &str =
	"https://api.github.com/repos/gmod-integration/auto-loader/releases/latest";
const DEST_DIR: &str = "garrysmod/lua/bin";
const VERSION_FILE: &str = "garrysmod/lua/bin/versions.json";

fn print_log(msg: &str) {
	let time = Local::now().format("%Y-%m-%d %H:%M:%S");
	println!(" | {} | Gmod Integration | Auto Loader: {}", time, msg);
}

fn load_loader_version_cache() -> LoaderVersionCache {
	fs::read_to_string(VERSION_FILE)
		.ok()
		.and_then(|content| serde_json::from_str(&content).ok())
		.unwrap_or_default()
}

fn save_loader_version_cache(cache: &LoaderVersionCache) {
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

fn download_asset(client: &Client, asset: &Asset) -> Result<(), Box<dyn std::error::Error>> {
	// Download the asset from GitHub releases
	let mut resp = client
		.get(&asset.browser_download_url)
		.header("User-Agent", "Gmod-Auto-Loader")
		.timeout(Duration::from_secs(30))
		.send()?;

	// Check if response is successful
	if !resp.status().is_success() {
		return Err(format!("HTTP error: {}", resp.status()).into());
	}

	// Create output path and temporary file for safe downloading
	let mut out_path = PathBuf::from(DEST_DIR);
	out_path.push(&asset.name);

	// Ensure parent directory exists
	if let Some(parent) = out_path.parent() {
		fs::create_dir_all(parent)?;
	}

	let tmp_path = out_path.with_extension("tmp");
	let mut file = fs::File::create(&tmp_path)?;
	copy(&mut resp, &mut file)?;
	
	// Verify file was written and has content
	let metadata = fs::metadata(&tmp_path)?;
	if metadata.len() == 0 {
		fs::remove_file(&tmp_path)?;
		return Err("Downloaded file is empty".into());
	}

	// Atomically rename temporary file to final destination
	fs::rename(tmp_path, &out_path)?;
	
	print_log(&format!("Downloaded {}", asset.name));
	Ok(())
}

fn delegate_to_real_loader(lua: State) -> i32 {
	unsafe {
		// Load the real integration library dynamically
		let suffix = get_platform_suffix();
		let lib_name = format!("{}/gmsv_gmod_integration_{}.dll", DEST_DIR, suffix);

		// Check if file exists before trying to load
		if !std::path::Path::new(&lib_name).exists() {
			print_log(&format!("Real integration file not found: {}", lib_name));
			return 1;
		}

		let lib = match libloading::Library::new(&lib_name) {
			Ok(lib) => lib,
			Err(e) => {
				print_log(&format!("Failed to load real integration: {}", e));
				return 1;
			}
		};

		// Get the gmod13_open function from the real integration
		let func: libloading::Symbol<unsafe extern "C" fn(State) -> i32> = match lib.get(b"gmod13_open") {
			Ok(func) => func,
			Err(e) => {
				print_log(&format!("Failed to find gmod13_open symbol: {}", e));
				return 1;
			}
		};
		
		print_log("Delegated to real integration");
		func(lua)
	}
}

#[gmod13_open]
fn gmod13_open(lua: State) -> i32 {
	print_log("Checking for updates...");
	
	// Ensure destination directory exists
	if let Err(e) = fs::create_dir_all(DEST_DIR) {
		print_log(&format!("Failed to create directory: {}", e));
		return 1; // Don't delegate if we can't even create directories
	}

	let mut version_cache = load_loader_version_cache();
	let client = Client::new();

	// Check if the real integration file exists on disk
	let suffix = get_platform_suffix();
	let lib_path = format!("{}/gmsv_gmod_integration_{}.dll", DEST_DIR, suffix);
	let file_exists = std::path::Path::new(&lib_path).exists();

	// Fetch latest release information from GitHub API
	let release: Release = match client
		.get(API_LATEST)
		.header("User-Agent", "Gmod-Auto-Loader")
		.timeout(Duration::from_secs(30))
		.send()
		.and_then(|r| r.error_for_status())
		.and_then(|r| r.json())
	{
		Ok(r) => r,
		Err(e) => {
			print_log(&format!("Error fetching release: {}", e));
			// If file exists, still try to delegate
			if file_exists {
				return delegate_to_real_loader(lua);
			}
			return 1;
		}
	};

	// Validate release data
	if release.tag_name.is_empty() {
		print_log("Invalid release: empty tag name");
		return delegate_to_real_loader(lua);
	}

	if release.assets.is_empty() {
		print_log("No assets found in release");
		return delegate_to_real_loader(lua);
	}

	// Skip update if version matches and file exists
	if let Some(current_version) = &version_cache.gmod_integration_loader {
		if current_version == &release.tag_name && file_exists {
			print_log(&format!("Already up to date ({})", release.tag_name));
			return delegate_to_real_loader(lua);
		}
	}

	if !file_exists {
		print_log("Real integration file missing, downloading...");
	} else {
		print_log(&format!("Updating from {} to {}", 
			version_cache.gmod_integration_loader.as_deref().unwrap_or("unknown"), 
			release.tag_name));
	}

	// Download the appropriate binary for current platform
	let target_asset = format!("gmsv_gmod_integration_{}.dll", suffix);
	let mut found_asset = false;
	
	for asset in &release.assets {
		if asset.name == target_asset {
			found_asset = true;
			if let Err(e) = download_asset(&client, asset) {
				print_log(&format!("Failed to download {}: {}", asset.name, e));
				// Clean up any partial download
				let partial_path = PathBuf::from(DEST_DIR).join(&asset.name);
				let _ = fs::remove_file(partial_path);
				return delegate_to_real_loader(lua);
			}
			break;
		}
	}

	if !found_asset {
		print_log(&format!("No matching asset found for platform: {}", suffix));
		return delegate_to_real_loader(lua);
	}

	// Update version cache with new version
	version_cache.gmod_integration_loader = Some(release.tag_name);
	if let Err(e) = serde_json::to_string_pretty(&version_cache)
		.map_err(|e| e.into())
		.and_then(|content| fs::write(VERSION_FILE, content).map_err(|e| e.into())) {
		print_log(&format!("Failed to save version cache: {}", e));
	}

	print_log("Update completed, delegating to real integration");
	delegate_to_real_loader(lua)
}

#[gmod13_close]
fn gmod13_close(lua: State) -> i32 {
	unsafe {
		// Load and call the real integration's close function
		let suffix = get_platform_suffix();
		let lib_name = format!("{}/gmsv_gmod_integration_{}.dll", DEST_DIR, suffix);

		// Check if file exists before trying to load
		if !std::path::Path::new(&lib_name).exists() {
			print_log(&format!("Real integration file not found during close: {}", lib_name));
			return 0; // Don't fail the close operation
		}

		let lib = match libloading::Library::new(&lib_name) {
			Ok(lib) => lib,
			Err(e) => {
				print_log(&format!("Failed to load real integration during close: {}", e));
				return 0; // Don't fail the close operation
			}
		};

		let func: libloading::Symbol<unsafe extern "C" fn(State) -> i32> = match lib.get(b"gmod13_close") {
			Ok(func) => func,
			Err(e) => {
				print_log(&format!("Failed to find gmod13_close symbol: {}", e));
				return 0; // Don't fail the close operation
			}
		};

		func(lua)
	}
}
