use crate::{constant::SDF_FORMAT, view::GiGpuConfig};
use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::*,
    render::{
        render_resource::{
            binding_types::{storage_buffer_read_only, uniform_buffer},
            BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState,
            ColorWrites, FragmentState, MultisampleState, PipelineCache, PrimitiveState,
            RenderPipelineDescriptor, ShaderStages, ShaderType, StorageBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        view::ViewUniform,
        Extract,
    },
};

#[derive(Resource)]
pub struct SdfPipeline {
    pub layout: BindGroupLayout,
    pub id: CachedRenderPipelineId,
}

impl FromWorld for SdfPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(
            "sdf_pipeline_bindgroup",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<ViewUniform>(true),
                    storage_buffer_read_only::<GpuCircleBuffer>(false),
                    storage_buffer_read_only::<GpuRectBuffer>(false),
                    uniform_buffer::<GiGpuConfig>(false),
                ),
            ),
        );

        let server = world.resource::<AssetServer>();
        let shader = server.load("embedded://solis_2d/shaders/sdf.wgsl");
        // let shader = server.load("sdf.wgsl");

        let id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("sdf_pipeline".into()),
                    layout: vec![layout.clone()],
                    push_constant_ranges: vec![],
                    vertex: fullscreen_shader_vertex_state(),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        shader,
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: SDF_FORMAT,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    zero_initialize_workgroup_memory: false,
                });

        Self { layout, id }
    }
}

// ---------------------------
// extract
#[derive(Clone)]
pub enum SdfShape {
    Circle(f32),
    Rect(Vec2),
}

#[derive(Component, Clone)]
pub struct Emitter {
    pub intensity: f32,
    pub color: Color,
    pub shape: SdfShape,
}

#[derive(Component, Default, Clone)]
pub struct DisableEmitter;

#[derive(Component, Debug, ShaderType, Clone)]
pub struct GpuRect {
    half_extends: Vec2,
    center: Vec2,
    rotation: f32,
    emit: Vec3,
    intensity: f32,
}

#[derive(Component, ShaderType, Debug, Clone)]
pub struct GpuCirlce {
    radius: f32,
    center: Vec2,
    emit: Vec3,
    intensity: f32,
}

#[derive(ShaderType, Default, Clone)]
pub struct GpuCircleBuffer {
    pub count: u32,
    #[size(runtime)]
    pub data: Vec<GpuCirlce>,
}

#[derive(ShaderType, Default, Clone)]
pub struct GpuRectBuffer {
    pub count: u32,
    #[size(runtime)]
    pub data: Vec<GpuRect>,
}

#[derive(Resource, Default)]
pub struct SdfBuffers {
    pub circle_buffer: StorageBuffer<GpuCircleBuffer>,
    pub rect_buffer: StorageBuffer<GpuRectBuffer>,
}

pub fn extract_emitter(
    occluders: Extract<
        Query<
            (
                &Emitter,
                &GlobalTransform,
                &InheritedVisibility,
                &ViewVisibility,
            ),
            Without<DisableEmitter>,
        >,
    >,
    mut buffers: ResMut<SdfBuffers>,
) {
    let mut sdf_rects = Vec::new();
    let mut sdf_circles = Vec::new();

    for (emitter, global, ihview, view) in occluders.iter() {
        if !view.get() || !ihview.get() {
            continue;
        }

        let transform = global.compute_transform();
        match emitter.shape {
            SdfShape::Circle(radius) => sdf_circles.push(GpuCirlce {
                radius,
                center: transform.translation.truncate(),
                emit: emitter.color.to_linear().to_vec3(),
                intensity: emitter.intensity,
            }),
            SdfShape::Rect(half_extend) => {
                let vec_a = transform.right().truncate();
                let vec_b = Vec2::X;
                let full_angle = vec_a.y.atan2(vec_a.x) - vec_b.y.atan2(vec_b.x);

                sdf_rects.push(GpuRect {
                    half_extends: half_extend,
                    center: transform.translation.truncate(),
                    rotation: full_angle,
                    emit: emitter.color.to_linear().to_vec3(),
                    intensity: emitter.intensity,
                });
            }
        }
    }

    let circle_occluders = buffers.circle_buffer.get_mut();
    circle_occluders.count = 0;
    circle_occluders.data.clear();
    for circle in sdf_circles.iter() {
        circle_occluders.count += 1;
        circle_occluders.data.push(circle.clone());
    }

    let rect_occluders = buffers.rect_buffer.get_mut();
    rect_occluders.count = 0;
    rect_occluders.data.clear();
    for rect in sdf_rects.iter() {
        rect_occluders.count += 1;
        rect_occluders.data.push(rect.clone());
    }
}

pub fn prepare_sdf_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut prepared: ResMut<SdfBuffers>,
) {
    prepared
        .circle_buffer
        .write_buffer(&render_device, &render_queue);
    prepared
        .rect_buffer
        .write_buffer(&render_device, &render_queue);
}
