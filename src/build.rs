extern crate winres;

fn main() {
  if cfg!(target_os = "windows") {
    let mut res = winres::WindowsResource::new();
    res.set_icon("PlayIcon.ico");
    res.set_resource_file("Resource.rc");
    res.compile().unwrap();
  }
}
