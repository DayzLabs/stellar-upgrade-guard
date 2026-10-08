use std::{fs, path::Path};

use stellar_xdr::ScSpecEntry;

use crate::{error::GuardError, model::ContractInterface, normalize::normalize_entries};

pub fn parse_contract_interface_file(
    path: impl AsRef<Path>,
) -> Result<ContractInterface, GuardError> {
    let path = path.as_ref();
    let wasm = fs::read(path).map_err(|source| GuardError::FileRead {
        path: path.to_path_buf(),
        source,
    })?;
    parse_contract_interface(&wasm)
}

pub fn parse_contract_interface(wasm: &[u8]) -> Result<ContractInterface, GuardError> {
    let entries = read_contract_spec(wasm)?;
    Ok(normalize_entries(entries))
}

pub fn read_contract_spec(wasm: &[u8]) -> Result<Vec<ScSpecEntry>, GuardError> {
    soroban_spec::read::from_wasm(wasm).map_err(|err| match err {
        soroban_spec::read::FromWasmError::Read(source) => {
            GuardError::InvalidWasm(source.to_string())
        }
        soroban_spec::read::FromWasmError::Parse(source) => {
            GuardError::InvalidContractSpec(source.to_string())
        }
        soroban_spec::read::FromWasmError::NotFound => GuardError::MissingContractSpec,
    })
}
