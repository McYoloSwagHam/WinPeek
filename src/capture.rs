use crate::WindowState;
use std::env::{temp_dir};
use std::fs::File;
use std::mem;
use std::mem::transmute as tcast;
use std::thread;
use captrs::*;
use std::sync::atomic::Ordering;
use std::time::Duration;
use winapi::shared::windef;
use winapi::um::winuser;
use image::codecs::gif as Igif;
use image::ImageBuffer;
use image::Rgba;
use std::io::Write;

const TIME_SLEEP_FRAME : f64 = 1.0/60.0;

// Start the recording here somehow
// and also start streaming to tmp file?
pub unsafe fn start_recording(hwnd : windef::HWND, window_state : &mut WindowState) {

    //Create a temp file to which we stream the video data
    //Then if the user decides to save, we move that file to the loc
    //otherwise we just delete the temporary file.
    let mut temp_path = temp_dir();

    let fake_hwnd : u64 = tcast::<windef::HWND, u64>(hwnd);

    temp_path.push("recording.buffer");

    let mut buffer_file = File::create(temp_path).unwrap();

    thread::spawn(move || {

        let mut view_area : windef::RECT = mem::zeroed();
        winuser::GetClientRect(tcast::<u64, windef::HWND>(fake_hwnd), &mut  view_area);

        //subtract the fake client area from the actual view area
        let frame_left = view_area.left + crate::LEFT_EXTEND;
        let frame_top = view_area.top + crate::TOP_EXTEND;

        let frame_width = (view_area.right - view_area.left - crate::LEFT_EXTEND - crate::RIGHT_EXTEND) as u32;
        let frame_height = (view_area.bottom - view_area.top - crate::TOP_EXTEND - crate::BOTTOM_EXTEND) as u32;

        view_area.left += crate::LEFT_EXTEND;
        view_area.top += crate::TOP_EXTEND;

        view_area.right -= crate::RIGHT_EXTEND;
        view_area.bottom -= crate::BOTTOM_EXTEND;

        let mut capturer = Capturer::new(0).unwrap();

        //Motherfucker retarded code in the example
        //the capture manager doesn't set it's own width and height
        //unless you capture a frame, unlike the fucking example
        //which tries to get the geometry without having captured 1 frame
        //this is stupid as fuck, send PR upstream
        let num_pixels = capturer.capture_frame().unwrap().len();
        let (cx, cy) = capturer.geometry();
        let cx = cx as usize;
        let cy = cy as usize;
        let rows = (num_pixels / cx) as u32;
        let cols = (num_pixels / cy) as u32;

        println!("rows {} cols {}", rows, cols);

        let mut encoder = Igif::GifEncoder::new(&mut buffer_file);
        //let mut pixels : Vec<u8> = Vec::with_capacity((cx * cy * 3) as usize);

        let mut frame_count = 0;
        let mut frames : Vec<image::Frame> = Vec::with_capacity(60);

        loop {

            //Atomic bool is true therefore we should stop.
            if crate::SHOULD_STOP.load(Ordering::Relaxed) {

                //Signal should main thread that all rendering operations have stopped
                drop(encoder);
                crate::SHOULD_STOP.store(false, Ordering::Relaxed);
                println!("we're here fucker");
                break;

            }

            let frame : Vec<Bgr8> = capturer.capture_frame().unwrap();
            println!("rect {} {} {} {}", view_area.left, view_area.top, view_area.right, view_area.bottom);
            let mut index = 0;
            let new_frame : Vec<u8> = frame.into_iter().filter_map(|pixel| {

                let offset_in_row = (index % cx) as i32;
                let num_row = (index / cx) as i32;
                index += 1;
                if (view_area.left <= offset_in_row && offset_in_row < view_area.right) && (view_area.top <= num_row && num_row < view_area.bottom) {
                    let arr : Vec<u8> = vec![pixel.r, pixel.g, pixel.b, pixel.a];
                    Some(arr)
                } else {
                    None
                }
            })
            .flatten()
            .collect();

            let image_buffer : ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_vec(frame_width, frame_height, new_frame).unwrap();
            let mut cur_frame = image::Frame::new(image_buffer)
            frames.push();

            if frame_count == 2 {
                encoder.encode_frames(frames.clone()).unwrap();
            }

            frame_count += 1;

        }


    });


}





