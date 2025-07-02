use gmod::{gmod13_close, gmod13_open, lua::State};
use serde::Deserialize;
use std::{fs, io::copy, path::{Path, PathBuf}};
use reqwest::blocking::Client;
use zip::ZipArchive;
use chrono::Local;

#[derive(Deserialize, Debug)]
struct Release {
	zipball_url: String,
}

fn print_log(msg: &str) {
	let time = Local::now().format("%Y-%m-%d %H:%M:%S");
	println!(" | {} | Gmod Integration | Auto Updater: {}", time, msg);
}

#[gmod13_open]
fn gmod13_open(_lua: State) -> i32 {
	print_log("Starting auto-updater...");

	let client = Client::new();

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

	print_log("Downloading latest version...");

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

	print_log("Update completed successfully!");

	0
}

#[gmod13_close]
fn exit(_: State) -> i32 {
	0
}
