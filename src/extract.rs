use eyre::{Result, WrapErr};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

#[derive(StructOpt, Debug, PartialEq)]
pub struct CliArgs {
    #[structopt(short, long)]
    overwrite: bool,

    #[structopt(parse(from_os_str))]
    image: PathBuf,
    #[structopt(parse(from_os_str))]
    output: PathBuf,
}

pub fn save_dir(opts: &CliArgs, target: &Path, folder: fatfs::Dir<fs::File>) -> Result<()> {
    for entry in folder.iter() {
        let entry = entry.wrap_err_with(|| {
            format!(
                "Unable to list directory for target folder {}",
                target.to_string_lossy()
            )
        })?;
        let out_path = target.join(entry.file_name());
        if entry.file_name() == "." || entry.file_name() == ".." {
            continue;
        } else if entry.is_dir() {
            if !out_path.is_dir() {
                fs::create_dir(&out_path).wrap_err_with(|| {
                    format!("Unable to create directory {}", out_path.to_string_lossy())
                })?;
            }
            save_dir(opts, &out_path, entry.to_dir())?;
        } else {
            if opts.overwrite || !out_path.is_file() {
                io::copy(
                    &mut entry.to_file(),
                    &mut fs::File::create(&out_path).wrap_err_with(|| {
                        format!(
                            "Unable to open file for writing: {}",
                            out_path.to_string_lossy()
                        )
                    })?,
                )
                .with_context(|| {
                    format!(
                        "Unable to copy file from the image to {}",
                        out_path.to_string_lossy()
                    )
                })?;
            } else {
                return Err(eyre::eyre!(
                    "The output path `{}` already exists, use `--overwrite` to overwrite it",
                    out_path.to_string_lossy()
                ));
            }
        }
    }
    Ok(())
}

pub fn main(_main_args: super::CliArgs, cmd_args: CliArgs) -> Result<()> {
    println!("Args: {:#?}", cmd_args);

    let fs = fatfs::FileSystem::new(
        fs::OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(&cmd_args.image)?,
        fatfs::FsOptions::new().update_accessed_date(false),
    )?;

    let output = &cmd_args.output;

    if !output.exists() {
        fs::create_dir_all(output)?;
    } else if output.is_file() {
        return Err(eyre::eyre!(
            "The output path should be a directory, or not exist".to_owned()
        ));
    }

    let folder: fatfs::Dir<fs::File> = fs.root_dir();
    save_dir(&cmd_args, &output, folder)?;

    Ok(())
}
