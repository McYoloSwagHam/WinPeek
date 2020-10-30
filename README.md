# WinPeek

![WinPeek recording itself](https://github.com/McYoloSwagHam/WinPeek/blob/master/winpeek.gif?raw=true)

WinPeek recording itself!

## About
WinPeek is a rust application written using low-level Win32 API calls, in an attempt to mimic the Peek application for linux

## Building
Building this requires you link against FFMPEG, because MPEG encoder uses ffmpeg internally.

1) Set the `FFMPEG_DIR` environment variable to a directory containing a full_shared build of FFMPEG (WinPeek links to FFMPEG 4.3.1, [Builds here](https://www.gyan.dev/ffmpeg/builds/))

2) `cargo build`#WinPeek


## Todo
- Increase performance from 10 fps default to maybe 60 fps
- add the ability to save files other than MP4.

