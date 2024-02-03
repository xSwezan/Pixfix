use std::{
    io::stdin,
    path::{Path, PathBuf},
};

use tokio::task::JoinSet;

use image::{GenericImage, GenericImageView, Rgba};
use spade::{DelaunayTriangulation, Point2, Triangulation};

static EXTENSIONS: &[&'static str] = &["png"];
static NEIGHBORS: &[(i32, i32)] = &[
    (-1, -1),
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
];

struct VoronoiColor {
    r: u8,
    g: u8,
    b: u8,
}

fn valid_extension(path: &Path) -> bool {
    let extension = match path.extension() {
        Some(str) => str.to_str().unwrap(),
        None => "",
    };

    EXTENSIONS.contains(&extension)
}

fn resolve_files(args: Vec<String>) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for arg in args {
        let path = Path::new(&arg);

        let metadata = match std::fs::metadata(path) {
            Ok(data) => data,
            Err(_) => {
                println!("Ignoring image: \"{}\" as it does not exist", arg);
                continue;
            }
        };

        if metadata.is_file() {
            if !valid_extension(path) {
                println!(
                    "Ignoring image: \"{}\" as only {} are accepted!",
                    arg,
                    EXTENSIONS.join("|")
                );
                continue;
            }

            files.push(path.to_path_buf());
        }

        if !metadata.is_dir() {
            continue;
        }

        let dir = match std::fs::read_dir(arg.clone()) {
            Ok(data) => data,
            Err(_) => {
                println!(
                    "Ignoring directory: \"{}\" as an error occured reading directory",
                    arg
                );

                continue;
            }
        };

        for path in dir {
            if path.is_err() {
                continue;
            }

            let path: std::path::PathBuf = path.unwrap().path();
            let metadata = match std::fs::metadata(path.clone()) {
                Ok(data) => data,
                Err(_) => {
                    println!(
                        "Ignoring file: \"{}\" as an error occured reading file metadata",
                        path.display()
                    );

                    continue;
                }
            };

            if !metadata.is_file() {
                continue;
            }

            if !valid_extension(&path) {
                println!(
                    "Ignoring image: \"{}\" as only {} are accepted!",
                    path.display(),
                    EXTENSIONS.join("|")
                );
                continue;
            }

            files.push(path)
        }
    }

    files
}

fn convert_image(path: &Path, debug: bool) {
    let mut img = match image::open(path) {
        Ok(value) => value,
        Err(err) => {
            println!(
                "Error occured opening image \"{}\":\n{:?}",
                path.display(),
                err
            );
            return;
        }
    };

    let mut points: Vec<Point2<f64>> = Vec::new();
    let mut colors: Vec<VoronoiColor> = Vec::new();
    // let mut point_positions: Vec<(u32, u32)> = Vec::new();
    let mut transparent_pixels: Vec<(u32, u32, Rgba<u8>)> = Vec::new();
    let mut position_to_index: std::collections::HashMap<(u32, u32), usize> = Default::default();

    let (width, height) = img.dimensions();

    for (x, y, color) in img.pixels() {
        let rgba = color.0;

        let a = rgba[3];

        if a == 0 {
            transparent_pixels.push((x, y, color));
            continue;
        }

        let r = rgba[0];
        let g = rgba[1];
        let b = rgba[2];

        for (nx, ny) in NEIGHBORS {
            let neighbor_x = x as i32 + nx;
            let neighbor_y = y as i32 + ny;

            if neighbor_x as u32 >= width || neighbor_y < 0 {
                continue;
            }

            if neighbor_y as u32 >= height || neighbor_y < 0 {
                continue;
            }

            let neighbor_rgba = img.get_pixel(neighbor_x as u32, neighbor_y as u32).0;

            if neighbor_rgba[3] != 0 {
                continue;
            }

            // point_positions.push((x, y));
            position_to_index.insert((x, y), points.len());
            points.push(Point2::new(x as f64, y as f64));
            colors.push(VoronoiColor { r, g, b });

            break;
        }
    }

    // for (index, point) in points.iter().enumerate() {
    //     let color = &colors[index];
    //     img.put_pixel(
    //         point.x as u32,
    //         point.y as u32,
    //         Rgba::<u8> {
    //             0: [color.r, color.g, color.b, 255],
    //         },
    //     );
    // }

    if points.len() == 0 {
        println!("No transparent pixels to fix: {:?}", path);
        return;
    }

    let triangulation: DelaunayTriangulation<Point2<f64>> =
        Triangulation::bulk_load(points).unwrap();

    for (x, y, color) in transparent_pixels.iter() {
        let rgba = color.0;
        let mut a = rgba[3];

        let closest_neighbor =
            match &triangulation.nearest_neighbor(Point2::new(*x as f64, *y as f64)) {
                Some(value) => value.clone(),
                None => continue,
            };

        // let closest_index = closest_neighbor.index();
        // if closest_index == 0 {
        //     continue;
        // }

        let closest_position = closest_neighbor.position();

        // if let Some((closest_index, _)) =
        //     point_positions.iter().enumerate().find(|(_, (x1, y1))| {
        //         *x1 == closest_position.x as u32 && *y1 == closest_position.y as u32
        //     })
        if let Some(closest_index) =
            position_to_index.get(&(closest_position.x as u32, closest_position.y as u32))
        {
            let closest_color = &colors[*closest_index];

            if debug {
                a = 255;
            }

            img.put_pixel(
                *x,
                *y,
                Rgba::<u8> {
                    0: [closest_color.r, closest_color.g, closest_color.b, a],
                },
            )
        }
    }

    img.save(path).expect("Unable to save image");
    println!("{:?}", path.display())
}

#[tokio::main]
async fn main() {
    let mut args: Vec<_> = std::env::args().collect();
    let mut debug = false;

    args.remove(0);

    for index in 0..args.len() {
        let arg = &args[index];
        if arg == &"-d".to_owned() {
            debug = true;
            args.remove(index);
            break;
        }
    }

    if args.len() == 0 {
        println!("You need to drop pngs on the exe!");
    } else {
        let mut threads = JoinSet::new();

        for path in resolve_files(args) {
            threads.spawn_blocking(move || convert_image(path.as_path(), debug));
        }

        loop {
            if let None = threads.join_next().await {
                break;
            }
        }
    }

    println!("\npress enter to exit");

    stdin().read_line(&mut String::new()).unwrap();
}
