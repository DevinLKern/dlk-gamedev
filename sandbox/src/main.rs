mod camera;
mod constants;
mod result;

use camera::Camera;
use constants::{WORLD_FORWARDS, WORLD_RIGHT, WORLD_UP};
use image::DynamicImage;
use renderer::{MaterialUBO, ShaderVertVertex};
use result::{Error, Result};

use ash::vk;

use std::str::FromStr;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, EventLoop},
    raw_window_handle::HasDisplayHandle,
    window::{Window, WindowId},
};

use math::Identity;
use math::Vec3;
use math::Vec4;
use math::{Quat, Zero};

enum ApplicationState {
    ObjectMode,
    CameraMode,
}

struct Application {
    state: ApplicationState,
    mouse_sensitivity: f64,
    focused_window: Option<WindowId>,
    active_window: Option<WindowId>,
    windows: HashMap<WindowId, (renderer::RenderContext, Window, Camera)>,
    renderer: renderer::Renderer,
    draw_infos: Box<[(vulkan::VertexBV, vulkan::IndexBV, u32)]>,
    model_transform: math::AffineTransform,
    global_light_direction: Vec3<f32>,
    global_light_color: Vec4<f32>,
    global_ambient_light: f32,
    exiting: bool,
}

const DEFAULT_IMAGE: &[u8] = include_bytes!("../../files/images/default.png");

impl Application {
    fn search_for(base: &Path, target: &Path) -> Option<PathBuf> {
        if !base.is_dir() {
            return None;
        }

        let mut ancestors = base.ancestors();
        while let Some(ancestor) = ancestors.next() {
            let cur = ancestor.join(target);

            if cur.exists() {
                return Some(cur);
            }
        }

        return None;
    }
    fn new(
        mouse_sensitivity: f64,
        derive_normals: bool,
        obj_to_world: math::Mat3<f32>,
        model_path: &std::path::Path,
        debug_enabled: bool,
        display_handle: &winit::raw_window_handle::DisplayHandle,
    ) -> Result<Self> {
        let state = ApplicationState::ObjectMode;

        // load materials
        let file_path = model_path.with_extension("mtl");
        let mtl_materials = obj_mtl::load_materials(&file_path)?;

        // load textures and images
        let (texture_data, texture_name_to_index) = {
            let mut texture_data = Vec::<DynamicImage>::with_capacity(8);
            let mut texture_indices = HashMap::<Box<str>, usize>::new();
            let default_texture_data =
                image::load_from_memory_with_format(DEFAULT_IMAGE, image::ImageFormat::Png)?;

            texture_data.push(default_texture_data);

            // load diffuse textures
            for mat in mtl_materials.iter() {
                let diffuse_texture = if let Some(texture) = &mat.diffuse.texture {
                    texture
                } else {
                    continue;
                };

                // skip if texture already loaded
                if texture_indices.contains_key(&diffuse_texture.file_path) {
                    continue;
                }

                // search for texture
                let path = {
                    let base = model_path.with_file_name("");
                    let target = match PathBuf::from_str(&diffuse_texture.file_path) {
                        Ok(path) => path,
                        Err(_) => {
                            tracing::warn!(
                                "Malformed diffuse texture file path. Reverting to base color."
                            );
                            continue;
                        }
                    };

                    match Self::search_for(&base, &target) {
                        Some(path) => path,
                        None => {
                            tracing::warn!(
                                "Could not find diffuse texture. Reverting to base color."
                            );
                            continue;
                        }
                    }
                };

                let data = image::open(&path)?;

                let index = texture_data.len();
                texture_indices.insert(diffuse_texture.file_path.clone(), index);
                texture_data.push(data);
            }

            (texture_data, texture_indices)
        };

        // create materials
        let mut materials = Vec::<renderer::MaterialUBO>::with_capacity(mtl_materials.len() + 1);
        // add default material
        materials.push(renderer::MaterialUBO {
            flags: 0,
            texture_index: 0,
            _pad2: [0; 8],
            base_color: [0.8, 0.2, 0.2, 1.0],
        });
        let mut name_to_material_index = HashMap::<Box<str>, usize>::new();
        for material in mtl_materials.into_iter() {
            if name_to_material_index.contains_key(&material.name) {
                continue;
            }

            let (color, texture) = (
                material.diffuse.color.as_ref(),
                material.diffuse.texture.as_ref(),
            );
            let (color, texture, flags) = match (color, texture) {
                (Some(c), Some(t)) => {
                    if let Some(&idx) = texture_name_to_index.get(&t.file_path) {
                        ([c[0], c[1], c[2], 1.0], idx as u32, 1u32)
                    } else {
                        tracing::warn!(
                            "Texture '{}' not found in loaded textures. Falling back to base color.",
                            t.file_path
                        );
                        ([c[0], c[1], c[2], 1.0], 0u32, 0u32)
                    }
                }
                (Some(c), None) => ([c[0], c[1], c[2], 1.0], 0u32, 0u32),
                (None, Some(t)) => {
                    if let Some(&idx) = texture_name_to_index.get(&t.file_path) {
                        ([0.0, 0.0, 0.0, 0.0], idx as u32, 1u32)
                    } else {
                        tracing::warn!(
                            "Texture '{}' not found in loaded textures. Disabling texture flag.",
                            t.file_path
                        );
                        ([1.0, 1.0, 1.0, 1.0], 0u32, 0u32)
                    }
                }
                (None, None) => ([1.0, 1.0, 1.0, 1.0], 0u32, 0u32),
            };

            let idx = materials.len();
            materials.push(MaterialUBO {
                flags,
                base_color: color,
                texture_index: texture,
                _pad2: [0; 8],
            });
            name_to_material_index.insert(material.name, idx);
        }

        let (model_transform, plane_transform, mesh_data) = {
            // vertex_data, index_data, material_index, model_transform_index
            let mut mesh_data = Vec::<(Vec<ShaderVertVertex>, Vec<u32>, u32)>::new();

            let plane_transform = math::AffineTransform {
                position: Vec3::ZERO.sub(WORLD_UP).scaled(0.5),
                orientation: Quat::IDENTITY,
                scalar: Vec3::new(1000.0, 1000.0, 1000.0),
            };

            let plane_vertex_buffer_data = {
                const F: Vec3<f32> = WORLD_FORWARDS;
                const B: Vec3<f32> = Vec3::ZERO.sub(WORLD_FORWARDS);
                const R: Vec3<f32> = WORLD_RIGHT;
                const L: Vec3<f32> = Vec3::ZERO.sub(WORLD_RIGHT);

                const FR: Vec3<f32> = F.add(R);
                const FL: Vec3<f32> = F.add(L);
                const BR: Vec3<f32> = B.add(R);
                const BL: Vec3<f32> = B.add(L);

                vec![
                    renderer::ShaderVertVertex {
                        position: FL.into_arr(),
                        tex_coord: [1.0, 0.0],
                        normal: WORLD_UP.into_arr(),
                    },
                    renderer::ShaderVertVertex {
                        position: FR.into_arr(),
                        tex_coord: [0.0, 0.0],
                        normal: WORLD_UP.into_arr(),
                    },
                    renderer::ShaderVertVertex {
                        position: BR.into_arr(),
                        tex_coord: [0.0, 1.0],
                        normal: WORLD_UP.into_arr(),
                    },
                    renderer::ShaderVertVertex {
                        position: BL.into_arr(),
                        tex_coord: [1.0, 1.0],
                        normal: WORLD_UP.into_arr(),
                    },
                ]
            };
            let plane_index_buffer_data = vec![0, 1, 2, 2, 3, 0];

            // 0 is the index of the default material
            mesh_data.push((plane_vertex_buffer_data, plane_index_buffer_data, 0));

            use obj_mtl::*;
            let objf = ObjScene::from_file(model_path)?;
            for shape in objf.get_shapes() {
                let mut vertices = Vec::<ShaderVertVertex>::new();
                let mut indices = Vec::<u32>::new();
                let mut vertex_map = HashMap::<VtnIndex, u32>::new();

                let material_idx: u32 = if shape.materials.len() > 0 {
                    if shape.materials.len() != 1 {
                        tracing::warn!("Multiple materials per shape not supported");
                    }

                    let mat = shape.materials.first().unwrap();
                    let idx = name_to_material_index.get(mat).unwrap_or_else(|| {
                        tracing::warn!("Could not find {} material. Defaulting to 0", mat);
                        &0
                    });
                    *idx as u32
                } else {
                    0
                };

                // Build a triangle list (fan triangulation for polygons/quads).
                let mut triangles = Vec::<(VtnIndex, VtnIndex, VtnIndex)>::with_capacity(64);
                for primitive in shape.get_primitives() {
                    match primitive {
                        Primitive::Triangle { v0, v1, v2 } => triangles.push((*v0, *v1, *v2)),
                        Primitive::Polygon(poly) => {
                            if poly.len() < 3 {
                                continue;
                            }
                            let v0 = poly[0];
                            for i in 1..(poly.len() - 1) {
                                triangles.push((v0, poly[i], poly[i + 1]));
                            }
                        }
                        _ => {}
                    }
                }

                for (v0, v1, v2) in triangles {
                    let derived_normal = if derive_normals {
                        match (v0.vn, v1.vn, v2.vn) {
                            (None, None, None) => {
                                let p0 = &objf.vs[v0.v];
                                let p0 = Vec3::new(p0.x as f32, p0.y as f32, p0.z as f32);
                                let p1 = &objf.vs[v1.v];
                                let p1 = Vec3::new(p1.x as f32, p1.y as f32, p1.z as f32);
                                let p2 = &objf.vs[v2.v];
                                let p2 = Vec3::new(p2.x as f32, p2.y as f32, p2.z as f32);

                                let face_normal = p1.sub(p0).cross(p2.sub(p0));

                                Some(obj_to_world.mul_vec(face_normal).into_arr())
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    let derived_normal = match derived_normal {
                        Some(n) => n,
                        None => [0.0, 0.0, 0.0],
                    };

                    for v in [v0, v1, v2] {
                        let index = if let Some(&i) = vertex_map.get(&v) {
                            i
                        } else {
                            let position = obj_to_world
                                .mul_vec(Vec3::new(
                                    objf.vs[v.v].x as f32,
                                    objf.vs[v.v].y as f32,
                                    objf.vs[v.v].z as f32,
                                ))
                                .into_arr();

                            let tex_coord = if let Some(i) = v.vt {
                                [objf.vts[i].u as f32, 1.0 - objf.vts[i].v as f32]
                            } else {
                                [0.0, 0.0]
                            };

                            let normal = if let Some(i) = v.vn {
                                obj_to_world
                                    .mul_vec(Vec3::new(
                                        objf.vns[i].x as f32,
                                        objf.vns[i].y as f32,
                                        objf.vns[i].z as f32,
                                    ))
                                    .into_arr()
                            } else {
                                derived_normal
                            };

                            let i = vertices.len() as u32;
                            vertices.push(ShaderVertVertex {
                                position,
                                tex_coord,
                                normal,
                            });
                            vertex_map.insert(v, i);
                            i
                        };

                        indices.push(index);
                    }
                }

                mesh_data.push((vertices, indices, material_idx));
            }

            let model_transform = {
                let mut min = [f32::MAX; 3];
                let mut max = [f32::MIN; 3];
                for (vertices, _, _) in mesh_data.iter() {
                    for v in vertices.iter() {
                        for i in 0..3 {
                            min[i] = min[i].min(v.position[i]);
                            max[i] = max[i].max(v.position[i]);
                        }
                    }
                }
                let min = Vec3::new(min[0], min[1], min[2]);
                let max = Vec3::new(max[0], max[1], max[2]);

                let model_scale = (max.x() - min.x())
                    .max(max.y() - min.y())
                    .max(max.z() - min.z());
                let model_scale = 1.0 / model_scale;

                let model_pos = {
                    let min_normalized = min.scaled(model_scale);
                    let min_reduced = min_normalized.scaled_nonuniform(WORLD_UP);
                    let plane_reduced = plane_transform.position.scaled_nonuniform(WORLD_UP);

                    plane_reduced.sub(min_reduced)
                };

                math::AffineTransform {
                    position: model_pos,
                    orientation: Quat::IDENTITY,
                    scalar: Vec3::new(model_scale, model_scale, model_scale),
                }
            };

            (model_transform, plane_transform, mesh_data)
        };

        let mut mesh_ubo_buffer_data: Box<[(math::AffineTransform, u32)]> = mesh_data
            .iter()
            .map(|(_, _, material_index)| (model_transform, *material_index))
            .collect();
        mesh_ubo_buffer_data[0] = (plane_transform, 0);

        let renderer = renderer::Renderer::new(
            debug_enabled,
            display_handle,
            mesh_ubo_buffer_data.len() as u64,
            &texture_data,
            &materials,
        )?;

        let mut draw_infos = Vec::<(vulkan::VertexBV, vulkan::IndexBV, u32)>::new();
        for (vb_data, ib_data, mesh_idx) in mesh_data.into_iter() {
            if vb_data.len() == 0 || ib_data.len() == 0 {
                continue;
            }
            let vb_data_u8 = unsafe {
                std::slice::from_raw_parts(
                    vb_data.as_ptr() as *const u8,
                    vb_data.len() * std::mem::size_of::<renderer::ShaderVertVertex>(),
                )
            };

            let vb = renderer.create_vertex_buffer(&vb_data_u8, vb_data.len() as u32)?;

            let ib_data_u8 = unsafe {
                std::slice::from_raw_parts(
                    ib_data.as_ptr() as *const u8,
                    ib_data.len() * std::mem::size_of::<u32>(),
                )
            };

            let ib = renderer.create_index_buffer(
                ib_data_u8,
                vk::IndexType::UINT32,
                ib_data.len() as u32,
                0,
            )?;

            draw_infos.push((vb, ib, mesh_idx))
        }

        for (i, (transform, material_index)) in mesh_ubo_buffer_data.iter().enumerate() {
            let ubo_data = renderer::MeshUBO {
                model: transform.as_mat4().into_2d_arr(),
                material_index: *material_index,
            };
            let src = &ubo_data;
            let offset = i as u64 * renderer.model_transform_buffer_element_size;
            unsafe {
                let dst = renderer
                    .model_transform_buffer
                    .map_memory(offset, renderer.model_transform_buffer_element_size)
                    .inspect_err(|e| tracing::error!("{e}"))
                    .unwrap();

                std::ptr::copy_nonoverlapping(src, dst as *mut renderer::MeshUBO, 1);

                renderer.model_transform_buffer.unmap()
            }
        }

        Ok(Self {
            state,
            mouse_sensitivity,
            focused_window: None,
            active_window: None,
            renderer,
            windows: std::collections::HashMap::new(),
            draw_infos: draw_infos.into_boxed_slice(),
            exiting: false,
            model_transform,
            global_light_direction: Vec3::ZERO.sub(WORLD_UP).add(WORLD_RIGHT.scaled(0.2)),
            global_light_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            global_ambient_light: 0.1,
        })
    }

    // returns true if a window close was requested.
    fn handle_window_event(
        &mut self,
        event: winit::event::WindowEvent,
        window_id: &winit::window::WindowId,
    ) -> Result<bool> {
        use winit::event::WindowEvent;

        let (context, window, camera) = self
            .windows
            .get_mut(window_id)
            .ok_or(Error::WindowIdInvalid)?;

        match event {
            WindowEvent::CloseRequested => {
                tracing::debug!("close requested!");
                // unsafe { self.renderer.destroy_render_context(context) };
                return Ok(true);
            }
            WindowEvent::Resized(s) => {
                unsafe { self.renderer.device.device_wait_idle() }
                    .inspect_err(|e| tracing::error!("{e}"))
                    .unwrap();

                {
                    let (w, h) = (s.width as f32, s.height as f32);
                    let aspect_ratio = w / h;

                    camera.set_aspect_ratio(aspect_ratio);
                }

                let new_context = self.renderer.create_render_context(window)?;
                *context = new_context;

                let camera_ubo = renderer::CameraUBO {
                    view: camera.get_view_matrix().into_2d_arr(),
                    proj: camera.get_projection_matrix().into_2d_arr(),
                };
                context.update_camera(camera_ubo)?;
            }
            WindowEvent::RedrawRequested => {
                let camera_ubo = renderer::CameraUBO {
                    view: camera.get_view_matrix().into_2d_arr(),
                    proj: camera.get_projection_matrix().into_2d_arr(),
                };
                context.update_camera(camera_ubo)?;

                let pipeline = context.get_pipeline();

                let temp = context.index as u32 * context.per_frame_buffer_element_size;

                let record_draw_commands = |cmd: vk::CommandBuffer| unsafe {
                    pipeline.bind(cmd);
                    {
                        let sets = [self.renderer.descriptor_sets[0]];
                        self.renderer.device.cmd_bind_descriptor_sets(
                            cmd,
                            self.renderer.pipeline_layout.bind_point,
                            self.renderer.pipeline_layout.handle,
                            0,
                            &sets,
                            &[temp],
                        );
                    }
                    {
                        let sets = [self.renderer.descriptor_sets[2]];
                        self.renderer.device.cmd_bind_descriptor_sets(
                            cmd,
                            self.renderer.pipeline_layout.bind_point,
                            self.renderer.pipeline_layout.handle,
                            2,
                            &sets,
                            &[],
                        );
                    }

                    for (vb, ib, mesh_idx) in self.draw_infos.iter() {
                        {
                            let sets = [self.renderer.descriptor_sets[1]];
                            self.renderer.device.cmd_bind_descriptor_sets(
                                cmd,
                                self.renderer.pipeline_layout.bind_point,
                                self.renderer.pipeline_layout.handle,
                                1,
                                &sets,
                                &[*mesh_idx
                                    * self.renderer.model_transform_buffer_element_size as u32],
                            );
                        }
                        vb.bind(cmd);
                        ib.bind(cmd);
                        ib.draw(cmd);
                    }
                };

                unsafe { context.draw(record_draw_commands) }?;

                window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::event::KeyEvent;
                use winit::keyboard::KeyCode;

                const SPEED: f32 = 0.025;
                match event {
                    KeyEvent { physical_key, .. } => match physical_key {
                        winit::keyboard::PhysicalKey::Code(c) => match c {
                            KeyCode::Escape => {
                                self.active_window = None;
                                match window.set_cursor_grab(winit::window::CursorGrabMode::None) {
                                    Err(e) => {
                                        tracing::error!("{}", e);
                                    }
                                    _ => {}
                                }
                                window.set_cursor_visible(true);
                            }
                            KeyCode::KeyE => {
                                if let ApplicationState::CameraMode = self.state {
                                    camera.move_local(WORLD_FORWARDS.scaled(SPEED));
                                }
                            }
                            KeyCode::KeyD => {
                                if let ApplicationState::CameraMode = self.state {
                                    camera.move_local(WORLD_FORWARDS.scaled(-SPEED));
                                }
                            }
                            KeyCode::KeyF => {
                                if let ApplicationState::CameraMode = self.state {
                                    camera.move_local(WORLD_RIGHT.scaled(SPEED));
                                }
                            }
                            KeyCode::KeyS => {
                                if let ApplicationState::CameraMode = self.state {
                                    camera.move_local(WORLD_RIGHT.scaled(-SPEED));
                                }
                            }
                            KeyCode::Space => {
                                if let ApplicationState::CameraMode = self.state {
                                    camera.move_local(WORLD_UP.scaled(SPEED));
                                }
                            }
                            KeyCode::ControlLeft => {
                                if let ApplicationState::CameraMode = self.state {
                                    camera.move_local(WORLD_UP.scaled(-SPEED));
                                }
                            }
                            KeyCode::KeyO => {
                                self.state = ApplicationState::ObjectMode;
                            }
                            KeyCode::KeyC => {
                                self.state = ApplicationState::CameraMode;
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                }
                // println!("Keyboard Input!");
            }
            WindowEvent::Moved(_) => {
                // println!("Moved!");
            }
            WindowEvent::Focused(b) => {
                if b {
                    self.focused_window = Some(*window_id);
                }
                // tracing::trace!("Focused!");
            }
            WindowEvent::MouseInput { button, state, .. } => {
                match state {
                    winit::event::ElementState::Released => {
                        return Ok(false);
                    }
                    _ => {
                        //
                    }
                }
                use winit::event::MouseButton;

                // tracing::trace!("Mouse Input!");
                match button {
                    MouseButton::Left => match self.active_window {
                        None => {
                            self.active_window = self.focused_window;
                            window
                                .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                                .or_else(|_| {
                                    window.set_cursor_grab(winit::window::CursorGrabMode::Confined)
                                })
                                .inspect_err(|e| tracing::error!("{e}"))?;
                            window.set_cursor_visible(false);
                        }
                        Some(_) => {
                            self.active_window = None;
                            match window.set_cursor_grab(winit::window::CursorGrabMode::None) {
                                Err(e) => {
                                    tracing::error!("{}", e);
                                }
                                _ => {}
                            }
                            window.set_cursor_visible(true);
                        }
                    },
                    _ => {}
                }
            }
            _ => {
                //
            }
        }

        Ok(false)
    }
}
impl ApplicationHandler for Application {
    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        if self.exiting {
            return;
        }

        self.exiting = true;

        return event_loop.exit();
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() {
            return;
        }

        let window_attributes =
            winit::window::WindowAttributes::default().with_title("dlk-objviewer");
        let window = match event_loop.create_window(window_attributes) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("{}", e);
                return self.exiting(event_loop);
            }
        };

        let window_id = window.id();

        let context = match self.renderer.create_render_context(&window) {
            Ok(context) => context,
            Err(e) => {
                tracing::error!("{}", e);
                return self.exiting(event_loop);
            }
        };
        let camera = {
            let s = window.inner_size();
            let (w, h) = (s.width as f32, s.height as f32);
            let aspect_ratio = w / h;

            Camera::new(
                65.0,
                aspect_ratio,
                self.model_transform
                    .position
                    .add(Vec3::ZERO.sub(WORLD_FORWARDS)),
                WORLD_FORWARDS,
            )
        };

        let camera_ubo = renderer::CameraUBO {
            view: camera.get_view_matrix().into_2d_arr(),
            proj: camera.get_projection_matrix().into_2d_arr(),
        };
        context
            .update_camera(camera_ubo)
            .inspect_err(|e| tracing::error!("{e}"))
            .unwrap();

        self.renderer
            .update_world_light(
                self.global_ambient_light,
                self.global_light_direction,
                self.global_light_color,
            )
            .unwrap();

        self.windows.insert(window_id, (context, window, camera));
    }

    #[allow(unused_variables)]
    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        use winit::event::DeviceEvent;

        let (_, _, camera) = match self.active_window {
            Some(id) => match self.windows.get_mut(&id) {
                Some(x) => x,
                None => {
                    return;
                }
            },
            None => {
                return;
            }
        };
        match self.state {
            ApplicationState::CameraMode => {
                match event {
                    DeviceEvent::MouseMotion { delta } => {
                        let dx = delta.0 * self.mouse_sensitivity;
                        let dy = delta.1 * self.mouse_sensitivity;

                        camera.rotate(dx as f32, dy as f32);
                    }
                    _ => {
                        // tracing::info!("Not implemented")
                    }
                }
            }
            ApplicationState::ObjectMode => {
                match event {
                    DeviceEvent::MouseMotion { delta } => {
                        let dx = delta.0 * self.mouse_sensitivity;
                        let dy = delta.1 * self.mouse_sensitivity;

                        let qx = Quat::unit_from_angle_axis(dx as f32, WORLD_UP);
                        let qy = Quat::unit_from_angle_axis(dy as f32, WORLD_RIGHT);

                        self.model_transform.rotate_local(qx.mul(qy));

                        for (i, (_, _, idx)) in self.draw_infos.iter().enumerate().skip(1) {
                            let src = renderer::MeshUBO {
                                model: self.model_transform.as_mat4().into_2d_arr(),
                                material_index: *idx,
                            };
                            unsafe {
                                let offset =
                                    i as u64 * self.renderer.model_transform_buffer_element_size;
                                let dst = self
                                    .renderer
                                    .model_transform_buffer
                                    .map_memory(
                                        offset,
                                        self.renderer.model_transform_buffer_element_size,
                                    )
                                    .inspect_err(|e| tracing::error!("{e}"))
                                    .unwrap();

                                std::ptr::copy_nonoverlapping(
                                    &src,
                                    dst as *mut renderer::MeshUBO,
                                    1,
                                );

                                self.renderer.model_transform_buffer.unmap();
                            }
                        }
                    }
                    _ => {
                        // tracing::info!("Not implemented")
                    }
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        if self.exiting {
            return;
        }

        match self.handle_window_event(event, &window_id) {
            Ok(b) => {
                if b {
                    self.exiting(event_loop);
                }
            }
            Err(e) => {
                tracing::error!("{}", e);
                self.exiting(event_loop);
            }
        }
    }
}

fn main() -> Result<()> {
    // let file_appender = tracing_appender::rolling::daily("logs", "app.log");
    // let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        //     .with_writer(non_blocking)
        .with_max_level(tracing::Level::DEBUG)
        .with_file(true)
        .with_line_number(true)
        .init();

    let args: Box<[String]> = std::env::args().collect();

    let print_usage = || -> Result<()> {
        let name = format!(
            "{}",
            std::env::current_exe()?.file_name().unwrap().display()
        );
        println!(
            "Invalid program arguments. Usage: {} <options> <model>",
            name
        );
        println!("To view all options type {} --help", name);
        return Ok(());
    };

    if args.len() < 2 {
        return print_usage();
    }

    if let Some(_) = args.iter().find(|s| s.as_str() == "--help") {
        println!("Options:");
        println!("    -f Specifies the forwards direction of the model. Defaults to +z.");
        println!("        may be one of: <+x|-x|+y|-y|+z|-z>.");
        println!("    -r Specifies the right direction of the model. Defaults to +x.");
        println!("        may be one of: <+x|-x|+y|-y|+z|-z>");
        println!("    -u Specifies the up direction of the model. Defaults to +y.");
        println!("        may be one of: <+x|-x|+y|-y|+z|-z>");
        println!("    --derive-normals normals to be derived when missing. Defaults to true.");
        println!("        may be one of of: <true|false>");
        println!(
            "    --mouse-sensitivity Specifies the sensitivity of the mouse. Defaults to 50.0"
        );
        println!("        may be any value from 1 to 100");

        return Ok(());
    }

    let model_path = {
        let args: Vec<String> = std::env::args().collect();
        std::path::PathBuf::from(args[args.len() - 1].clone())
    };

    let mouse_sensitivity = {
        let idx = args.iter().enumerate().find_map(|(i, s)| {
            if s == "--mouse-sensitivity" {
                Some(i)
            } else {
                None
            }
        });

        let sensitivity = if let Some(i) = idx {
            if let Some(s) = args.get(i + 1) {
                if let Ok(ms) = s.parse::<f64>() {
                    ms
                } else {
                    println!("Error: {} is not a valid mouse sensitivity.", s);
                    return Ok(());
                }
            } else {
                println!("Eror: Could not get mouse sentitivy. Terminating program.");
                return Ok(());
            }
        } else {
            50.0
        };

        if sensitivity < 1.0 || 100.0 < sensitivity {
            println!("Warning: Sensitivity should be set to a value between 1 and 100.");
        }

        sensitivity / 50000.0
    };

    let derive_normals = {
        let idx = args.iter().enumerate().find_map(|(i, s)| {
            if s == "--derive-normals" {
                Some(i)
            } else {
                None
            }
        });
        if let Some(i) = idx {
            match args.get(i + 1).map(|x| x.as_str()) {
                Some("true") => true,
                Some("false") => false,
                _ => {
                    return print_usage();
                }
            }
        } else {
            true
        }
    };

    let (obj_r, obj_u, obj_f) = {
        let ri = args
            .iter()
            .enumerate()
            .find_map(|(i, s)| if s == "-r" { Some(i) } else { None });
        let ui = args
            .iter()
            .enumerate()
            .find_map(|(i, s)| if s == "-u" { Some(i) } else { None });
        let fi = args
            .iter()
            .enumerate()
            .find_map(|(i, s)| if s == "-f" { Some(i) } else { None });

        let rs = if let Some(i) = ri {
            args.get(i + 1).map(|x| x.as_str())
        } else {
            Some("+x")
        };
        let us = if let Some(i) = ui {
            args.get(i + 1).map(|x| x.as_str())
        } else {
            Some("+y")
        };
        let fs = if let Some(i) = fi {
            args.get(i + 1).map(|x| x.as_str())
        } else {
            Some("+z")
        };

        let str_to_vec = |s: Option<&str>| -> Option<math::Vec3<f32>> {
            match s {
                Some("+x") => Some(Vec3::new(1.0, 0.0, 0.0)),
                Some("-x") => Some(Vec3::new(-1.0, 0.0, 0.0)),
                Some("+y") => Some(Vec3::new(0.0, 1.0, 0.0)),
                Some("-y") => Some(Vec3::new(0.0, -1.0, 0.0)),
                Some("+z") => Some(Vec3::new(0.0, 0.0, 1.0)),
                Some("-z") => Some(Vec3::new(0.0, 0.0, -1.0)),
                _ => None,
            }
        };

        match (str_to_vec(rs), str_to_vec(us), str_to_vec(fs)) {
            (Some(rv), Some(uv), Some(fv)) => (rv, uv, fv),
            _ => {
                return print_usage();
            }
        }
    };

    if obj_r.cross(obj_u) != obj_f && obj_u.cross(obj_r) != obj_f {
        println!("Invalid input. Right, left, and up must form a valid coordinate system.");
        return Ok(());
    }

    let obj_to_world = {
        let to_obj = math::Mat3::<f32>::from_cols(obj_r, obj_u, obj_f);

        // The transpose is equivalent to the inverse of a matrix when the matrix is orthonormal.
        let from_obj = to_obj.transposed();

        const INTO_WORLD: math::Mat3<f32> =
            math::Mat3::from_cols(WORLD_RIGHT, WORLD_UP, WORLD_FORWARDS);

        from_obj.mul(&INTO_WORLD)
    };

    let event_loop = EventLoop::new().inspect_err(|e| tracing::error!("{e}"))?;

    let mut app = {
        let debug_enabled = cfg!(debug_assertions);
        let owned_display_handle = event_loop.owned_display_handle();
        let display_handle = owned_display_handle.display_handle()?;
        Application::new(
            mouse_sensitivity,
            derive_normals,
            obj_to_world,
            model_path.as_path(),
            debug_enabled,
            &display_handle,
        )?
    };

    event_loop
        .run_app(&mut app)
        .inspect_err(|e| tracing::error!("{e}"))?;

    Ok(())
}
