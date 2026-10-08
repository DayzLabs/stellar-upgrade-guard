use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use stellar_xdr::{
    ContractCodeEntry, ContractDataDurability, ContractDataEntry, ContractExecutable, ContractId,
    Hash, LedgerEntryData, LedgerKey, LedgerKeyContractCode, LedgerKeyContractData, Limits,
    ReadXdr, ScAddress, ScVal, WriteXdr,
};
use thiserror::Error;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DeployedContract {
    pub contract_id: String,
    pub wasm_hash: [u8; 32],
    pub wasm_hash_hex: String,
    pub wasm: Vec<u8>,
    pub last_modified_ledger: Option<u32>,
}

#[derive(Debug, Error)]
pub enum RpcError {
    #[error("invalid contract id `{0}`")]
    InvalidContractId(String),
    #[error("rpc transport error: {0}")]
    Transport(String),
    #[error("rpc returned error {code}: {message}")]
    RpcReturned { code: i64, message: String },
    #[error("rpc response was malformed: {0}")]
    MalformedResponse(String),
    #[error("contract `{contract_id}` was not found")]
    ContractNotFound { contract_id: String },
    #[error("contract code `{wasm_hash}` was not found")]
    ContractCodeNotFound { wasm_hash: String },
    #[error("invalid ledger entry xdr: {0}")]
    InvalidLedgerEntryXdr(String),
    #[error("unexpected ledger entry: expected {expected}, got {actual}")]
    UnexpectedLedgerEntry {
        expected: &'static str,
        actual: &'static str,
    },
    #[error("contract data entry does not contain a contract instance")]
    MissingContractInstance,
    #[error("stellar asset contracts do not reference deployable WASM")]
    StellarAssetContract,
    #[error("external contract executable references are not supported yet")]
    ExternalContractReference,
    #[error("returned contract code hash did not match requested hash")]
    ContractCodeHashMismatch,
    #[error("xdr encoding failed: {0}")]
    XdrEncode(String),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Network {
    Testnet,
    Mainnet,
}

impl Network {
    #[must_use]
    pub fn default_rpc_url(self) -> Option<&'static str> {
        match self {
            Self::Testnet => Some("https://soroban-testnet.stellar.org"),
            Self::Mainnet => None,
        }
    }
}

#[async_trait]
pub trait LedgerEntriesClient {
    async fn get_ledger_entries(
        &self,
        keys: Vec<String>,
    ) -> Result<GetLedgerEntriesResult, RpcError>;
}

pub struct HttpRpcClient {
    rpc_url: String,
    client: reqwest::Client,
}

impl HttpRpcClient {
    #[must_use]
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LedgerEntriesClient for HttpRpcClient {
    async fn get_ledger_entries(
        &self,
        keys: Vec<String>,
    ) -> Result<GetLedgerEntriesResult, RpcError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "getLedgerEntries",
            params: GetLedgerEntriesParams { keys },
        };
        let response = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .map_err(|err| RpcError::Transport(err.to_string()))?
            .error_for_status()
            .map_err(|err| RpcError::Transport(err.to_string()))?;
        let body = response
            .json::<JsonRpcResponse<GetLedgerEntriesResult>>()
            .await
            .map_err(|err| RpcError::MalformedResponse(err.to_string()))?;
        body.into_result()
    }
}

pub async fn fetch_contract_wasm(
    rpc_url: &str,
    contract_id: &str,
) -> Result<DeployedContract, RpcError> {
    fetch_contract_wasm_with_client(&HttpRpcClient::new(rpc_url), contract_id).await
}

pub async fn fetch_contract_wasm_with_client(
    client: &impl LedgerEntriesClient,
    contract_id: &str,
) -> Result<DeployedContract, RpcError> {
    let contract_key = contract_instance_key(contract_id)?;
    let contract_entries = client.get_ledger_entries(vec![contract_key]).await?;
    let contract_entry = first_entry(contract_entries, || RpcError::ContractNotFound {
        contract_id: contract_id.to_owned(),
    })?;
    let contract_data = decode_contract_data_entry(&contract_entry.xdr)?;
    let wasm_hash = wasm_hash_from_contract_data(&contract_data)?;
    let wasm_hash_hex = hex::encode(wasm_hash);

    let code_key = contract_code_key(wasm_hash)?;
    let code_entries = client.get_ledger_entries(vec![code_key]).await?;
    let code_entry = first_entry(code_entries, || RpcError::ContractCodeNotFound {
        wasm_hash: wasm_hash_hex.clone(),
    })?;
    let contract_code = decode_contract_code_entry(&code_entry.xdr)?;
    if contract_code.hash.0 != wasm_hash {
        return Err(RpcError::ContractCodeHashMismatch);
    }

    Ok(DeployedContract {
        contract_id: contract_id.to_owned(),
        wasm_hash,
        wasm_hash_hex,
        wasm: contract_code.code.to_vec(),
        last_modified_ledger: code_entry.last_modified_ledger,
    })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLedgerEntriesResult {
    #[allow(dead_code)]
    pub latest_ledger: Option<u32>,
    #[serde(default)]
    pub entries: Vec<LedgerEntryResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntryResult {
    pub xdr: String,
    pub last_modified_ledger: Option<u32>,
}

#[derive(Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'a str,
    id: u64,
    method: &'a str,
    params: GetLedgerEntriesParams,
}

#[derive(Serialize)]
struct GetLedgerEntriesParams {
    keys: Vec<String>,
}

#[derive(Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl<T> JsonRpcResponse<T> {
    fn into_result(self) -> Result<T, RpcError> {
        if let Some(error) = self.error {
            return Err(RpcError::RpcReturned {
                code: error.code,
                message: error.message,
            });
        }
        self.result
            .ok_or_else(|| RpcError::MalformedResponse("missing result".to_owned()))
    }
}

fn contract_instance_key(contract_id: &str) -> Result<String, RpcError> {
    let contract = stellar_strkey::Contract::from_string(contract_id)
        .map_err(|_| RpcError::InvalidContractId(contract_id.to_owned()))?;
    let key = LedgerKey::ContractData(LedgerKeyContractData {
        contract: ScAddress::Contract(ContractId(Hash(contract.0))),
        key: ScVal::LedgerKeyContractInstance,
        durability: ContractDataDurability::Persistent,
    });
    key.to_xdr_base64(Limits::none())
        .map_err(|err| RpcError::XdrEncode(err.to_string()))
}

fn contract_code_key(wasm_hash: [u8; 32]) -> Result<String, RpcError> {
    let key = LedgerKey::ContractCode(LedgerKeyContractCode {
        hash: Hash(wasm_hash),
    });
    key.to_xdr_base64(Limits::none())
        .map_err(|err| RpcError::XdrEncode(err.to_string()))
}

fn first_entry(
    result: GetLedgerEntriesResult,
    not_found: impl FnOnce() -> RpcError,
) -> Result<LedgerEntryResult, RpcError> {
    result.entries.into_iter().next().ok_or_else(not_found)
}

fn decode_contract_data_entry(xdr: &str) -> Result<ContractDataEntry, RpcError> {
    match decode_ledger_entry_data(xdr)? {
        LedgerEntryData::ContractData(entry) => Ok(entry),
        other => Err(RpcError::UnexpectedLedgerEntry {
            expected: "ContractData",
            actual: ledger_entry_data_name(&other),
        }),
    }
}

fn decode_contract_code_entry(xdr: &str) -> Result<ContractCodeEntry, RpcError> {
    match decode_ledger_entry_data(xdr)? {
        LedgerEntryData::ContractCode(entry) => Ok(entry),
        other => Err(RpcError::UnexpectedLedgerEntry {
            expected: "ContractCode",
            actual: ledger_entry_data_name(&other),
        }),
    }
}

fn decode_ledger_entry_data(xdr: &str) -> Result<LedgerEntryData, RpcError> {
    LedgerEntryData::from_xdr_base64(xdr, Limits::none())
        .map_err(|err| RpcError::InvalidLedgerEntryXdr(err.to_string()))
}

fn wasm_hash_from_contract_data(entry: &ContractDataEntry) -> Result<[u8; 32], RpcError> {
    let ScVal::ContractInstance(instance) = &entry.val else {
        return Err(RpcError::MissingContractInstance);
    };
    match &instance.executable {
        ContractExecutable::Wasm(hash) => Ok(hash.0),
        ContractExecutable::StellarAsset => Err(RpcError::StellarAssetContract),
        ContractExecutable::ExternalRef(_) => Err(RpcError::ExternalContractReference),
    }
}

fn ledger_entry_data_name(data: &LedgerEntryData) -> &'static str {
    match data {
        LedgerEntryData::Account(_) => "Account",
        LedgerEntryData::Trustline(_) => "Trustline",
        LedgerEntryData::Offer(_) => "Offer",
        LedgerEntryData::Data(_) => "Data",
        LedgerEntryData::ClaimableBalance(_) => "ClaimableBalance",
        LedgerEntryData::LiquidityPool(_) => "LiquidityPool",
        LedgerEntryData::ContractData(_) => "ContractData",
        LedgerEntryData::ContractCode(_) => "ContractCode",
        LedgerEntryData::ConfigSetting(_) => "ConfigSetting",
        LedgerEntryData::Ttl(_) => "Ttl",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::{
        BytesM, ContractCodeEntryExt, ContractDataEntry, ExtensionPoint, ScContractInstance,
    };

    const CONTRACT_ID: &str = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4";

    #[derive(Default)]
    struct MockClient {
        responses: std::sync::Mutex<Vec<Result<GetLedgerEntriesResult, RpcError>>>,
    }

    impl MockClient {
        fn new(responses: Vec<Result<GetLedgerEntriesResult, RpcError>>) -> Self {
            Self {
                responses: std::sync::Mutex::new(responses),
            }
        }
    }

    #[async_trait]
    impl LedgerEntriesClient for MockClient {
        async fn get_ledger_entries(
            &self,
            _keys: Vec<String>,
        ) -> Result<GetLedgerEntriesResult, RpcError> {
            self.responses.lock().expect("lock").remove(0)
        }
    }

    #[tokio::test]
    async fn fetches_wasm_from_contract_instance_and_code_entries() {
        let hash = [7; 32];
        let wasm = b"\0asm-test".to_vec();
        let client = MockClient::new(vec![
            Ok(result_with_xdr(contract_data_xdr(
                ContractExecutable::Wasm(Hash(hash)),
            ))),
            Ok(result_with_xdr(contract_code_xdr(hash, wasm.clone()))),
        ]);

        let deployed = fetch_contract_wasm_with_client(&client, CONTRACT_ID)
            .await
            .expect("fetch succeeds");

        assert_eq!(deployed.contract_id, CONTRACT_ID);
        assert_eq!(deployed.wasm_hash, hash);
        assert_eq!(deployed.wasm_hash_hex, hex::encode(hash));
        assert_eq!(deployed.wasm, wasm);
    }

    #[tokio::test]
    async fn reports_missing_contract_instance() {
        let client = MockClient::new(vec![Ok(GetLedgerEntriesResult {
            latest_ledger: None,
            entries: Vec::new(),
        })]);

        let err = fetch_contract_wasm_with_client(&client, CONTRACT_ID)
            .await
            .expect_err("missing contract fails");

        assert!(matches!(err, RpcError::ContractNotFound { .. }));
    }

    #[tokio::test]
    async fn reports_stellar_asset_contract() {
        let client = MockClient::new(vec![Ok(result_with_xdr(contract_data_xdr(
            ContractExecutable::StellarAsset,
        )))]);

        let err = fetch_contract_wasm_with_client(&client, CONTRACT_ID)
            .await
            .expect_err("asset contract fails");

        assert!(matches!(err, RpcError::StellarAssetContract));
    }

    #[tokio::test]
    async fn reports_missing_contract_code() {
        let client = MockClient::new(vec![
            Ok(result_with_xdr(contract_data_xdr(
                ContractExecutable::Wasm(Hash([1; 32])),
            ))),
            Ok(GetLedgerEntriesResult {
                latest_ledger: None,
                entries: Vec::new(),
            }),
        ]);

        let err = fetch_contract_wasm_with_client(&client, CONTRACT_ID)
            .await
            .expect_err("missing code fails");

        assert!(matches!(err, RpcError::ContractCodeNotFound { .. }));
    }

    #[tokio::test]
    async fn reports_malformed_xdr() {
        let client = MockClient::new(vec![Ok(result_with_xdr("not-xdr".to_owned()))]);

        let err = fetch_contract_wasm_with_client(&client, CONTRACT_ID)
            .await
            .expect_err("malformed xdr fails");

        assert!(matches!(err, RpcError::InvalidLedgerEntryXdr(_)));
    }

    #[test]
    fn encodes_contract_instance_key() {
        let key = contract_instance_key(CONTRACT_ID).expect("valid contract id");
        let decoded = LedgerKey::from_xdr_base64(&key, Limits::none()).expect("valid xdr");

        let LedgerKey::ContractData(contract_data) = decoded else {
            panic!("expected contract data key");
        };
        assert_eq!(contract_data.key, ScVal::LedgerKeyContractInstance);
        assert_eq!(contract_data.durability, ContractDataDurability::Persistent);
    }

    fn result_with_xdr(xdr: String) -> GetLedgerEntriesResult {
        GetLedgerEntriesResult {
            latest_ledger: Some(1),
            entries: vec![LedgerEntryResult {
                xdr,
                last_modified_ledger: Some(1),
            }],
        }
    }

    fn contract_data_xdr(executable: ContractExecutable) -> String {
        let contract = stellar_strkey::Contract::from_string(CONTRACT_ID).expect("contract id");
        let entry = ContractDataEntry {
            ext: ExtensionPoint::V0,
            contract: ScAddress::Contract(ContractId(Hash(contract.0))),
            key: ScVal::LedgerKeyContractInstance,
            durability: ContractDataDurability::Persistent,
            val: ScVal::ContractInstance(ScContractInstance {
                executable,
                storage: None,
            }),
        };
        LedgerEntryData::ContractData(entry)
            .to_xdr_base64(Limits::none())
            .expect("xdr")
    }

    fn contract_code_xdr(hash: [u8; 32], wasm: Vec<u8>) -> String {
        let entry = ContractCodeEntry {
            ext: ContractCodeEntryExt::V0,
            hash: Hash(hash),
            code: BytesM::try_from(wasm).expect("bytes"),
        };
        LedgerEntryData::ContractCode(entry)
            .to_xdr_base64(Limits::none())
            .expect("xdr")
    }
}
