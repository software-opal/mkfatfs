use eyre::ContextCompat;
use eyre::{Result, WrapErr};
use regex::RegexBuilder;
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

fn try_from_size_str(input: &str) -> Result<u32> {
    let re = RegexBuilder::new(r"^(\d+(?:\.\d*)?)\s*([gmk]?[i]?)[b]?$")
        .case_insensitive(true)
        .build()
        .unwrap();

    let m = re.captures(input).wrap_err_with(||format!("Unable to parse input, it should be a number optionally followed by a size modifier(G/Gi/M/Mi/K/Ki): {}", input))?;
    if let Some(i_part) = m.get(1).map(|m| m.as_str()) {
        let size_multiplier: &str = &m.get(2).map_or("", |m| m.as_str()).to_ascii_lowercase();
        let base_size = i_part
            .parse::<f64>()
            .wrap_err_with(|| format!("Unable to parse extracted float: {:?}", i_part))?;
        let size = base_size
            * match size_multiplier {
                "gi" => 1024.0 * 1024.0 * 1024.0,
                "g" => 1000.0 * 1000.0 * 1000.0,
                "mi" => 1024.0 * 1024.0,
                "m" => 1000.0 * 1000.0,
                "ki" => 1024.0,
                "k" => 1000.0,
                _ => 1.0,
            };
        return Ok(size as u32);
    } else {
        unreachable!()
    }
}

fn load_path(input: &OsStr) -> (PathBuf, bool) {
    let slashy_boi: bool =
        input.to_string_lossy().ends_with('/') || input.to_string_lossy().ends_with('\\');
    (input.into(), slashy_boi)
}

#[derive(StructOpt, Debug, PartialEq)]
pub struct CliArgs {
    #[structopt(parse(try_from_str=try_from_size_str), long="size", short="s")]
    size: u32,

    #[structopt(parse(from_os_str))]
    output: PathBuf,
    #[structopt(parse(from_os_str=load_path))]
    input: Vec<(PathBuf, bool)>,
}

pub fn path_to_array(path: &Path, strip_segments: usize) -> Result<Vec<String>> {
    let mut segment_names = path
        .ancestors()
        .filter_map(|s| s.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    segment_names.reverse();
    let re = if segment_names.len() > strip_segments {
        segment_names.split_off(strip_segments )
    } else {
        segment_names.split_off(segment_names.len() -1 )
    };
    assert!(re.len() >= 1);
    return Ok(re);
}

pub fn add_paths(
    out: &mut Vec<(PathBuf, Vec<String>)>,
    path: PathBuf,
    strip_segments: usize,
) -> Result<()> {
    if path.is_dir() {
        for entry in path.read_dir()? {
            add_paths(out, entry?.path(), strip_segments)?;
        }
    } else if path.is_file() {
        let segments = path_to_array(&path, strip_segments)?;
        out.push((path, segments))
    }
    Ok(())
}

pub fn main(_main_args: super::CliArgs, cmd_args: CliArgs) -> Result<()> {
    println!("Args: {:#?}", cmd_args);

    let paths = {
        let mut paths = Vec::with_capacity(cmd_args.input.len() * 10);
        for (path, trailing_slash) in cmd_args.input {
            let strip_segments = if trailing_slash { path.ancestors().filter(|s| s.file_name().is_some()).count() } else { 0 };

            add_paths(&mut paths, path, strip_segments)?;
        }
        paths
    };

    let sector_size: u32 = 512;
    let sectors: u32 = (cmd_args.size / sector_size)
        + if (cmd_args.size % sector_size) == 0 {
            0
        } else {
            1
        };

    fatfs::format_volume(
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&cmd_args.output)?,
        fatfs::FormatVolumeOptions::new()
            .fat_type(fatfs::FatType::Fat32)
            .bytes_per_sector(sector_size as u16)
            .total_sectors(sectors),
    )?;

    let fs = fatfs::FileSystem::new(
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .open(&cmd_args.output)?,
        fatfs::FsOptions::new().update_accessed_date(false),
    )?;

    for (path, segments) in paths {
        let mut folder = fs.root_dir();
        let fs_file_name = segments.join("/");
        println!("Putting {:?} into the disk image at {}", path, fs_file_name);
        for segment in &segments[..(segments.len() - 1)] {
            folder = folder.create_dir(segment).wrap_err_with(|| {
                format!(
                    "Failed to create directory '{}' when creating '{}'",
                    segment, fs_file_name
                )
            })?;
        }

        let mut file = fs.root_dir().create_file(&fs_file_name).wrap_err_with(|| {
            format!(
                "Failed to create file '{}' in image for '{}'",
                fs_file_name,
                path.to_string_lossy()
            )
        })?;
        std::io::copy(&mut fs::File::open(path)?, &mut file)?;
    }

    Ok(())
}
