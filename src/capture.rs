use crate::WindowState;
use std::env::{temp_dir};
use std::fs::File;
use std::mem;
use std::mem::transmute as tcast;
use std::thread;
use captrs::*;
use std::sync::{Arc, atomic::Ordering};
use std::time::Duration;
use winapi::shared::windef;
use winapi::um::winuser;
use scrap;
use mpeg_encoder;

const TIME_SLEEP_FRAME : f64 = 1.0/60.0;

// Start the recording here somehow
// and also start streaming to tmp file?
pub unsafe fn start_recording(hwnd : windef::HWND, window_state : &mut WindowState) {

    //Create a temp file to which we stream the video data
    //Then if the user decides to save, we move that file to the loc
    //otherwise we just delete the temporary file.
    let mut temp_path = temp_dir();

    let fake_hwnd : u64 = tcast::<windef::HWND, u64>(hwnd);

    temp_path.push("recording_buffer.mp4");

    thread::spawn(move || {

        let mut view_area : windef::RECT = mem::zeroed();
        winuser::GetWindowRect(tcast::<u64, windef::HWND>(fake_hwnd), &mut  view_area);

        //subtract the fake client area from the actual view area
        let frame_width = (view_area.right - view_area.left) as u32;
        let frame_height = (view_area.bottom - view_area.top) as u32;

        
        let mut pd = scrap::Display::primary().unwrap();
        let mut capt = scrap::Capturer::new(pd).unwrap();

        let cx = capt.width();
        let cy = capt.height();

        let mut frames : Vec<Vec<u8>> = Vec::with_capacity(2);

        let mut encoder = mpeg_encoder::Encoder::new(temp_path, cx, cy);

        loop {

            //println!("rect {} {} {} {}", view_area.left, view_area.top, view_area.right, view_area.bottom);
            let mut frame : Vec<u8> = loop {
                match capt.frame() {
                    Ok(frame) => { break frame.to_vec(); },
                    Err(_) => continue,
                }
            };

            println!("sanity3");

            for chunk in frame.chunks_exact_mut(4) {
                chunk.swap(0,2);
            }

            frames.push(frame);

            if frames.len() == 2 {
                println!("lmao {}", frames.len());
                for gif_frame in &frames {
                    encoder.encode_rgba(cx, cy, gif_frame, false);
                }

                frames.clear();

                //Atomic bool is true therefore we should stop.
                if crate::SHOULD_STOP.load(Ordering::Relaxed) {

                    drop(encoder);
                    crate::SHOULD_STOP.store(false, Ordering::Relaxed);
                    println!("we're here fucker");
                    break;

                }

            }


        }


    });


}





