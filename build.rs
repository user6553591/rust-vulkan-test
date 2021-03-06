extern crate vulkano_shaders;

fn main() {
    // building the shaders used in the examples
    vulkano_shaders::build_glsl_shaders([
        ("assets/build/shaders/vs.glsl", vulkano_shaders::ShaderType::Vertex),
        ("assets/build/shaders/fs.glsl", vulkano_shaders::ShaderType::Fragment),
    ].iter().cloned());
}
