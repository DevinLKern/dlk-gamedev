pub mod render_context;
pub mod result;

pub fn create_vertex_buffer(
    device: std::rc::Rc<vulkan::device::Device>,
    data: &[u8],
    vertex_count: u32,
    first_vertex: u32,
) -> vulkan::result::Result<std::rc::Rc<vulkan::buffer::BufferView>> {
    let buffer = {
        let buffer_create_info = vulkan::buffer::BufferCreateInfo {
            size: data.len() as u64,
            usage: ash::vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_property_flags: ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
        };

        vulkan::buffer::Buffer::new(device.clone(), &buffer_create_info)?
    };

    let buffer = std::rc::Rc::new(buffer);

    unsafe {
        let dst = buffer.map()?;

        std::ptr::copy_nonoverlapping(data.as_ptr(), dst as *mut u8, data.len());

        buffer.unmap();
    }

    let view = vulkan::buffer::BufferView::Vertex {
        buffer,
        vertex_count,
        instance_count: 1,
        first_vertex,
        first_instance: 0,
    };

    Ok(std::rc::Rc::new(view))
}
pub fn create_index_buffer(
    device: std::rc::Rc<vulkan::device::Device>,
    data: &[u8],
    index_type: ash::vk::IndexType,
    index_count: u32,
    first_index: u32,
) -> vulkan::result::Result<std::rc::Rc<vulkan::buffer::BufferView>> {
    let buffer = {
        let buffer_create_info = vulkan::buffer::BufferCreateInfo {
            size: data.len() as u64,
            usage: ash::vk::BufferUsageFlags::INDEX_BUFFER,
            memory_property_flags: ash::vk::MemoryPropertyFlags::HOST_VISIBLE
                | ash::vk::MemoryPropertyFlags::HOST_COHERENT,
        };

        vulkan::buffer::Buffer::new(device.clone(), &buffer_create_info)?
    };

    let buffer = std::rc::Rc::new(buffer);

    unsafe {
        let dst = buffer.map()?;

        std::ptr::copy_nonoverlapping(data.as_ptr(), dst as *mut u8, data.len());

        buffer.unmap();
    }

    let view = vulkan::buffer::BufferView::Index {
        buffer,
        index_count,
        instance_count: 1,
        first_index,
        index_type,
    };

    Ok(std::rc::Rc::new(view))
}
