use std::env::{temp_dir};
use std::mem;
use std::mem::transmute as tcast;
use std::thread;
use std::sync::{atomic::Ordering, mpsc::channel};
use std::time::Duration;
use winapi::shared::windef;
use winapi::um::winuser;
use scrap;
use mpeg_encoder;
use rayon::prelude::*;
use ratelimit;

const FRAME_COUNT : usize = 10;


// Start the recording here somehow
// and also start streaming to tmp file?
pub unsafe fn start_recording(hwnd : windef::HWND) {

    //Create a temp file to which we stream the video data
    //Then if the user decides to save, we move that file to the loc
    //otherwise we just delete the temporary file.
    let mut temp_path = temp_dir();

    //cast hwnd to u64 because it actually is Send + Sync
    //since even though by typedef it is a pointer it is
    //actually an integer handle, so it can be copied.
    let fake_hwnd : u64 = tcast::<windef::HWND, u64>(hwnd);

    temp_path.push("recording_buffer.mp4");

    let (tx, rx) = channel();

    let cx = winuser::GetSystemMetrics(winuser::SM_CXSCREEN);

    thread::spawn(move || {

        let mut view_area : windef::RECT = mem::zeroed();
        winuser::GetWindowRect(tcast::<u64, windef::HWND>(fake_hwnd), &mut  view_area);

        //subtract the fake client area from the actual view area
        let frame_width = (view_area.right - view_area.left - crate::LEFT_EXTEND - crate::RIGHT_EXTEND) as usize;
        let frame_height = (view_area.bottom - view_area.top - crate::TOP_EXTEND - crate::BOTTOM_EXTEND) as usize;
        
        //Move view area to account for margins.
        view_area.left += crate::LEFT_EXTEND;
        view_area.right -= crate::RIGHT_EXTEND;

        view_area.top += crate::TOP_EXTEND;
        view_area.bottom -= crate::BOTTOM_EXTEND;
        
        let receiver = rx;
        let mut encoder = mpeg_encoder::Encoder::new_with_params(temp_path, frame_width, frame_height, None, Some((1, FRAME_COUNT)), None, None, None);

        loop {

            let mut frames : [Vec<u8>; FRAME_COUNT] = receiver.recv().unwrap();

            frames.par_iter_mut().for_each(|frame| {

                let mut idx = 0;
                frame.retain(|_| {

                    let rows = ((idx/4) / cx) as i32;
                    let cols = ((idx/4) % cx) as i32;

                    idx += 1;

                    if (view_area.left <= cols && cols < view_area.right) && (view_area.top <= rows && rows < view_area.bottom) {
                        true
                    } else { 
                        false
                    }

                });

                for chunk in frame.chunks_exact_mut(4) {
                    chunk.swap(0,2);
                }


            });


            for frame in &frames{
                encoder.encode_rgba(frame_width, frame_height, frame, false);
            }

            //Atomic bool is true therefore we should stop.
            if crate::SHOULD_STOP.load(Ordering::Relaxed) {

                //drop encoder before letting other thread touch tmp file
                //because encoder releases file.
                drop(encoder);
                crate::SHOULD_STOP.store(false, Ordering::Relaxed);
                break;

            }

        }

    });


    //recording thread
    thread::spawn(move || {
        
        let pd = scrap::Display::primary().unwrap();
        let mut capt = scrap::Capturer::new(pd).unwrap();

        let transmitter = tx;

        let mut ratelimit = ratelimit::Builder::new()
            .capacity(1)
            .quantum(1)
            .interval(Duration::new(1, 0))
            .build();

        loop {

            let mut frames : [Vec<u8>; FRAME_COUNT] = Default::default();

            for frame_count in 0..FRAME_COUNT {

                let frame : Vec<u8> = loop {
                    match capt.frame() {
                        Ok(frame) => { break frame.to_vec(); },
                        Err(_) => continue,
                    }
                };

                //equal distributition of captures a second.
                frames[frame_count] = frame;
                thread::sleep(Duration::from_millis(80));
            }

            transmitter.send(frames).unwrap();

            if crate::SHOULD_STOP.load(Ordering::Relaxed) {
                break;
            }

            ratelimit.wait();

        }

    });



}





