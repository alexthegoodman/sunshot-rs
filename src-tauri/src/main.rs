// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ffmpeg_next::{
    software::scaling::{context::Context, flag::Flags},
    util::frame::video::Video,
};
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::thread;
use std::time;
use std::time::Instant;

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
    x: u32,
    y: u32,
    width: i32,
    height: i32,
    scale_factor: f64,
}

#[tauri::command]
fn transform_video(config_path: String) -> Result<String, String> {
    println!("Loading configuration...");

    // Load and parse the JSON configuration
    let config: Config = match fs::read_to_string(&config_path) {
        Ok(json_str) => serde_json::from_str(&json_str).map_err(|e| e.to_string())?,
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
                .map_err(|e| format!("Failed to parse window data JSON: {}", e))?
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

    let codec = ffmpeg::encoder::find_by_name("libx264").ok_or("Could not find libx264 encoder")?;

    let mut output_stream = output_context
        .add_stream(codec)
        .map_err(|e| format!("Failed to add output stream: {}", e))?;

    let mut encoder = ffmpeg::codec::context::Context::new()
        .encoder()
        .video()
        .map_err(|e| format!("Failed to create video encoder: {}", e))?;

    // output_stream.set_parameters(encoder.parameters());

    println!("Setting up codec context...");
    println!("Bit Rate: {}", decoder.bit_rate());

    let fps_int = 60; // Assuming 60 FPS, adjust as needed

    encoder.set_bit_rate(decoder.bit_rate());
    encoder.set_width(decoder.width());
    encoder.set_height(decoder.height());
    encoder.set_time_base((1, fps_int));
    encoder.set_frame_rate(Some(ffmpeg::Rational(fps_int, 1)));
    encoder.set_gop(10);
    encoder.set_max_b_frames(1);
    encoder.set_format(ffmpeg::util::format::Pixel::YUV420P);

    output_stream.set_time_base((1, fps_int));

    // Set encoder options
    // let mut dict = ffmpeg::Dictionary::new();
    // dict.set("preset", "medium");
    // dict.set("crf", "23");

    // encoder.set_options(&dict);

    // Open the encoder
    let mut encoder = encoder
        .open_as(codec)
        .map_err(|e| format!("Failed to open encoder: {}", e))?;

    // Copy encoder parameters to output stream
    // output_stream.set_parameters(encoder.parameters());

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
    let mut velocity_mouse_x = 0.0;
    let mut velocity_mouse_y = 0.0;
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

    let enable_dimension_smoothing = true;
    let enable_coord_smoothing = true;

    let mut frame_index = 0;
    let mut successful_frame_index = 0;

    // Get a packet iterator
    let mut packet_iter = input_context.packets();

    // Main loop
    'main_loop: loop {
        match packet_iter.next() {
            Some((stream, packet)) => {
                if stream.index() == video_stream_index {
                    // Process video packets
                    decoder
                        .send_packet(&packet)
                        .map_err(|e| format!("Error sending packet for decoding: {}", e))?;

                    'decode_loop: loop {
                        let mut decoded_frame = ffmpeg::frame::Video::empty();
                        match decoder.receive_frame(&mut decoded_frame) {
                            Ok(_) => {
                                // *** Frame transformation logic ***

                                // Send the transformed frame to the encoder
                                // encoder.send_frame(&bg_frame).map_err(|e| {
                                //     format!("Error sending frame for encoding: {}", e)
                                // })?;

                                // Create a new frame for the background
                                let mut bg_frame = ffmpeg::frame::Video::new(
                                    encoder.format(),
                                    encoder.width(),
                                    encoder.height(),
                                );
                                bg_frame.set_pts(Some(frame_index as i64));

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

                                // Color shift
                                let color_shift = 128.0 as f64;

                                // Fill the background frame with gradient
                                // TODO: double check f64 is correct
                                for y in 0..bg_frame.height() {
                                    for x in 0..bg_frame.width() {
                                        // Calculate normalized gradient position
                                        let gradient_position = x as f64 / bg_frame.width() as f64;

                                        // Calculate RGB color values
                                        let color_r = (start_color.0 as f64
                                            + gradient_position
                                                * (end_color.0 as f64 - start_color.0 as f64));
                                        let color_g = (start_color.1 as f64
                                            + gradient_position
                                                * (end_color.1 as f64 - start_color.1 as f64));
                                        let color_b = (start_color.2 as f64
                                            + gradient_position
                                                * (end_color.2 as f64 - start_color.2 as f64));

                                        // Convert RGB to YUV
                                        let color_y = calculate_y(color_r, color_g, color_b);
                                        let color_u =
                                            calculate_u(color_r, color_g, color_b) + color_shift;
                                        let color_v =
                                            calculate_v(color_r, color_g, color_b) + color_shift;

                                        // Fill Y plane
                                        let y_index = (y as usize * bg_frame.stride(0) as usize
                                            + x as usize)
                                            as usize;
                                        bg_frame.data_mut(0)[y_index] = color_y as u8;

                                        // Fill U and V planes
                                        if y % 2 == 0 && x % 2 == 0 {
                                            let uv_index = ((y / 2) as usize
                                                * bg_frame.stride(1) as usize
                                                + (x / 2) as usize)
                                                as usize;
                                            bg_frame.data_mut(1)[uv_index] = color_u as u8;
                                            bg_frame.data_mut(2)[uv_index] = color_v as u8;
                                        }
                                    }
                                }

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
                                let mut scaled_frame = Video::empty();
                                scaled_frame.set_format(decoded_frame.format());
                                scaled_frame.set_width(scaled_width);
                                scaled_frame.set_height(scaled_height);
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
                                let offset_x = (bg_frame.width() - scaled_frame.width()) / 2; // Center the video
                                let offset_y = (bg_frame.height() - scaled_frame.height()) / 2;

                                for y in 0..scaled_frame.height() {
                                    for x in 0..scaled_frame.width() {
                                        // Copy Y plane
                                        let bg_y_index = (y + offset_y) as usize
                                            * bg_frame.stride(0)
                                            + (x + offset_x) as usize;
                                        let scaled_y_index =
                                            y as usize * scaled_frame.stride(0) + x as usize;
                                        bg_frame.data_mut(0)[bg_y_index] =
                                            scaled_frame.data(0)[scaled_y_index];

                                        // Copy U and V planes
                                        if y % 2 == 0 && x % 2 == 0 {
                                            // U plane
                                            let bg_u_index = ((y + offset_y) / 2) as usize
                                                * bg_frame.stride(1)
                                                + ((x + offset_x) / 2) as usize;
                                            let scaled_u_index = (y / 2) as usize
                                                * scaled_frame.stride(1)
                                                + (x / 2) as usize;
                                            bg_frame.data_mut(1)[bg_u_index] =
                                                scaled_frame.data(1)[scaled_u_index];

                                            // V plane
                                            let bg_v_index = ((y + offset_y) / 2) as usize
                                                * bg_frame.stride(2)
                                                + ((x + offset_x) / 2) as usize;
                                            let scaled_v_index = (y / 2) as usize
                                                * scaled_frame.stride(2)
                                                + (x / 2) as usize;
                                            bg_frame.data_mut(2)[bg_v_index] =
                                                scaled_frame.data(2)[scaled_v_index];
                                        }
                                    }
                                }

                                // *** Zoom *** //
                                let time_elapsed = frame_index * 1000 / fps_int;

                                println!("Time Elapsed: {}", time_elapsed);

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
                                let mut target_multiplier = 1.0; // Default value when no zoom effect is active
                                let mut zooming_in = false;

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
                                            println!("Zooming In");
                                        }
                                        target_multiplier = zoom_factor;
                                        // Calculate the interpolation factor t based on the animation progress
                                        t = time_elapsed as f64
                                            / (start + animation_duration) as f64;
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
                                            println!("Zooming Out");
                                        }
                                        target_multiplier = 1.0;
                                    }
                                }

                                current_multiplier = target_multiplier;

                                // (ex. 1.0 is 100% while 0.8 is ~120%)
                                // println!("currentMultiplier {}", current_multiplier);

                                target_width = bg_frame.width() as f64 * current_multiplier;
                                target_height = bg_frame.height() as f64 * current_multiplier;

                                // These should be declared outside the loop and updated each iteration
                                let mut current_width = bg_frame.width() as f64;
                                let mut current_height = bg_frame.height() as f64;

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

                                // println!("Dimensions: {} {} {} {} {} {}",
                                //     target_width, target_height, current_width,
                                //     current_height, displacement_width, displacement_height);

                                velocity_width = velocity_width.clamp(-10000.0, 10000.0);
                                velocity_height = velocity_height.clamp(-10000.0, 10000.0);

                                let (used_width, used_height) = if enable_dimension_smoothing {
                                    let smoothing_factor1 = 0.02;
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

                                // Make sure the dimensions are integers and within the frame size.
                                let zoom_width =
                                    (used_width.round() as i32).clamp(1, bg_frame.width() as i32);
                                let zoom_height =
                                    (used_height.round() as i32).clamp(1, bg_frame.height() as i32);

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

                                            println!(
                                                "setting mouse {} {} {} {}",
                                                mouse_x,
                                                scale_multiple,
                                                current_width,
                                                window_data.x
                                            );

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
                                            break 'encode_loop;
                                        }
                                        Err(e) => {
                                            return Err(format!(
                                                "Error receiving encoded packet: {}",
                                                e
                                            ))
                                        }
                                    }
                                }
                            }
                            Err(ffmpeg::Error::Other {
                                errno: ffmpeg::error::EAGAIN,
                            }) => {
                                break 'decode_loop;
                            }
                            Err(e) => return Err(format!("Error receiving decoded frame: {}", e)),
                        }
                    }
                }
            }
            // Some(Err(e)) => return Err(format!("Error reading packet: {}", e)),
            None => break 'main_loop, // End of stream
        }

        // ... (progress update code)
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
