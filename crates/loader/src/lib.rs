use chrono::Local;
use gmod::{lua::State, gmod13_close, gmod13_open};
use libloading;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::{
	env,
	fs,
	io::copy,
	path::PathBuf,
	time::Duration,
};

#[derive(Deserialize)]
struct Release {
	assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
	name: String,
	browser_download_url: String,
}

const API_LATEST: &str =
	"https://api.github.com/repos/gmod-integration/auto-loader/releases/latest";
const DEST_DIR: &str = "garrysmod/lua/bin";

fn download_asset(client: &Client, asset: &Asset) -> Result<(), Box<dyn std::error::Error>> {
	println!("[Loader] Preparing to download asset: {}", asset.name);

	let mut resp = client
		.get(&asset.browser_download_url)
		.header("User-Agent", "Gmod-Auto-Loader")
		.timeout(Duration::from_secs(30))
		.send()?;
	println!("[Loader] HTTP request sent for asset: {}", asset.name);

	let mut out_path = PathBuf::from(DEST_DIR);
	out_path.push(&asset.name);

	let tmp_path = out_path.with_extension("tmp");
	println!("[Loader] Writing to temporary file: {:?}", tmp_path);
	let mut file = fs::File::create(&tmp_path)?;
	copy(&mut resp, &mut file)?;
	println!("[Loader] Download complete, renaming to final destination: {:?}", out_path);
	fs::rename(tmp_path, &out_path)?;
	println!("[Loader] Saved {}", asset.name);
	Ok(())
}

fn write_success_marker() {
	let date = Local::now().format("%Y-%m-%d").to_string();
	let mut marker_path = PathBuf::from(DEST_DIR);
	marker_path.push(format!("gmi_success_{}.txt", date));

	let version = env!("CARGO_PKG_VERSION");
	let os = env::consts::OS;
	let arch = env::consts::ARCH;

	let content = format!(
		"Auto-loader run: {date}\n\
         Loader version: {version}\n\
         Platform: {os}/{arch}\n",
		date = date,
		version = version,
		os = os,
		arch = arch,
	);

	println!("[Loader] Writing success marker to: {:?}", marker_path);
	match fs::write(&marker_path, content) {
		Ok(_) => println!("[Loader] Wrote success marker: {:?}", marker_path),
		Err(e) => println!("[Loader] Failed to write marker file: {}", e),
	}
}

#[gmod13_open]
fn gmod13_open(lua: State) -> i32 {
	println!("[Loader] Checking latest release…");
	let client = Client::new();

	// 1) Récupère la release
	println!("[Loader] Fetching latest release info from GitHub API: {}", API_LATEST);
	let release: Release = match client
		.get(API_LATEST)
		.header("User-Agent", "Gmod-Auto-Loader")
		.send()
		.and_then(|r| r.error_for_status())
		.and_then(|r| r.json())
	{
		Ok(r) => {
			println!("[Loader] Successfully fetched release info.");
			r
		},
		Err(e) => {
			println!("[Loader] Error fetching release info: {}", e);
			return 1;
		}
	};

	// 2) Determine the correct asset names for the current platform
	let suffix = if cfg!(target_os = "windows") {
		if cfg!(target_arch = "x86_64") { "win64" } else { "win32" }
	} else {
		if cfg!(target_arch = "x86_64") { "linux64" } else { "linux" }
	};
	
	let target_assets = [
		format!("gmod_integration_{}.dll", suffix),
		format!("gmsv_gmod_integration_loader_{}.dll", suffix),
	];
	
	println!("[Loader] Downloading assets for platform: {}", suffix);
	for asset in &release.assets {
		if target_assets.contains(&asset.name) {
			println!("[Loader] Found matching asset: {}", asset.name);
			if let Err(e) = download_asset(&client, asset) {
				println!("[Loader] Failed to download {}: {}", asset.name, e);
			}
		}
	}

	// 3) Écrit le marker de succès
	println!("[Loader] Writing success marker file…");
	write_success_marker();

	// 4) Délègue à la vraie DLL loader
	println!("[Loader] Delegating to the real loader DLL…");
	unsafe {
		let suffix = if cfg!(target_os = "windows") {
			if cfg!(target_arch = "x86_64") { "win64" } else { "win32" }
		} else {
			if cfg!(target_arch = "x86_64") { "linux64" } else { "linux" }
		};
		let lib_name = format!("{}/gmsv_gmod_integration_loader_{}.dll", DEST_DIR, suffix);
		println!("[Loader] Loading library: {}", lib_name);

		let lib = libloading::Library::new(&lib_name)
			.unwrap_or_else(|_| panic!("[Loader] cannot load real loader: {}", lib_name));
		let func: libloading::Symbol<unsafe extern "C" fn(State) -> i32> =
			lib.get(b"gmod13_open").expect("symbol not found");
		println!("[Loader] Calling gmod13_open in real loader.");
		func(lua)
	}
}

#[gmod13_close]
fn gmod13_close(lua: State) -> i32 {
	println!("[Loader] Delegating to the real loader DLL for close…");
	unsafe {
		let suffix = if cfg!(target_os = "windows") {
			if cfg!(target_arch = "x86_64") { "win64" } else { "win32" }
		} else {
			if cfg!(target_arch = "x86_64") { "linux64" } else { "linux" }
		};
		let lib_name = format!("{}/gmsv_gmod_integration_loader_{}.dll", DEST_DIR, suffix);
		println!("[Loader] Loading library: {}", lib_name);

		let lib = libloading::Library::new(&lib_name)
			.unwrap_or_else(|_| panic!("[Loader] cannot load real loader: {}", lib_name));
		let func: libloading::Symbol<unsafe extern "C" fn(State) -> i32> =
			lib.get(b"gmod13_close").expect("symbol not found");
		println!("[Loader] Calling gmod13_close in real loader.");
		func(lua)
	}
}
