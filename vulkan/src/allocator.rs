pub fn find_memory_index(
    memory_properties: ash::vk::PhysicalDeviceMemoryProperties,
    memory_requirements: ash::vk::MemoryRequirements,
    required_properties: ash::vk::MemoryPropertyFlags,
) -> Option<u32> {
    for i in 0..memory_properties.memory_type_count {
        let mem_type = memory_properties.memory_types[i as usize];
        let type_supported = (memory_requirements.memory_type_bits & (1 << i)) != 0;
        let properties_match = mem_type.property_flags.contains(required_properties);

        if type_supported && properties_match {
            return Some(i);
        }
    }
    return None;
}
