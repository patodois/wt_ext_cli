use std::{
	fs,
	fs::create_dir_all,
	path::{Path, PathBuf},
	str::FromStr,
};

use clap::ArgMatches;
use color_eyre::eyre::{Context, Result};
use log::warn;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use wt_blk::dxp_and_grp::{dxp::DxpPack, parse_buffered};

use crate::{
	error::{
		CliError,
		CliError::{DxpGrpError, DxpGrpSplitMissing},
	},
	fs_util::fd_recurse_folder_filtered,
};

pub fn unpack_dxp_and_grp(args: &ArgMatches) -> Result<()> {
	let input_dir = args
		.get_one::<String>("Input directory")
		.ok_or(CliError::RequiredFlagMissing)?;
	let parsed_input_dir = PathBuf::from_str(&input_dir)
		.or(Err(CliError::InvalidPath))
		.context(format!(
			"The provided input directory {} is not valid",
			input_dir
		))?;
	let input_read_dir = fs::read_dir(input_dir)?;

	let out_dir = args.get_one::<String>("Output directory");
	let complete_out_dir = out_dir.and_then(|p| PathBuf::from_str(p).ok());

	let keep_suffix = args.get_flag("Keep suffix");
	let textures = args.get_one::<String>("Textures").cloned();

	let mut prepared_files = vec![];
	fd_recurse_folder_filtered(&mut prepared_files, input_read_dir, |path| {
		let Some(name) = path.file_name() else {
			return false;
		};
		let fname = name.to_str().unwrap_or("");
		fname.ends_with(".dxp.bin") || fname.ends_with(".grp")
	})
	.unwrap();

	prepared_files
		.into_par_iter()
		.map(|prepared_file| {
			let mut dxp_or_grp = parse_buffered(&prepared_file.1).map_err(|e| DxpGrpError {
				dxp_error: e,
				file_name: prepared_file.0.to_str().unwrap().to_string(),
			})?;
			if !keep_suffix {
				for name in &mut dxp_or_grp {
					*name = name
						.split("*")
						.next()
						.ok_or(DxpGrpSplitMissing {
							line: name.to_string(),
						})?
						.to_owned();
				}
			}

			let parsed = dxp_or_grp.join("\n");
			let file_name = prepared_file
				.0
				.as_path()
				.file_name()
				.ok_or(CliError::MissingFileName)?
				.to_str()
				.ok_or(CliError::InvalidPath)?;
			let final_content = format!("folder {file_name}\n\n{parsed}");

			let mut final_path = prepared_file.0.clone();
			if file_name.ends_with("grp") {
				final_path.set_extension("grp.txt");
			} else {
				final_path.set_extension("txt");
			}
			let file = (final_path, final_content);

			let final_out = if let Some(out_dir) = &complete_out_dir {
				let out = out_dir.join(file.0.strip_prefix(&parsed_input_dir)?);
				create_dir_all(&out.parent().unwrap())?;
				out
			} else {
				file.0
			};
			if let Some(format) = &textures {
				if file_name.ends_with(".dxp.bin") {
					// Next to the .txt: "<pack>.dxp.bin" -> "<pack>/"
					let folder = final_out.with_file_name(file_name.trim_end_matches(".dxp.bin"));
					export_textures(&prepared_file.1, &folder, format == "png", file_name)?;
				}
			}
			fs::write(final_out, file.1)?;
			Ok(())
		})
		.collect::<Result<(), _>>()
}

/// Writes every decodable texture of a DXP pack into `folder`.
/// The pack is memory-mapped: packs reach over a gigabyte and are processed in parallel.
fn export_textures(file: &fs::File, folder: &Path, png: bool, pack_name: &str) -> Result<()> {
	// Safety: the pack is only read, and game files are not modified while exporting
	let map = unsafe { memmap2::Mmap::map(file)? };
	let pack = DxpPack::parse(&map).map_err(|e| DxpGrpError {
		dxp_error: e,
		file_name: pack_name.to_string(),
	})?;
	create_dir_all(folder)?;
	let mut skipped = 0;
	for (i, entry) in pack.entries.iter().enumerate() {
		if entry.header.is_empty() {
			continue;
		}
		let converted = pack
			.texture(i)
			.and_then(|t| if png { t.to_png() } else { t.to_dds() });
		let bytes = match converted {
			Ok(bytes) => bytes,
			Err(e) => {
				warn!("{pack_name}: skipping {}: {e}", entry.name);
				skipped += 1;
				continue;
			},
		};
		// Names look like "tex_c*", "tex_c$hq*" or "tex_c*?q0-1-1u01"; keep what precedes '*'
		// and replace characters Windows does not allow in file names
		let name: String = entry
			.name
			.split('*')
			.next()
			.unwrap_or(&entry.name)
			.chars()
			.map(|c| if r#"\/:?"<>|"#.contains(c) { '_' } else { c })
			.collect();
		fs::write(
			folder.join(format!("{name}.{}", if png { "png" } else { "dds" })),
			bytes,
		)?;
	}
	if skipped > 0 {
		warn!("{pack_name}: {skipped} textures could not be exported");
	}
	Ok(())
}
