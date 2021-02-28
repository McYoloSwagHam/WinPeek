mod util;
mod capture;
use std::ptr::null_mut;
use std::mem::{self, size_of};
use winapi::um::{libloaderapi, winuser, wingdi, uxtheme, dwmapi};
use winapi::shared::{windef, windowsx};
use winapi::ctypes::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::mem::transmute as tcast;
use std::fs::{remove_file, rename};
use std::env::temp_dir;
use std::path::PathBuf;
use wfd;
use rand::Rng;
use rand::distributions::Alphanumeric;

//Little endian set the last byte
// R  G  B  A
// 00 00 00 ff
pub const LEFT_EXTEND : i32 = 2;
pub const TOP_EXTEND : i32 = 40;
pub const RIGHT_EXTEND : i32 = 2;
pub const BOTTOM_EXTEND : i32 = 2;

//This is the color key we use to show that something
//is transparent using SetLayeredWindowAttributes
const TRANSPARENCY_COLOR : u32 = 0x00696900;
const ICON_SIZE : i32 = 128;


// This atomic bool that tells the renderer loop to stop
// scrap is sync so, it needs to run in another thread
// in a LOOP and check every iter whether to stop or not.
pub static SHOULD_STOP : AtomicBool = AtomicBool::new(false);

pub struct WindowState {
    play_button : windef::HICON,
    pause_button: windef::HICON,
    current_button : windef::HICON,
    fp : PathBuf,
}

impl WindowState {
 
    pub unsafe fn new() -> WindowState {

        let base = libloaderapi::GetModuleHandleW(null_mut());

        let play_button =
            winuser::LoadImageW(base,
            tcast::<*mut i8, *const u16>(winuser::MAKEINTRESOURCEA(91)),
            winuser::IMAGE_ICON,
            ICON_SIZE,
            ICON_SIZE,
            0) as windef::HICON;
        
            

        let pause_button =
            winuser::LoadImageW(base,
            tcast::<*mut i8, *const u16>(winuser::MAKEINTRESOURCEA(101)),
            winuser::IMAGE_ICON,
            ICON_SIZE,
            ICON_SIZE,
            0) as windef::HICON;

        let mut temp_path = temp_dir();

        let mut random_file_name : String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .collect();

        random_file_name.push_str(".mp4");
        temp_path.push(random_file_name); 

        WindowState {
            play_button,
            pause_button,
            current_button : play_button,
            fp : temp_path,
        }

    }

}


// Message Loop
fn handle_message( window_handle : windef::HWND) -> bool {
    unsafe {
        let mut message = mem::MaybeUninit::<winuser::MSG>::uninit();
        if winuser::GetMessageW(message.as_mut_ptr(), window_handle, 0, 0 ) > 0 {
            winuser::TranslateMessage(message.as_ptr());
            winuser::DispatchMessageW(message.as_ptr());
            true
        } else {
            false
        }
    }
}

unsafe fn on_activate(hwnd : windef::HWND) -> isize {

    let margins = uxtheme::MARGINS {
        cxLeftWidth : LEFT_EXTEND,
        cxRightWidth : RIGHT_EXTEND,
        cyBottomHeight : BOTTOM_EXTEND,
        cyTopHeight : TOP_EXTEND,
    };

    dwmapi::DwmExtendFrameIntoClientArea(hwnd, &margins);

    0
}

unsafe fn on_nccalcsize(_ : isize) -> isize {

    //by not handling it we return a borderless window
    0
}

unsafe fn on_create(hwnd : windef::HWND, lparam : isize) -> isize {

    let mut create_rect : windef::RECT = mem::zeroed();


    winuser::GetWindowRect(hwnd, &mut create_rect);

    let rect_width = create_rect.right - create_rect.left;
    let rect_height = create_rect.bottom - create_rect.top;


    //Trigger WM_NCCALCSIZE
    winuser::SetWindowPos(hwnd,
                          null_mut(),
                          create_rect.left, create_rect.top,
                          rect_width, rect_height,
                          winuser::SWP_FRAMECHANGED);


    let create_struct = tcast::<_, &winuser::CREATESTRUCTW>(lparam);

    let window_state_ptr = create_struct.lpCreateParams;

    //Store &mut WindowState in window UserData.
    winuser::SetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA, window_state_ptr as isize);
    winuser::SetLayeredWindowAttributes(hwnd, TRANSPARENCY_COLOR, 0xff, winuser::LWA_COLORKEY);
    0

}

//
// Hit test the frame for resizing and moving.
unsafe fn hit_test_nca(mouse_loc : &windef::POINT, window_rect : &windef::RECT) -> isize {
    let mut frame_rect = mem::zeroed();

    winuser::AdjustWindowRectEx(&mut frame_rect,
                                winuser::WS_OVERLAPPEDWINDOW & !winuser::WS_CAPTION,
                                0,
                                0);

    let mut u_row = 1;
    let mut u_col = 1;
    let mut is_resize = false;

    //we're just checking which side to return
    if mouse_loc.y >= window_rect.top && mouse_loc.y < window_rect.top + TOP_EXTEND {
        //check if the button is closer to bottom of top or top of top
        is_resize = mouse_loc.y < (window_rect.top - frame_rect.top);
        u_row = 0;
    } 
    else if mouse_loc.y < window_rect.bottom && mouse_loc.y >= window_rect.bottom - BOTTOM_EXTEND{
        u_row = 2;
    }

    if mouse_loc.x >= window_rect.left && mouse_loc.x < window_rect.left + LEFT_EXTEND {
        u_col = 0;
    } 
    else if mouse_loc.x < window_rect.right && mouse_loc.x >= window_rect.right - RIGHT_EXTEND {
        u_col = 2;
    }

    //index position_table to find the cursor position.
    let position_table: [[isize; 3] ; 3] = [
        [winuser::HTTOPLEFT, if is_resize { winuser::HTTOP } else { winuser::HTCAPTION }, winuser::HTTOPRIGHT],
        [winuser::HTLEFT, winuser::HTNOWHERE, winuser::HTRIGHT],
        [winuser::HTBOTTOMLEFT, winuser::HTBOTTOM, winuser::HTBOTTOMRIGHT],
    ];

        position_table[u_row][u_col]

}

unsafe fn on_nc_up(hwnd : windef::HWND) -> isize {

    let window_state = tcast::<_, &mut WindowState>(winuser::GetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA));
    //these are screen points

    if window_state.current_button == window_state.play_button {

        window_state.current_button = window_state.pause_button;
        capture::start_recording(hwnd, window_state.fp.clone());
        winuser::SetWindowPos(hwnd, winuser::HWND_TOPMOST, 0, 0, 0, 0, winuser::SWP_NOMOVE | winuser::SWP_NOSIZE);

    } else {
        window_state.current_button = window_state.play_button;
        winuser::SetWindowPos(hwnd, winuser::HWND_NOTOPMOST, 0, 0, 0, 0, winuser::SWP_NOMOVE | winuser::SWP_NOSIZE);

        SHOULD_STOP.store(true, Ordering::Relaxed);

        //loop while the other thread hasn't responded.
        while SHOULD_STOP.load(Ordering::Relaxed) {

        }

        let params = wfd::DialogParams {
            title: "Save as",
            file_types: vec![("MP4", "*.mp4")],
            default_extension: "mp4",
            ..Default::default()
        };

        let dialog_result = wfd::save_dialog(params);

        match dialog_result {
            Ok(dialog_result) => {

                let target_path = dialog_result.selected_file_path;
                rename(&window_state.fp, target_path).unwrap();

            },
            Err(_) => {
                remove_file(&window_state.fp).unwrap();
            }
        }
    }

    winuser::RedrawWindow(hwnd, null_mut(), null_mut(), winuser::RDW_INVALIDATE | winuser::RDW_ERASE);

    0
}


unsafe fn dwm_check(hwnd : windef::HWND,
                    msg : u32,
                    wparam : usize,
                    lparam : isize) -> isize {


    let mut lresult : isize = 0;
    let mut window_rect : windef::RECT = mem::zeroed();

    winuser::GetWindowRect(hwnd, &mut window_rect);

    let mouse_loc = windef::POINT {
        x : windowsx::GET_X_LPARAM(lparam),
        y : windowsx::GET_Y_LPARAM(lparam),
    };

    if mouse_loc.x > (window_rect.left + 64) && mouse_loc.x < (window_rect.left + 112) &&
        (mouse_loc.y < (window_rect.top + 37) && mouse_loc.y > (window_rect.top +5)){
        return winuser::HTBORDER;
    }

    dwmapi::DwmDefWindowProc(hwnd, msg, wparam, lparam, &mut lresult);

    if lresult == 0 {
        return hit_test_nca(&mouse_loc, &window_rect);
    }

    lresult
}

unsafe fn on_paint(hwnd : windef::HWND) -> isize {

    let window_state = tcast::<_, &mut WindowState>(winuser::GetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA));

    let mut ps :winuser::PAINTSTRUCT = mem::zeroed();
    let mut client_rect : windef::RECT = mem::zeroed();

    winuser::GetClientRect(hwnd, &mut client_rect);
    winuser::BeginPaint(hwnd, &mut ps);
    winuser::DrawIconEx(ps.hdc, 64, 5, window_state.current_button, 32, 32, 0, null_mut(), 0x3);
    winuser::EndPaint(hwnd, &ps);

    0
}

unsafe fn on_stop_resize(hwnd : windef::HWND) -> isize {

    winuser::RedrawWindow(hwnd, null_mut(), null_mut(), winuser::RDW_INVALIDATE | winuser::RDW_ERASE);
    0
}


unsafe extern "system" fn window_message_handler(hwnd : windef::HWND,
                                                 msg : u32,
                                                 wparam : usize,
                                                 lparam : isize) -> isize {


    match msg {
        winuser::WM_CREATE => on_create(hwnd, lparam),
        winuser::WM_ACTIVATE => on_activate(hwnd),
        winuser::WM_NCCALCSIZE if wparam == 1 => on_nccalcsize(lparam),
        winuser::WM_PAINT => on_paint(hwnd),
        winuser::WM_NCLBUTTONUP => on_nc_up(hwnd),
        winuser::WM_EXITSIZEMOVE => on_stop_resize(hwnd),
        winuser::WM_NCHITTEST => dwm_check(hwnd, msg, wparam, lparam),
        _ => winuser::DefWindowProcW(hwnd, msg, wparam, lparam),
    }


}

//almost everything interacting with win32 at this level is unsafe
//so we're going to make main unsafe as otherwise the unsafe blocks 
//become too verbose
unsafe fn unsafe_main() {

    //No idea how to set subsystem gui with cargo.
    winapi::um::wincon::FreeConsole();

    //High res ICONs and High res Dialogs
    winuser::SetProcessDPIAware();

    let class_name = util::to_wstring("WinPeekClass");
    let window_name = util::to_wstring("WinPeek");

    let brush = wingdi::CreateSolidBrush(TRANSPARENCY_COLOR);

    let wnd_class = winuser::WNDCLASSEXW {
        cbSize : size_of::<winuser::WNDCLASSEXW>() as u32,
        style : 0,
        lpfnWndProc : Some(window_message_handler),
        cbClsExtra : 0,
        cbWndExtra : 0,
        hInstance : null_mut(),
        hIcon : null_mut(),
        hCursor : null_mut(),
        hbrBackground : brush,
        lpszMenuName : null_mut(),
        lpszClassName : class_name.as_ptr(),
        hIconSm : null_mut(),
    };

    if winuser::RegisterClassExW(&wnd_class) == 0 {
        panic!("RegisterClass Failed");
    }

    let mut window_state = WindowState::new();

    let window_handle = winuser::CreateWindowExW(winuser::WS_EX_LAYERED | winuser::WS_EX_TOPMOST,
                             class_name.as_ptr(),
                             window_name.as_ptr(),
                             winuser::WS_OVERLAPPEDWINDOW, 
                             winuser::CW_USEDEFAULT,
                             winuser::CW_USEDEFAULT,
                             winuser::CW_USEDEFAULT,
                             winuser::CW_USEDEFAULT,
                             null_mut(),
                             null_mut(),
                             null_mut(),
                             tcast::<&mut WindowState, *mut c_void>(&mut window_state));

        winuser::ShowWindow(window_handle, winuser::SW_SHOW);

    loop {
        if !handle_message(window_handle) {
            break;
        }
    }

}

fn main() {
    unsafe { unsafe_main(); }
}


