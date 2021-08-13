use std::{fs, env};
use std::path::Path;

use convert_case::{Case, Casing};
use ethers::abi::{Contract, ParamType};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use tera::{Tera, Context};
use std::collections::HashMap;
use std::io::Write;

#[derive(Debug, Deserialize)]
struct BuildConfig {
    out_name: String,
    contract_file: String,
}

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=templates/*");
    let config: BuildConfig = toml::from_str(include_str!("build-config.toml"))?;
    println!("cargo:rerun-if-changed={}", config.contract_file);

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join(&config.out_name);

    let contract_file = fs::read_to_string(&config.contract_file)?;
    let parsed_contract: Value = serde_json::from_str(contract_file.as_str())?;
    let abi_string = serde_json::to_vec(
        parsed_contract
            .get("abi")
            .expect("missing abi in contract file")
    )?;
    let contract = Contract::load(abi_string.as_slice())?;

    let events: Vec<Event> = contract
        .events()
        .map(|event| {
            Event {
                name: event.name.to_owned(),
                signature: format!("{:?}", event.signature().as_fixed_bytes()),
                inputs: event.inputs
                    .iter()
                    .map(|input| Input {
                        name: input.name.to_owned(),
                        kind: Type::from(&input.kind)
                    })
                    .collect()
            }
        })
        .collect();

    let mut out_file = fs::File::create(&dest_path)?;

    let mut tera = Tera::default();
    tera.add_template_file("templates/events.rs", Some("events.rs"))?;

    tera.register_filter("upper_snake", upper_snake);
    tera.register_filter("lower_snake", lower_snake);
    tera.register_filter("upper_camel", upper_camel);
    tera.register_filter("normalize_type", normalize_type);
    tera.register_filter("normalized_param_type", normalized_param_type);
    tera.register_filter("normalized_parse", normalized_parse);

    let mut ctx = Context::new();
    ctx.insert("events", &events);
    tera.render_to("events.rs", &ctx, &mut out_file)?;
    out_file.flush()?;

    Ok(())
}

fn upper_snake(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    Ok(Value::String(value.as_str().unwrap().to_case(Case::UpperSnake)))
}
fn lower_snake(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    Ok(Value::String(value.as_str().unwrap().to_case(Case::Snake)))
}
fn upper_camel(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    Ok(Value::String(value.as_str().unwrap().to_case(Case::UpperCamel)))
}
fn normalize_type(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let literal = match Type::deserialize(value)? {
        Type::Address => "::ethers::types::Address".to_string(),
        Type::Bytes => "Vec<u8>".to_string(),
        Type::Int(size) => if size <= 128 {
            format!("i{}", size)
        } else {
            "::ethers::types::I256".to_string()
        }
        Type::Uint(size) => if size <= 128 {
            format!("u{}", size)
        } else {
            "::ethers::types::U256".to_string()
        }
        Type::Bool => "bool".to_string(),
        Type::String => "String".to_string(),
        Type::FixedBytes(size) => format!("[u8; {}]", size)
    };
    Ok(Value::String(literal))
}
fn normalized_param_type(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let literal = match Type::deserialize(value)? {
        Type::Address => "::ethers::abi::ParamType::Address".to_string(),
        Type::Bytes => "::ethers::abi::ParamType::Bytes".to_string(),
        Type::Int(size) => format!("::ethers::abi::ParamType::Int({})", size),
        Type::Uint(size) => format!("::ethers::abi::ParamType::Uint({})", size),
        Type::Bool => "::ethers::abi::ParamType::Bool".to_string(),
        Type::String => "::ethers::abi::ParamType::String".to_string(),
        Type::FixedBytes(size) => format!("::ethers::abi::ParamType::FixedBytes({})", size),
    };
    Ok(Value::String(literal))
}
fn normalized_parse(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let literal = match Type::deserialize(value)? {
        Type::Address => ".into_address().unwrap()".to_string(),
        Type::Bytes => ".into_bytes().unwrap()".to_string(),
        Type::Int(size) => if size <= 64 {
            format!(".into_int().unwrap().as_u64() as u{}", size)
        } else if size <= 128 {
            ".into_int().unwrap().as_u128()".to_string()
        } else {
            ".into_int().unwrap()".to_string()
        }
        Type::Uint(size) => if size <= 64 {
            format!(".into_uint().unwrap().as_u64() as u{}", size)
        } else if size <= 128 {
            ".into_uint().unwrap().as_u128()".to_string()
        } else {
            ".into_uint().unwrap()".to_string()
        }
        Type::Bool => ".into_bool().unwrap()".to_string(),
        Type::String => ".into_string().unwrap()".to_string(),
        Type::FixedBytes(_) => ".into_fixed_bytes().unwrap().try_into().unwrap()".to_string(),
    };
    Ok(Value::String(literal))
}

#[derive(Debug, Serialize, Deserialize)]
struct Event {
    pub name: String,
    pub signature: String,
    pub inputs: Vec<Input>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Input {
    pub name: String,
    pub kind: Type,
}

#[derive(Debug, Serialize, Deserialize)]
enum Type {
    /// Address.
    Address,
    /// Bytes.
    Bytes,
    /// Signed integer.
    Int(usize),
    /// Unsigned integer.
    Uint(usize),
    /// Boolean.
    Bool,
    /// String.
    String,
    /// Vector of bytes with fixed size.
    FixedBytes(usize),
}

impl From<&ParamType> for Type {
    fn from(t: &ParamType) -> Self {
        use Type::*;
        match t {
            ParamType::Address => Address,
            ParamType::Bytes => Bytes,
            ParamType::Int(size) => Int(*size),
            ParamType::Uint(size) => Uint(*size),
            ParamType::Bool => Bool,
            ParamType::String => String,
            ParamType::Array(_) => unimplemented!("dynamic recursive type not supported"),
            ParamType::FixedBytes(size) => FixedBytes(*size),
            ParamType::FixedArray(_, _) => unimplemented!("dynamic recursive type not supported"),
            ParamType::Tuple(_) => unimplemented!("dynamic recursive type not supported"),
        }
    }
}