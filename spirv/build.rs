use std::{
    env,
    fs::File,
    io::{self, BufReader, BufWriter, Write},
    path::PathBuf,
};

use serde_json::Value;

fn to_snake_caps(s: &String) -> String {
    let mut res = String::new();

    let mut c1: Option<char> = None;
    for c2 in s.chars() {
        if let Some(c1) = c1 {
            if c1.is_lowercase() && c2.is_uppercase() {
                res.push('_');
            }
        }

        if c2.is_lowercase() {
            res.push(c2.to_ascii_uppercase())
        } else {
            res.push(c2);
        }

        c1 = Some(c2);
    }

    res
}

#[allow(dead_code)]
fn generate_opcode_enum(opcode_path: PathBuf, json: &Value) -> Result<(), io::Error> {
    let instructions = json["instructions"]
        .as_array()
        .expect("No instructions array!");

    let opocode_file = File::create(opcode_path)?;
    let mut w = BufWriter::new(opocode_file);

    for instruction in instructions {
        let opcode = instruction.as_object().and_then(|obj| obj.get("opcode"));
        let name = instruction.as_object().and_then(|obj| obj.get("opname"));
        match (name, opcode) {
            (Some(Value::String(s)), Some(Value::Number(c))) => {
                writeln!(w, "#[allow(unused)]")?;
                writeln!(w, "const {}: u32 = {};", to_snake_caps(s), c)?;
            }
            _ => {
                panic!("Not covered!");
            }
        }
    }

    Ok(())
}

#[allow(dead_code)]
fn generate_opkind_enum(opkind_path: PathBuf, json: &Value) -> Result<(), io::Error> {
    let kinds = json["operand_kinds"].as_array().expect("No opkinds array!");

    let opkind_file = File::create(opkind_path)?;
    let mut w = BufWriter::new(opkind_file);

    for kind in kinds {
        let kind_name = kind
            .as_object()
            .and_then(|obj| obj.get("kind"))
            .unwrap()
            .as_str()
            .unwrap();
        writeln!(w, "pub struct {}(pub u32);", kind_name)?;

        let enumerants = kind.as_object().and_then(|obj| {
            println!("{:?}", obj.keys());
            obj.get("enumerants")
        });
        if enumerants.is_none() {
            continue;
        }
        let enumerants = enumerants.unwrap().as_array().unwrap();
        for enumerant in enumerants {
            let enumerant_name = enumerant
                .as_object()
                .and_then(|obj| obj.get("enumerant"))
                .unwrap()
                .as_str()
                .unwrap();
            let enumerant_name = format!("{}{}", kind_name, enumerant_name);
            let enumerant_name = to_snake_caps(&enumerant_name);
            let enumerant_value = enumerant.as_object().and_then(|obj| obj.get("value"));
            if enumerant_value.is_none() {
                continue;
            }
            match enumerant_value.expect("Enumerant value not found!") {
                Value::Number(n) => {
                    writeln!(w, "#[allow(unused)]")?;
                    writeln!(w, "pub const {}: u32 = {};", enumerant_name, n)?;
                }
                Value::String(s) => {
                    writeln!(w, "#[allow(unused)]")?;
                    writeln!(w, "pub const {}: u32 = {};", enumerant_name, s)?;
                }
                _ => {
                    //
                }
            }
        }
    }

    Ok(())
}

fn generate_numbers(magic_path: PathBuf, spirv_file_object: &Value) -> Result<(), io::Error> {
    let magic_number = String::from(spirv_file_object["magic_number"].as_str().unwrap());
    let magic_number = magic_number.strip_prefix("0x").unwrap();
    let magic_number = u32::from_str_radix(magic_number, 16).unwrap();

    let magic_file = File::create(magic_path)?;
    let mut w = BufWriter::new(magic_file);

    writeln!(w, "const MAGIC_NUMBER: u32 = {};", magic_number)?;

    let major_version = spirv_file_object["major_version"]
        .as_number()
        .unwrap()
        .as_u64()
        .unwrap() as u32;
    let major_version = major_version << 16;
    let minor_version = spirv_file_object["minor_version"]
        .as_number()
        .unwrap()
        .as_u64()
        .unwrap() as u32;
    let minor_version = minor_version << 8;
    let spirv_version = major_version | minor_version;

    writeln!(w, "const SPIRV_VERSION: u32 = {};", spirv_version)
}

fn main() {
    println!("cargo:rerun-if-changed=../external/SPIRV-Headers/include/spirv/unified1/spirv.core.grammar.json");

    let spirv_file_path = PathBuf::from("../external/SPIRV-Headers/include/spirv/unified1/spirv.core.grammar.json");
    let spirv_file = File::open(spirv_file_path).unwrap();
    let spirv_file_reader = BufReader::new(spirv_file);
    let spirv_file_object: Value = serde_json::from_reader(spirv_file_reader).unwrap();

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let magic_path = out_dir.join("magic_numbers.rs");
    generate_numbers(magic_path, &spirv_file_object).unwrap();

    let opcode_path = out_dir.join("opcode.rs");
    generate_opcode_enum(opcode_path, &spirv_file_object).unwrap();

    let opkind_path = out_dir.join("opkind.rs");
    generate_opkind_enum(opkind_path, &spirv_file_object).unwrap();
}
