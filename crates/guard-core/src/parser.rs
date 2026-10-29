use std::{fs, io::Cursor, path::Path};

use stellar_xdr::{Limited, Limits, ReadXdr, ScEnvMetaEntry, ScSpecEntry};
use wasmparser::{Parser, Payload};

use crate::{
    error::GuardError,
    model::{ContractInterface, EnvInterfaceVersion, EnvMeta},
    normalize::normalize_entries,
};

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
    let mut interface = normalize_entries(entries);
    interface.env_meta = read_env_meta(wasm)?;
    Ok(interface)
}

/// Reads the first `contractenvmetav0` custom section.
///
/// A missing or empty section is [`EnvMeta::Absent`]. A section whose bytes are not a valid
/// `SCEnvMetaEntry` stream is [`EnvMeta::Unreadable`] rather than an error, so contracts built
/// against a newer environment than this tool's XDR still compare; the comparison reports that
/// the metadata could not be read. Only a structurally invalid WASM file is an error.
pub fn read_env_meta(wasm: &[u8]) -> Result<EnvMeta, GuardError> {
    for payload in Parser::new(0).parse_all(wasm) {
        let payload = payload.map_err(|err| GuardError::InvalidWasm(err.to_string()))?;
        if let Payload::CustomSection(section) = payload
            && section.name() == "contractenvmetav0"
        {
            return Ok(decode_env_meta(section.data()));
        }
    }
    Ok(EnvMeta::Absent)
}

fn decode_env_meta(data: &[u8]) -> EnvMeta {
    let limits = Limits {
        depth: 500,
        len: 0x0100_0000,
    };
    let entries = ScEnvMetaEntry::read_xdr_iter(&mut Limited::new(Cursor::new(data), limits))
        .collect::<Result<Vec<_>, _>>();
    match entries {
        Err(_) => EnvMeta::Unreadable,
        Ok(entries) => match entries.into_iter().next() {
            None => EnvMeta::Absent,
            Some(ScEnvMetaEntry::ScEnvMetaKindInterfaceVersion(version)) => {
                EnvMeta::InterfaceVersion(EnvInterfaceVersion {
                    protocol: version.protocol,
                    pre_release: version.pre_release,
                })
            }
        },
    }
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
