use crate::{constant::CASCADE_FORMAT, view::GiGpuConfig};
use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::*,
    render::{
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState,
            ColorWrites, FilterMode, FragmentState, MultisampleState, PipelineCache,
            PrimitiveState, RenderPipelineDescriptor, Sampler, SamplerBindingType,
            SamplerDescriptor, ShaderStages, ShaderType, TextureSampleType,
        },
        renderer::RenderDevice,
    },
};

#[derive(Resource)]
pub struct RadiancePipeline {
    pub cascade_layout: BindGroupLayout,
    pub cascade_id: CachedRenderPipelineId,
    pub composite_id: CachedRenderPipelineId,
    pub composite_layout: BindGroupLayout,
    pub mipmap_id: CachedRenderPipelineId,
    pub mipmap_layout: BindGroupLayout,
    pub radiance_sampler: Sampler,
    pub point_sampler: Sampler,
}

impl FromWorld for RadiancePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let cascade_layout = create_cascade_layout(&render_device);
        let composite_layout = create_composite_layout(&render_device);
        let mipmap_layout = create_mipmap_layout(&render_device);
        let server = world.resource_ref::<AssetServer>();
        let cascade_shader = server.load("embedded://solis_2d/shaders/cascade.wgsl");
        let composite_shader = server.load("embedded://solis_2d/shaders/composite.wgsl");
        let mipmap_shader = server.load("embedded://solis_2d/shaders/mipmap.wgsl");
        let cache = world.resource::<PipelineCache>();

        let cascade_id = cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("cascade_pipeline".into()),
            layout: vec![cascade_layout.clone()],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                shader: cascade_shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: CASCADE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            zero_initialize_workgroup_memory: false,
        });

        let composite_id = cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("composite_pipeline".into()),
            layout: vec![composite_layout.clone()],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                shader: composite_shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: CASCADE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            zero_initialize_workgroup_memory: false,
        });

        let mipmap_id = cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("mipmap_pipeline".into()),
            layout: vec![mipmap_layout.clone()],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                shader: mipmap_shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: CASCADE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            zero_initialize_workgroup_memory: false,
        });

        let radiance_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("radiance sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..default()
        });

        Self {
            cascade_id,
            cascade_layout,
            composite_id,
            mipmap_id,
            mipmap_layout,
            composite_layout,
            radiance_sampler,
            point_sampler: render_device.create_sampler(&SamplerDescriptor::default()),
        }
    }
}

fn create_composite_layout(render_device: &RenderDevice) -> BindGroupLayout {
    return render_device.create_bind_group_layout(
        "composite_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                //main tex
                texture_2d(TextureSampleType::Float { filterable: true }),
                //sdf tex
                texture_2d(TextureSampleType::Float { filterable: true }),
                //merge tex 0
                texture_2d(TextureSampleType::Float { filterable: true }),
                //merge tex 1
                texture_2d(TextureSampleType::Float { filterable: true }),
                //mipmap tex
                texture_2d(TextureSampleType::Float { filterable: true }),
                // normal
                texture_2d(TextureSampleType::Float { filterable: true }),
                //linear sample
                sampler(SamplerBindingType::Filtering),
                //point sample
                sampler(SamplerBindingType::NonFiltering),
                //config
                uniform_buffer::<GiGpuConfig>(false),
            ),
        ),
    );
}
fn create_cascade_layout(render_device: &RenderDevice) -> BindGroupLayout {
    return render_device.create_bind_group_layout(
        "cascade_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // sdf
                texture_2d(TextureSampleType::Float { filterable: true }),
                // last cascade
                texture_2d(TextureSampleType::Float { filterable: true }),
                // normal
                texture_2d(TextureSampleType::Float { filterable: true }),
                // radiance sampler
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<GiGpuConfig>(false),
                uniform_buffer::<Probe>(true),
            ),
        ),
    );
}

fn create_mipmap_layout(render_device: &RenderDevice) -> BindGroupLayout {
    return render_device.create_bind_group_layout(
        "mipmap_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                uniform_buffer::<GiGpuConfig>(false),
            ),
        ),
    );
}

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct Probe {
    /// index of current
    pub cascade_index: u32,
}
