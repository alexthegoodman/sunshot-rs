// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ffmpeg_next::format::Pixel;
use ffmpeg_next::software::scaling::{context::Context, flag::Flags};
use ffmpeg_next::util::frame::video::Video;
use ffmpeg_next::Dictionary;
use ffmpeg_next::Rescale;
use ffmpeg_next::{frame, Packet, Rational};
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;
use std::time::Instant;

use rayon::prelude::*;

// FFmpeg bindings
use ffmpeg_next as ffmpeg;

// Constants
const GRADIENT_SPEED: i32 = 2;
const ZOOM_FACTOR: f32 = 2.0;

fn spring_animation(
    target: f64,
    current: f64,
    velocity: f64,
    tension: f64,
    friction: f64,
    direction: f64,
) -> f64 {
    let spring_force = -(current - target);
    let damping_force = -velocity;
    let force = (spring_force * tension) + (damping_force * friction);

    let acceleration = force;
    let new_velocity = velocity + acceleration;
    let displacement = new_velocity;

    // println!("Spring Animation: {} {} {} {} {}", acceleration, new_velocity, displacement, current, target);

    if direction < 0.0 {
        if current < target {
            return 0.0;
        }
    } else if direction > 0.0 {
        if current > target {
            return 0.0;
        }
    }

    displacement
}

// based on air friction physics
fn frictional_animation(target: f64, current: f64, velocity: f64, friction: f64) -> f64 {
    let direction = target - current;
    let new_velocity = direction * (-friction).exp();
    new_velocity
}

fn calculate_y(r: f64, g: f64, b: f64) -> f64 {
    0.299 * r + 0.587 * g + 0.114 * b
}

fn calculate_u(r: f64, g: f64, b: f64) -> f64 {
    -0.16874 * r - 0.33126 * g + 0.5 * b
}

fn calculate_v(r: f64, g: f64, b: f64) -> f64 {
    0.5 * r - 0.41869 * g - 0.08131 * b
}

fn precalculate_gradient(
    width: usize,
    start_color: (f64, f64, f64),
    end_color: (f64, f64, f64),
) -> Vec<(f64, f64, f64)> {
    (0..width)
        .map(|x| {
            let gradient_position = x as f64 / (width - 1) as f64;
            let color_r = start_color.0 as f64
                + gradient_position * (end_color.0 as f64 - start_color.0 as f64);
            let color_g = start_color.1 as f64
                + gradient_position * (end_color.1 as f64 - start_color.1 as f64);
            let color_b = start_color.2 as f64
                + gradient_position * (end_color.2 as f64 - start_color.2 as f64);
            (color_r, color_g, color_b)
        })
        .collect()
}

// fn rgb_to_yuv(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
//     let y = (0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64) as u8;
//     let u = (128.0 - 0.168736 * r as f64 - 0.331264 * g as f64 + 0.5 * b as f64) as u8;
//     let v = (128.0 + 0.5 * r as f64 - 0.418688 * g as f64 - 0.081312 * b as f64) as u8;
//     (y, u, v)
// }

#[derive(Deserialize, Serialize, Debug)]
struct ZoomInfo {
    start: i32,
    end: i32,
    zoom: f64,
}

#[derive(Deserialize, Serialize, Debug)]
struct RgbField {
    r: f64,
    g: f64,
    b: f64,
}

#[derive(Deserialize, Serialize, Debug)]
struct BackgroundInfo {
    start: RgbField,
    end: RgbField,
}

#[derive(Deserialize, Serialize, Debug)]
struct Config {
    duration: i32,
    positions_file: String,
    source_file: String,
    input_file: String,
    output_file: String,
    zoom_info: Vec<ZoomInfo>,
    background_info: Vec<BackgroundInfo>,
}

#[derive(Deserialize, Serialize, Debug)]
struct MouseEvents {
    x: u32,
    y: u32,
    timestamp: i32,
}

#[derive(Deserialize, Serialize, Debug)]
struct SourceFile {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scale_factor: f64,
}

#[derive(Deserialize, Serialize, Clone)]
struct FramePixel {
    color_y: u8,
    color_u: u8,
    color_v: u8,
    x: usize,
    y: usize,
}

#[tauri::command]
fn transform_video(configPath: String) -> Result<String, String> {
    // let start1 = Instant::now();

    // for debugging purposes
    match env::current_dir() {
        Ok(path) => println!("Current directory is: {:?}", path),
        Err(e) => println!("Failed to get current directory: {}", e),
    }

    println!("Loading configuration... {}", configPath);

    // Load and parse the JSON configuration
    let config: Config = match fs::read_to_string(&configPath) {
        Ok(json_str) => serde_json::from_str(&json_str)
            .map_err(|e| e.to_string())
            .expect("Couldn't transform json string"),
        Err(e) => return Err(format!("Failed to read config file: {}", e)),
    };

    println!("Configuration loaded successfully.");
    println!("Duration: {}", config.duration);
    println!("Positions file: {}", config.positions_file);
    println!("Source file: {}", config.source_file);
    println!("Input file: {}", config.input_file);
    println!("Output file: {}", config.output_file);

    println!("Opening Mouse Events...");

    let mouse_events: Vec<MouseEvents> = match File::open(&config.positions_file) {
        Ok(file) => {
            let reader = BufReader::new(file);
            serde_json::from_reader(reader)
                .map_err(|e| format!("Failed to parse mouse events JSON: {}", e))?
        }
        Err(e) => return Err(format!("Could not open mouse events file: {}", e)),
    };

    println!("Mouse events loaded successfully.");

    println!("Opening Window Data...");

    let window_data: SourceFile = match File::open(&config.source_file) {
        Ok(file) => {
            let reader = BufReader::new(file);
            serde_json::from_reader(reader)
                .map_err(|e| format!("Failed to parse window data JSON: {}", e))
                .expect("Couldn't transform json data")
        }
        Err(e) => return Err(format!("Could not open window data file: {}", e)),
    };

    println!("Window data loaded successfully.");

    println!("Initializing FFmpeg...");

    // Initialize FFmpeg
    ffmpeg::init().map_err(|e| format!("Failed to initialize FFmpeg: {}", e))?;

    // *** decode video ***
    let input_filename = config.input_file;
    let output_filename = config.output_file;
    let fps_int = 60;

    println!("Opening input file...");

    let mut input_context = ffmpeg::format::input(&input_filename)
        .map_err(|e| format!("Could not open file: {}", e))?;

    println!("Finding Video Info...");

    // input_context.dump();

    println!("Finding Video Stream...");

    let video_stream = input_context
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or("No video stream found")?;

    let video_stream_index = video_stream.index();

    println!(
        "Video Stream found. Num Streams: {}, Num Frames: {}",
        input_context.streams().count(),
        video_stream.frames()
    );

    println!("Setting up decoder...");

    let mut decoder = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
        .map_err(|e| format!("Failed to create decoder context: {}", e))?
        .decoder()
        .video()
        .map_err(|e| format!("Failed to create video decoder: {}", e))?;

    // note: open() should be called automatically with from_paramaters()
    // decoder
    //     .open()
    //     .map_err(|e| format!("Failed to open decoder: {}", e))?;

    println!(
        "Found Decoder: {}",
        decoder
            .codec()
            .map(|c| c.name().to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    );
    println!("Decoder Pixel Format: {:?}", decoder.format());

    // *** prep encoding ***
    println!("Setting up encoder...");

    let mut output_context = ffmpeg::format::output(&output_filename)
        .map_err(|e| format!("Could not create output context: {}", e))?;

    let global_header = output_context
        .format()
        .flags()
        .contains(ffmpeg::format::Flags::GLOBAL_HEADER);

    // let codec = ffmpeg::encoder::find_by_name("libx264").ok_or("Could not find libx264 encoder")?;
    let codec =
        ffmpeg::encoder::find(ffmpeg::codec::Id::H264).ok_or("Could not find libx264 encoder")?;

    let mut output_stream = output_context
        .add_stream(codec)
        .map_err(|e| format!("Failed to add output stream: {}", e))?;

    // let mut encoder = ffmpeg::codec::context::Context::new()
    //     .encoder()
    //     .video()
    //     .map_err(|e| format!("Failed to create video encoder: {}", e))?;

    let mut encoder = ffmpeg::codec::context::Context::new_with_codec(codec)
        .encoder()
        .video()
        .map_err(|e| format!("Failed to create video encoder: {}", e))?;

    // output_stream.set_parameters(encoder.parameters());

    // encoder.set_codec(&codec);

    println!("Setting up codec context...");
    println!("Bit Rate: {}", decoder.bit_rate());
    println!("Codec name: {}", codec.name());

    let fps_int = 60; // Assuming 60 FPS, adjust as needed

    encoder.set_bit_rate(decoder.bit_rate());
    encoder.set_width(decoder.width());
    encoder.set_height(decoder.height());
    // encoder.set_time_base((1, fps_int));
    encoder.set_time_base(ffmpeg::Rational(1, fps_int));
    encoder.set_frame_rate(Some(ffmpeg::Rational(fps_int, 1)));
    encoder.set_gop(10);
    encoder.set_max_b_frames(1);
    encoder.set_format(ffmpeg::util::format::Pixel::YUV420P);
    // encoder.set_quality(50); // "-qscale is ignored, -crf is recommended."
    // encoder.set_compression(Some(23));

    println!("Continuing 1");

    // Create a Dictionary to hold the encoder parameters
    let mut parameters = Dictionary::new();
    parameters.set("preset", "faster");
    // parameters.set("x264-params", "level=4.0");
    // parameters.set("tune", "zerolatency"); // "good for streaming scenarios"??
    parameters.set("crf", "27");

    println!("Continuing 2");

    let mut encoder = encoder
        .open_with(parameters)
        .expect("Couldn't open encoder");

    println!("Continuing 3");

    output_stream.set_time_base((1, fps_int));

    // Copy encoder parameters to output stream
    // output_stream.set_parameters(encoder.parameters());
    output_stream.set_parameters(video_stream.parameters());

    // Open output file
    println!("Opening output file...");
    output_context
        .write_header()
        .map_err(|e| format!("Error occurred when opening output file: {}", e))?;

    let mut y = 0;
    let mut zoom = 1.0;

    println!("Starting read frames...");

    let mut current_multiplier = 1.0;
    let mut velocity = 0.0;

    let total_frames = (config.duration / 1000) * fps_int;
    let mut frame_index = 0;
    let mut successful_frame_index = 0;

    let start_time = Instant::now();

    let mut mouse_x = 0.0;
    let mut mouse_y = 0.0;
    let mut current_mouse_x = decoder.width() as f64 / 2.0;
    let mut current_mouse_y = decoder.height() as f64 / 2.0;
    let mut velocity_mouse_x: f64 = 0.0;
    let mut velocity_mouse_y: f64 = 0.0;
    let mut zoom_top = 0;
    let mut zoom_left = 0;
    let mut zooming_in = false;
    let mut zooming_in2 = false;
    let mut velocity_width = 0.0;
    let mut velocity_height = 0.0;
    let auto_zoom = false;

    let friction1 = 2.5;
    let friction2 = 3.0;
    let friction3 = 5.0;
    let easing_factor = 1.0;

    let mut direction_x = 0.0;
    let mut direction_y = 0.0;

    let mut prev_zoom_top = 0.0;
    let mut prev_zoom_left = 0.0;
    let mut smooth_zoom_top = 0.0;
    let mut smooth_zoom_left = 0.0;
    let mut used_zoom_top = 0.0;
    let mut used_zoom_left = 0.0;

    let mut smooth_width = 0.0;
    let mut smooth_height = 0.0;
    let mut used_width = 0.0;
    let mut used_height = 0.0;

    let animation_duration = 2000;

    let enable_dimension_smoothing = false;
    let enable_coord_smoothing = false;

    let mut frame_index = 0;
    let mut successful_frame_index = 0;

    let mut target_multiplier = 1.0; // Default value when no zoom effect is active
    let mut zooming_in = false;
    let mut zooming_out = false;

    // Get a packet iterator
    let mut packet_iter = input_context.packets();

    // Parameters for gradient colors
    let start_color = (
        config.background_info[0].start.r,
        config.background_info[0].start.g,
        config.background_info[0].start.b,
    );
    let end_color = (
        config.background_info[0].end.r,
        config.background_info[0].end.g,
        config.background_info[0].end.b,
    );

    let precalculated_gradient =
        precalculate_gradient(decoder.width() as usize, start_color, end_color);

    // Color shift
    let color_shift = 128.0 as f64;

    let precalculated_yuv: Vec<(u8, u8, u8)> = precalculated_gradient
        .iter()
        .map(|&(r, g, b)| {
            let y = calculate_y(r, g, b) as u8;
            let u = (calculate_u(r, g, b) + color_shift) as u8;
            let v = (calculate_v(r, g, b) + color_shift) as u8;
            (y, u, v)
        })
        .collect();

    let decoderHeight = decoder.height();
    let decoderWidth = decoder.width();

    let precalculated_data: Vec<FramePixel> = (0..decoderHeight)
        .into_par_iter() // Parallel iterator
        .flat_map(|y| {
            let precalculated_gradient = precalculated_gradient.clone();

            (0..decoderWidth).into_par_iter().map(move |x| {
                let (color_r, color_g, color_b) = precalculated_gradient[x as usize];
                let color_y = calculate_y(color_r, color_g, color_b);
                let color_u = calculate_u(color_r, color_g, color_b) + 128.0;
                let color_v = calculate_v(color_r, color_g, color_b) + 128.0;

                // Return the calculated data
                FramePixel {
                    color_y: color_y as u8,
                    color_u: color_u as u8,
                    color_v: color_v as u8,
                    x: x.try_into().unwrap(),
                    y: y.try_into().unwrap(),
                }
            })
        })
        .collect(); // Collect all the results

    // let duration1 = start1.elapsed();
    // println!(
    //     "start1 Time elapsed: {:?}, {:?}",
    //     duration1,
    //     precalculated_data.len()
    // );

    // Main loop
    'main_loop: loop {
        match packet_iter.next() {
            Some((stream, packet)) => {
                if stream.index() == video_stream_index {
                    // Process video packets
                    // let start2 = Instant::now();

                    decoder
                        .send_packet(&packet)
                        .map_err(|e| format!("Error sending packet for decoding: {}", e))?;

                    'decode_loop: loop {
                        let mut decoded_frame = ffmpeg::frame::Video::empty();
                        match decoder.receive_frame(&mut decoded_frame) {
                            Ok(_) => {
                                // *** Frame transformation logic ***

                                // Create a new frame for the background
                                let mut bg_frame = ffmpeg::frame::Video::new(
                                    encoder.format(),
                                    encoder.width(),
                                    encoder.height(),
                                );
                                bg_frame.set_pts(Some(frame_index as i64));

                                let precalculated_data = precalculated_data.clone();

                                // let start3 = Instant::now();

                                // Get the strides first
                                let y_stride = bg_frame.stride(0) as usize;
                                let uv_stride = bg_frame.stride(1) as usize;

                                // Process the frame plane by plane
                                {
                                    let y_plane = bg_frame.data_mut(0);
                                    for pixel in &precalculated_data {
                                        let y_index = pixel.y * y_stride + pixel.x;
                                        y_plane[y_index] = pixel.color_y;
                                    }
                                }

                                {
                                    let u_plane = bg_frame.data_mut(1);
                                    for pixel in precalculated_data
                                        .iter()
                                        .filter(|p| p.y % 2 == 0 && p.x % 2 == 0)
                                    {
                                        let uv_index = (pixel.y / 2) * uv_stride + (pixel.x / 2);
                                        u_plane[uv_index] = pixel.color_u;
                                    }
                                }

                                {
                                    let v_plane = bg_frame.data_mut(2);
                                    for pixel in precalculated_data
                                        .iter()
                                        .filter(|p| p.y % 2 == 0 && p.x % 2 == 0)
                                    {
                                        let uv_index = (pixel.y / 2) * uv_stride + (pixel.x / 2);
                                        v_plane[uv_index] = pixel.color_v;
                                    }
                                }

                                // let duration3 = start3.elapsed();
                                // println!("start3 Time elapsed: {:?}", duration3);

                                // let start10 = Instant::now();

                                // *** Inset Video *** //

                                // Scale down the frame using libswscale
                                let scale_multiple = 0.8;

                                let scaled_width =
                                    (decoded_frame.width() as f64 * scale_multiple) as u32;
                                let scaled_height =
                                    (decoded_frame.height() as f64 * scale_multiple) as u32;

                                // Create a new scaling context
                                let mut sws_context = Context::get(
                                    decoded_frame.format(),
                                    decoded_frame.width(),
                                    decoded_frame.height(),
                                    decoded_frame.format(),
                                    scaled_width,
                                    scaled_height,
                                    Flags::BILINEAR,
                                )
                                .map_err(|e| format!("Failed to create scaling context: {}", e))?;

                                // Create a new frame for the scaled video
                                // let mut scaled_frame = Video::empty();
                                // scaled_frame.set_format(decoded_frame.format());
                                // scaled_frame.set_width(scaled_width);
                                // scaled_frame.set_height(scaled_height);

                                let mut scaled_frame = frame::Video::new(
                                    Pixel::from(decoded_frame.format()),
                                    scaled_width,
                                    scaled_height,
                                );

                                // TODO: needed?
                                // scaled_frame.alloc_buffer().map_err(|e| {
                                //     format!("Failed to allocate buffer for scaled frame: {}", e)
                                // })?;

                                // Perform the scaling
                                sws_context
                                    .run(&decoded_frame, &mut scaled_frame)
                                    .map_err(|e| format!("Failed to scale frame: {}", e))?;

                                // Now `scaled_frame` contains the scaled-down version of the original frame

                                // Insert the scaled frame into the background frame
                                // let offset_x = (bg_frame.width() - scaled_frame.width()) / 2; // Center the video
                                // let offset_y = (bg_frame.height() - scaled_frame.height()) / 2;

                                // for y in 0..scaled_frame.height() {
                                //     for x in 0..scaled_frame.width() {
                                //         // Copy Y plane
                                //         let bg_y_index = (y + offset_y) as usize
                                //             * bg_frame.stride(0)
                                //             + (x + offset_x) as usize;
                                //         let scaled_y_index =
                                //             y as usize * scaled_frame.stride(0) + x as usize;
                                //         bg_frame.data_mut(0)[bg_y_index] =
                                //             scaled_frame.data(0)[scaled_y_index];

                                //         // Copy U and V planes
                                //         if y % 2 == 0 && x % 2 == 0 {
                                //             // U plane
                                //             let bg_u_index = ((y + offset_y) / 2) as usize
                                //                 * bg_frame.stride(1)
                                //                 + ((x + offset_x) / 2) as usize;
                                //             let scaled_u_index = (y / 2) as usize
                                //                 * scaled_frame.stride(1)
                                //                 + (x / 2) as usize;
                                //             bg_frame.data_mut(1)[bg_u_index] =
                                //                 scaled_frame.data(1)[scaled_u_index];

                                //             // V plane
                                //             let bg_v_index = ((y + offset_y) / 2) as usize
                                //                 * bg_frame.stride(2)
                                //                 + ((x + offset_x) / 2) as usize;
                                //             let scaled_v_index = (y / 2) as usize
                                //                 * scaled_frame.stride(2)
                                //                 + (x / 2) as usize;
                                //             bg_frame.data_mut(2)[bg_v_index] =
                                //                 scaled_frame.data(2)[scaled_v_index];
                                //         }
                                //     }
                                // }

                                let offset_x = (bg_frame.width() - scaled_frame.width()) / 2;
                                let offset_y = (bg_frame.height() - scaled_frame.height()) / 2;

                                let bg_stride = bg_frame.stride(0);

                                // Y plane
                                {
                                    let bg_y = bg_frame.data_mut(0);
                                    let scaled_y = scaled_frame.data(0);
                                    let scaled_stride = scaled_frame.stride(0);

                                    for y in 0..scaled_frame.height() {
                                        let bg_row_start =
                                            (y + offset_y) as usize * bg_stride + offset_x as usize;
                                        let scaled_row_start = y as usize * scaled_stride;
                                        let row_width = scaled_frame.width() as usize;

                                        bg_y[bg_row_start..bg_row_start + row_width]
                                            .copy_from_slice(
                                                &scaled_y[scaled_row_start
                                                    ..scaled_row_start + row_width],
                                            );
                                    }
                                }

                                let bg_stride = bg_frame.stride(1);

                                // U plane
                                {
                                    let bg_u = bg_frame.data_mut(1);
                                    let scaled_u = scaled_frame.data(1);
                                    let scaled_stride = scaled_frame.stride(1);

                                    for y in (0..scaled_frame.height()).step_by(2) {
                                        let bg_row_start = ((y + offset_y) / 2) as usize
                                            * bg_stride
                                            + (offset_x / 2) as usize;
                                        let scaled_row_start = (y / 2) as usize * scaled_stride;
                                        let row_width = (scaled_frame.width() / 2) as usize;

                                        bg_u[bg_row_start..bg_row_start + row_width]
                                            .copy_from_slice(
                                                &scaled_u[scaled_row_start
                                                    ..scaled_row_start + row_width],
                                            );
                                    }
                                }

                                let bg_stride = bg_frame.stride(2);

                                // V plane
                                {
                                    let bg_v = bg_frame.data_mut(2);
                                    let scaled_v = scaled_frame.data(2);
                                    let scaled_stride = scaled_frame.stride(2);

                                    for y in (0..scaled_frame.height()).step_by(2) {
                                        let bg_row_start = ((y + offset_y) / 2) as usize
                                            * bg_stride
                                            + (offset_x / 2) as usize;
                                        let scaled_row_start = (y / 2) as usize * scaled_stride;
                                        let row_width = (scaled_frame.width() / 2) as usize;

                                        bg_v[bg_row_start..bg_row_start + row_width]
                                            .copy_from_slice(
                                                &scaled_v[scaled_row_start
                                                    ..scaled_row_start + row_width],
                                            );
                                    }
                                }

                                // *** Zoom *** //
                                let time_elapsed = frame_index * 1000 / fps_int;

                                // let duration10 = start10.elapsed();
                                // println!("start10 Time elapsed: {:?}", duration10);

                                println!("Time Elapsed: {}", time_elapsed);

                                // let start4 = Instant::now();

                                // // Determine the portion of the background to zoom in on.
                                // // Start with the entire frame and gradually decrease these dimensions.
                                // static mut TARGET_WIDTH: f64 = 0.0;
                                // static mut TARGET_HEIGHT: f64 = 0.0;

                                // // Initialize the static variables if they haven't been set yet
                                // unsafe {
                                //     if TARGET_WIDTH == 0.0 {
                                //         TARGET_WIDTH = bg_frame.width() as f64;
                                //     }
                                //     if TARGET_HEIGHT == 0.0 {
                                //         TARGET_HEIGHT = bg_frame.height() as f64;
                                //     }
                                // }

                                let mut target_width = bg_frame.width() as f64;
                                let mut target_height = bg_frame.height() as f64;

                                // Search for the current zoom level
                                let mut t = 1.0;

                                for zoom in &config.zoom_info {
                                    let start = zoom.start as i32;
                                    let end = zoom.end as i32;
                                    let zoom_factor = zoom.zoom;

                                    // Process each zoom info...
                                    if time_elapsed >= start && time_elapsed < end {
                                        if !zooming_in {
                                            velocity = 0.0;
                                            velocity_mouse_x = 0.0;
                                            velocity_mouse_y = 0.0;
                                            velocity_width = 0.0;
                                            velocity_height = 0.0;
                                            zooming_in = true;
                                            zooming_out = false;
                                            println!("Zooming In");
                                            target_multiplier = zoom_factor;
                                        }

                                        // Calculate the interpolation factor t based on the animation progress
                                        // t = time_elapsed as f64
                                        //     / (start + animation_duration) as f64;
                                    } else if time_elapsed >= end
                                        && time_elapsed < end + animation_duration
                                    {
                                        if zooming_in {
                                            velocity = 0.0;
                                            velocity_mouse_x = 0.0;
                                            velocity_mouse_y = 0.0;
                                            velocity_width = 0.0;
                                            velocity_height = 0.0;
                                            zooming_in = false;
                                            zooming_out = true;
                                            println!("Zooming Out");
                                            target_multiplier = 1.0;
                                        }
                                    } else if (time_elapsed >= end + animation_duration) {
                                        if zooming_out {
                                            zooming_out = true;
                                        }
                                    }
                                }

                                current_multiplier = target_multiplier;

                                // (ex. 1.0 is 100% while 0.8 is ~120%)
                                // println!("currentMultiplier {}", current_multiplier);

                                target_width = bg_frame.width() as f64 * current_multiplier;
                                target_height = bg_frame.height() as f64 * current_multiplier;

                                // println!("target_width {}", target_width);

                                // These should be declared outside the loop and updated each iteration
                                let mut current_width = bg_frame.width() as f64;
                                let mut current_height = bg_frame.height() as f64;

                                if zooming_in || zooming_out {
                                    let displacement_width = frictional_animation(
                                        target_width,
                                        current_width,
                                        velocity_width,
                                        friction2,
                                    );
                                    let displacement_height = frictional_animation(
                                        target_height,
                                        current_height,
                                        velocity_height,
                                        friction2,
                                    );

                                    current_width += displacement_width;
                                    current_height += displacement_height;
                                    velocity_width += displacement_width;
                                    velocity_height += displacement_height;
                                }

                                // println!("zooming_in {}", zooming_in);
                                if zooming_in {
                                    // when zooming in, the target_width should be LESS than the current_width
                                    // want to prevent current_width from being less than target_width
                                    current_width = current_width.max(target_width);
                                    current_height = current_height.max(target_height);
                                } else {
                                    current_width = current_width.min(target_width);
                                    current_height = current_height.min(target_height);
                                }

                                // println!(
                                //     "Dimensions: {} {} {} {}",
                                //     target_width, target_height, current_width, current_height
                                // );

                                velocity_width = velocity_width.clamp(-10000.0, 10000.0);
                                velocity_height = velocity_height.clamp(-10000.0, 10000.0);

                                let (used_width, used_height) = if (enable_dimension_smoothing
                                    && (zooming_in || zooming_out))
                                {
                                    let smoothing_factor1 = 0.2;
                                    let (smooth_width, smooth_height) = if successful_frame_index
                                        == 0
                                    {
                                        (
                                            current_width
                                                + (smoothing_factor1 * current_width
                                                    + (1.0 - smoothing_factor1) * smooth_width),
                                            current_height
                                                + (smoothing_factor1 * current_height
                                                    + (1.0 - smoothing_factor1) * smooth_height),
                                        )
                                    } else {
                                        (
                                            smoothing_factor1 * current_width
                                                + (1.0 - smoothing_factor1) * smooth_width,
                                            smoothing_factor1 * current_height
                                                + (1.0 - smoothing_factor1) * smooth_height,
                                        )
                                    };

                                    // Ensure dimensions are within frame size
                                    let smooth_width =
                                        smooth_width.clamp(1.0, bg_frame.width() as f64);
                                    let smooth_height =
                                        smooth_height.clamp(1.0, bg_frame.height() as f64);

                                    // println!("Smooth Dimensions: {} x {} and {} x {}", smooth_width, smooth_height, target_width, target_height);

                                    (smooth_width, smooth_height)
                                } else {
                                    (current_width, current_height)
                                };

                                // println!("used: {}", used_width);

                                // Make sure the dimensions are integers and within the frame size.
                                let zoom_width =
                                    (used_width.round() as u32).clamp(1, bg_frame.width() as u32);
                                let zoom_height =
                                    (used_height.round() as u32).clamp(1, bg_frame.height() as u32);

                                for zoom in &config.zoom_info {
                                    let start = zoom.start;
                                    let end = zoom.end;
                                    let zoom_factor = zoom.zoom;

                                    // Process each zoom info...
                                    if time_elapsed >= start
                                        && time_elapsed < start + animation_duration
                                    {
                                        if !zooming_in2 {
                                            // Set mouse coords to the first mouse event after the start timestamp
                                            let mouse_event = mouse_events
                                                .iter()
                                                .find(|event| event.timestamp >= time_elapsed);

                                            if let Some(event) = mouse_event {
                                                mouse_x = event.x as f64;
                                                mouse_y = event.y as f64;
                                            }

                                            zooming_in2 = true;

                                            // println!(
                                            //     "setting mouse {} {} {} {}",
                                            //     mouse_x,
                                            //     scale_multiple,
                                            //     current_width,
                                            //     window_data.x
                                            // );

                                            // DPI scaling
                                            let scale_factor = window_data.scale_factor;

                                            mouse_x *= scale_factor;
                                            mouse_y *= scale_factor;

                                            // add windowOffset
                                            mouse_x -= window_data.x as f64;
                                            mouse_y -= window_data.y as f64;

                                            // scale mouse positions
                                            mouse_x = mouse_x * scale_multiple
                                                + bg_frame.width() as f64 * 0.1; // TODO: bg_frame or frame?
                                            mouse_y = mouse_y * scale_multiple
                                                + bg_frame.height() as f64 * 0.1;

                                            // println!("Mouse {} {}\n", mouse_x, mouse_y);

                                            if !auto_zoom {
                                                current_mouse_x = mouse_x;
                                                current_mouse_y = mouse_y;
                                            }

                                            direction_x = mouse_x - current_mouse_x;
                                            direction_y = mouse_y - current_mouse_y;
                                            println!("Zooming In 2");
                                        }
                                    } else if time_elapsed >= end
                                        && time_elapsed < end + animation_duration
                                    {
                                        if zooming_in2 {
                                            zooming_in2 = false;
                                            println!("Zooming Out 2");
                                        }
                                    }
                                }

                                // clamp max
                                let frame_width = bg_frame.width() as f64; // TODO: frame or bg_frame?
                                let frame_height = bg_frame.height() as f64;

                                // alternative clamp
                                current_mouse_x = current_mouse_x.clamp(0.0, frame_width);
                                current_mouse_y = current_mouse_y.clamp(0.0, frame_height);

                                velocity_mouse_x = velocity_mouse_x.clamp(-mouse_x, frame_width);
                                velocity_mouse_y = velocity_mouse_y.clamp(-mouse_y, frame_height);

                                // println!(
                                //     "Mouse Positions: {}, {} and {}, {}",
                                //     mouse_x, mouse_y, current_mouse_x, current_mouse_y
                                // );
                                // println!("Spring Position: {}, {}", current_mouse_x, current_mouse_y);
                                // println!("Smooth Info: {}, {}", smooth_height, smooth_width);

                                // Center the zoom on the current mouse position
                                let zoom_top = (current_mouse_y - zoom_height as f64 / 2.0)
                                    .clamp(0.0, bg_frame.height() as f64 - zoom_height as f64)
                                    .max(0.0) as u32;

                                let zoom_left = (current_mouse_x - zoom_width as f64 / 2.0)
                                    .clamp(0.0, bg_frame.width() as f64 - zoom_width as f64)
                                    .max(0.0)
                                    as u32;

                                // println!("Mid Info: {}, {}", zoom_top, zoom_left);

                                let target_zoom_top = (current_mouse_y - target_height as f64 / 2.0)
                                    .clamp(0.0, bg_frame.height() as f64 - target_height as f64)
                                    .max(0.0)
                                    as f64;

                                let target_zoom_left = (current_mouse_x - target_width as f64 / 2.0)
                                    .clamp(0.0, bg_frame.width() as f64 - target_width as f64)
                                    .max(0.0)
                                    as f64;

                                // max clamps
                                let zoom_top = if zoom_top + zoom_height > bg_frame.height() {
                                    bg_frame.height() - zoom_height
                                } else {
                                    zoom_top
                                };

                                let zoom_left = if zoom_left + zoom_width > bg_frame.width() {
                                    bg_frame.width() - zoom_width
                                } else {
                                    zoom_left
                                };

                                // clamp zoom_top and zoom_left
                                let zoom_top = zoom_top.min(bg_frame.height());
                                let zoom_left = zoom_left.min(bg_frame.width());

                                if enable_coord_smoothing {
                                    let prev_zoom_top = smooth_zoom_top;
                                    let prev_zoom_left = smooth_zoom_left;

                                    if frame_index == 0 {
                                        smooth_zoom_top = zoom_top as f64;
                                        smooth_zoom_left = zoom_left as f64;
                                    }

                                    let frame_proportion =
                                        bg_frame.height() as f64 / bg_frame.width() as f64;

                                    let smoothing_factor = 0.95; // Adjust this value to change the amount of smoothing (0-1)
                                    let top_change = (1.0 - smoothing_factor) * smooth_zoom_top;
                                    smooth_zoom_top = zoom_top as f64 + top_change;
                                    smooth_zoom_left =
                                        zoom_left as f64 + (top_change * frame_proportion);

                                    // println!("Smooth Info: {}, {}", smooth_zoom_top, smooth_zoom_left);

                                    // Ensure non-negative values
                                    smooth_zoom_top = smooth_zoom_top.max(0.0);
                                    smooth_zoom_left = smooth_zoom_left.max(0.0);

                                    // Round and ensure even numbers
                                    smooth_zoom_top = (smooth_zoom_top.round() / 2.0).floor() * 2.0;
                                    smooth_zoom_left =
                                        (smooth_zoom_left.round() / 2.0).floor() * 2.0;

                                    // Apply max clamps
                                    smooth_zoom_top = smooth_zoom_top
                                        .min(bg_frame.height() as f64 - zoom_height as f64);
                                    smooth_zoom_left = smooth_zoom_left
                                        .min(bg_frame.width() as f64 - zoom_width as f64);

                                    // Double-check even numbers (though this should be redundant now)
                                    smooth_zoom_top = (smooth_zoom_top / 2.0).floor() * 2.0;
                                    smooth_zoom_left = (smooth_zoom_left / 2.0).floor() * 2.0;

                                    used_zoom_top = smooth_zoom_top;
                                    used_zoom_left = smooth_zoom_left;
                                } else {
                                    // Ensure even numbers for non-smoothed zoom
                                    used_zoom_top = (zoom_top as f64 / 2.0).floor() * 2.0;
                                    used_zoom_left = (zoom_left as f64 / 2.0).floor() * 2.0;
                                }

                                // println!(
                                //     "Used Info: {}, {} and {}, {}",
                                //     used_zoom_top,
                                //     used_zoom_left,
                                //     target_zoom_top,
                                //     target_zoom_left
                                // );

                                // let duration4: time::Duration = start4.elapsed();
                                // println!("start4 Time elapsed: {:?}", duration4);

                                // let start5 = Instant::now();

                                // Create a new Video frame for the zoomed portion
                                let mut zoom_frame = frame::Video::new(
                                    Pixel::from(bg_frame.format()),
                                    bg_frame.width(),
                                    bg_frame.height(),
                                    // zoom_width,
                                    // zoom_height,
                                );

                                let scaled_pts =
                                    frame_index.rescale(encoder.time_base(), stream.time_base());
                                zoom_frame.set_pts(Some(scaled_pts));

                                // println!("Zoom Frame: {} x {}", zoom_width, zoom_height);
                                // println!("Diagnostic Info: {} x {}", zoom_frame.width(), zoom_frame.height());

                                // Create a scaling context for zooming
                                let mut sws_ctx_zoom = Context::get(
                                    bg_frame.format(),
                                    zoom_width,
                                    zoom_height,
                                    zoom_frame.format(),
                                    zoom_frame.width(),
                                    zoom_frame.height(),
                                    Flags::BILINEAR,
                                )
                                .expect("Failed to create scaling context");
                                // let mut sws_ctx_zoom = Context::get(
                                //     bg_frame.format(),
                                //     zoom_width,
                                //     zoom_height,
                                //     bg_frame.format(),
                                //     bg_frame.width(),
                                //     bg_frame.height(),
                                //     Flags::BILINEAR,
                                // )
                                // .expect("Failed to create scaling context");

                                // println!(
                                //     "widths: {}, {}, {}, {}, {}",
                                //     encoder.width(),
                                //     decoded_frame.width(),
                                //     zoom_frame.width(),
                                //     scaled_frame.width(),
                                //     zoom_width
                                // );

                                // Get pointers to the zoomed portion in the background frame
                                let used_zoom_top_int = used_zoom_top.round() as usize;
                                let used_zoom_left_int = used_zoom_left.round() as usize;
                                // let zoom_frame_height = zoom_frame.height() as usize;
                                // let zoom_frame_width = zoom_frame.width() as usize;
                                let zoom_frame_height = zoom_height as usize;
                                let zoom_frame_width = zoom_width as usize;

                                let zoom_frame_stride_0 = zoom_frame.stride(0) as usize;
                                let zoom_frame_stride_1 = zoom_frame.stride(1) as usize;

                                let used_zoom_bottom_int = used_zoom_top_int + zoom_frame_height;
                                let used_zoom_right_int = used_zoom_left_int + zoom_frame_width;
                                // let used_zoom_bottom_int = bg_frame.height() as usize
                                //     - zoom_frame_height
                                //     - used_zoom_top_int;
                                // let used_zoom_right_int = bg_frame.width() as usize
                                //     - zoom_frame_width
                                //     - used_zoom_left_int;

                                // println!(
                                //     "vars: {} {} {} {} {} {} {} {}",
                                //     used_zoom_top_int,
                                //     bg_frame.stride(0),
                                //     bg_frame.stride(1),
                                //     used_zoom_left_int,
                                //     zoom_frame_stride_0,
                                //     zoom_frame_height,
                                //     used_zoom_bottom_int,
                                //     used_zoom_right_int
                                // );

                                let mut zoom_data: [*const u8; 3] = [std::ptr::null(); 3];

                                let start_y =
                                    used_zoom_top_int * bg_frame.stride(0) + used_zoom_left_int;

                                zoom_data[0] = bg_frame.data(0)[start_y..].as_ptr();

                                if used_zoom_top_int % 2 == 0 && used_zoom_left_int % 2 == 0 {
                                    let start_u = (used_zoom_top_int / 2) * bg_frame.stride(1)
                                        + (used_zoom_left_int / 2);
                                    zoom_data[1] = bg_frame.data(1)[start_u..].as_ptr();

                                    let start_v = (used_zoom_top_int / 2) * bg_frame.stride(2)
                                        + (used_zoom_left_int / 2);
                                    zoom_data[2] = bg_frame.data(2)[start_v..].as_ptr();
                                } else {
                                    zoom_data[1] = std::ptr::null();
                                    zoom_data[2] = std::ptr::null();
                                }

                                // approach u
                                // Check zoom_data
                                for (i, &ptr) in zoom_data.iter().enumerate() {
                                    if ptr.is_null() {
                                        println!("zoom_data[{}] is null", i);
                                    }
                                }

                                // We can't directly get the length of a raw pointer, so we'll need to calculate it
                                let y_plane_size = (zoom_width * zoom_height) as usize;
                                // println!("Y plane size: {}", y_plane_size);

                                let mut data_frame = frame::Video::new(
                                    Pixel::from(bg_frame.format()),
                                    zoom_width,
                                    zoom_height,
                                );

                                data_frame.set_pts(Some(scaled_pts));

                                // Set the data from the zoom_data
                                unsafe {
                                    // Copy Y plane
                                    std::ptr::copy_nonoverlapping(
                                        zoom_data[0],
                                        data_frame.data_mut(0).as_mut_ptr(),
                                        y_plane_size,
                                    );

                                    // Check if U and V planes are available
                                    if !zoom_data[1].is_null() && !zoom_data[2].is_null() {
                                        let uv_plane_size = y_plane_size / 4; // Assuming 4:2:0 chroma subsampling
                                        std::ptr::copy_nonoverlapping(
                                            zoom_data[1],
                                            data_frame.data_mut(1).as_mut_ptr(),
                                            uv_plane_size,
                                        );
                                        std::ptr::copy_nonoverlapping(
                                            zoom_data[2],
                                            data_frame.data_mut(2).as_mut_ptr(),
                                            uv_plane_size,
                                        );
                                    }
                                }

                                // Continue with scaling logic...

                                // let mut final_frame = Video::empty();
                                // let mut final_frame = frame::Video::new(
                                //     Pixel::from(bg_frame.format()),
                                //     bg_frame.width(),
                                //     bg_frame.height(),
                                // );

                                // final_frame.set_pts(Some(scaled_pts));

                                // TODO: but zoom_frame needs to have a height of zoom_height to be passed to sws_scale
                                sws_ctx_zoom
                                    .run(&data_frame, &mut zoom_frame)
                                    .expect("Failed to scale (zoom) the frame");

                                // The scaling context will be automatically dropped when it goes out of scope

                                // We don't need to manually free the frames in Rust, as they will be automatically
                                // dropped when they go out of scope. The memory management is handled by Rust's
                                // ownership system.

                                // let duration5: time::Duration = start5.elapsed();
                                // println!("start5 Time elapsed: {:?}", duration5);

                                // Send the zoom_frame to the encoder
                                encoder.send_frame(&zoom_frame).map_err(|e| {
                                    format!("Error sending frame for encoding: {}", e)
                                })?;

                                // Receive and write encoded packets
                                'encode_loop: loop {
                                    let mut output_packet = ffmpeg::Packet::empty();
                                    match encoder.receive_packet(&mut output_packet) {
                                        Ok(_) => {
                                            output_packet.set_stream(0); // Set the stream index
                                            output_packet
                                                .write_interleaved(&mut output_context)
                                                .map_err(|e| {
                                                    format!("Error writing packet: {}", e)
                                                })?;
                                            successful_frame_index += 1;
                                        }
                                        Err(ffmpeg::Error::Other {
                                            errno: ffmpeg::error::EAGAIN,
                                        }) => {
                                            // No more packets to receive, break the loop
                                            break 'encode_loop;
                                        }
                                        Err(e) => {
                                            return Err(format!(
                                                "Error receiving encoded packet: {}",
                                                e
                                            ));
                                        }
                                    }
                                }

                                // The zoom_frame will be automatically dropped here when it goes out of scope
                            }
                            Err(ffmpeg::Error::Other {
                                errno: ffmpeg::error::EAGAIN,
                            }) => {
                                break 'decode_loop;
                            }
                            Err(e) => return Err(format!("Error receiving decoded frame: {}", e)),
                        }
                    }

                    // let duration2 = start2.elapsed();
                    // println!("start2 Time elapsed: {:?}", duration2);
                }
            }
            // Some(Err(e)) => return Err(format!("Error reading packet: {}", e)),
            None => break 'main_loop, // End of stream
        }

        // ... (progress update code)
        frame_index += 1;
    }

    // After the main loop
    output_context
        .write_trailer()
        .map_err(|e| format!("Error occurred when writing trailer: {}", e))?;

    Ok("Video transformation completed successfully".to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![transform_video])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
