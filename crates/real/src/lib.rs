use gmod::{gmod13_close, gmod13_open, lua::State};
use serde::{Deserialize, Serialize};
use std::{fs, io::copy, path::{Path, PathBuf}};
use reqwest::blocking::Client;
use zip::ZipArchive;
use chrono::Local;

#[derive(Deserialize, Debug)]
struct Release {
	zipball_url: String,
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
	gmod_integration: Option<String>,
	gwsockets: Option<String>,
	reqwest: Option<String>,
}

const VERSION_FILE: &str = "garrysmod/lua/bin/versions.json";
const BIN_DIR: &str = "garrysmod/lua/bin";
const GWSOCKETS_API: &str = "https://api.github.com/repos/FredyH/GWSockets/releases/latest";
const REQWEST_API: &str = "https://api.github.com/repos/WilliamVenner/gmsv_reqwest/releases/latest";

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
	let mut resp = client
		.get(&asset.browser_download_url)
		.header("User-Agent", "Gmod-Integration-Updater")
		.send()?;

	let mut out_path = PathBuf::from(BIN_DIR);
	out_path.push(&asset.name);

	// Ensure bin directory exists
	fs::create_dir_all(BIN_DIR)?;

	let tmp_path = out_path.with_extension("tmp");
	let mut file = fs::File::create(&tmp_path)?;
	copy(&mut resp, &mut file)?;
	fs::rename(tmp_path, &out_path)?;
	
	print_log(&format!("Downloaded {}", asset.name));
	Ok(())
}

fn download_dependency(client: &Client, api_url: &str, dep_name: &str, current_version: Option<&String>) -> Result<Option<String>, Box<dyn std::error::Error>> {
	let release: Release = client
		.get(api_url)
		.header("User-Agent", "Gmod-Integration-Updater")
		.send()?
		.error_for_status()?
		.json()?;

	// Check if we need to update
	if let Some(current) = current_version {
		if current == &release.tag_name {
			print_log(&format!("{} is up to date ({})", dep_name, release.tag_name));
			return Ok(None);
		}
	}

	let suffix = get_platform_suffix();
	let target_name = format!("gmsv_{}_{}.dll", dep_name.to_lowercase(), suffix);
	
	for asset in &release.assets {
		if asset.name == target_name {
			if let Err(e) = download_dependency_asset(client, asset) {
				print_log(&format!("Failed to download {}: {}", asset.name, e));
				return Err(e);
			}
			return Ok(Some(release.tag_name));
		}
	}
	
	print_log(&format!("No matching asset found for {} on {}", dep_name, suffix));
	Ok(None)
}

#[gmod13_open]
fn gmod13_open(_lua: State) -> i32 {
	print_log("Starting auto-updater...");

	let mut version_cache = load_version_cache();
	let client = Client::new();

	// Download dependencies first
	print_log("Checking dependencies...");

	// Download GWsockets
	print_log("Checking GWsockets...");
	match download_dependency(&client, GWSOCKETS_API, "gwsockets", version_cache.gwsockets.as_ref()) {
		Ok(Some(new_version)) => {
			version_cache.gwsockets = Some(new_version);
			print_log("GWsockets updated");
		}
		Ok(None) => {}, // Up to date
		Err(e) => print_log(&format!("Failed to update GWsockets: {}", e)),
	}

	// Download reqwest
	print_log("Checking reqwest...");
	match download_dependency(&client, REQWEST_API, "reqwest", version_cache.reqwest.as_ref()) {
		Ok(Some(new_version)) => {
			version_cache.reqwest = Some(new_version);
			print_log("reqwest updated");
		}
		Ok(None) => {}, // Up to date
		Err(e) => print_log(&format!("Failed to update reqwest: {}", e)),
	}

	// Save dependency versions
	save_version_cache(&version_cache);

	// Continue with main integration update
	print_log("Checking main integration...");

	let res = match client
		.get("https://api.github.com/repos/gmod-integration/gmod-integration/releases/latest")
		.header("User-Agent", "Gmod-Integration-Updater")
		.send()
	{
		Ok(r) => r,
		Err(e) => {
			print_log(&format!("Failed to fetch release info: {:?}", e));
			return 1;
		}
	};

	let release: Release = match res.json() {
		Ok(r) => r,
		Err(e) => {
			print_log(&format!("Failed to parse release data: {:?}", e));
			return 1;
		}
	};

	// Check if main integration needs update
	if let Some(current) = &version_cache.gmod_integration {
		if current == &release.tag_name {
			print_log(&format!("Main integration is up to date ({})", release.tag_name));
			return 0;
		}
	}

	print_log("Downloading latest version...");

	// ...existing code for downloading and extracting main integration...
	let response = match client
		.get(&release.zipball_url)
		.header("User-Agent", "Gmod-Integration-Updater")
		.send()
	{
		Ok(r) => r,
		Err(e) => {
			print_log(&format!("Failed to download release: {:?}", e));
			return 1;
		}
	};

	let bytes = match response.bytes() {
		Ok(b) => b,
		Err(e) => {
			print_log(&format!("Failed to read download data: {:?}", e));
			return 1;
		}
	};

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
			return 1;
		}
	};

	let target_dir = PathBuf::from("./garrysmod/addons/gmod_integration_latest");

	if target_dir.exists() {
		let _ = fs::remove_dir_all(&target_dir);
	}

	if let Err(e) = fs::create_dir_all(&target_dir) {
		print_log(&format!("Failed to create installation directory: {:?}", e));
		return 1;
	}

	for i in 0..archive.len() {
		let mut file = archive.by_index(i).expect("Bad zip entry");
		let out_path = target_dir.join(file.name());

		if file.is_dir() {
			let _ = fs::create_dir_all(&out_path);
		} else {
			if let Some(parent) = out_path.parent() {
				let _ = fs::create_dir_all(parent);
			}
			let mut out_file = fs::File::create(&out_path).expect("Failed to create file");
			let _ = copy(&mut file, &mut out_file);
		}
	}

	print_log("Installing update...");

	let extracted_root = match fs::read_dir(&target_dir)
		.expect("Failed to read target dir")
		.next()
	{
		Some(Ok(entry)) => entry.path(),
		_ => {
			print_log("Error: No extracted folder found");
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

	// Update main integration version and save
	version_cache.gmod_integration = Some(release.tag_name);
	save_version_cache(&version_cache);

	print_log("Update completed successfully!");

	0
}

#[gmod13_close]
fn exit(_: State) -> i32 {
	0
}
