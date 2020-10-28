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
use gif;
use std::io::Write;
use scrap;

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
        winuser::GetWindowRect(tcast::<u64, windef::HWND>(fake_hwnd), &mut  view_area);

        //subtract the fake client area from the actual view area
        let frame_width = (view_area.right - view_area.left) as u32;
        let frame_height = (view_area.bottom - view_area.top) as u32;

        
        //view_area.left += crate::LEFT_EXTEND;
        //view_area.top += crate::TOP_EXTEND;

        //view_area.right -= crate::RIGHT_EXTEND;
        //view_area.bottom -= crate::BOTTOM_EXTEND;

        //let mut capturer = Capturer::new(0).unwrap();
        //let frame_other : Vec<Bgr8> = capturer.capture_frame().unwrap();
        //println!("frame_other {:?}", frame_other.get(20).unwrap());
        let mut pd = scrap::Display::primary().unwrap();
        let mut capt = scrap::Capturer::new(pd).unwrap();

        let cx = capt.width() as u16;
        let cy = capt.height() as u16;
        //let rows = (num_pixels / cx) as u32;
        //let cols = (num_pixels / cy) as u32;

        println!("cx {} cy {}", cx, cy);

        let mut encoder = gif::Encoder::new(&mut buffer_file, cx, cy, &[]).unwrap();
        //let mut encoder = Igif::GifEncoder::new(&mut buffer_file);
        //let mut pixels : Vec<u8> = Vec::with_capacity((cx * cy * 3) as usize);

        let mut frame_count = 0;
        gif::Encoder::set_repeat(&mut encoder, gif::Repeat::Infinite).unwrap();
        let mut frames : Vec<gif::Frame> = Vec::with_capacity(2);

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

            let mut cur_frame = gif::Frame::from_rgba_speed(cx as u16, cy as u16, &mut frame, 30); 
            cur_frame.dispose = gif::DisposalMethod::Any;

            frames.push(cur_frame);
            frame_count += 1;

            if frame_count == 2 {
                println!("lmao {}", frames.len());
                for gif_frame in &mut frames {
                    gif_frame.delay = 50;
                    encoder.write_frame(gif_frame).unwrap();
                }
                frame_count = 0;
                frames.clear();

                //Atomic bool is true therefore we should stop.
                if crate::SHOULD_STOP.load(Ordering::Relaxed) {

                    crate::SHOULD_STOP.store(false, Ordering::Relaxed);
                    println!("we're here fucker");
                    break;

                }

            }


        }


    });


}





