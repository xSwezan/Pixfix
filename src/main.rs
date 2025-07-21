use std::{
    collections::HashMap,
    io::stdin,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU16, Ordering},
    },
    time::Instant,
};

use image::Rgba;
use spade::{DelaunayTriangulation, Point2, Triangulation};
use tokio::task::JoinSet;

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

#[derive(Clone, Copy)]
struct VoronoiColor {
    r: u8,
    g: u8,
    b: u8,
}

fn is_png_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map_or(false, |ext| ext.eq_ignore_ascii_case("png"))
}

fn resolve_files(args: Vec<String>) -> (Vec<PathBuf>, u16) {
    let mut files = Vec::new();
    let mut all_files: u16 = 0;

    for arg in args {
        let path = Path::new(&arg);

        let metadata = match std::fs::metadata(path) {
            Ok(data) => data,
            Err(_) => {
                println!("Ignoring \"{}\" - It does not exist!", arg);
                continue;
            }
        };

        all_files += 1;

        if metadata.is_file() {
            if !is_png_file(path) {
                println!("Ignoring \"{}\" - Only PNG files are accepted!", arg);
                continue;
            }
            files.push(path.to_path_buf());
            continue;
        }

        if !metadata.is_dir() {
            continue;
        }

        let dir = match std::fs::read_dir(&arg) {
            Ok(data) => data,
            Err(_) => {
                println!(
                    "Ignoring \"{}\" - An error occurred reading directory!",
                    arg
                );
                continue;
            }
        };

        all_files -= 1;

        for entry in dir.flatten() {
            let path = entry.path();

            if let Ok(metadata) = std::fs::metadata(&path) {
                if metadata.is_file() {
                    all_files += 1;

                    if is_png_file(&path) {
                        files.push(path);
                    } else {
                        println!(
                            "Ignoring \"{}\" - Only PNG files are accepted!",
                            path.display()
                        );
                    }
                }
            }
        }
    }

    (files, all_files)
}

fn convert_image(path: &Path, debug: bool) -> bool {
    let img = match image::open(path) {
        Ok(value) => value,
        Err(err) => {
            println!(
                "Error occurred opening image \"{}\":\n{:?}",
                path.display(),
                err
            );
            return false;
        }
    };

    let mut rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();

    // Pre-allocate with estimated capacity
    let estimated_border_pixels = ((width + height) * 4) as usize;
    let mut points = Vec::with_capacity(estimated_border_pixels);
    let mut colors = Vec::with_capacity(estimated_border_pixels);
    let mut transparent_pixels = Vec::new();
    let mut position_to_index = HashMap::with_capacity(estimated_border_pixels);

    // Single pass to find border pixels and collect transparent pixels
    let pixels = rgba_img.as_raw();
    let stride = width as usize * 4;

    for y in 0..height {
        for x in 0..width {
            let idx = y as usize * stride + x as usize * 4;
            let a = pixels[idx + 3];

            if a == 0 {
                transparent_pixels.push((x, y));
                continue;
            }

            // Check if this pixel is adjacent to a transparent pixel
            let is_border = NEIGHBORS.iter().any(|&(nx, ny)| {
                let neighbor_x = x as i32 + nx;
                let neighbor_y = y as i32 + ny;

                if neighbor_x < 0
                    || neighbor_y < 0
                    || neighbor_x >= width as i32
                    || neighbor_y >= height as i32
                {
                    return false;
                }

                let neighbor_idx = neighbor_y as usize * stride + neighbor_x as usize * 4;
                pixels[neighbor_idx + 3] == 0
            });

            if is_border {
                position_to_index.insert((x, y), points.len());
                points.push(Point2::new(x as f64, y as f64));
                colors.push(VoronoiColor {
                    r: pixels[idx],
                    g: pixels[idx + 1],
                    b: pixels[idx + 2],
                });
            }
        }
    }

    if points.is_empty() {
        println!("No transparent pixels to fix: {:?}", path);
        return false;
    }

    let triangulation: DelaunayTriangulation<Point2<f64>> = match Triangulation::bulk_load(points) {
        Ok(tri) => tri,
        Err(_) => {
            println!("Failed to create triangulation for: {:?}", path);
            return false;
        }
    };

    // Process transparent pixels
    for &(x, y) in &transparent_pixels {
        if let Some(closest_neighbor) =
            triangulation.nearest_neighbor(Point2::new(x as f64, y as f64))
        {
            let closest_position = closest_neighbor.position();

            if let Some(&closest_index) =
                position_to_index.get(&(closest_position.x as u32, closest_position.y as u32))
            {
                let closest_color = colors[closest_index];
                let a = if debug { 255 } else { 0 };

                rgba_img.put_pixel(
                    x,
                    y,
                    Rgba([closest_color.r, closest_color.g, closest_color.b, a]),
                );
            }
        }
    }

    match rgba_img.save(path) {
        Ok(_) => true,
        Err(err) => {
            println!("Failed to save image \"{}\": {:?}", path.display(), err);
            false
        }
    }
}

fn draw_watermark() {
    println!(
        "   ____ _____  _______ _____  __
  |  _ \\_ _\\ \\/ /  ___|_ _\\ \\/ /
  | |_) | | \\  /| |_   | | \\  /
  |  __/| | /  \\|  _|  | | /  \\
  |_|  |___/_/\\_\\_|   |___/_/\\_\\\n"
    );
}

#[tokio::main]
async fn main() {
    let mut args: Vec<_> = std::env::args().collect();
    let mut debug = false;

    args.remove(0);

    if let Some(pos) = args.iter().position(|x| x == "-d") {
        debug = true;
        args.remove(pos);
    }

    let start = Instant::now();
    let files_fixed: Arc<AtomicU16> = Arc::new(AtomicU16::new(0));
    let files_failed: Arc<AtomicU16> = Arc::new(AtomicU16::new(0));

    draw_watermark();

    if args.is_empty() {
        println!("Drop png files on the exe to fix them!");
    } else {
        println!("Processing your files, please wait!");

        let mut threads = JoinSet::new();

        let (files, all_files) = resolve_files(args);
        let num_failed = all_files - files.len() as u16;
        files_failed.fetch_add(num_failed, Ordering::Relaxed);

        for path in files {
            let files_fixed_thread = files_fixed.clone();
            let files_failed_thread = files_failed.clone();

            threads.spawn_blocking(move || {
                let converted = convert_image(&path, debug);
                if converted {
                    files_fixed_thread.fetch_add(1, Ordering::Relaxed);
                } else {
                    files_failed_thread.fetch_add(1, Ordering::Relaxed);
                }
            });
        }

        while threads.join_next().await.is_some() {}
    }

    let time_taken = Instant::now()
        .saturating_duration_since(start)
        .as_secs_f32();

    println!();

    let fixed_count = files_fixed.load(Ordering::Relaxed);
    let failed_count = files_failed.load(Ordering::Relaxed);

    if fixed_count > 0 {
        println!(
            "Successfully fixed {} images in {:.4} seconds!",
            fixed_count, time_taken
        );
    } else {
        println!("No files were able to be fixed!");
    }

    if failed_count > 0 {
        println!("Skipped {} files that couldn't be fixed!", failed_count);
    }

    println!("\nPress enter to exit");
    let _ = stdin().read_line(&mut String::new());
}
