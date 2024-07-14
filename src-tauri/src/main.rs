// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
    // Add fields as needed
}

#[derive(Deserialize, Serialize, Debug)]
struct BackgroundInfo {
    // Add fields as needed
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

    let mouse_events: Value = match File::open(&config.positions_file) {
        Ok(file) => {
            let reader = BufReader::new(file);
            serde_json::from_reader(reader)
                .map_err(|e| format!("Failed to parse mouse events JSON: {}", e))?
        }
        Err(e) => return Err(format!("Could not open mouse events file: {}", e)),
    };

    println!("Mouse events loaded successfully.");

    println!("Opening Window Data...");

    let window_data: Value = match File::open(&config.source_file) {
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
                                // Frame transformation logic
                                // ...

                                // Send the transformed frame to the encoder
                                // encoder.send_frame(&bg_frame).map_err(|e| {
                                //     format!("Error sending frame for encoding: {}", e)
                                // })?;

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
