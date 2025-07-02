use chrono::Local;
use gmod::{lua::State, gmod13_close, gmod13_open};
use libloading;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::{
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

fn print_log(msg: &str) {
	let time = Local::now().format("%Y-%m-%d %H:%M:%S");
	println!(" | {} | Gmod Integration | Auto Loader: {}", time, msg);
}

fn download_asset(client: &Client, asset: &Asset) -> Result<(), Box<dyn std::error::Error>> {
	let mut resp = client
		.get(&asset.browser_download_url)
		.header("User-Agent", "Gmod-Auto-Loader")
		.timeout(Duration::from_secs(30))
		.send()?;

	let mut out_path = PathBuf::from(DEST_DIR);
	out_path.push(&asset.name);

	let tmp_path = out_path.with_extension("tmp");
	let mut file = fs::File::create(&tmp_path)?;
	copy(&mut resp, &mut file)?;
	fs::rename(tmp_path, &out_path)?;
	
	print_log(&format!("Downloaded {}", asset.name));
	Ok(())
}

#[gmod13_open]
fn gmod13_open(lua: State) -> i32 {
	print_log("Checking for updates...");
	let client = Client::new();

	let release: Release = match client
		.get(API_LATEST)
		.header("User-Agent", "Gmod-Auto-Loader")
		.send()
		.and_then(|r| r.error_for_status())
		.and_then(|r| r.json())
	{
		Ok(r) => r,
		Err(e) => {
			print_log(&format!("Error fetching release: {}", e));
			return 1;
		}
	};

	// Determine the correct asset names for the current platform
	let suffix = if cfg!(target_os = "windows") {
		if cfg!(target_arch = "x86_64") { "win64" } else { "win32" }
	} else {
		if cfg!(target_arch = "x86_64") { "linux64" } else { "linux" }
	};
	
	let target_assets = [
		format!("gmod_integration_{}.dll", suffix),
		format!("gmsv_gmod_integration_loader_{}.dll", suffix),
	];
	
	for asset in &release.assets {
		if target_assets.contains(&asset.name) {
			if let Err(e) = download_asset(&client, asset) {
				print_log(&format!("Failed to download {}: {}", asset.name, e));
			}
		}
	}

	// Delegate to the real loader DLL
	unsafe {
		let suffix = if cfg!(target_os = "windows") {
			if cfg!(target_arch = "x86_64") { "win64" } else { "win32" }
		} else {
			if cfg!(target_arch = "x86_64") { "linux64" } else { "linux" }
		};
		let lib_name = format!("{}/gmsv_gmod_integration_loader_{}.dll", DEST_DIR, suffix);

		let lib = libloading::Library::new(&lib_name)
			.unwrap_or_else(|_| panic!("Cannot load real loader: {}", lib_name));
		let func: libloading::Symbol<unsafe extern "C" fn(State) -> i32> =
			lib.get(b"gmod13_open").expect("symbol not found");
		
		print_log("Loaded successfully");
		func(lua)
	}
}

#[gmod13_close]
fn gmod13_close(lua: State) -> i32 {
	unsafe {
		let suffix = if cfg!(target_os = "windows") {
			if cfg!(target_arch = "x86_64") { "win64" } else { "win32" }
		} else {
			if cfg!(target_arch = "x86_64") { "linux64" } else { "linux" }
		};
		let lib_name = format!("{}/gmsv_gmod_integration_loader_{}.dll", DEST_DIR, suffix);

		let lib = libloading::Library::new(&lib_name)
			.unwrap_or_else(|_| panic!("Cannot load real loader: {}", lib_name));
		let func: libloading::Symbol<unsafe extern "C" fn(State) -> i32> =
			lib.get(b"gmod13_close").expect("symbol not found");
		func(lua)
	}
}
