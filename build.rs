use std::fs;
use std::path::Path;

fn main() {
    let shader_dir = Path::new("src/renderer/shaders");
    let out_dir = std::env::var("OUT_DIR").unwrap();

    let shaders = [
        ("chunk.wgsl", &[("vs_main", naga::ShaderStage::Vertex), ("fs_main", naga::ShaderStage::Fragment)][..]),
        ("panorama.wgsl", &[("vs_main", naga::ShaderStage::Vertex), ("fs_main", naga::ShaderStage::Fragment)][..]),
    ];

    for (file, entries) in &shaders {
        let path = shader_dir.join(file);
        println!("cargo:rerun-if-changed={}", path.display());

        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|e| panic!("failed to parse {file}: {e}"));

        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("validation failed for {file}: {e}"));

        for &(entry_name, stage) in *entries {
            let extension = match stage {
                naga::ShaderStage::Vertex => "vert",
                naga::ShaderStage::Fragment => "frag",
                naga::ShaderStage::Compute => "comp",
            };
            let stem = file.strip_suffix(".wgsl").unwrap();
            let out_name = format!("{stem}.{extension}.spv");

            let pipeline = naga::back::spv::PipelineOptions {
                shader_stage: stage,
                entry_point: entry_name.to_string(),
            };

            let spv = naga::back::spv::write_vec(
                &module,
                &info,
                &naga::back::spv::Options {
                    lang_version: (1, 0),
                    ..Default::default()
                },
                Some(&pipeline),
            )
            .unwrap_or_else(|e| panic!("failed to generate SPIR-V for {file}/{entry_name}: {e}"));

            let bytes: Vec<u8> = spv.iter().flat_map(|w| w.to_le_bytes()).collect();
            let out_path = Path::new(&out_dir).join(&out_name);
            fs::write(&out_path, &bytes)
                .unwrap_or_else(|e| panic!("failed to write {}: {e}", out_path.display()));
        }
    }
}
