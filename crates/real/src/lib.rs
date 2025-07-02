use gmod::{gmod13_close, gmod13_open, lua::State};
use serde::Deserialize;
use std::{fs, io::copy, path::{Path, PathBuf}};
use reqwest::blocking::Client;
use zip::ZipArchive;

#[derive(Deserialize, Debug)]
struct Release {
	zipball_url: String,
}

#[gmod13_open]
fn gmod13_open(_lua: State) -> i32 {
	println!("[Gmod Integration - Auto Update] Checking latest release...");

	let client = Client::new();

	let res = match client
		.get("https://api.github.com/repos/gmod-integration/gmod-integration/releases/latest")
		.header("User-Agent", "Gmod-Integration-Updater")
		.send()
	{
		Ok(r) => r,
		Err(e) => {
			println!("[Gmod Integration - Auto Update] Error: {:?}", e);
			return 1;
		}
	};

	let release: Release = match res.json() {
		Ok(r) => r,
		Err(e) => {
			println!("[Gmod Integration - Auto Update] Error: {:?}", e);
			return 1;
		}
	};

	println!("[Gmod Integration - Auto Update] Downloading latest version...");

	let response = match client
		.get(&release.zipball_url)
		.header("User-Agent", "Gmod-Integration-Updater")
		.send()
	{
		Ok(r) => r,
		Err(e) => {
			println!("[Gmod Integration - Auto Update] Error: {:?}", e);
			return 1;
		}
	};

	let bytes = match response.bytes() {
		Ok(b) => b,
		Err(e) => {
			println!("[Gmod Integration - Auto Update] Error: {:?}", e);
			return 1;
		}
	};

	let zip_path = Path::new("gmod-integration.zip");

	if let Err(e) = fs::write(&zip_path, &bytes) {
		println!("[Gmod Integration - Auto Update] Error: {:?}", e);
		return 1;
	}

	println!("[Gmod Integration - Auto Update] Extracting files...");

	let file = match fs::File::open(&zip_path) {
		Ok(f) => f,
		Err(e) => {
			println!("[Gmod Integration - Auto Update] Error: {:?}", e);
			return 1;
		}
	};

	let mut archive = match ZipArchive::new(file) {
		Ok(a) => a,
		Err(e) => {
			println!("[Gmod Integration - Auto Update] Error: {:?}", e);
			return 1;
		}
	};

	let target_dir = PathBuf::from("./garrysmod/addons/gmod_integration_latest");

	if target_dir.exists() {
		let _ = fs::remove_dir_all(&target_dir);
	}

	if let Err(e) = fs::create_dir_all(&target_dir) {
		println!("[Gmod Integration - Auto Update] Error: {:?}", e);
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

	println!("[Gmod Integration - Auto Update] Installing update...");

	let extracted_root = match fs::read_dir(&target_dir)
		.expect("Failed to read target dir")
		.next()
	{
		Some(Ok(entry)) => entry.path(),
		_ => {
			println!("[Gmod Integration - Auto Update] Error: No extracted folder found");
			return 1;
		}
	};

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


	println!("[Gmod Integration - Auto Update] Executing gmod-integration.lua...");

	0
}

#[gmod13_close]
fn exit(_: State) -> i32 {
	0
}
