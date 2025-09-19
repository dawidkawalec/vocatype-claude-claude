use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::time::{Duration, Instant};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Host, Stream, SampleFormat, SampleRate, StreamConfig
};
use tracing::{debug, info, warn, error};

use crate::utils::error::{AudioError, AudioResult};
use super::types::{AudioConfig, AudioFrame, AudioStats, CaptureState, AudioDevice};
use super::buffer::CircularBuffer;
use super::vad::VoiceActivityDetector;

/// High-performance audio capture system with <10ms latency
/// Thread-safe implementation with atomic operations as per PRD requirements
pub struct AudioCapture {
    /// Current capture state
    state: Arc<Mutex<CaptureState>>,
    
    /// Audio configuration
    config: AudioConfig,
    
    /// Input device
    device: Option<Device>,
    
    /// Active audio stream
    stream: Option<Stream>,
    
    /// Circular buffer for 30-second rolling window
    buffer: Arc<CircularBuffer>,
    
    /// Voice Activity Detector
    vad: Arc<Mutex<VoiceActivityDetector>>,
    
    /// Real-time statistics
    stats: Arc<Mutex<AudioStats>>,
    
    /// Control flags
    is_recording: Arc<AtomicBool>,
    should_stop: Arc<AtomicBool>,
    
    /// Performance monitoring
    frames_processed: Arc<AtomicUsize>,
    frames_dropped: Arc<AtomicUsize>,
    
    /// Callbacks for real-time updates
    level_callback: Option<Arc<dyn Fn(f32) + Send + Sync>>,
    speech_callback: Option<Arc<dyn Fn(bool) + Send + Sync>>,
}

impl AudioCapture {
    /// Create new AudioCapture instance with performance-optimized configuration
    pub fn new(config: AudioConfig) -> AudioResult<Self> {
        let start_time = Instant::now();
        
        info!("🎤 Initializing AudioCapture with config: {:?}", config);
        
        // Create circular buffer for 30-second rolling window
        let buffer = Arc::new(CircularBuffer::new(
            config.sample_rate, 
            config.buffer_duration
        )?);
        
        // Initialize Voice Activity Detector
        let vad = Arc::new(Mutex::new(VoiceActivityDetector::new(
            config.sample_rate,
            config.vad_threshold,
            Duration::from_millis(config.vad_frame_size),
        )?));
        
        let init_time = start_time.elapsed();
        debug!("⚡ AudioCapture initialized in {:?}", init_time);
        
        Ok(Self {
            state: Arc::new(Mutex::new(CaptureState::Stopped)),
            config,
            device: None,
            stream: None,
            buffer,
            vad,
            stats: Arc::new(Mutex::new(AudioStats::default())),
            is_recording: Arc::new(AtomicBool::new(false)),
            should_stop: Arc::new(AtomicBool::new(false)),
            frames_processed: Arc::new(AtomicUsize::new(0)),
            frames_dropped: Arc::new(AtomicUsize::new(0)),
            level_callback: None,
            speech_callback: None,
        })
    }
    
    /// Get available audio input devices
    pub fn get_available_devices() -> AudioResult<Vec<AudioDevice>> {
        let host = cpal::default_host();
        let devices = host.input_devices()
            .map_err(|e| AudioError::CaptureInitFailed(format!("Failed to enumerate devices: {}", e)))?;
        
        let default_device = host.default_input_device();
        let default_name = default_device
            .as_ref()
            .and_then(|d| d.name().ok())
            .unwrap_or_default();
        
        let mut audio_devices = Vec::new();
        
        for device in devices {
            let name = device.name()
                .map_err(|e| AudioError::DeviceNotFound { 
                    device_name: format!("Unknown device: {}", e) 
                })?;
            
            // Get device capabilities
            let supported_configs = device.supported_input_configs()
                .map_err(|e| AudioError::UnsupportedFormat { 
                    details: format!("Could not get configs: {}", e) 
                })?;
            
            // Find best matching config for our requirements
            if let Some(config) = supported_configs
                .filter(|config| config.channels() <= 2)
                .max_by_key(|config| config.max_sample_rate().0)
            {
                audio_devices.push(AudioDevice {
                    name: name.clone(),
                    is_default: name == default_name,
                    channels: config.channels(),
                    sample_rate: config.max_sample_rate().0,
                });
            }
        }
        
        info!("🔍 Found {} audio input devices", audio_devices.len());
        Ok(audio_devices)
    }
    
    /// Start audio capture with specified device
    pub async fn start_capture(&mut self, device_name: Option<String>) -> AudioResult<()> {
        let start_time = Instant::now();
        
        // Set state to starting
        *self.state.lock().unwrap() = CaptureState::Starting;
        
        info!("🎬 Starting audio capture...");
        
        // Select input device
        let host = cpal::default_host();
        let device = match device_name {
            Some(name) => self.find_device_by_name(&host, &name)?,
            None => host.default_input_device()
                .ok_or(AudioError::NoInputDevice)?,
        };
        
        let device_name = device.name()
            .unwrap_or_else(|_| "Unknown Device".to_string());
        info!("🎧 Using audio device: {}", device_name);
        
        // Configure optimal stream settings for <10ms latency
        let mut supported_configs = device.supported_input_configs()
            .map_err(|e| AudioError::UnsupportedFormat { 
                details: format!("Device config error: {}", e) 
            })?;
        
        let supported_config = supported_configs
            .find(|config| {
                config.channels() <= 2 && 
                config.min_sample_rate() <= SampleRate(self.config.sample_rate) &&
                config.max_sample_rate() >= SampleRate(self.config.sample_rate)
            })
            .ok_or(AudioError::UnsupportedFormat {
                details: format!("16kHz not supported on device {}", device_name)
            })?;
        
        let stream_config = StreamConfig {
            channels: 1, // Force mono for better STT performance
            sample_rate: SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.config.buffer_size as u32),
        };
        
        info!("📊 Stream config: {:?}", stream_config);
        
        // Create audio processing stream
        let stream = self.create_audio_stream(&device, &stream_config, supported_config.sample_format())?;
        
        // Start the stream
        stream.play()
            .map_err(|e| AudioError::StreamError(format!("Failed to start stream: {}", e)))?;
        
        // Update state
        self.device = Some(device);
        self.stream = Some(stream);
        self.is_recording.store(true, Ordering::Release);
        *self.state.lock().unwrap() = CaptureState::Recording;
        
        let startup_time = start_time.elapsed();
        info!("✅ Audio capture started in {:?}", startup_time);
        
        // Performance check
        if startup_time > Duration::from_millis(100) {
            warn!("⚠️  Slow audio capture startup: {:?}", startup_time);
        }
        
        Ok(())
    }
    
    /// Create optimized audio stream with real-time processing
    fn create_audio_stream(
        &self, 
        device: &Device, 
        config: &StreamConfig,
        sample_format: SampleFormat,
    ) -> AudioResult<Stream> {
        let buffer = Arc::clone(&self.buffer);
        let vad = Arc::clone(&self.vad);
        let stats = Arc::clone(&self.stats);
        let frames_processed = Arc::clone(&self.frames_processed);
        let frames_dropped = Arc::clone(&self.frames_dropped);
        let level_callback = self.level_callback.clone();
        let speech_callback = self.speech_callback.clone();
        
        // Clone config to avoid lifetime issues
        let sample_rate = config.sample_rate.0;
        let config_clone = config.clone();
        
        let stream = match sample_format {
            SampleFormat::F32 => {
                device.build_input_stream(
                    &config_clone,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        Self::process_audio_data(
                            data,
                            &buffer,
                            &vad,
                            &stats,
                            &frames_processed,
                            &frames_dropped,
                            &level_callback,
                            &speech_callback,
                            sample_rate,
                        );
                    },
                    |err| error!("🚨 Audio stream error: {}", err),
                    None,
                )
            },
            SampleFormat::I16 => {
                device.build_input_stream(
                    &config_clone,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        // Convert i16 to f32
                        let f32_data: Vec<f32> = data.iter()
                            .map(|&sample| sample as f32 / 32768.0)
                            .collect();
                        
                        Self::process_audio_data(
                            &f32_data,
                            &buffer,
                            &vad,
                            &stats,
                            &frames_processed,
                            &frames_dropped,
                            &level_callback,
                            &speech_callback,
                            sample_rate,
                        );
                    },
                    |err| error!("🚨 Audio stream error: {}", err),
                    None,
                )
            },
            SampleFormat::U16 => {
                device.build_input_stream(
                    &config_clone,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        // Convert u16 to f32
                        let f32_data: Vec<f32> = data.iter()
                            .map(|&sample| (sample as f32 - 32768.0) / 32768.0)
                            .collect();
                        
                        Self::process_audio_data(
                            &f32_data,
                            &buffer,
                            &vad,
                            &stats,
                            &frames_processed,
                            &frames_dropped,
                            &level_callback,
                            &speech_callback,
                            sample_rate,
                        );
                    },
                    |err| error!("🚨 Audio stream error: {}", err),
                    None,
                )
            },
            _ => {
                return Err(AudioError::UnsupportedFormat {
                    details: format!("Unsupported sample format: {:?}", sample_format)
                });
            }
        }.map_err(|e| AudioError::StreamError(format!("Failed to build stream: {}", e)))?;
        
        Ok(stream)
    }
    
    /// High-performance audio processing function (called from audio thread)
    /// Must complete in <5ms as per PRD requirements  
    fn process_audio_data(
        data: &[f32],
        buffer: &Arc<CircularBuffer>,
        vad: &Arc<Mutex<VoiceActivityDetector>>,
        stats: &Arc<Mutex<AudioStats>>,
        frames_processed: &Arc<AtomicUsize>,
        frames_dropped: &Arc<AtomicUsize>,
        level_callback: &Option<Arc<dyn Fn(f32) + Send + Sync>>,
        speech_callback: &Option<Arc<dyn Fn(bool) + Send + Sync>>,
        sample_rate: u32,
    ) {
        let process_start = Instant::now();
        
        // Create audio frame
        let mut frame = AudioFrame {
            data: data.to_vec(),
            timestamp: process_start,
            sample_rate,
            energy_level: 0.0,
            is_speech: false,
        };
        
        // Calculate energy level for visualization
        frame.calculate_energy();
        
        // Voice Activity Detection (<5ms target)
        let vad_start = Instant::now();
        if let Ok(mut vad_detector) = vad.try_lock() {
            if let Ok(vad_result) = vad_detector.process_frame(&frame.data) {
                frame.is_speech = vad_result.is_speech;
                
                let vad_time = vad_start.elapsed();
                if vad_time > Duration::from_millis(5) {
                    warn!("⚠️  VAD processing too slow: {:?}", vad_time);
                }
                
                // Notify speech detection callback
                if let Some(callback) = speech_callback {
                    callback(vad_result.is_speech);
                }
            }
        }
        
        // Write to circular buffer
        if let Err(e) = buffer.write_frame(&frame) {
            warn!("⚠️  Buffer write failed: {}", e);
            frames_dropped.fetch_add(1, Ordering::Relaxed);
        }
        
        // Update statistics
        if let Ok(mut stats_guard) = stats.try_lock() {
            stats_guard.current_level = frame.energy_level;
            stats_guard.frames_processed = frames_processed.fetch_add(1, Ordering::Relaxed) as u64 + 1;
            stats_guard.processing_latency_ms = process_start.elapsed().as_secs_f32() * 1000.0;
            
            if frame.energy_level > stats_guard.peak_level {
                stats_guard.peak_level = frame.energy_level;
            }
        }
        
        // Real-time level callback for UI
        if let Some(callback) = level_callback {
            callback(frame.energy_level);
        }
        
        // Performance monitoring
        let total_time = process_start.elapsed();
        if total_time > Duration::from_millis(10) {
            warn!("⚠️  Audio processing too slow: {:?}", total_time);
        }
    }
    
    /// Stop audio capture
    pub async fn stop_capture(&mut self) -> AudioResult<()> {
        info!("🛑 Stopping audio capture...");
        
        self.should_stop.store(true, Ordering::Release);
        self.is_recording.store(false, Ordering::Release);
        
        if let Some(stream) = self.stream.take() {
            drop(stream); // This stops the stream
        }
        
        *self.state.lock().unwrap() = CaptureState::Stopped;
        
        let stats = self.stats.lock().unwrap();
        info!("📊 Capture stopped. Processed {} frames, dropped {}", 
              stats.frames_processed, self.frames_dropped.load(Ordering::Acquire));
        
        Ok(())
    }
    
    /// Get recent audio data for STT processing
    pub fn get_recent_audio(&self, duration: Duration) -> AudioResult<Vec<f32>> {
        self.buffer.read_recent(duration)
    }
    
    /// Get real-time audio statistics
    pub fn get_stats(&self) -> AudioStats {
        self.stats.lock().unwrap().clone()
    }
    
    /// Set callback for real-time audio level updates
    pub fn set_level_callback<F>(&mut self, callback: F) 
    where
        F: Fn(f32) + Send + Sync + 'static,
    {
        self.level_callback = Some(Arc::new(callback));
    }
    
    /// Set callback for speech detection updates
    pub fn set_speech_callback<F>(&mut self, callback: F) 
    where
        F: Fn(bool) + Send + Sync + 'static,
    {
        self.speech_callback = Some(Arc::new(callback));
    }
    
    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Acquire)
    }
    
    /// Get current capture state
    pub fn get_state(&self) -> CaptureState {
        *self.state.lock().unwrap()
    }
    
    /// Find device by name
    fn find_device_by_name(&self, host: &Host, name: &str) -> AudioResult<Device> {
        let devices = host.input_devices()
            .map_err(|e| AudioError::CaptureInitFailed(format!("Failed to enumerate devices: {}", e)))?;
        
        for device in devices {
            if let Ok(device_name) = device.name() {
                if device_name == name {
                    return Ok(device);
                }
            }
        }
        
        Err(AudioError::DeviceNotFound { 
            device_name: name.to_string() 
        })
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        if self.is_recording() {
            warn!("⚠️  AudioCapture dropped while recording - stopping capture");
            // Just set flags since we can't use async in Drop
            self.should_stop.store(true, Ordering::Release);
            self.is_recording.store(false, Ordering::Release);
        }
    }
}
