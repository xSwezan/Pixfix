use std::{
    env,
    fs::{read, write},
    io::stdin,
};

use zune_png::{
    zune_core::{
        bit_depth::BitDepth, colorspace::ColorSpace, options::EncoderOptions,
        result::DecodingResult,
    },
    PngDecoder, PngEncoder,
};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum BleedStage {
    Unprocessed,
    Staged,
    Processed,
}

const NEIGHBOR_LOCATIONS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
];

fn main() {
    let args: Vec<String> = env::args().collect();
    let png_paths: Vec<&String> = args
        .iter()
        .enumerate()
        .filter_map(|(index, path)| {
            if index as i32 != 0 && path.ends_with(".png") {
                return Some(path);
            }
            return None;
        })
        .collect();

    if png_paths.len() == 0 {
        println!("No PNG files where provided!");
        press_enter_to_close();
        return;
    }

    for path in png_paths.iter() {
        let (success, error) = fix_alpha_bleed(path.to_string());
        match success {
            true => {
                println!("Finished '{}'", path)
            }
            false => {
                println!("Could not fix '{}' \n\t→ ERROR: '{}'", path, error.unwrap());
            }
        }
    }

    println!("\nFinished!");
    press_enter_to_close();
}

fn press_enter_to_close() {
    println!("Press enter to close!");
    stdin().read_line(&mut String::new()).unwrap();
}

// fn find_closest_point(
//     triangulation: &Triangulation,
//     points: &Vec<Point>,
//     x: f64,
//     y: f64,
// ) -> Option<usize> {
//     let mut min_distance = f64::MAX;
//     let mut closest_point_index = None;

//     for i in 0..triangulation.triangles.len() / 3 {
//         let vertex_index = triangulation.triangles[i * 3];

//         let point = &points[vertex_index];
//         let distance = (point.x - x).powi(2) + (point.y - y).powi(2);

//         if distance < min_distance {
//             min_distance = distance;
//             closest_point_index = Some(vertex_index);
//         }
//     }
//     // for (index, point) in points.iter().enumerate() {
//     //     let distance = (point.x - x).powi(2) + (point.y - y).powi(2);

//     //     if distance < min_distance {
//     //         min_distance = distance;
//     //         closest_point_index = Some(index);
//     //     }
//     // }

//     closest_point_index
// }

fn neighbors(x: i32, y: i32, w: i32, h: i32) -> impl Iterator<Item = (i32, i32)> {
    NEIGHBOR_LOCATIONS
        .iter()
        .filter(move |(u, v)| {
            let x1 = x + *u as i32;
            let y1 = y + *v as i32;
            x1 > 0 && y1 > 0 && x1 < w && y1 < h
        })
        .map(move |(u, v)| (x + *u as i32, y + *v as i32))
}

fn fix_alpha_bleed(path: String) -> (bool, Option<String>) {
    let bytes = read(path.clone()).unwrap();
    let mut decoder = PngDecoder::new(bytes);

    let result = decoder.decode().unwrap();

    let color_space = decoder.get_colorspace().unwrap();
    dbg!(color_space);
    if color_space != ColorSpace::RGBA {
        return (
            false,
            Some(format!(
                "Wrong Color Space! Expected RGBA, got {:?}!",
                color_space
            )),
        );
    }

    match result {
        // DecodingResult::U8(mut value) => {
        //     let (width, height) = decoder.get_dimensions().unwrap();

        //     let value_clone = value.clone();
        //     let lookup_chunks = value_clone;

        //     let chunks = value.chunks_mut(4);

        //     let mut voronoi_points: Vec<Point> = Vec::new();
        //     let mut voronoi_colors: Vec<[u8; 3]> = Vec::new();

        //     let (mut x, mut y) = (-1i32, 0i32);
        //     for rgba in chunks {
        //         x += 1;
        //         if x > width as i32 {
        //             x = 1;
        //             y += 1;
        //         }

        //         if rgba[3] == 0 {
        //             continue;
        //         }

        //         for offset in NEIGHBOR_LOCATIONS.iter() {
        //             let lookup_x = x + offset[0];
        //             let lookup_y = y + offset[1];

        //             if lookup_x < 0
        //                 || lookup_x >= width as i32
        //                 || lookup_y < 0
        //                 || lookup_y >= height as i32
        //             {
        //                 continue;
        //             }

        //             let r = (lookup_x as usize + lookup_y as usize * width) * 4;
        //             let alpha = lookup_chunks[r + 3];

        //             if alpha != 0u8 {
        //                 continue;
        //             }

        //             // rgba[0] = 255;
        //             // rgba[1] = 255;
        //             // rgba[2] = 255;
        //             // rgba[3] = 255;

        //             voronoi_points.push(Point {
        //                 x: x as f64,
        //                 y: y as f64,
        //             });
        //             voronoi_colors.push([rgba[0], rgba[1], rgba[2]]);

        //             break;
        //         }

        //         // https://github.com/Corecii/Transparent-Pixel-Fix/blob/master/index.js
        //     }

        //     if voronoi_points.len() == 0 {
        //         return (
        //             false,
        //             Some("Could not find any fully transparent pixels!".into()),
        //         );
        //     }

        //     let result = triangulate(&voronoi_points);

        //     let chunks = value.chunks_mut(4);
        //     let (mut x, mut y) = (-1i32, 0i32);
        //     for rgba in chunks {
        //         x += 1;
        //         if x > width as i32 {
        //             x = 1;
        //             y += 1;
        //         }

        //         if rgba[3] != 0 {
        //             rgba[3] = 255; //-! DEBUG
        //             continue;
        //         }

        //         match find_closest_point(&result, &voronoi_points, x as f64, y as f64) {
        //             Some(index) => {
        //                 let closest_color = &voronoi_colors[index];
        //                 rgba[0] = closest_color[0];
        //                 rgba[1] = closest_color[1];
        //                 rgba[2] = closest_color[2];
        //                 rgba[3] = 255; //-! DEBUG
        //             }
        //             _ => {}
        //         }
        //     }

        //     let mut encoder = PngEncoder::new(
        //         &value,
        //         EncoderOptions::new(
        //             width,
        //             height,
        //             color_space,
        //             decoder.get_depth().unwrap_or(BitDepth::default()),
        //         ),
        //     );

        //     let _ = write(path, encoder.encode());
        // }
        DecodingResult::U8(mut value) => {
            let dimensions = decoder.get_dimensions().unwrap();
            let (width, height) = (dimensions.0 as i32, dimensions.1 as i32);

            let lookup = value.clone();

            // let chunks = value.chunks_mut(4);

            // let mut voronoi_points: Vec<Point> = Vec::new();
            // let mut voronoi_colors: Vec<[u8; 3]> = Vec::new();
            let mut queue0: Vec<(i32, i32)> = Vec::new();
            let mut queue1: Vec<(i32, i32)> = Vec::new();
            let mut stages: Vec<BleedStage> =
                vec![BleedStage::Unprocessed; (width * height) as usize];

            println!("Pre");
            for x in 0..width {
                for y in 0..height {
                    let pixel_index = (y * width + x) as usize;
                    let alpha = lookup[pixel_index * 4 + 3];
                    if alpha > 0 {
                        stages[pixel_index] = BleedStage::Processed;
                    }
                }
            }

            println!("Praweae");
            for x in 0..width {
                for y in 0..height {
                    let pixel_index = (y * width + x) as usize;
                    let stage = stages[pixel_index];
                    if stage != BleedStage::Processed {
                        continue;
                    }

                    for (this_x, this_y) in neighbors(x, y, width, height) {
                        if this_x < 0 || this_x >= width || this_y < 0 || this_y >= height {
                            continue;
                        }

                        let this_pixel_index = (this_y * width + this_x) as usize;
                        if stages[this_pixel_index] == BleedStage::Unprocessed {
                            queue0.push((this_x, this_y));
                            stages[this_pixel_index] = BleedStage::Staged;
                            break;
                        }
                    }
                }
            }

            println!("AWEOIJOAEIJWE");
            while !queue0.is_empty() {
                for (x, y) in queue0.iter() {
                    let (x, y) = (*x, *y);
                    let index = (y * width + x) as usize;

                    let mut c: u32 = 0;
                    let mut r: u32 = 0;
                    let mut g: u32 = 0;
                    let mut b: u32 = 0;

                    for (x1, y1) in neighbors(x, y, width, height) {
                        let index1 = (y1 * width + x1) as usize;
                        let stage = stages[index1];
                        if stage == BleedStage::Processed {
                            c += 1;
                            r += lookup[index1 * 4 + 0] as u32;
                            g += lookup[index1 * 4 + 1] as u32;
                            b += lookup[index1 * 4 + 2] as u32;
                        } else if stage == BleedStage::Unprocessed {
                            stages[index1] = BleedStage::Staged;
                            queue1.push((x1, y1));
                        }
                    }
                    if c > 0 {
                        r /= c;
                        g /= c;
                        b /= c;

                        value[index * 4 + 0] = r as u8;
                        value[index * 4 + 1] = g as u8;
                        value[index * 4 + 2] = b as u8;
                    }
                }
                // set pixels to processed
                for &(x, y) in queue0.iter() {
                    let index = (y * width + x) as usize;
                    stages[index] = BleedStage::Processed;
                }

                // clear and switch queue
                queue0.clear();
                std::mem::swap(&mut queue0, &mut queue1);
            }

            for x in 0..width {
                for y in 0..height {
                    let pixel_index = (y * width + x) as usize;
                    value[pixel_index * 4 + 3] = 255;
                }
            }

            let mut encoder = PngEncoder::new(
                &value,
                EncoderOptions::new(
                    width as usize,
                    height as usize,
                    color_space,
                    decoder.get_depth().unwrap_or(BitDepth::default()),
                ),
            );

            let _ = write(path, encoder.encode());
        }
        DecodingResult::U16(value) => {
            dbg!(value);
        }
        _ => {}
    };

    return (true, None);
}
