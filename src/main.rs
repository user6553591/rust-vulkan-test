// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

extern crate cgmath;
extern crate winit;
extern crate time;
extern crate obj;

#[macro_use]
extern crate vulkano;
extern crate vulkano_win;

use vulkano_win::VkSurfaceBuild;

mod vs { include!{concat!(env!("OUT_DIR"), "/shaders/assets/build/shaders/vs.glsl")} }
mod fs { include!{concat!(env!("OUT_DIR"), "/shaders/assets/build/shaders/fs.glsl")} }

fn main() {
    // FPS variables.
    let mut frame_number: u64 = 0;
    let mut last_frame_number: u64 = 0;
    let mut last_nanosecond: u64 = 0;

    // The start of this example is exactly the same as `triangle`. You should read the
    // `triangle` example if you haven't done so yet.

    let extensions = vulkano_win::required_extensions();
    let instance = vulkano::instance::Instance::new(None, &extensions, None).expect("failed to create instance");

    let physical = vulkano::instance::PhysicalDevice::enumerate(&instance)
                            .next().expect("no device available");
    println!("Using device: {} (type: {:?})", physical.name(), physical.ty());

    let window = winit::WindowBuilder::new().build_vk_surface(&instance).unwrap();

    let queue = physical.queue_families().find(|q| q.supports_graphics() &&
                                                   window.surface().is_supported(q).unwrap_or(false))
                                                .expect("couldn't find a graphical queue family");

    let device_ext = vulkano::device::DeviceExtensions {
        khr_swapchain: true,
        .. vulkano::device::DeviceExtensions::none()
    };

    let (device, mut queues) = vulkano::device::Device::new(&physical, physical.supported_features(),
                                                            &device_ext, [(queue, 0.5)].iter().cloned())
                               .expect("failed to create device");
    let queue = queues.next().unwrap();

    let (swapchain, images) = {
        let caps = window.surface().get_capabilities(&physical).expect("failed to get surface capabilities");

        let dimensions = caps.current_extent.unwrap_or([1280, 1024]);
        let present = caps.present_modes.iter().next().unwrap();
        let usage = caps.supported_usage_flags;
        let format = caps.supported_formats[0].0;

        vulkano::swapchain::Swapchain::new(&device, &window.surface(), caps.min_image_count, format, dimensions, 1,
                                           &usage, &queue, vulkano::swapchain::SurfaceTransform::Identity,
                                           vulkano::swapchain::CompositeAlpha::Opaque,
                                           present, true, None).expect("failed to create swapchain")
    };


    let obj_filepath = std::io::BufReader::new(std::fs::File::open("assets/models/suzanne.obj").unwrap());
    let input_obj: obj::Obj = obj::load_obj(obj_filepath).unwrap();

    #[derive(Copy, Clone)]
    pub struct Vertex {
        position: [f32; 3]
    }
    impl_vertex!(Vertex, position);
    let mut vertices: Vec<Vertex> = Vec::new();

    #[derive(Copy, Clone)]
    pub struct Normal {
        normal: [f32; 3]
    }
    impl_vertex!(Normal, normal);
    let mut normals: Vec<Normal> = Vec::new();
    for input_vertex in input_obj.vertices {
        vertices.append(&mut vec!(Vertex {position: input_vertex.position}));
        normals.append(&mut vec!(Normal {normal: input_vertex.normal}));
    }

    let indices: Vec<u16> = input_obj.indices;

    let depth_buffer = vulkano::image::attachment::AttachmentImage::transient(&device, images[0].dimensions(), vulkano::format::D16Unorm).unwrap();

    let vertex_buffer = vulkano::buffer::cpu_access::CpuAccessibleBuffer
                                ::from_iter(&device, &vulkano::buffer::BufferUsage::all(), Some(queue.family()), vertices.iter().cloned())
                                .expect("failed to create buffer");

    let normals_buffer = vulkano::buffer::cpu_access::CpuAccessibleBuffer
                                ::from_iter(&device, &vulkano::buffer::BufferUsage::all(), Some(queue.family()), normals.iter().cloned())
                                .expect("failed to create buffer");

    let index_buffer = vulkano::buffer::cpu_access::CpuAccessibleBuffer
                                ::from_iter(&device, &vulkano::buffer::BufferUsage::all(), Some(queue.family()), indices.iter().cloned())
                                .expect("failed to create buffer");

    let proj = cgmath::perspective(cgmath::Rad(std::f32::consts::FRAC_PI_2), { let d = images[0].dimensions(); d[0] as f32 / d[1] as f32 }, 0.01, 100.0);
    let view = cgmath::Matrix4::look_at(cgmath::Point3::new(0.0, 0.0, 5.0), cgmath::Point3::new(0.0, 0.0, 0.0), cgmath::Vector3::new(0.0, 1.0, 0.0));

    let uniform_buffer = vulkano::buffer::cpu_access::CpuAccessibleBuffer::<vs::ty::Data>
                               ::from_data(&device, &vulkano::buffer::BufferUsage::all(), Some(queue.family()),
                                vs::ty::Data {
                                    world : <cgmath::Matrix4<f32> as cgmath::SquareMatrix>::identity().into(),
                                    view : (view).into(),
                                    proj : proj.into(),
                                })
                               .expect("failed to create buffer");

    let vs = vs::Shader::load(&device).expect("failed to create shader module");
    let fs = fs::Shader::load(&device).expect("failed to create shader module");

    mod renderpass {
        single_pass_renderpass!{
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: ::vulkano::format::Format,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: ::vulkano::format::D16Unorm,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {depth}
            }
        }
    }

    let renderpass = renderpass::CustomRenderPass::new(&device, &renderpass::Formats {
        color: (images[0].format(), 1),
        depth: (vulkano::format::D16Unorm, 1)
    }).unwrap();

    let descriptor_pool = vulkano::descriptor::descriptor_set::DescriptorPool::new(&device);

    mod pipeline_layout {
        pipeline_layout!{
            set0: {
                uniforms: UniformBuffer<::vs::ty::Data>
            }
        }
    }

    let pipeline_layout = pipeline_layout::CustomPipeline::new(&device).unwrap();
    let set = pipeline_layout::set0::Set::new(&descriptor_pool, &pipeline_layout, &pipeline_layout::set0::Descriptors {
        uniforms: &uniform_buffer
    });

    let pipeline = vulkano::pipeline::GraphicsPipeline::new(&device, vulkano::pipeline::GraphicsPipelineParams {
        vertex_input: vulkano::pipeline::vertex::TwoBuffersDefinition::new(),
        vertex_shader: vs.main_entry_point(),
        input_assembly: vulkano::pipeline::input_assembly::InputAssembly::triangle_list(),
        tessellation: None,
        geometry_shader: None,
        viewport: vulkano::pipeline::viewport::ViewportsState::Fixed {
            data: vec![(
                vulkano::pipeline::viewport::Viewport {
                    origin: [0.0, 0.0],
                    depth_range: 0.0 .. 1.0,
                    dimensions: [images[0].dimensions()[0] as f32, images[0].dimensions()[1] as f32],
                },
                vulkano::pipeline::viewport::Scissor::irrelevant()
            )],
        },
        raster: Default::default(),
        multisample: vulkano::pipeline::multisample::Multisample::disabled(),
        fragment_shader: fs.main_entry_point(),
        depth_stencil: vulkano::pipeline::depth_stencil::DepthStencil::simple_depth_test(),
        blend: vulkano::pipeline::blend::Blend::pass_through(),
        layout: &pipeline_layout,
        render_pass: vulkano::framebuffer::Subpass::from(&renderpass, 0).unwrap(),
    }).unwrap();

    let framebuffers = images.iter().map(|image| {
        let attachments = renderpass::AList {
            color: &image,
            depth: &depth_buffer,
        };

        vulkano::framebuffer::Framebuffer::new(&renderpass, [image.dimensions()[0], image.dimensions()[1], 1], attachments).unwrap()
    }).collect::<Vec<_>>();


    let command_buffers = framebuffers.iter().map(|framebuffer| {
        vulkano::command_buffer::PrimaryCommandBufferBuilder::new(&device, queue.family())
            .draw_inline(&renderpass, &framebuffer, renderpass::ClearValues {
                 color: [0.25, 0.25, 0.25, 1.0],
                 depth: 1.0,
             })
            .draw_indexed(&pipeline, (&vertex_buffer, &normals_buffer), &index_buffer,
                          &vulkano::command_buffer::DynamicState::none(), &set, &())
            .draw_end()
            .build()
    }).collect::<Vec<_>>();

    let mut submissions: Vec<std::sync::Arc<vulkano::command_buffer::Submission>> = Vec::new();


    loop {
        submissions.retain(|s| s.destroying_would_block());

        {
            // aquiring write lock for the uniform buffer
            let mut buffer_content = uniform_buffer.write(std::time::Duration::new(1, 0)).unwrap();
            if time::precise_time_ns() > last_nanosecond + 1000000000 {
                last_nanosecond = time::precise_time_ns();
                println!("{} FPS.", frame_number - last_frame_number);
                last_frame_number = frame_number;
            }
            frame_number += 1;

            let rotation = cgmath::Matrix3::from_angle_y(cgmath::Rad(time::precise_time_ns() as f32 * 0.000000001));

            // since write lock implementd Deref and DerefMut traits,
            // we can update content directly
            buffer_content.world = cgmath::Matrix4::from(rotation).into();
        }

        let image_num = swapchain.acquire_next_image(std::time::Duration::from_millis(1)).unwrap();
        submissions.push(vulkano::command_buffer::submit(&command_buffers[image_num], &queue).unwrap());
        swapchain.present(&queue, image_num).unwrap();

        for ev in window.window().poll_events() {
            match ev {
                winit::Event::Closed => return,
                _ => ()
            }
        }
    }
}
