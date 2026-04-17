use glam::U8Vec3;
use image::imageops::FilterType;
use image::{Pixel, Rgb, Rgba};
use std::fs::File;
use std::path::PathBuf;
use std::time::Instant;
use utilities::distributed_mapped_image::TileMeta;
use utilities::sen2::{UserData, convert_sen2_img_to_epsg4326};

fn main() {
    let src_dir = PathBuf::from("../../test-data/sen2/conv/43TDF/src");
    let out_dir = PathBuf::from("../../test-data/sen2/conv/43TDF/expected");

    let src = image::open(src_dir.join("merged.png")).unwrap().to_rgba8();
    let ud: UserData =
        serde_json::from_reader(File::open(src_dir.join("userdata.json")).unwrap()).unwrap();

    let start = Instant::now();
    let (out, bounds) = convert_sen2_img_to_epsg4326(&src, &ud).unwrap();
    println!("Elapsed: {:?}", start.elapsed());

    let mut out =
        image::imageops::resize(&out, out.width() / 4, out.height() / 4, FilterType::Nearest);

    out.save(out_dir.join("merged-repr-updated.tiff")).unwrap();

    let expected = image::open(out_dir.join("merged-repr.tiff"))
        .unwrap()
        .to_rgb8();
    for (x, y, p) in out.enumerate_pixels_mut() {
        *p = Rgba::from(
            (U8Vec3::from_array(expected.get_pixel(x, y).0).as_vec3()
                - U8Vec3::from_array(p.to_rgb().0).as_vec3())
            .abs()
            .as_u8vec3()
            .extend(255)
            .to_array(),
        );
    }

    out.save(out_dir.join("merged-repr-diff.tiff")).unwrap();
    serde_json::to_writer_pretty(
        File::create(out_dir.join("merged-repr.tiff.meta.json")).unwrap(),
        &TileMeta { bounds },
    )
    .unwrap();
}
