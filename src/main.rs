use eyre::Result;
use structopt::StructOpt;

mod generate;

#[derive(StructOpt, Debug, PartialEq)]
#[structopt(author)]
pub struct Cli {
    #[structopt(flatten)]
    args: CliArgs,
    #[structopt(subcommand)]
    subcommand: CliSubcommand,
}
#[derive(StructOpt, Debug, PartialEq)]
pub struct CliArgs {}

#[derive(StructOpt, Debug, PartialEq)]
pub enum CliSubcommand {
    #[structopt(name = "generate")]
    Generate(generate::CliArgs),
}

fn main() -> Result<()> {
    let opt = Cli::from_args();

    match opt.subcommand {
        CliSubcommand::Generate(args) => generate::main(opt.args, args),
    }
}
