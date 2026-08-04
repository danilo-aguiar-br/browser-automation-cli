// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession media methods (componentized).

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Start Page.screencast frame capture into an optional directory.
    pub async fn screencast_start(&mut self, path: Option<&Path>) -> Result<Value, CliError> {
        self.pump_events().await;
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        self.screencast_frames.clear();
        self.screencast_ack_ids.clear();
        let dir = path.map(|p| p.to_path_buf()).unwrap_or_else(|| {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            PathBuf::from(format!("screencast-{stamp}"))
        });
        // spawn_blocking consumes a PathBuf; clone once for mkdir, then move into field.
        crate::concurrency::create_dir_all_blocking(dir.clone())
            .await
            .map_err(|e| CliError::new(ErrorKind::Io, format!("screencast dir: {e}")))?;
        let dir_display = dir.to_string_lossy().into_owned();
        self.screencast_dir = Some(dir);
        // Page domain must be enabled for screencast frames.
        let _ = self
            .manager
            .client
            .send_command_no_params("Page.enable", Some(&session_id))
            .await;
        self.manager
            .client
            .send_command(
                "Page.startScreencast",
                Some(json!({
                    "format": "png",
                    "quality": crate::xdg::resolve_screencast_jpeg_quality(),
                    "maxWidth": crate::xdg::policy::policy_u32(crate::xdg::policy::key::DEFAULT_VIEWPORT_WIDTH),
                    "maxHeight": crate::xdg::policy::policy_u32(crate::xdg::policy::key::DEFAULT_VIEWPORT_HEIGHT),
                    "everyNthFrame": 1,
                })),
                Some(&session_id),
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("screencast start: {e}")))?;
        self.screencast_active = true;
        // Pump a few frames immediately so FrameAck unblocks the pipeline.
        for _ in 0..crate::xdg::policy::policy_u32(
            crate::xdg::policy::key::DEFAULT_SCREENCAST_START_PUMP_ITERS,
        ) {
            self.pump_events().await;
            tokio::time::sleep(std::time::Duration::from_millis(
                crate::xdg::resolve_event_pump_slice_ms(),
            ))
            .await;
        }
        Ok(json!({
            "screencast": "start",
            "dir": dir_display,
            "note": "Frames buffered in process; stop writes PNG files + manifest.json",
            "frames_buffered": self.screencast_frames.len(),
        }))
    }

    /// Stop screencast and flush remaining frames to disk.
    pub async fn screencast_stop(&mut self, path: Option<&Path>) -> Result<Value, CliError> {
        for _ in 0..crate::xdg::policy::policy_u32(
            crate::xdg::policy::key::DEFAULT_SCREENCAST_STOP_PUMP_ITERS,
        ) {
            self.pump_events().await;
            tokio::time::sleep(std::time::Duration::from_millis(
                crate::xdg::resolve_event_pump_slice_ms(),
            ))
            .await;
        }
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        if self.screencast_active {
            let _ = self
                .manager
                .client
                .send_command("Page.stopScreencast", None, Some(&session_id))
                .await;
            self.screencast_active = false;
        }
        self.pump_events().await;

        // Take ownership of the session dir (stop ends the capture).
        let dir = self.screencast_dir.take().unwrap_or_else(|| {
            PathBuf::from(format!(
                "screencast-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            ))
        });
        crate::concurrency::create_dir_all_blocking(dir.clone())
            .await
            .map_err(|e| CliError::new(ErrorKind::Io, format!("screencast stop mkdir: {e}")))?;

        // PAR-51: decode+write N frames off the async worker with Rayon (docsrs:
        // CPU + std::fs must not pin Tokio workers; bound via install_rayon_pool).
        let frames = std::mem::take(&mut self.screencast_frames);
        let dir_for_write = dir.clone();
        let (written, paths) = tokio::task::spawn_blocking(move || {
            crate::concurrency::install_rayon_pool_once();
            use base64::Engine;
            use rayon::prelude::*;
            let engine = base64::engine::general_purpose::STANDARD;
            let mut indexed: Vec<(usize, String)> = frames
                .par_iter()
                .enumerate()
                .filter_map(|(i, b64)| {
                    let bytes = engine.decode(b64).ok()?;
                    let name = format!("frame-{:05}.png", i + 1);
                    let out = dir_for_write.join(&name);
                    std::fs::write(&out, &bytes).ok()?;
                    Some((i, out.to_string_lossy().into_owned()))
                })
                .collect();
            indexed.sort_by_key(|(i, _)| *i);
            let paths: Vec<String> = indexed.into_iter().map(|(_, p)| p).collect();
            let written = paths.len() as u64;
            (written, paths)
        })
        .await
        .map_err(|e| CliError::new(ErrorKind::Software, format!("screencast frames join: {e}")))?;
        let video_path = path.map(|p| p.to_path_buf()).or_else(|| {
            // If start path looked like a video file, encode there (use local dir after take).
            let s = dir.to_string_lossy();
            if s.ends_with(".webm") || s.ends_with(".mp4") {
                Some(dir.clone())
            } else {
                None
            }
        });
        let mut video_out: Option<String> = None;
        let mut encode_note: Option<String> = None;
        if let Some(ref vp) = video_path {
            let ext = vp
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("mp4")
                .to_ascii_lowercase();
            let is_video = ext == "webm" || ext == "mp4";
            if is_video && written > 0 {
                if let Some(parent) = vp.parent() {
                    if !parent.as_os_str().is_empty() {
                        let _ =
                            crate::concurrency::create_dir_all_blocking(parent.to_path_buf()).await;
                    }
                }
                let pattern = dir.join("frame-%05d.png");
                let vcodec = if ext == "webm" {
                    crate::constants::SCREENCAST_FFMPEG_VCODEC_WEBM
                } else {
                    crate::constants::SCREENCAST_FFMPEG_VCODEC_MP4
                };
                // Optional OS binary: no pure-Rust H.264/VP9 encoder is production-ready
                // as a drop-in (muxide needs pre-encoded NALs; ffmpeg-next still links
                // system libav). DRY with video_local (XDG ffmpeg_path → PATH).
                let ffmpeg_bin = crate::video_local::resolve_ffmpeg_bin();
                let vp_owned = vp.clone();
                let pattern_owned = pattern.clone();
                let framerate = crate::xdg::policy::policy_u32(
                    crate::xdg::policy::key::SCREENCAST_FFMPEG_FRAMERATE,
                )
                .to_string();
                let pix_fmt = crate::constants::SCREENCAST_FFMPEG_PIX_FMT;
                let ffmpeg_timeout =
                    std::time::Duration::from_secs(crate::xdg::resolve_ffmpeg_timeout_secs());
                match ffmpeg_bin {
                    None => {
                        encode_note = Some(
                            "ffmpeg not found (set XDG ffmpeg_path or install on PATH); PNG frames kept"
                                .into(),
                        );
                    }
                    Some(ffmpeg) => {
                        let encode_res = tokio::task::spawn_blocking(move || {
                            let mut cmd = std::process::Command::new(&ffmpeg);
                            cmd.arg("-y")
                                .arg("-framerate")
                                .arg(&framerate)
                                .arg("-i")
                                .arg(&pattern_owned)
                                .arg("-c:v")
                                .arg(vcodec)
                                .arg("-pix_fmt")
                                .arg(pix_fmt)
                                .arg(&vp_owned);
                            crate::platform::run_capture_with_timeout(&mut cmd, ffmpeg_timeout)
                        })
                        .await;
                        match encode_res {
                            Ok(Ok(out)) if out.status.success() => {
                                video_out = Some(vp.to_string_lossy().into_owned());
                                encode_note = Some("encoded via ffmpeg".into());
                            }
                            Ok(Ok(out)) => {
                                encode_note = Some(format!(
                                    "ffmpeg failed: {}",
                                    String::from_utf8_lossy(&out.stderr)
                                ));
                            }
                            Ok(Err(crate::platform::ProcessCaptureError::Timeout)) => {
                                encode_note = Some(format!(
                                    "ffmpeg timed out after {}s; PNG frames kept",
                                    crate::xdg::resolve_ffmpeg_timeout_secs()
                                ));
                            }
                            Ok(Err(e)) => {
                                encode_note = Some(format!(
                                    "ffmpeg not available: {e}; PNG frames kept in dir"
                                ));
                            }
                            Err(e) => {
                                encode_note = Some(format!("ffmpeg join error: {e}"));
                            }
                        }
                    }
                }
            }
        }
        let manifest = json!({
            "format": "png",
            "frame_count": written,
            "frames": paths,
            "video": video_out,
            "encode_note": encode_note,
            "parallel_frames": true,
            "ffmpeg_hint": format!(
                "ffmpeg -y -framerate {} -i {}/frame-%05d.png -c:v {} -pix_fmt {} {}.mp4",
                crate::xdg::policy::policy_u32(crate::xdg::policy::key::SCREENCAST_FFMPEG_FRAMERATE),
                dir.display(),
                crate::constants::SCREENCAST_FFMPEG_VCODEC_MP4,
                crate::constants::SCREENCAST_FFMPEG_PIX_FMT,
                dir.display()
            ),
        });
        let manifest_path = dir.join("manifest.json");
        let _ = crate::concurrency::write_bytes_blocking(
            manifest_path.clone(),
            serde_json::to_vec_pretty(&manifest).unwrap_or_default(),
        )
        .await;
        self.screencast_ack_ids.clear();
        Ok(json!({
            "screencast": "stop",
            "dir": dir.to_string_lossy(),
            "frame_count": written,
            "manifest": manifest_path.to_string_lossy(),
            "video": video_out,
            "encode_note": encode_note,
        }))
    }
}
