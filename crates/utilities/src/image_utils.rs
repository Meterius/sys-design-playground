use glam::UVec2;
use image::GenericImageView;

pub fn copy_from_sub_image(img: &image::RgbImage, dest: &mut image::RgbImage, img_area: (UVec2, UVec2), dest_offset: UVec2) {
    for x in img_area.0.x..=img_area.1.x {
        for y in img_area.0.y..=img_area.1.y {
            dest.put_pixel(x - img_area.0.x + dest_offset.x, y - img_area.0.y + dest_offset.y, *img.get_pixel(x, y));
        }
    }
}