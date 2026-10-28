use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use clap::{Parser, Subcommand, ValueEnum};
use guard_core::{
    CompatibilityImpact, CompatibilityReport, CompatibilityResult, ContractInterface, EnvMeta,
    compare_interfaces, compare_wasm, parse_contract_interface_file,
};
use guard_rpc::{Network as RpcNetwork, fetch_contract_wasm_with_timeout};
use serde::Serialize;

/// Default per-request Stellar RPC timeout, in seconds.
const DEFAULT_RPC_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Parser)]
#[command(name = "stellar-upgrade-guard")]
#[command(about = "Inspect and compare Soroban contract specifications")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect the contractspecv0 interface embedded in a Soroban WASM file.
    Inspect {
        /// Path to the compiled Soroban WASM file.
        wasm: PathBuf,

        /// Output format.
        #[arg(long, default_value = "text")]
        format: OutputFormat,
    },
    /// Compare two local WASM files, or compare a deployed contract to a candidate WASM.
    Compare {
        /// Path to the previous compiled Soroban WASM file.
        #[arg(long)]
        old: Option<PathBuf>,

        /// Path to the candidate compiled Soroban WASM file.
        #[arg(long)]
        new: Option<PathBuf>,

        /// Deployed contract id to use as the old interface.
        #[arg(long)]
        contract: Option<String>,

        /// Candidate compiled Soroban WASM file to compare against the deployed contract.
        #[arg(long)]
        candidate: Option<PathBuf>,

        /// Stellar network used to resolve the default RPC endpoint.
        #[arg(long)]
        network: Option<Network>,

        /// Explicit Stellar RPC endpoint. Overrides the network default when supplied.
        #[arg(long)]
        rpc_url: Option<String>,

        /// Seconds each Stellar RPC request may take before the comparison fails with exit code 2.
        /// Only used for deployed comparison.
        #[arg(
            long,
            default_value_t = DEFAULT_RPC_TIMEOUT_SECS,
            value_parser = clap::value_parser!(u64).range(1..=600)
        )]
        rpc_timeout: u64,

        /// Output format.
        #[arg(long, default_value = "text")]
        format: OutputFormat,

        /// Lowest finding severity that produces exit code 1. Each level also fails on every
        /// more severe level: `warning` fails on warning, unknown, and breaking findings;
        /// `unknown` on unknown and breaking; `breaking` only on breaking.
        #[arg(long, value_enum, default_value = "unknown")]
        fail_on: FailOn,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, ValueEnum)]
enum FailOn {
    Breaking,
    Unknown,
    Warning,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Network {
    Testnet,
    Mainnet,
}

/// Version of the `compare --format json` document layout. Increment only for changes that can
/// break existing consumers; adding fields does not require a new version.
const COMPARE_JSON_SCHEMA_VERSION: u32 = 1;

/// The `compare --format json` document, identical in shape for local and deployed comparison.
#[derive(Serialize)]
struct CompareJson<'a> {
    schema_version: u32,
    source: CompareSource<'a>,
    result: CompatibilityResult,
    summary: &'a guard_core::CompatibilitySummary,
    findings: &'a [guard_core::CompatibilityFinding],
}

impl<'a> CompareJson<'a> {
    fn new(source: CompareSource<'a>, report: &'a CompatibilityReport) -> Self {
        Self {
            schema_version: COMPARE_JSON_SCHEMA_VERSION,
            source,
            result: report.result,
            summary: &report.summary,
            findings: &report.findings,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CompareSource<'a> {
    LocalFiles {
        old: &'a str,
        new: &'a str,
    },
    DeployedContract {
        contract_id: &'a str,
        network: Option<&'static str>,
        candidate: &'a str,
        wasm_hash: &'a str,
        last_modified_ledger: Option<u32>,
    },
}
#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(2)
        }
    }
}

async fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    match Cli::parse().command {
        Command::Inspect { wasm, format } => {
            let interface = parse_contract_interface_file(wasm)?;
            match format {
                OutputFormat::Text => print_text(&interface),
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&interface)?),
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Compare {
            old,
            new,
            contract,
            candidate,
            network,
            rpc_url,
            rpc_timeout,
            format,
            fail_on,
        } => {
            run_compare(
                old,
                new,
                contract,
                candidate,
                network,
                rpc_url,
                Duration::from_secs(rpc_timeout),
                format,
                fail_on,
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_compare(
    old: Option<PathBuf>,
    new: Option<PathBuf>,
    contract: Option<String>,
    candidate: Option<PathBuf>,
    network: Option<Network>,
    rpc_url: Option<String>,
    rpc_timeout: Duration,
    format: OutputFormat,
    fail_on: FailOn,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    match compare_mode(old, new, contract, candidate, network, rpc_url)? {
        CompareMode::Local { old, new } => {
            let old_interface = parse_contract_interface_file(&old)?;
            let new_interface = parse_contract_interface_file(&new)?;
            let report = compare_interfaces(&old_interface, &new_interface);
            match format {
                OutputFormat::Text => print_compare_text(&old, &new, &report),
                OutputFormat::Json => {
                    let (old, new) = (old.display().to_string(), new.display().to_string());
                    let source = CompareSource::LocalFiles {
                        old: &old,
                        new: &new,
                    };
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&CompareJson::new(source, &report))?
                    );
                }
            }
            Ok(compare_exit_code(&report, fail_on))
        }
        CompareMode::Deployed {
            contract,
            candidate,
            network,
            rpc_url,
        } => {
            let deployed =
                fetch_contract_wasm_with_timeout(&rpc_url, &contract, rpc_timeout).await?;
            let candidate_wasm = fs::read(&candidate)?;
            let report = compare_wasm(&deployed.wasm, &candidate_wasm)?;
            match format {
                OutputFormat::Text => {
                    print_deployed_compare_text(&contract, &candidate, network, &deployed, &report);
                }
                OutputFormat::Json => {
                    let candidate = candidate.display().to_string();
                    let source = CompareSource::DeployedContract {
                        contract_id: &contract,
                        network: network.map(Network::name),
                        candidate: &candidate,
                        wasm_hash: &deployed.wasm_hash_hex,
                        last_modified_ledger: deployed.last_modified_ledger,
                    };
                    let body = CompareJson::new(source, &report);
                    println!("{}", serde_json::to_string_pretty(&body)?);
                }
            }
            Ok(compare_exit_code(&report, fail_on))
        }
    }
}

enum CompareMode {
    Local {
        old: PathBuf,
        new: PathBuf,
    },
    Deployed {
        contract: String,
        candidate: PathBuf,
        network: Option<Network>,
        rpc_url: String,
    },
}

fn compare_mode(
    old: Option<PathBuf>,
    new: Option<PathBuf>,
    contract: Option<String>,
    candidate: Option<PathBuf>,
    network: Option<Network>,
    rpc_url: Option<String>,
) -> Result<CompareMode, String> {
    let local_present = old.is_some() || new.is_some();
    let deployed_present =
        contract.is_some() || candidate.is_some() || network.is_some() || rpc_url.is_some();
    if local_present && deployed_present {
        return Err(
            "local --old/--new mode cannot be combined with deployed --contract/--candidate mode"
                .to_owned(),
        );
    }
    if local_present {
        let old = old.ok_or_else(|| "local compare requires --old".to_owned())?;
        let new = new.ok_or_else(|| "local compare requires --new".to_owned())?;
        return Ok(CompareMode::Local { old, new });
    }

    let contract = contract.ok_or_else(|| {
        "compare requires either --old/--new or --contract/--candidate".to_owned()
    })?;
    let candidate = candidate.ok_or_else(|| "deployed compare requires --candidate".to_owned())?;
    let rpc_url = match rpc_url {
        Some(url) => url,
        None => {
            let network = network
                .ok_or_else(|| "deployed compare requires --network or --rpc-url".to_owned())?;
            network
                .to_rpc_network()
                .default_rpc_url()
                .ok_or_else(|| "mainnet deployed compare requires --rpc-url".to_owned())?
                .to_owned()
        }
    };
    Ok(CompareMode::Deployed {
        contract,
        candidate,
        network,
        rpc_url,
    })
}

impl Network {
    fn to_rpc_network(self) -> RpcNetwork {
        match self {
            Self::Testnet => RpcNetwork::Testnet,
            Self::Mainnet => RpcNetwork::Mainnet,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Testnet => "testnet",
            Self::Mainnet => "mainnet",
        }
    }
}

fn print_text(interface: &ContractInterface) {
    println!("Contract Interface");
    print_functions(interface);
    print_types(interface);
    print_errors(interface);
    print_events(interface);
    print_env_meta(interface);
}

fn print_env_meta(interface: &ContractInterface) {
    println!();
    println!("Env interface version:");
    match interface.env_meta {
        EnvMeta::Absent => println!("- none"),
        EnvMeta::InterfaceVersion(version) => println!("- {version}"),
        EnvMeta::Unreadable => println!("- unreadable"),
    }
}

fn print_functions(interface: &ContractInterface) {
    println!();
    println!("Functions:");
    if interface.functions.is_empty() {
        println!("- none");
        return;
    }
    for function in &interface.functions {
        let inputs = function
            .inputs
            .iter()
            .map(|input| format!("{}: {}", input.name, input.type_ref.0))
            .collect::<Vec<_>>()
            .join(", ");
        let outputs = function
            .outputs
            .iter()
            .map(|output| output.0.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        if outputs.is_empty() {
            println!("- {}({inputs})", function.name);
        } else {
            println!("- {}({inputs}) -> {outputs}", function.name);
        }
    }
}

fn print_types(interface: &ContractInterface) {
    println!();
    println!("Types:");
    let mut printed = false;
    for item in &interface.structs {
        printed = true;
        println!("- struct {}", item.name);
    }
    for item in &interface.enums {
        printed = true;
        println!("- enum {}", item.name);
    }
    for item in &interface.unions {
        printed = true;
        println!("- union {}", item.name);
    }
    if !printed {
        println!("- none");
    }
}

fn print_errors(interface: &ContractInterface) {
    println!();
    println!("Errors:");
    if interface.errors.is_empty() {
        println!("- none");
        return;
    }
    for error in &interface.errors {
        for case in &error.cases {
            println!("- {}.{} = {}", error.name, case.name, case.value);
        }
    }
}

fn print_events(interface: &ContractInterface) {
    println!();
    println!("Events:");
    if interface.events.is_empty() {
        println!("- none");
        return;
    }
    for event in &interface.events {
        println!("- {} ({})", event.name, event.data_format);
    }
}

fn print_compare_text(old: &Path, new: &Path, report: &CompatibilityReport) {
    println!("Stellar Upgrade Guard");
    println!();
    println!("Comparing:");
    println!("{}", old.display());
    println!("-> {}", new.display());
    print_findings("Breaking changes", CompatibilityImpact::Breaking, report);
    print_findings("Unknown changes", CompatibilityImpact::Unknown, report);
    print_findings("Warnings", CompatibilityImpact::Warning, report);
    print_findings(
        "Compatible changes",
        CompatibilityImpact::Compatible,
        report,
    );
    println!();
    match report.result {
        CompatibilityResult::Breaking => println!("Result: BREAKING"),
        CompatibilityResult::Unknown => println!("Result: UNKNOWN"),
        CompatibilityResult::Warning => println!("Result: WARNING"),
        CompatibilityResult::Compatible => println!("Result: COMPATIBLE"),
    }
}

fn print_deployed_compare_text(
    contract: &str,
    candidate: &Path,
    network: Option<Network>,
    deployed: &guard_rpc::DeployedContract,
    report: &CompatibilityReport,
) {
    println!("Stellar Upgrade Guard");
    println!();
    println!("Comparing deployed contract:");
    println!("{contract}");
    if let Some(network) = network {
        println!("Network: {}", network.name());
    }
    println!("Deployed WASM hash: {}", deployed.wasm_hash_hex);
    if let Some(ledger) = deployed.last_modified_ledger {
        println!("Contract code ledger: {ledger}");
    }
    println!("-> {}", candidate.display());
    print_findings("Breaking changes", CompatibilityImpact::Breaking, report);
    print_findings("Unknown changes", CompatibilityImpact::Unknown, report);
    print_findings("Warnings", CompatibilityImpact::Warning, report);
    print_findings(
        "Compatible changes",
        CompatibilityImpact::Compatible,
        report,
    );
    println!();
    match report.result {
        CompatibilityResult::Breaking => println!("Result: BREAKING"),
        CompatibilityResult::Unknown => println!("Result: UNKNOWN"),
        CompatibilityResult::Warning => println!("Result: WARNING"),
        CompatibilityResult::Compatible => println!("Result: COMPATIBLE"),
    }
}

fn print_findings(title: &str, impact: CompatibilityImpact, report: &CompatibilityReport) {
    println!();
    println!("{title}:");
    let findings = report
        .findings
        .iter()
        .filter(|finding| finding.impact == impact)
        .collect::<Vec<_>>();
    if findings.is_empty() {
        println!("- none");
        return;
    }
    for finding in findings {
        println!("- {}", finding.message);
    }
}

fn compare_exit_code(report: &CompatibilityReport, fail_on: FailOn) -> ExitCode {
    let summary = &report.summary;
    let failing = match fail_on {
        FailOn::Breaking => summary.breaking,
        FailOn::Unknown => summary.breaking + summary.unknown,
        FailOn::Warning => summary.breaking + summary.unknown + summary.warning,
    };
    if failing > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
