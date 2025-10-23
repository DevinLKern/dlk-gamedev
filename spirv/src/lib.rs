pub mod result;

use result::{Error, Result};

use std::collections::HashMap;
use std::io::Read;
use std::rc::Rc;

#[derive(Debug)]
struct EntryPointData {
    execution_model: u32,
    entry_point_id: u32,
    name: Option<Rc<str>>,
    interface_ids: Box<[u32]>,
}

#[derive(Debug)]
pub(crate) struct OpDecorateInfo {
    pub(crate) target_id: u32,
    pub(crate) decoration: u32,
    pub(crate) extra_operands: Box<[u32]>,
}

pub(crate) struct OpMemberDecorateInfo {
    pub(crate) structure_type_id: u32,
    pub(crate) literal_member: u32,
    pub(crate) decoration: u32,
    pub(crate) extra_operands: Box<[u32]>,
}

#[derive(Debug, Clone)]
pub enum ScalarType {
    Int,
    Unsigned,
    Float,
}

#[derive(Debug, Clone)]
pub enum ShaderIoType {
    Scalar {
        component_type: ScalarType,
        component_width: u32,
    },
    Vector {
        component_type: ScalarType,
        component_width: u32,
        component_count: u32,
    },
    Matrix {
        component_type: ScalarType,
        component_width: u32,
        cols: u32,
        rows: u32,
    },
}

fn get_shader_io_type_size(io_type: &ShaderIoType) -> u32 {
    match io_type {
        ShaderIoType::Scalar { component_width, .. } => component_width / 8,
        ShaderIoType::Vector { component_width, component_count, .. } => component_width * component_count / 8,
        ShaderIoType::Matrix { component_width, cols, rows, .. } => component_width * cols * rows / 8
    }
}

#[derive(Debug, Clone)]
pub struct ShaderIoInfo {
    pub id: u32,
    pub binding: u32,
    pub location: u32,
    pub io_type: ShaderIoType,
    pub stride: u32,
    pub name: Option<Rc<str>>,
}

#[derive(Debug)]
#[allow(dead_code)]
enum OpTypeInfo {
    Void,
    Bool,
    Int {
        width: u32,
        signed: bool,
    },
    Float {
        width: u32,
    },
    Vector {
        component_type_id: u32,
        component_count: u32,
    },
    Matrix {
        column_type_id: u32,
        column_count: u32,
    },
    Pointer {
        storage_class: u32,
        type_id: u32,
    },
    Struct {
        member_types: Rc<[u32]>,
    },
    Image {
        sampled_type: u32,
        dim: u32,
        depth: u32,
        arrayed: u32,
        ms: u32,
        sampled: u32,
        format: u32,
    },
    Sampler,
    SampledImage {
        image_type: u32,
    },
    Other,
}

#[repr(u32)]
#[derive(Debug, PartialEq, Eq)]
pub enum UniformType {
    Sampler,
    SampledImage,
    StorageImage,
    UniformBuffer,
    StorageBuffer,
    Other,
}

#[derive(Debug)]
pub struct UniformInfo {
    pub binding: u32,
    pub set: u32,
    pub uniform_type: UniformType,
    pub name: Option<Rc<str>>,
}

#[allow(dead_code)]
pub struct ShaderModule {
    version: u32,
    generator: u32,
    bound: u32,
    schema: u32,
    entry_points: Box<[EntryPointData]>,
    decorations: HashMap<u32, Rc<[OpDecorateInfo]>>,
    member_decorations: HashMap<u32, Rc<[OpMemberDecorateInfo]>>,
    variables: HashMap<u32, (u32, u32)>,
    names: HashMap<u32, Rc<str>>,
    types: HashMap<u32, OpTypeInfo>,
}

impl std::fmt::Display for UniformType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match *self {
            UniformType::Sampler => "Sampler",
            UniformType::SampledImage => "SampledImage",
            UniformType::StorageImage => "StorageImage",
            UniformType::UniformBuffer => "UniformBuffer",
            UniformType::StorageBuffer => "StorageBuffer",
            _ => "Other",
        };

        write!(f, "{}", str)
    }
}

impl std::fmt::Display for EntryPointData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Optional: map execution_model u32 to a human-readable name
        let exec_model = match self.execution_model {
            0 => "Vertex",
            1 => "TessellationControl",
            2 => "TessellationEvaluation",
            3 => "Geometry",
            4 => "Fragment",
            5 => "GLCompute",
            _ => "Unknown",
        };

        let name = self.name.as_deref().unwrap_or("<unnamed>");

        write!(
            f,
            "{{name: {} id: {}, execution_model: {} ({}), interface_ids: {:?}}}",
            name, self.entry_point_id, self.execution_model, exec_model, self.interface_ids
        )
    }
}

impl std::fmt::Display for ShaderIoInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;

        write!(f, "id: {}", self.id)?;

        write!(f, ", {}", self.location)?;

        if let Some(n) = self.name.clone() {
            write!(f, ", {}", n)?;
        }

        write!(f, "}}")
    }
}

impl std::fmt::Display for UniformInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ binding: {}, set: {}, uniform_type: {}, name: {} }}",
            self.binding,
            self.set,
            self.uniform_type,
            self.name.as_deref().unwrap_or("<unnamed>")
        )
    }
}

impl std::fmt::Display for OpDecorateInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ target_id: {}, decoration: {}, extra_operands: {:?} }}",
            self.target_id, self.decoration, self.extra_operands
        )
    }
}

impl std::fmt::Display for OpMemberDecorateInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ structure_type_id: {}, literal_member: {}, decoration: {}, extra_operands: {:?} }}",
            self.structure_type_id, self.literal_member, self.decoration, self.extra_operands
        )
    }
}

impl std::fmt::Display for ShaderModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let major = (self.version >> 16) & 0xFF;
        let minor = (self.version >> 8) & 0xFF;
        let gen_id = self.generator >> 16;
        let gen_name = match gen_id {
            0 => "Reserved by Khronos",
            1 => "LunarG",
            2 => "Valve",
            3 => "Codeplay",
            4 => "NVIDIA",
            5 => "ARM",
            6 => "Khronos LLVM/SPIR-V Translator",
            7 => "Khronos SPIR-V Tools Assembler",
            8 => "Khronos Glslang Reference Front End",
            9 => "Qualcomm",
            10 => "AMD",
            11 => "Intel",
            12 => "Imagination",
            13 => "Google Shaderc over Glslang",
            14 => "Google spiregg",
            15 => "Google rspirv",
            16 => "X-LEGEND Mesa-IR/SPIR-V Translator",
            17 => "Khronos SPIR-V Tools Linker",
            18 => "Wine VKD3D Shader Compiler",
            19 => "Tellusim Clay Shader Compiler",
            20 => "W3C WebGPU WHLSL Shader Translator",
            21 => "Google Clspv",
            22 => "LLVM MLIR SPIR-V Serializer",
            23 => "Google Tint Compiler",
            24 => "Google ANGLE Shader Compiler",
            25 => "Netease Games Messiah Shader Compiler",
            26 => "Xenia Emulator Microcode Translator",
            27 => "Embark Studios Rust GPU Compiler Backend",
            28 => "gfx-rs community Naga",
            29 => "Mikkosoft Productions MSP Shader Compiler",
            30 => "SpvGenTwo community SpvGenTwo SPIR-V IR Tools",
            31 => "Google Skia SkSL",
            32 => "TornadoVM Beehive SPIRV Toolkit",
            33 => "DragonJoker ShaderWriter",
            34 => "Rayan Hatout SPIRVSmith",
            35 => "Saarland University Shady",
            36 => "Taichi Graphics Taichi",
            37 => "heroseh Hero C Compiler",
            38 => "Meta SparkSL",
            39 => "SirLynix Nazara ShaderLang Compiler",
            40 => "Khronos Slang Compiler",
            41 => "Zig Software Foundation Zig Compiler",
            42 => "Rendong Liang spq",
            43 => "LLVM LLVM SPIR-V Backend",
            44 => "Robert Konrad Kongruent",
            45 => "Kitsunebi Games Nuvk SPIR-V Emitter and DLSL compiler",
            46 => "Nintendo",
            47 => "ARM",
            48 => "Goopax",
            _ => "Other/Unregistered",
        };

        write!(f, "{{")?;

        write!(
            f,
            "version: {} (SPIR-V v{}.{}), ",
            self.version, major, minor
        )?;
        write!(
            f,
            "generator: {} (id: {}, name: {}), ",
            self.generator, gen_id, gen_name
        )?;

        write!(f, "bound: {}, ", self.bound)?;

        write!(f, "schema: {}, ", self.schema)?;

        write!(f, "entry_point_data: {:?}, ", self.entry_points)?;

        write!(f, "decorations: {:?}", self.decorations)?;

        Ok(())
    }
}

impl ShaderModule {
    pub fn from_code(shader_code: &[u8]) -> Result<ShaderModule> {
        if shader_code.len() < 4 * 5 || shader_code.len() % 4 != 0 {
            return Err(Error::InvalidFileLength(shader_code.len()));
        }
        let words: Box<[u32]> = shader_code
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        if words[0] != 0x07230203 {
            return Err(Error::IncorrectMagicWord(words[0]));
        }

        let mut entry_point_data = Vec::new();
        let mut decorations: HashMap<u32, Vec<OpDecorateInfo>> = HashMap::new();
        let mut member_decorations: HashMap<u32, Vec<OpMemberDecorateInfo>> = HashMap::new();
        let mut variables = HashMap::new();
        let mut names = HashMap::new();
        let mut types = HashMap::new();

        let mut i = 5;
        while i < words.len() {
            let word_count = (words[i] >> 16) as usize;
            let opcode = words[i] & 0xFFFF;

            let operand_end = i + word_count;
            if word_count == 0 || operand_end > words.len() {
                return Err(Error::InvalidOperandEnd((words.len(), word_count)));
            }

            // debug output
            match opcode {
                5 => {
                    // OpName
                    let target_id = words[i + 1];
                    let name = {
                        let mut name_bytes = Vec::with_capacity((word_count - 1) * 4);
                        'outer: for j in (i + 2)..(i + word_count) {
                            let word_bytes = words[j].to_le_bytes();
                            for &byte in &word_bytes {
                                if byte == 0 {
                                    break 'outer;
                                }
                                name_bytes.push(byte);
                            }
                        }

                        String::from_utf8_lossy(&name_bytes).into_owned()
                    };
                    if !name.is_empty() {
                        names.insert(target_id, name.into());
                    }
                }
                // 14 => { // OpMemoryModel
                //     //
                // }
                15 => {
                    // OpEntryPoint
                    let execution_model = words[i + 1];
                    let entry_point_id = words[i + 2];

                    let (name, name_word_count) = {
                        let mut name_word_count = 0;
                        let mut bytes = Vec::with_capacity((word_count - 2) * 4);
                        'name_loop: for j in (i + 3)..(i + word_count) {
                            name_word_count += 1;
                            for &byte in &words[j].to_le_bytes() {
                                if byte == 0 {
                                    break 'name_loop;
                                }
                                bytes.push(byte);
                            }
                        }

                        if bytes.is_empty() {
                            (None, name_word_count)
                        } else {
                            (
                                Some(String::from_utf8_lossy(&bytes).into_owned().into()),
                                name_word_count,
                            )
                        }
                    };

                    let interface_start = i + 3 + name_word_count;
                    let interface_ids = &words[interface_start..(i + word_count)];
                    entry_point_data.push(EntryPointData {
                        execution_model,
                        entry_point_id,
                        name,
                        interface_ids: interface_ids.into(),
                    });
                }
                19 => {
                    // OpTypeVoid
                    types.insert(words[i + 1], OpTypeInfo::Void);
                }
                20 => {
                    // OpTypeBool
                    types.insert(words[i + 1], OpTypeInfo::Bool);
                }
                21 => {
                    // OpTypeInt
                    types.insert(
                        words[i + 1],
                        OpTypeInfo::Int {
                            width: words[i + 2],
                            signed: words[i + 3] != 0,
                        },
                    );
                }
                22 => {
                    // OpTypeFloat
                    types.insert(
                        words[i + 1],
                        OpTypeInfo::Float {
                            width: words[i + 2],
                        },
                    );
                }
                23 => {
                    // OpTypeVector
                    types.insert(
                        words[i + 1],
                        OpTypeInfo::Vector {
                            component_type_id: words[i + 2],
                            component_count: words[i + 3],
                        },
                    );
                }
                24 => {
                    // OpTypeMatrix
                    types.insert(
                        words[i + 1],
                        OpTypeInfo::Matrix {
                            column_type_id: words[i + 2],
                            column_count: words[i + 3],
                        },
                    );
                }
                25 => {
                    // OpTypeImage
                    types.insert(
                        words[i + 1],
                        OpTypeInfo::Image {
                            sampled_type: words[i + 2],
                            dim: words[i + 3],
                            depth: words[i + 4],
                            arrayed: words[i + 5],
                            ms: words[i + 6],
                            sampled: words[i + 7],
                            format: words[i + 8],
                        },
                    );
                }
                26 => {
                    // OpTypeSampler
                    types.insert(words[i + 1], OpTypeInfo::Sampler);
                }
                27 => {
                    // OpTypeSampledImage
                    types.insert(
                        words[i + 1],
                        OpTypeInfo::SampledImage {
                            image_type: words[i + 2],
                        },
                    );
                }
                30 => {
                    // OpTypeStruct
                    types.insert(
                        words[i + 1],
                        OpTypeInfo::Struct {
                            member_types: words[(i + 2)..operand_end].into(),
                        },
                    );
                }
                32 => {
                    // OpTypePointer
                    types.insert(
                        words[i + 1],
                        OpTypeInfo::Pointer {
                            storage_class: words[i + 2],
                            type_id: words[i + 3],
                        },
                    );
                }
                59 => {
                    // OpVariable
                    let result_type_id = words[i + 1];
                    let result_id = words[i + 2];
                    let storage_class = words[i + 3];

                    variables.insert(result_id, (result_type_id, storage_class));
                }
                71 => {
                    let decoration_data = OpDecorateInfo {
                        target_id: words[i + 1],
                        decoration: words[i + 2],
                        extra_operands: (&words[(i + 3)..operand_end]).into(),
                    }; // OpDecorate
                    if let Some(decoration_datas) = decorations.get_mut(&words[i + 1]) {
                        decoration_datas.push(decoration_data);
                    } else {
                        decorations.insert(words[i + 1], vec![decoration_data]);
                    }
                }
                72 => {
                    // OpMemberDecorate
                    let decoration_data = OpMemberDecorateInfo {
                        structure_type_id: words[i + 1],
                        literal_member: words[i + 2],
                        decoration: words[i + 3],
                        extra_operands: (&words[(i + 4)..operand_end]).into(),
                    };
                    if let Some(decoration_datas) = member_decorations.get_mut(&words[i + 1]) {
                        decoration_datas.push(decoration_data);
                    } else {
                        member_decorations.insert(words[i + 1], vec![decoration_data]);
                    }
                }
                _ => {
                    //
                }
            }

            i = operand_end;
        }

        Ok(ShaderModule {
            version: words[1],
            generator: words[2],
            bound: words[3],
            schema: words[4],
            entry_points: entry_point_data.into_boxed_slice(),
            decorations: decorations
                .into_iter()
                .map(|(k, v)| (k, Rc::from(v.into_boxed_slice())))
                .collect(),
            member_decorations: member_decorations
                .into_iter()
                .map(|(k, v)| (k, Rc::from(v.into_boxed_slice())))
                .collect(),
            variables,
            names,
            types,
        })
    }
    pub fn from_file(shader_path: &std::path::Path) -> Result<ShaderModule> {
        let mut file = std::fs::File::open(shader_path).map_err(|e| Error::Io(e))?;

        let mut data = Vec::<u8>::new();

        let _ = file.read_to_end(&mut data).map_err(|e| Error::Io(e))?;

        return Self::from_code(data.as_slice());
    }

    pub fn get_input_names(&self) -> Vec<Rc<str>> {
        let mut names = Vec::new();

        for ep in self.entry_points.iter() {
            if let Some(name) = ep.name.clone() {
                names.push(name);
            }
        }

        names
    }
    fn get_io_type_from_id(&self, type_id: &u32) -> Result<ShaderIoType> {
        match self
            .types
            .get(type_id)
            .ok_or(Error::NoAssociatedType(*type_id))?
        {
            &OpTypeInfo::Int { width, signed } => Ok(ShaderIoType::Scalar {
                component_type: if signed {
                    ScalarType::Int
                } else {
                    ScalarType::Unsigned
                },
                component_width: width,
            }),
            &OpTypeInfo::Float { width } => Ok(ShaderIoType::Scalar {
                component_type: ScalarType::Float,
                component_width: width,
            }),
            &OpTypeInfo::Vector {
                component_type_id,
                component_count,
            } => match self.get_io_type_from_id(&component_type_id)? {
                ShaderIoType::Scalar {
                    component_type,
                    component_width,
                } => Ok(ShaderIoType::Vector {
                    component_type,
                    component_width,
                    component_count,
                }),
                _ => Err(Error::InvalidType),
            },
            &OpTypeInfo::Matrix {
                column_type_id,
                column_count,
            } => match self.get_io_type_from_id(&column_type_id)? {
                ShaderIoType::Vector {
                    component_type,
                    component_width,
                    component_count,
                } => Ok(ShaderIoType::Matrix {
                    component_type,
                    component_width,
                    cols: column_count,
                    rows: component_count,
                }),
                _ => Err(Error::InvalidType),
            },
            _ => Err(Error::InvalidType),
        }
    }
    #[inline]
    pub fn get_inputs(&self) -> Result<Vec<ShaderIoInfo>> {
        // self.get_io_infos(1)
        let mut input_ids = Vec::new();
        for (id, (type_id, storage_class)) in self.variables.iter() {
            // 1 == input storage class
            if *storage_class != 1 {
                continue;
            }

            if let Some(t) = self.types.get(type_id) {
                match t {
                    &OpTypeInfo::Pointer { type_id, .. } => {
                        input_ids.push((*id, type_id));
                    }
                    _ => continue
                }
            }
        }

        let mut inputs = Vec::<ShaderIoInfo>::with_capacity(input_ids.len());
        for (id, type_id) in input_ids.iter() {
            let name: Option<Rc<str>> = self.names.get(id).cloned();

            let mut location: Option<u32> = None;
            for decorate_info in self.decorations.get(id).unwrap().iter() {
                // 30 = Location
                if decorate_info.decoration == 30 {
                    location = Some(decorate_info.extra_operands[0]);
                    break;
                }
            }
            let location = location.ok_or(Error::LocationMissing(*id))?;
            
            let mut binding: Option<u32> = None;
            for decorate_info in self.decorations.get(id).unwrap().iter() {
                // 33 = Binding
                if decorate_info.decoration == 33 {
                    binding = Some(decorate_info.extra_operands[0]);
                    break;
                }
            }

            let io_type = self.get_io_type_from_id(type_id)?;
            let stride = get_shader_io_type_size(&io_type);

            inputs.push(ShaderIoInfo {
                id: *id,
                binding: binding.unwrap_or(0),
                location,
                io_type,
                stride,
                name
            });
        }

        Ok(inputs)
    }
    #[inline]
    pub fn get_outputs(&self) -> Result<Vec<ShaderIoInfo>> {
        Err(Error::InvalidType)
    }

    pub fn get_uniforms(&self) -> Result<Vec<UniformInfo>> {
        let mut uniforms = Vec::new();

        for (id, (type_id, storage_class)) in self.variables.iter() {
            // Only UniformConstant (0), Uniform (2), StorageBuffer (12)
            match *storage_class {
                0 | 2 | 12 => {}
                _ => continue,
            }

            // Defaults if decoration is missing
            let mut set: Option<u32> = None;
            let mut binding: Option<u32> = None;

            if let Some(decos) = self.decorations.get(id) {
                for d in decos.iter() {
                    match d.decoration {
                        33 => {
                            // Binding
                            if let Some(&b) = d.extra_operands.get(0) {
                                binding = Some(b);
                            }
                        }
                        35 => {
                            // DescriptorSet
                            if let Some(&s) = d.extra_operands.get(0) {
                                set = Some(s);
                            }
                        }
                        _ => {}
                    }
                }
            }

            let binding = binding.ok_or(Error::DecorationMissing(*id))?;
            let set = set.unwrap_or(0); // default to 0 if no DescriptorSet

            // Determine uniform type
            let uniform_type = if *storage_class == 12 {
                UniformType::StorageBuffer
            } else {
                match self.types.get(type_id).unwrap_or(&OpTypeInfo::Other) {
                    OpTypeInfo::Pointer { .. } => UniformType::Other,
                    OpTypeInfo::Struct { .. } => UniformType::UniformBuffer,
                    OpTypeInfo::Image { sampled, .. } => {
                        if *sampled == 2 {
                            UniformType::SampledImage
                        } else {
                            UniformType::StorageBuffer
                        }
                    }
                    OpTypeInfo::Sampler => UniformType::Sampler,
                    OpTypeInfo::SampledImage { .. } => UniformType::SampledImage,
                    _ => UniformType::Other,
                }
            };

            uniforms.push(UniformInfo {
                binding,
                set,
                uniform_type,
                name: self.names.get(id).cloned(),
            });
        }

        Ok(uniforms)
    }
}
