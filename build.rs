use convert_case::{Case, Casing};
use ethers::abi::{Contract, ParamType};

use serde::Deserialize;
use std::io::Write;
use std::{fs, env};
use std::path::Path;
use ethers::core::types::H256;

#[derive(Debug, Deserialize)]
struct BuildConfig {
    out_name: String,
    contract_file: String,
}

fn main() -> anyhow::Result<()> {
    let config: BuildConfig = toml::from_str(include_str!("build-config.toml"))?;

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join(&config.out_name);

    let contract_file = fs::read_to_string(&config.contract_file)?;
    let parsed_contract: serde_json::Value = serde_json::from_str(contract_file.as_str())?;
    let abi_string = serde_json::to_vec(
        parsed_contract
            .get("abi")
            .expect("missing abi in contract file")
    )?;
    let contract = Contract::load(abi_string.as_slice())?;

    let mut out_file = fs::File::create(&dest_path)?;

    writeln!(out_file, "#![allow(dead_code)]")?;
    writeln!(out_file, "use std::convert::TryInto;")?;

    writeln!(out_file, r#"
#[derive(::thiserror::Error, Debug)]
pub enum EventParseError {{
    #[error("event topic mismatch")]
    TopicMismatch,
    #[error(transparent)]
    DecodeError(#[from] ::ethers::abi::Error),
}}
"#)?;

    for event in contract.events() {
        writeln!(out_file, "static {}_SIGNATURE: [u8; 32] = {:?};",
                 &event.name.to_case(Case::UpperSnake),
                 event.signature().as_fixed_bytes(),
        )?;
    }
    writeln!(out_file)?;

    writeln!(out_file, "#[derive(Debug, Clone)]")?;
    writeln!(out_file, "pub enum Events {{")?;
    for event in contract.events() {
        writeln!(out_file, "    {0}({0}),", &event.name)?;
    }
    writeln!(out_file, "}}")?;
    writeln!(out_file)?;

    for event in contract.events() {
        writeln!(out_file, "#[derive(Debug, Clone)]")?;
        writeln!(out_file, "pub struct {} {{", &event.name)?;
        for input in event.inputs.iter() {
            let normalized_name = input.name.to_case(Case::Snake);
            let normalized_type = match &input.kind {
                ParamType::Bytes => "Vec<u8>".to_string(),
                ParamType::Uint(size) => if *size <= 128usize {
                    format!("u{}", size)
                } else {
                    format!("::ethers::types::U{}", size)
                },
                ParamType::FixedBytes(size) => format!("[u8; {}]", size),
                _ => format!("::ethers::types::{}",
                             input.kind.to_string().to_case(Case::UpperCamel))
            };
            writeln!(out_file, "    {}: {},", normalized_name, normalized_type)?;
        }
        writeln!(out_file, "}}")?;
        writeln!(out_file)?;
    }

    writeln!(out_file, r#"impl Events {{
    pub fn signature(&self) -> ::ethers::abi::Hash {{
        use Events::*;
        match self {{"#)?;
    for event in contract.events() {
        writeln!(out_file, "            {}(_) => ::ethers::abi::Hash::from({}_SIGNATURE),",
                 &event.name,
                 &event.name.to_case(Case::UpperSnake))?;
    }
    writeln!(out_file, r#"        }}
    }}
}}
"#)?;

    for event in contract.events() {
        writeln!(out_file, r#"
impl {0} {{
    pub fn signature() -> ::ethers::abi::Hash {{
        ::ethers::abi::Hash::from({1}_SIGNATURE)
    }}
}}

impl ::std::convert::TryFrom<::ethers::types::Log> for {0} {{
    type Error = EventParseError;

    fn try_from(log: ::ethers::types::Log) -> Result<Self, Self::Error> {{
        if !log.topics.iter().any(|t| *t == Self::signature()) {{
            return Err(EventParseError::TopicMismatch)
        }}
        let mut decoded = ::ethers::abi::decode(&["#,
                 &event.name,
                 &event.name.to_case(Case::UpperSnake)
        )?;
        for input in event.inputs.iter() {
            writeln!(out_file, "            ::ethers::abi::ParamType::{:?},", input.kind)?;
        }
        writeln!(out_file, "        ], log.data.as_ref())?;")?;
        writeln!(out_file, "        Ok(Self {{")?;
        for input in event.inputs.iter() {
            let normalized_name = input.name.to_case(Case::Snake);
            write!(out_file, "            {}: decoded.remove(0)", normalized_name)?;
            let remaining = match input.kind {
                ParamType::Uint(size) => if size <= 64usize {
                    format!(".into_uint().unwrap().as_u64() as u{}", size)
                } else if size <= 128usize {
                    format!(".into_uint().unwrap().as_u128() as u{}", size)
                } else {
                    format!(".into_uint().unwrap()")
                },
                ParamType::FixedBytes(_) => format!(".into_fixed_bytes().unwrap().try_into().unwrap()"),
                _ => format!(".into_{}().unwrap()",
                             input.kind.to_string().to_case(Case::Snake))
            };
            writeln!(out_file, "{},", remaining)?;
        }
        writeln!(out_file, "        }})")?;
        writeln!(out_file, "    }}")?;
        writeln!(out_file, "}}")?;
    }

    Ok(())
}