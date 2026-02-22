use crate::allocator::find_memory_index;
use crate::device::SharedDeviceRef;
use crate::result::{Error, Result};
use crate::trace_error;

use ash::vk;
use std::rc::Rc;

pub enum IndexType {
    UInt32,
    UInt16,
}

pub struct BufferCreateInfo {
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub memory_property_flags: vk::MemoryPropertyFlags,
}

pub struct Buffer {
    device: SharedDeviceRef,
    pub handle: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: vk::DeviceSize,
    pub offset: vk::DeviceSize,
}

impl Buffer {
    pub fn new(device: SharedDeviceRef, create_info: &BufferCreateInfo) -> Result<Self> {
        let buffer_create_info = ash::vk::BufferCreateInfo {
            size: create_info.size,
            usage: create_info.usage,
            sharing_mode: ash::vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe { device.create_buffer(&buffer_create_info) }
            .inspect_err(|e| trace_error!(e))?;

        let memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let memory_properties = unsafe { device.get_physical_device_memory_properties() };
        let memory_type_index = find_memory_index(
            memory_properties,
            memory_requirements,
            create_info.memory_property_flags,
        )
        .ok_or(Error::CouldNotFindMemoryTypeIndex(
            create_info.memory_property_flags,
        ))
        .inspect_err(|e| {
            trace_error!(e);
            unsafe {
                device.destroy_buffer(buffer);
            }
        })?;

        let allocate_info = ash::vk::MemoryAllocateInfo {
            allocation_size: memory_requirements.size,
            memory_type_index,
            ..Default::default()
        };
        let memory = unsafe { device.allocate_memory(&allocate_info) }.inspect_err(|e| {
            trace_error!(e);
            unsafe {
                device.destroy_buffer(buffer);
            }
        })?;

        let offset = 0;

        unsafe { device.bind_buffer_memory(buffer, memory, offset) }.inspect_err(|e| {
            trace_error!(e);
            unsafe {
                device.destroy_buffer(buffer);
                device.free_memory(memory);
            }
        })?;

        Ok(Buffer {
            device,
            handle: buffer,
            memory,
            size: create_info.size,
            offset,
        })
    }

    #[inline]
    pub unsafe fn map_memory(
        &self,
        offset: vk::DeviceSize,
        size: vk::DeviceSize,
    ) -> ash::prelude::VkResult<*mut std::ffi::c_void> {
        unsafe {
            self.device
                .map_memory(self.memory, offset, size, vk::MemoryMapFlags::empty())
        }
    }

    #[inline]
    pub unsafe fn unmap(&self) {
        unsafe { self.device.unmap_memory(self.memory) }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.memory);
            self.device.destroy_buffer(self.handle);
        }
    }
}

pub enum BufferView {
    Vertex {
        buffer: Rc<Buffer>,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    },
    Index {
        buffer: Rc<Buffer>,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        index_type: vk::IndexType,
    },
    Uniform {
        buffer: Rc<Buffer>,
        offset: vk::DeviceSize,
        size: vk::DeviceSize,
    },
    DynamicUniform {
        buffer: Rc<Buffer>,
        offset: vk::DeviceSize,
        size: vk::DeviceSize,
    },
}

impl BufferView {
    pub unsafe fn bind(&self, command_buffer: vk::CommandBuffer) {
        match self {
            Self::Vertex { buffer, .. } => unsafe {
                buffer
                    .device
                    .cmd_bind_vertex_buffers(command_buffer, 0, &[buffer.handle], &[0])
            },
            Self::Index {
                buffer, index_type, ..
            } => unsafe {
                buffer
                    .device
                    .cmd_bind_index_buffer(command_buffer, buffer.handle, 0, *index_type)
            },
            _ => todo!(),
        }
    }

    pub unsafe fn draw(&self, command_buffer: vk::CommandBuffer) {
        match self {
            Self::Index {
                buffer,
                index_count,
                instance_count,
                first_index,
                ..
            } => unsafe {
                buffer.device.cmd_draw_indexed(
                    command_buffer,
                    *index_count,
                    *instance_count,
                    *first_index,
                    0,
                    0,
                );
            },
            _ => todo!(),
        }
    }
}
