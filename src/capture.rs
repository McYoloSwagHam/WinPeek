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
use gif;

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

        let frame_width = view_area.right - view_area.left - crate::LEFT_EXTEND - crate::RIGHT_EXTEND;
        let frame_height = view_area.bottom - view_area.top - crate::TOP_EXTEND - crate::BOTTOM_EXTEND;

        let mut capturer = Capturer::new(0).unwrap();

        //Motherfucker retarded code in the example
        //the capture manager doesn't set it's own width and height
        //unless you capture a frame, unlike the fucking example
        //which tries to get the geometry without having captured 1 frame
        //this is stupid as fuck, send PR upstream
        capturer.capture_frame().unwrap();

        let (cx, cy) = capturer.geometry();

        let mut encoder = gif::Encoder::new(&mut buffer_file, frame_width as u16, frame_height as u16, &[]).unwrap();
        //encoder.set_repeat(gif::Repeat::Infinite).unwrap();

        loop {

            //Atomic bool is true therefore we should stop.
            if crate::SHOULD_STOP.load(Ordering::Relaxed) {

                //Signal should main thread that all rendering operations have stopped
                crate::SHOULD_STOP.store(false, Ordering::Relaxed);
                println!("we're here fucker");
                break;

            }

            let frame = capturer.capture_frame().unwrap();

            //reorder BGRA8 to RGBA and also flatten into Vec<u8>
            let mut pixels = Vec::with_capacity((cx * cy * 4) as usize);

            for Bgr8 {b, g, r, a} in frame {
                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
                pixels.push(a);
            }

            println!("cx : {}, cy : {}, pixels_len : {}", cx, cy, pixels.len());

            let mut gif_frame = gif::Frame::from_rgba_speed(cx as u16, cy as u16, &mut pixels, 10);

            gif_frame.left  =   frame_left as u16;
            gif_frame.top   =   frame_top as u16;
            gif_frame.width =   frame_width as u16;
            gif_frame.left  =   frame_height as u16;

            encoder.write_frame(&gif_frame).unwrap();

            thread::sleep(Duration::from_secs_f64(TIME_SLEEP_FRAME));
        }


    });


}





