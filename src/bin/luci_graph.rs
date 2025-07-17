use serde::Deserialize;
use std::{
    fs::{read_to_string, File},
    io::{Read, Write},
    path::PathBuf,
};

use clap::{Parser, ValueEnum};
use luci::{execution::Executable, scenario::Scenario, visualization::RenderGraph};

#[derive(Clone, Copy, Debug, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum GraphDrawSource {
    #[clap(name = "raw")]
    Scenario,
    #[clap(name = "graph")]
    Executable,
}

#[derive(Parser, Debug)]
#[command(
    name = "luci-graph",
    about = "Generate a Graphviz DOT graph from a scenario description."
)]
struct Args {
    #[clap(long = "input", short = 'i', help = "Scenario file (default: stdin)")]
    scenario_file: Option<PathBuf>,
    #[clap(long = "output", short = 'o', help = "Graphviz file (default: stdout")]
    output_file: Option<PathBuf>,
    #[clap(
        long = "source",
        short = 's',
        help = "Source of data. Raw scenario, or parsed graph"
    )]
    source: GraphDrawSource,
}

fn main() {
    let args = Args::parse();

    let result = run(&args);

    match args.output_file {
        Some(path) => {
            let mut file = File::create(path).expect("Failed to create output file");
            file.write(result.as_bytes())
                .expect("Failed to write to output file");
        }
        None => {
            println!("{}", result);
        }
    }
}

fn run(args: &Args) -> String {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    let scenario = match &args.scenario_file {
        Some(path) => read_to_string(path).expect("Failed to read scenario file"),
        None => {
            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .expect("Failed to read from stdin");
            input.trim().to_string()
        }
    };

    let scenario: Scenario =
        serde_yaml::from_str(&scenario).expect("Failed to parse YAML scenario file");

    match args.source {
        GraphDrawSource::Scenario => scenario.render(),
        GraphDrawSource::Executable => {
            let executable =
                Executable::build(&scenario, None).expect("Failed to build execution graph");
            executable.render()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::GraphDrawSource;
    use test_case::test_case;

    use super::run;

    #[test_case(GraphDrawSource::Scenario; "graph from yaml")]
    #[test_case(GraphDrawSource::Executable; "graph from executable")]
    fn output_snapshot(source: GraphDrawSource) {
        let args = super::Args {
            scenario_file: Some("tests/luci_graph/sample.yml".into()),
            output_file: None,
            source,
        };
        let result = run(&args);

        insta::assert_snapshot!(format!("graph from {:?}", source), result);
    }
}
