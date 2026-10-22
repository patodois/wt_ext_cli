use clap::{Arg, ArgAction, Command, ValueHint};

pub fn unpack_dxp_and_grp() -> Command {
	Command::new("unpack_dxp_and_grp")
		.long_flag("unpack_dxp")
		.about("Unpacks folder and subfolder DXP and GRP files to text-formatted file, and optionally exports DXP textures")
		.arg(
			Arg::new("Input directory")
				.short('i')
				.long("input_dir")
				.help("Folder with DXP/GRP files inside")
				.required(true)
				.value_hint(ValueHint::AnyPath)
		)
		.arg(
			Arg::new("Output directory")
				.short('o')
				.long("output_dir")
				.help("Target folder that will be created to contain new files, preserving file structure")
				.value_hint(ValueHint::FilePath)
		)
		.arg(
			Arg::new("Keep suffix")
				.long("keep_suffix")
				.help("Paths and Names inside the final DXP/GRP are always followed by \"water_garbage_pile_b_tex_d$hq*\" or random unicode chars \"u+4575\"")
				.num_args(0) // Expects only a flag, no data
				.action(ArgAction::SetTrue)
		)
		.arg(
			Arg::new("Textures")
				.long("textures")
				.help("Also export every texture of each DXP, as DDS (original format and mips) or PNG (top mip), into a folder named after the pack")
				.value_parser(["dds", "png"])
		)
}
