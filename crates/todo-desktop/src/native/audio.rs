use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

static PLAYING: AtomicBool = AtomicBool::new(false);

/// The same original two-note chime as the browser host. Playback never blocks
/// the dispatcher, and a burst of completed tasks shares the current chime.
pub fn play(directory: &Path) {
    if PLAYING.swap(true, Ordering::AcqRel) {
        return;
    }
    let directory = directory.to_owned();
    if std::thread::Builder::new()
        .name("todo-completion-sound".into())
        .spawn(move || {
            struct Reset;
            impl Drop for Reset {
                fn drop(&mut self) {
                    PLAYING.store(false, Ordering::Release);
                }
            }
            let _reset = Reset;
            play_platform(&directory);
        })
        .is_err()
    {
        PLAYING.store(false, Ordering::Release);
    }
}

fn wave() -> &'static [u8] {
    static WAVE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    WAVE.get_or_init(|| {
        const RATE: u32 = 22_050;
        let frames = (0.265 * f64::from(RATE)).ceil() as u32;
        let bytes = frames * 2;
        let mut wave = Vec::with_capacity(44 + bytes as usize);
        wave.extend_from_slice(b"RIFF");
        wave.extend_from_slice(&(36 + bytes).to_le_bytes());
        wave.extend_from_slice(b"WAVEfmt ");
        wave.extend_from_slice(&16u32.to_le_bytes());
        wave.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wave.extend_from_slice(&1u16.to_le_bytes()); // mono
        wave.extend_from_slice(&RATE.to_le_bytes());
        wave.extend_from_slice(&(RATE * 2).to_le_bytes());
        wave.extend_from_slice(&2u16.to_le_bytes());
        wave.extend_from_slice(&16u16.to_le_bytes());
        wave.extend_from_slice(b"data");
        wave.extend_from_slice(&bytes.to_le_bytes());
        for frame in 0..frames {
            let time = f64::from(frame) / f64::from(RATE);
            let sample = [(523.25, 0.0), (783.99, 0.065)]
                .into_iter()
                .map(|(frequency, start)| {
                    let age = time - start;
                    if !(0.0..0.2).contains(&age) {
                        return 0.0;
                    }
                    let gain = if age < 0.008 {
                        0.055 * age / 0.008
                    } else if age < 0.18 {
                        0.055 * (0.001_f64 / 0.055).powf((age - 0.008) / 0.172)
                    } else {
                        0.001
                    };
                    gain * (std::f64::consts::TAU * frequency * age).sin()
                })
                .sum::<f64>();
            wave.extend_from_slice(&((sample * f64::from(i16::MAX)) as i16).to_le_bytes());
        }
        wave
    })
}

#[cfg(windows)]
fn play_platform(_: &Path) {
    use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_MEMORY, SND_NODEFAULT, SND_SYNC};
    // SND_MEMORY reads the complete static PCM buffer, not a UTF-16 path. The
    // synchronous call is confined to this worker and finishes before reuse.
    unsafe {
        PlaySoundW(
            wave().as_ptr().cast(),
            std::ptr::null_mut(),
            SND_MEMORY | SND_NODEFAULT | SND_SYNC,
        );
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn play_platform(directory: &Path) {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let path = directory.join("completion-v1.wav");
    if std::fs::read(&path).ok().as_deref() != Some(wave()) {
        let temporary = directory.join(format!("completion-{}.tmp", std::process::id()));
        if std::fs::create_dir_all(directory)
            .and_then(|_| std::fs::write(&temporary, wave()))
            .and_then(|_| std::fs::rename(&temporary, &path))
            .is_err()
        {
            let _ = std::fs::remove_file(temporary);
            return;
        }
    }
    #[cfg(target_os = "linux")]
    let players = ["pw-play", "paplay", "aplay"];
    #[cfg(target_os = "macos")]
    let players = ["/usr/bin/afplay"];
    for player in players {
        let Ok(mut child) = Command::new(player)
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue;
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        return;
                    }
                    break;
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return;
                }
            }
        }
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn play_platform(_: &Path) {}
