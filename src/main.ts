import { invoke } from "@tauri-apps/api/core";

// Application State
interface AppState {
  isRecording: boolean;
  audioLevel: number;
  status: 'ready' | 'recording' | 'processing' | 'error';
  devices: AudioDevice[];
  selectedDevice: string;
  sensitivity: number;
  transcribedText: string;
  latency: number;
}

interface AudioDevice {
  name: string;
  isDefault: boolean;
  channels: number;
  sampleRate: number;
}

interface AudioStats {
  currentLevel: number;
  peakLevel: number;
  averageLevel: number;
  framesProcessed: number;
  framesDropped: number;
  bufferUsage: number;
  processingLatencyMs: number;
  speechDetectionRate: number;
}

class VocaTypeApp {
  private state: AppState = {
    isRecording: false,
    audioLevel: 0,
    status: 'ready',
    devices: [],
    selectedDevice: '',
    sensitivity: 50,
    transcribedText: '',
    latency: 0
  };

  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private animationFrame: number = 0;

  constructor() {
    this.canvas = document.getElementById('audio-canvas') as HTMLCanvasElement;
    this.ctx = this.canvas.getContext('2d')!;
    this.initializeUI();
    this.startAudioLevelUpdates();
    this.loadAudioDevices();
  }

  private initializeUI() {
    // Record button
    const recordBtn = document.getElementById('record-btn');
    recordBtn?.addEventListener('click', () => this.toggleRecording());

    // Settings button
    const settingsBtn = document.getElementById('settings-btn');
    settingsBtn?.addEventListener('click', () => this.toggleSettings());

    // Close settings button
    const closeSettingsBtn = document.getElementById('close-settings-btn');
    closeSettingsBtn?.addEventListener('click', () => this.toggleSettings());

    // Copy button
    const copyBtn = document.getElementById('copy-btn');
    copyBtn?.addEventListener('click', () => this.copyToClipboard());

    // Clear button
    const clearBtn = document.getElementById('clear-btn');
    clearBtn?.addEventListener('click', () => this.clearOutput());

    // Sensitivity slider
    const sensitivitySlider = document.getElementById('sensitivity-slider') as HTMLInputElement;
    sensitivitySlider?.addEventListener('input', (e) => {
      const target = e.target as HTMLInputElement;
      this.updateSensitivity(parseInt(target.value));
    });

    // Device selector
    const deviceSelect = document.getElementById('device-select') as HTMLSelectElement;
    deviceSelect?.addEventListener('change', (e) => {
      const target = e.target as HTMLSelectElement;
      this.selectDevice(target.value);
    });

    this.updateUI();
  }

  private async toggleRecording() {
    if (this.state.isRecording) {
      await this.stopRecording();
    } else {
      await this.startRecording();
    }
  }

  private async startRecording() {
    console.log('🎬 Starting recording...');

    try {
      this.updateStatus('recording');

      const result = await invoke('start_audio_capture', {
        deviceName: this.state.selectedDevice || null,
        config: null // Use default config
      });

      console.log('Recording started:', result);
      this.state.isRecording = true;
      this.updateUI();
    } catch (error) {
      console.error('Failed to start recording:', error);
      this.updateStatus('error');
      alert(`Failed to start recording: ${error}`);
    }
  }

  private async stopRecording() {
    console.log('🛑 Stopping recording...');

    try {
      this.updateStatus('processing');

      const result = await invoke('stop_audio_capture');
      console.log('Recording stopped:', result);

      // Get recent audio for transcription
      const audioData = await invoke('get_recent_audio', { durationMs: 5000 });

      // Transcribe audio
      const transcription = await invoke('transcribe_audio', { audioData });
      console.log('Transcription:', transcription);

      this.state.transcribedText = transcription.data || 'No speech detected';
      this.showOutput();

      this.state.isRecording = false;
      this.updateStatus('ready');
      this.updateUI();
    } catch (error) {
      console.error('Failed to stop recording:', error);
      this.updateStatus('error');
      alert(`Failed to stop recording: ${error}`);
    }
  }

  private async loadAudioDevices() {
    try {
      const response = await invoke('get_audio_devices');
      console.log('Audio devices:', response);

      if (response.success && response.data) {
        this.state.devices = response.data;
        this.updateDeviceList();
      }
    } catch (error) {
      console.error('Failed to load audio devices:', error);
    }
  }

  private updateDeviceList() {
    const deviceSelect = document.getElementById('device-select') as HTMLSelectElement;
    if (!deviceSelect) return;

    deviceSelect.innerHTML = '';

    this.state.devices.forEach(device => {
      const option = document.createElement('option');
      option.value = device.name;
      option.textContent = `${device.name} (${device.channels}ch, ${device.sampleRate}Hz)`;
      if (device.isDefault) {
        option.selected = true;
        this.state.selectedDevice = device.name;
      }
      deviceSelect.appendChild(option);
    });
  }

  private selectDevice(deviceName: string) {
    this.state.selectedDevice = deviceName;
    console.log('Selected device:', deviceName);
  }

  private updateSensitivity(value: number) {
    this.state.sensitivity = value;
    const valueSpan = document.getElementById('sensitivity-value');
    if (valueSpan) {
      valueSpan.textContent = `${value}%`;
    }
    console.log('VAD sensitivity:', value);
  }

  private toggleSettings() {
    const settingsPanel = document.getElementById('settings-panel');
    if (!settingsPanel) return;

    const isHidden = settingsPanel.style.display === 'none';
    settingsPanel.style.display = isHidden ? 'block' : 'none';
  }

  private showOutput() {
    const outputSection = document.getElementById('output-section');
    const outputText = document.getElementById('output-text');

    if (outputSection && outputText) {
      outputText.textContent = this.state.transcribedText;
      outputSection.style.display = 'block';
    }
  }

  private copyToClipboard() {
    navigator.clipboard.writeText(this.state.transcribedText)
      .then(() => {
        console.log('Text copied to clipboard');
        // Could add a toast notification here
      })
      .catch(err => {
        console.error('Failed to copy text:', err);
      });
  }

  private clearOutput() {
    this.state.transcribedText = '';
    const outputSection = document.getElementById('output-section');
    if (outputSection) {
      outputSection.style.display = 'none';
    }
  }

  private updateStatus(status: AppState['status']) {
    this.state.status = status;

    const statusText = document.getElementById('status-text');
    const statusDot = document.getElementById('status-dot');

    if (!statusText || !statusDot) return;

    // Remove all status classes
    statusDot.classList.remove('ready', 'recording', 'processing');

    switch (status) {
      case 'ready':
        statusText.textContent = 'Ready';
        statusDot.classList.add('ready');
        break;
      case 'recording':
        statusText.textContent = 'Recording...';
        statusDot.classList.add('recording');
        break;
      case 'processing':
        statusText.textContent = 'Processing...';
        statusDot.classList.add('processing');
        break;
      case 'error':
        statusText.textContent = 'Error';
        break;
    }
  }

  private updateUI() {
    const recordBtn = document.getElementById('record-btn');
    const recordIcon = document.getElementById('record-icon');
    const recordText = document.getElementById('record-text');

    if (!recordBtn || !recordIcon || !recordText) return;

    if (this.state.isRecording) {
      recordIcon.textContent = '⏹️';
      recordText.textContent = 'Stop Recording';
      recordBtn.classList.add('recording');
    } else {
      recordIcon.textContent = '🎤';
      recordText.textContent = 'Start Recording';
      recordBtn.classList.remove('recording');
    }
  }

  private async startAudioLevelUpdates() {
    const updateLevel = async () => {
      try {
        const response = await invoke('get_audio_level');
        if (response.success) {
          this.state.audioLevel = response.data || 0;
          this.updateAudioVisualizer();
        }
      } catch (error) {
        // Silently handle errors for frequent polling
      }

      this.animationFrame = requestAnimationFrame(updateLevel);
    };

    updateLevel();
  }

  private updateAudioVisualizer() {
    const canvas = this.canvas;
    const ctx = this.ctx;

    // Clear canvas
    ctx.fillStyle = '#0f0f0f';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Draw audio level bar
    const level = this.state.audioLevel;
    const barWidth = canvas.width * 0.8;
    const barHeight = 20;
    const x = (canvas.width - barWidth) / 2;
    const y = (canvas.height - barHeight) / 2;

    // Background bar
    ctx.fillStyle = '#333333';
    ctx.fillRect(x, y, barWidth, barHeight);

    // Level bar
    if (level > 0) {
      const levelWidth = barWidth * Math.min(level, 1);
      const color = level > 0.8 ? '#ff4444' : level > 0.5 ? '#ff8800' : '#00c851';
      ctx.fillStyle = color;
      ctx.fillRect(x, y, levelWidth, barHeight);
    }

    // Update level text
    const levelText = document.getElementById('audio-level-text');
    if (levelText) {
      levelText.textContent = `${Math.round(level * 100)}%`;
    }
  }

  public destroy() {
    if (this.animationFrame) {
      cancelAnimationFrame(this.animationFrame);
    }
  }
}

// Initialize app when DOM is ready
window.addEventListener("DOMContentLoaded", () => {
  console.log('🚀 VocaType initializing...');
  new VocaTypeApp();
});
