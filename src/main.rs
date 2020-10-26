mod util;
mod capture;
use std::ptr::null_mut;
use std::mem::{self, size_of, uninitialized};
use winapi::um::{libloaderapi, winuser, wingdi, uxtheme, dwmapi};
use winapi::shared::{windef, windowsx};
use winapi::ctypes::c_void;
use std::sync::{Arc, Mutex};
use std::mem::transmute as tcast;

//Little endian set the last byte
// R  G  B  A
// 00 00 00 ff
const LEFT_EXTEND : i32 = 2;
const TOP_EXTEND : i32 = 40;
const RIGHT_EXTEND : i32 = 2;
const BOTTOM_EXTEND : i32 = 2;


pub struct WindowState {
    play_button : windef::HICON,
    pause_button: windef::HICON,
    current_button : windef::HICON,
}


impl WindowState {
 
    pub unsafe fn new() -> WindowState {

        let base = libloaderapi::GetModuleHandleW(null_mut());

        let play_button =
            winuser::LoadImageW(base,
            mem::transmute::<*mut i8, *const u16>(winuser::MAKEINTRESOURCEA(91)),
            winuser::IMAGE_ICON,
            128,
            128,
            0) as windef::HICON;
        
            

        let pause_button =
            winuser::LoadImageW(base,
            mem::transmute::<*mut i8, *const u16>(winuser::MAKEINTRESOURCEA(101)),
            winuser::IMAGE_ICON,
            128,
            128,
            0) as windef::HICON;

        WindowState {
            play_button,
            pause_button,
            current_button : play_button,
        }

    }

}


// Application Cood

fn handle_message( window_handle : windef::HWND) -> bool {
    unsafe {
        let mut message : winuser::MSG = uninitialized();
        if winuser::GetMessageW( &mut message as *mut winuser::MSG, window_handle, 0, 0 ) > 0 {
            winuser::TranslateMessage( &message as *const winuser::MSG );
            winuser::DispatchMessageW( &message as *const winuser::MSG );
            true
        } else {
            false
        }
    }
}


//AlphaKey on Transparent windows affects the border
//we want the borders to remain opaque
unsafe fn draw_transparent_parts(window_handle : windef::HWND) {

    let general_dc = winuser::GetDC(window_handle);

    let mut window_rect : windef::RECT = mem::zeroed();

    winuser::GetClientRect(window_handle, &mut window_rect as *mut windef::RECT);

    let window_width = window_rect.right - window_rect.left;
    let window_height = window_rect.bottom - window_rect.top;

    if general_dc == null_mut() {
        panic!("Couldn't get DC for window");
    }

    //zero everything
    let mut bitmap_header : wingdi::BITMAPINFOHEADER = mem::zeroed();

    bitmap_header.biSize = size_of::<wingdi::BITMAPINFOHEADER>() as u32;
    bitmap_header.biBitCount = 32;
    bitmap_header.biWidth = window_width;
    bitmap_header.biHeight = window_height;
    bitmap_header.biPlanes = 1;

    let mut bitmap_info : wingdi::BITMAPINFO = mem::zeroed();

    bitmap_info.bmiHeader = bitmap_header;

    let mut bits: *mut c_void = mem::zeroed();

    //Create bitmap to color in
    let bitmap_handle = wingdi::CreateDIBSection(general_dc,
                             &bitmap_info,
                             wingdi::DIB_RGB_COLORS, 
                             &mut bits,
                             null_mut(),
                             0);


    let compat_dc = wingdi::CreateCompatibleDC(general_dc);

    //select CompatDC to draw in
    wingdi::SelectObject(compat_dc, bitmap_handle as *mut c_void);

    //winuser::FillRect(compat_dc,
    //                  &window_rect,
    //                  wingdi::GetStockObject(wingdi::BLACK_BRUSH as i32) as windef::HBRUSH);
    //
    wingdi::BitBlt(compat_dc, 0, 0, window_width, window_height, general_dc, 0, 0, wingdi::SRCCOPY);

    //these are the pixels we are given access to
    let pixels : *mut u32 = mem::transmute::<_, *mut u32>(bits);

    for i in 0..window_height {
        for j in 0..window_width {
            //rippo pointer math
            //*pixels.add((i * window_width + j) as usize) &= 0xffffffff;
            *pixels.add((i * window_width + j) as usize) = 0xffffffff;
        }
    }

    let mut blend_function = wingdi::BLENDFUNCTION {
        BlendOp : wingdi::AC_SRC_OVER,
        BlendFlags : 0,
        SourceConstantAlpha : 255,
        AlphaFormat : wingdi::AC_SRC_ALPHA,
    };


    let mut size = windef::SIZE {
        cx : window_width,
        cy : window_height,
    };

    let mut point = windef::POINT {
        x : 0,
        y : 0,
    };


    //Copy CompatDC to actual scren
    //let ret_val = winuser::UpdateLayeredWindow(window_handle,
    //                             general_dc,
    //                             null_mut(),
    //                             &mut size,
    //                             compat_dc,
    //                             &mut point,
    //                             0,
    //                             &mut blend_function,
    //                             winuser::ULW_ALPHA);

    //println!("retval {}", ret_val);
    //if ret_val == 0 {
    //    println!("last_error {}", Error::last_os_error());
    //}

    winuser::ReleaseDC(window_handle, compat_dc);
    winuser::ReleaseDC(window_handle, general_dc);
    wingdi::DeleteObject(bitmap_handle as *mut c_void);

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
    winuser::SetLayeredWindowAttributes(hwnd, 0x00696900, 0xff, winuser::LWA_COLORKEY);
    0

}

//
// Hit test the frame for resizing and moving.
unsafe fn hit_test_nca(mouse_loc : &windef::POINT, window_rect : &windef::RECT, hwnd : windef::HWND, wparam : usize, lparam : isize) -> isize {
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

    let position_table: [[isize; 3] ; 3] = [
        [winuser::HTTOPLEFT, if is_resize { winuser::HTTOP } else { winuser::HTCAPTION }, winuser::HTTOPRIGHT],
        [winuser::HTLEFT, winuser::HTNOWHERE, winuser::HTRIGHT],
        [winuser::HTBOTTOMLEFT, winuser::HTBOTTOM, winuser::HTBOTTOMRIGHT],
    ];

        position_table[u_row][u_col]

}

unsafe fn on_up_nc(hwnd : windef::HWND, lparam : isize) -> isize {

    let window_state = tcast::<_, &mut WindowState>(winuser::GetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA));
    //these are screen points

    if window_state.current_button == window_state.play_button {
        window_state.current_button = window_state.pause_button;
    } else {
        window_state.current_button = window_state.play_button;
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

    let cx = window_rect.right - window_rect.left;

    if mouse_loc.x > (window_rect.left + 64) && mouse_loc.x < (window_rect.left + 96) &&
        (mouse_loc.y < (window_rect.top + 37) && mouse_loc.y > (window_rect.top +5)){
        return winuser::HTBORDER;
    }


    dwmapi::DwmDefWindowProc(hwnd, msg, wparam, lparam, &mut lresult);

    if lresult == 0 {
        return hit_test_nca(&mouse_loc, &window_rect, hwnd, wparam, lparam);
    }

    lresult
}

unsafe fn on_paint(hwnd : windef::HWND) -> isize {

    let window_state = tcast::<_, &mut WindowState>(winuser::GetWindowLongPtrW(hwnd, winuser::GWLP_USERDATA));

    let mut ps :winuser::PAINTSTRUCT = mem::zeroed();
    let mut client_rect : windef::RECT = mem::zeroed();

    winuser::GetClientRect(hwnd, &mut client_rect);

    let cx = client_rect.right - client_rect.top;

    

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


    //let mut lresult : isize = 0;
    //dwmapi::DwmDefWindowProc(hwnd, msg, wparam, lparam, &mut lresult);
    //println!("lresult : {}", lresult);

    match msg {
        winuser::WM_CREATE => on_create(hwnd, lparam),
        winuser::WM_ACTIVATE => on_activate(hwnd),
        winuser::WM_NCCALCSIZE if wparam == 1 => on_nccalcsize(lparam),
        winuser::WM_PAINT => on_paint(hwnd),
        winuser::WM_NCLBUTTONUP => on_up_nc(hwnd, lparam),
        winuser::WM_EXITSIZEMOVE => on_stop_resize(hwnd),
        winuser::WM_NCHITTEST => dwm_check(hwnd, msg, wparam, lparam),
        _ => winuser::DefWindowProcW(hwnd, msg, wparam, lparam),
    }


}

fn main() {
    let class_name = util::to_wstring("WinPeekClass");
    let window_name = util::to_wstring("WinPeek");

    let brush = unsafe {
            wingdi::CreateSolidBrush(0x00696900)
    };

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

    unsafe { 
        if winuser::RegisterClassExW(&wnd_class) == 0 {
            panic!("RegisterClass Failed");
        }
    };

    let mut window_state = unsafe { WindowState::new() };

    let window_handle = unsafe {
        winuser::CreateWindowExW(winuser::WS_EX_LAYERED,
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
                             mem::transmute::<&mut WindowState, *mut c_void>(&mut window_state))
    };

    unsafe {
        winuser::ShowWindow(window_handle, winuser::SW_SHOW);

    }

    loop {
        if !handle_message(window_handle) {
            break;
        }
    }

}
