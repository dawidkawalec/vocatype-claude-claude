import { invoke } from "@tauri-apps/api/core";

// VocaType Always-On Mini Interface
// PRD Compliant: 40×300px floating window with 4 states

interface AppState {
  currentState: 'idle' | 'listening' | 'processing' | 'error';
  isActionPanelOpen: boolean;
  audioLevel: number;
  recordingDuration: number;
  currentAction: string | null;
}

class VocaTypeMini {
  private state: AppState = {
    currentState: 'idle',
    isActionPanelOpen: false,
    audioLevel: 0,
    recordingDuration: 0,
    currentAction: null
  };

  private recordingTimer: number = 0;
  private audioVisualizerInterval: number = 0;

  constructor() {
    console.log('🚀 VocaType Mini Interface starting...');
    setTimeout(() => {
      this.initializeUI();
      console.log('✅ VocaType Mini Interface initialized');
    }, 100); // Delay to ensure DOM is ready
  }

  private initializeUI() {
    console.log('🔧 Initializing UI...');
    
    // Status indicator click handler (expand/collapse action panel)
    const statusIndicator = document.getElementById('status-indicator');
    console.log('🎯 Status indicator found:', !!statusIndicator);
    
    if (statusIndicator) {
      statusIndicator.addEventListener('click', (e) => {
        console.log('🎤 Status indicator clicked!');
        e.preventDefault();
        this.toggleActionPanel();
      });
      
      // Also add visual feedback
      statusIndicator.style.cursor = 'pointer';
      statusIndicator.style.border = '2px solid #00ff00'; // Green border for debug
    }

    // Action button handlers (5 pre-defined actions from PRD)
    const actionButtons = document.querySelectorAll('.action-btn');
    console.log('🎯 Action buttons found:', actionButtons.length);
    
    actionButtons.forEach((button, index) => {
      const element = button as HTMLElement;
      element.style.border = '1px solid #ff0000'; // Red border for debug
      
      element.addEventListener('click', (e) => {
        console.log(`🎯 Action button ${index} clicked!`);
        e.preventDefault();
        e.stopPropagation();
        
        const target = e.currentTarget as HTMLElement;
        const action = target.dataset.action;
        console.log('🎯 Action:', action);
        
        if (action === 'settings') {
          console.log('⚙️ Opening settings...');
          this.openSettings();
        } else if (action) {
          console.log(`🎯 Executing action: ${action}`);
          this.executeAction(action);
        }
      });
    });

    // Hover behavior for action panel
    const miniApp = document.getElementById('mini-app');
    miniApp?.addEventListener('mouseenter', () => {
      if (!this.state.isActionPanelOpen) {
        setTimeout(() => {
          if (!this.state.isActionPanelOpen) {
            this.showActionPanel();
          }
        }, 500); // 0.5s hover delay as per PRD
      }
    });

    miniApp?.addEventListener('mouseleave', () => {
      if (this.state.isActionPanelOpen) {
        setTimeout(() => {
          this.hideActionPanel();
        }, 1000); // 1s delay before hiding
      }
    });

    this.updateUI();
  }

  // Window behavior will be configured via tauri.conf.json

  private toggleActionPanel() {
    console.log('🔄 Toggling action panel. Current state:', this.state.isActionPanelOpen);
    
    if (this.state.isActionPanelOpen) {
      this.hideActionPanel();
    } else {
      this.showActionPanel();
    }
  }

  private showActionPanel() {
    console.log('📂 Showing action panel...');
    const actionPanel = document.getElementById('action-panel');
    console.log('📂 Action panel found:', !!actionPanel);
    
    if (actionPanel) {
      actionPanel.style.display = 'block';
      actionPanel.style.background = '#444444'; // Dark background for visibility
      actionPanel.classList.remove('closing');
      this.state.isActionPanelOpen = true;
      console.log('✅ Action panel shown');
    }
  }

  private hideActionPanel() {
    const actionPanel = document.getElementById('action-panel');
    if (actionPanel) {
      actionPanel.classList.add('closing');
      setTimeout(() => {
        actionPanel.style.display = 'none';
        actionPanel.classList.remove('closing');
      }, 300); // Match CSS animation duration
      this.state.isActionPanelOpen = false;
    }
  }

  private async executeAction(action: string) {
    console.log(`🎯 Executing action: ${action}`);

    this.setState('processing');
    this.state.currentAction = action;
    this.hideActionPanel();

    try {
      // Get selected text from system (via Rust backend)
      const selectedText = await invoke('get_selected_text') as any;

      if (selectedText.success && selectedText.data) {
        // Process with AI
        const result = await invoke('process_text_action', {
          text: selectedText.data,
          action: action
        }) as any;

        if (result.success) {
          // Replace text in original location
          await invoke('replace_selected_text', {
            newText: result.data
          });

          this.setState('idle');
          setTimeout(() => this.setState('idle'), 1000); // Success feedback
        } else {
          throw new Error(result.error);
        }
      } else {
        throw new Error('No text selected');
      }
    } catch (error) {
      console.error('Action failed:', error);
      this.setState('error');
      setTimeout(() => this.setState('idle'), 3000);
    }

    this.state.currentAction = null;
  }

  private setState(newState: AppState['currentState']) {
    this.state.currentState = newState;
    this.updateUI();
  }

  private updateUI() {
    const statusIndicator = document.getElementById('status-indicator');
    const statusIcon = document.getElementById('status-icon');

    if (!statusIndicator || !statusIcon) return;

    // Update state attribute for CSS styling
    statusIndicator.setAttribute('data-state', this.state.currentState);

    // Update status icon based on state
    switch (this.state.currentState) {
      case 'idle':
        statusIcon.textContent = '🎤';
        this.stopRecordingTimer();
        this.stopAudioVisualization();
        break;
      case 'listening':
        statusIcon.textContent = '🔴';
        this.startRecordingTimer();
        this.startAudioVisualization();
        break;
      case 'processing':
        statusIcon.textContent = '⚡';
        this.stopRecordingTimer();
        this.stopAudioVisualization();
        break;
      case 'error':
        statusIcon.textContent = '❌';
        this.stopRecordingTimer();
        this.stopAudioVisualization();
        break;
    }
  }

  private startRecordingTimer() {
    this.state.recordingDuration = 0;
    this.recordingTimer = setInterval(() => {
      this.state.recordingDuration++;
      const minutes = Math.floor(this.state.recordingDuration / 60);
      const seconds = this.state.recordingDuration % 60;
      const timer = document.getElementById('status-timer');
      if (timer) {
        timer.textContent = `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
      }
    }, 1000);
  }

  private stopRecordingTimer() {
    if (this.recordingTimer) {
      clearInterval(this.recordingTimer);
      this.recordingTimer = 0;
    }
  }

  private startAudioVisualization() {
    this.audioVisualizerInterval = setInterval(async () => {
      try {
        // Get audio level from Rust backend
        const level = await invoke('get_audio_level') as any;
        this.updateAudioBars(level.data || 0);
      } catch (error) {
        // Silent fail for audio level polling
      }
    }, 50); // 20fps for smooth animation
  }

  private stopAudioVisualization() {
    if (this.audioVisualizerInterval) {
      clearInterval(this.audioVisualizerInterval);
      this.audioVisualizerInterval = 0;
    }
    this.updateAudioBars(0); // Reset bars
  }

  private updateAudioBars(level: number) {
    const bars = document.querySelectorAll('.audio-bars .bar');
    const normalizedLevel = Math.max(0, Math.min(1, level));

    bars.forEach((bar, index) => {
      const barThreshold = (index + 1) / bars.length;
      const height = normalizedLevel > barThreshold ?
        Math.floor(normalizedLevel * 20) : 2; // 2px minimum height

      (bar as HTMLElement).style.height = `${height}px`;
    });
  }

  // Removed startDictation method - will be implemented in Phase 3

  private async openSettings() {
    try {
      // Open settings window via Rust backend
      await invoke('open_settings_window');
    } catch (error) {
      console.error('Failed to open settings:', error);
    }
  }

  public destroy() {
    this.stopRecordingTimer();
    this.stopAudioVisualization();
  }
}

// Initialize when DOM is ready
window.addEventListener("DOMContentLoaded", () => {
  console.log('🚀 VocaType initializing...');
  new VocaTypeMini();
});

// Handle app lifecycle
window.addEventListener("beforeunload", () => {
  console.log('👋 VocaType shutting down...');
});