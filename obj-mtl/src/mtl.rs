use crate::MtlTokenizer;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Channel {
    Red,
    Green,
    Blue,
    Matte,
    Luminance,
    Depth,
}

impl Channel {
    pub(crate) fn from_str(s: &str) -> Option<Channel> {
        let channel = match s {
            "r" | "R" => Self::Red,
            "g" | "G" => Self::Green,
            "b" | "B" => Self::Blue,
            "m" | "M" => Self::Matte,
            "l" | "L" => Self::Luminance,
            "z" | "Z" => Self::Depth,
            _ => return None
        };

        Some(channel)
    }
}

#[derive(Debug)]
pub enum IllumModel {
    UnlitColor,
    LitColorAmbient,
    LitWithSpecularHighlight,
    ReflectionRayTraced,
    GlassTransparencyRayTraced,
    ReflectionFresnelRayTraced,
    RefractionRayTraced,
    RefractionFresnelRayTraced,
    ReflectionRaster,
    GlassTransparencyRaster,
    ShadowCastsOnInvisible,
}

impl IllumModel {
    pub fn from_u32(illum: u32) -> Option<Self> {
        let illum = match illum {
            0 => Self::UnlitColor,
            1 => Self::LitColorAmbient,
            2 => Self::LitWithSpecularHighlight,
            3 => Self::ReflectionRayTraced,
            4 => Self::GlassTransparencyRayTraced,
            5 => Self::ReflectionFresnelRayTraced,
            6 => Self::RefractionRayTraced,
            7 => Self::RefractionFresnelRayTraced,
            8 => Self::ReflectionRaster,
            9 => Self::GlassTransparencyRaster,
            10 => Self::ShadowCastsOnInvisible,
            _ => return None
        };

        Some(illum)
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct Texture {
    file_path: Box<str>,
    // specified with -o
    offset: [f32; 3],
    // specified with -s
    scale: [f32; 3],
    // specidied with -t
    turbulence: [f32; 3],
    // specified with -mm base option. defaults to 0.0
    brightness: f32,
    // specified with -mm gain option. defaults to 1.0
    contrast: f32,
    // specified with -bm option.
    bump_multiplier: f32,
    blend_v: bool,
    blend_u: bool,
    clamp: bool,
    // specified with texres. Defaults to 1.0
    resolution: u32,
    imfchan: Channel,
}

impl Default for Texture {
    fn default() -> Self {
        Self {
            file_path: "".into(),
            offset: [0.0; 3],
            scale: [1.0; 3],
            turbulence: [0.0; 3],
            brightness: 0.0,
            contrast: 1.0,
            bump_multiplier: 1.0,
            blend_v: false,
            blend_u: false,
            clamp: false,
            resolution: 1,
            imfchan: Channel::Red,
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
struct TexturedValue<T> {
    color: Option<T>,
    texture: Option<Texture>,
}

impl<T> Default for TexturedValue<T> {
    fn default() -> Self {
        Self {
            color: None,
            texture: None
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct MtlMaterial {
    name: Box<str>,

    ambient: TexturedValue<[f32; 3]>,
    diffuse: TexturedValue<[f32; 3]>,
    specular: TexturedValue<[f32; 3]>,

    shininess: TexturedValue<f32>,
    opacity: TexturedValue<f32>,
    roughness: TexturedValue<f32>,
    metallic: TexturedValue<f32>,

    illum: IllumModel,
    ior: Option<f32>,
    
    normal_map: Option<Texture>,
    bump_map: Option<Texture>,
    displacement_map: Option<Texture>,
}

impl Default for MtlMaterial {
    fn default() -> Self {
        Self {
            name: "".into(),
            ambient: TexturedValue::default(),
            diffuse: TexturedValue::default(),
            specular: TexturedValue::default(),
            shininess: TexturedValue::default(),
            opacity: TexturedValue::default(),
            roughness: TexturedValue::default(),
            metallic: TexturedValue::default(),
            illum: IllumModel::UnlitColor,
            ior: None,
            normal_map: None,
            bump_map: None,
            displacement_map: None,
        }
    }
}

pub fn load_materials(file_path: &std::path::Path) -> crate::Result<Box<[MtlMaterial]>> {
    let mut tokenizer = MtlTokenizer::from_path(file_path)?;
    
    let mut materials = Vec::<MtlMaterial>::with_capacity(4);
    
    while let Some(token) = tokenizer.next_token() {
        use crate::MtlToken;
        match token? {
            MtlToken::Material(name) => {
                materials.push(MtlMaterial{ name, ..Default::default() });
            }
            MtlToken::Ka { r, g, b } => {
                let mat = materials
                    .last_mut()
                    .ok_or(crate::Error::Parse("Mtl 'Ka' before any 'newmtl' material"))?;
                mat.ambient.color = Some([r, g, b]);
            }
            MtlToken::MapKa { options, file_name } => {
                let mat = materials
                    .last_mut()
                    .ok_or(crate::Error::Parse("Mtl 'map_Ka' before any 'newmtl' material"))?;
                let mm = options.mm.unwrap_or(crate::Mm{base: 0.0, gain: 1.0});
                mat.ambient.texture = Some(Texture {
                    file_path: file_name,
                    offset: options.o.unwrap_or([0.0; 3]),
                    scale: options.s.unwrap_or([1.0; 3]),
                    turbulence: options.t.unwrap_or([0.0; 3]),
                    brightness: mm.base,
                    contrast: mm.gain, 
                    bump_multiplier: 1.0,
                    blend_v: options.blendv.unwrap_or(true),
                    blend_u: options.blendu.unwrap_or(true),
                    clamp: options.clamp.unwrap_or(false),
                    resolution: options.texres.unwrap_or(1),
                    imfchan: options.imfchan.unwrap_or(Channel::Red),
                })
            }
            MtlToken::Kd { r, g, b } => {
                let mat = materials
                    .last_mut()
                    .ok_or(crate::Error::Parse("Mtl 'Kd' before any 'newmtl' material"))?;
                mat.diffuse.color = Some([r, g, b]);
            }
            MtlToken::MapKd { options, file_name } => {
                let mat = materials
                    .last_mut()
                    .ok_or(crate::Error::Parse("Mtl 'map_Kd' before any 'newmtl' material"))?;
                let mm = options.mm.unwrap_or(crate::Mm{base: 0.0, gain: 1.0});
                mat.diffuse.texture = Some(Texture {
                    file_path: file_name,
                    offset: options.o.unwrap_or([0.0; 3]),
                    scale: options.s.unwrap_or([1.0; 3]),
                    turbulence: options.t.unwrap_or([0.0; 3]),
                    brightness: mm.base,
                    contrast: mm.gain, 
                    bump_multiplier: 1.0,
                    blend_v: options.blendv.unwrap_or(true),
                    blend_u: options.blendu.unwrap_or(true),
                    clamp: options.clamp.unwrap_or(false),
                    resolution: options.texres.unwrap_or(1),
                    imfchan: options.imfchan.unwrap_or(Channel::Red),
                })
            }
            MtlToken::Ks { r, g, b } => {
                let mat = materials
                    .last_mut()
                    .ok_or(crate::Error::Parse("Mtl 'Ks' before any 'newmtl' material"))?;
                mat.specular.color = Some([r, g, b]);
            }
            MtlToken::MapKs { options, file_name } => {
                let mat = materials
                    .last_mut()
                    .ok_or(crate::Error::Parse("Mtl 'map_Ks' before any 'newmtl' material"))?;
                let mm = options.mm.unwrap_or(crate::Mm{base: 0.0, gain: 1.0});

                mat.specular.texture = Some(Texture {
                    file_path: file_name,
                    offset: options.o.unwrap_or([0.0; 3]),
                    scale: options.s.unwrap_or([1.0; 3]),
                    turbulence: options.t.unwrap_or([0.0; 3]),
                    brightness: mm.base,
                    contrast: mm.gain, 
                    bump_multiplier: 1.0,
                    blend_v: options.blendv.unwrap_or(true),
                    blend_u: options.blendu.unwrap_or(true),
                    clamp: options.clamp.unwrap_or(false),
                    resolution: options.texres.unwrap_or(1),
                    imfchan: options.imfchan.unwrap_or(Channel::Red),
                })
            }
            MtlToken::Ns(specular_exponent) => {
                let mat = materials
                    .last_mut()
                    .ok_or(crate::Error::Parse("Mtl 'Ns' before any 'newmtl' material"))?;
                mat.shininess.color = Some(specular_exponent);
            }
            MtlToken::MapNs { options, file_name } => {
                let mat = materials
                    .last_mut()
                    .ok_or(crate::Error::Parse("Mtl 'map_Ns' before any 'newmtl' material"))?;
                let mm = options.mm.unwrap_or(crate::Mm{base: 0.0, gain: 1.0});

                mat.shininess.texture = Some(Texture {
                    file_path: file_name,
                    offset: options.o.unwrap_or([0.0; 3]),
                    scale: options.s.unwrap_or([1.0; 3]),
                    turbulence: options.t.unwrap_or([0.0; 3]),
                    brightness: mm.base,
                    contrast: mm.gain,
                    bump_multiplier: 1.0,
                    blend_v: options.blendv.unwrap_or(true),
                    blend_u: options.blendu.unwrap_or(true),
                    clamp: options.clamp.unwrap_or(false),
                    resolution: options.texres.unwrap_or(1),
                    imfchan: options.imfchan.unwrap_or(Channel::Red),
                });
            }
            MtlToken::Ni(optical_density) => {
                let mat = materials
                    .last_mut()
                    .ok_or(crate::Error::Parse("Mtl 'Ni' before any 'newmtl' material"))?;
                mat.ior = Some(optical_density);
            }
            MtlToken::Illum(illum) => {
                let mat = materials
                    .last_mut()
                    .ok_or(crate::Error::Parse("Mtl 'illum' before any 'newmtl' material"))?;
                mat.illum = IllumModel::from_u32(illum).ok_or(crate::Error::Parse("Unrecognized Illum value"))?;
            }
            MtlToken::Bump { options, file_name } => {
                let mat = materials
                    .last_mut()
                    .ok_or(crate::Error::Parse("Mtl 'bump' before any 'newmtl' material"))?;
                let mm = options.mm.unwrap_or(crate::Mm{base: 0.0, gain: 1.0});
                mat.bump_map = Some(Texture {
                    file_path: file_name,
                    offset: options.o.unwrap_or([0.0; 3]),
                    scale: options.s.unwrap_or([1.0; 3]),
                    turbulence: options.t.unwrap_or([0.0; 3]),
                    brightness: mm.base,
                    contrast: mm.gain, 
                    bump_multiplier: options.bm.unwrap_or(1.0),
                    blend_v: options.blendv.unwrap_or(true),
                    blend_u: options.blendu.unwrap_or(true),
                    clamp: options.clamp.unwrap_or(false),
                    resolution: options.texres.unwrap_or(1),
                    imfchan: options.imfchan.unwrap_or(Channel::Red),
                });
            }
        }
    }

    Ok(materials.into_boxed_slice())
}
