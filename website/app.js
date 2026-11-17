/**
 * WinScroller 2D — Interactive Showcase Application
 * Pure Vanilla JavaScript ES6 + Web Audio API + HTML5 Canvas Physics
 */

(function () {
  'use strict';

  // ==========================================================================
  // 1. WEB AUDIO SYNTHESIZER (Crisp UI Feedback)
  // ==========================================================================
  class SoundFX {
    constructor() {
      this.enabled = true;
      this.ctx = null;
    }

    init() {
      if (!this.ctx && (window.AudioContext || window.webkitAudioContext)) {
        const AudioCtx = window.AudioContext || window.webkitAudioContext;
        this.ctx = new AudioCtx();
      }
    }

    playTone(freq, type, duration, gainVal = 0.08) {
      if (!this.enabled) return;
      try {
        this.init();
        if (this.ctx.state === 'suspended') {
          this.ctx.resume();
        }
        const osc = this.ctx.createOscillator();
        const gain = this.ctx.createGain();
        osc.type = type;
        osc.frequency.setValueAtTime(freq, this.ctx.currentTime);
        gain.gain.setValueAtTime(gainVal, this.ctx.currentTime);
        gain.gain.exponentialRampToValueAtTime(0.001, this.ctx.currentTime + duration);
        osc.connect(gain);
        gain.connect(this.ctx.destination);
        osc.start();
        osc.stop(this.ctx.currentTime + duration);
      } catch (e) {
        // Audio disabled or unsupported
      }
    }

    nav() {
      this.playTone(520, 'sine', 0.08, 0.06);
    }

    overview() {
      this.playTone(340, 'triangle', 0.22, 0.09);
    }

    snap() {
      this.playTone(780, 'sine', 0.12, 0.08);
    }

    click() {
      this.playTone(640, 'sine', 0.04, 0.04);
    }

    key() {
      this.playTone(420, 'square', 0.03, 0.02);
    }
  }

  const sfx = new SoundFX();

  // ==========================================================================
  // 2. BACKGROUND 2D SPARSE GRID PARTICLES CANVAS
  // ==========================================================================
  const bgCanvas = document.getElementById('bg-canvas');
  if (bgCanvas) {
    const ctx = bgCanvas.getContext('2d');
    let width, height;
    let particles = [];
    const particleCount = 45;
    let mouse = { x: -1000, y: -1000 };

    function resizeCanvas() {
      width = bgCanvas.width = window.innerWidth;
      height = bgCanvas.height = window.innerHeight;
      initParticles();
    }

    function initParticles() {
      particles = [];
      for (let i = 0; i < particleCount; i++) {
        particles.push({
          x: Math.random() * width,
          y: Math.random() * height,
          vx: (Math.random() - 0.5) * 0.4,
          vy: (Math.random() - 0.5) * 0.4,
          radius: Math.random() * 2 + 1,
          color: Math.random() > 0.6 ? '#00f5d4' : (Math.random() > 0.3 ? '#a855f7' : '#ff5722'),
          alpha: Math.random() * 0.4 + 0.1
        });
      }
    }

    function drawGridLines() {
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.025)';
      ctx.lineWidth = 1;
      const step = 80;
      for (let x = 0; x < width; x += step) {
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
        ctx.stroke();
      }
      for (let y = 0; y < height; y += step) {
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(width, y);
        ctx.stroke();
      }
    }

    function animateParticles() {
      ctx.clearRect(0, 0, width, height);
      drawGridLines();

      // Draw connections
      for (let i = 0; i < particles.length; i++) {
        for (let j = i + 1; j < particles.length; j++) {
          const dx = particles[i].x - particles[j].x;
          const dy = particles[i].y - particles[j].y;
          const dist = Math.sqrt(dx * dx + dy * dy);
          if (dist < 140) {
            ctx.strokeStyle = `rgba(0, 245, 212, ${0.12 * (1 - dist / 140)})`;
            ctx.lineWidth = 0.8;
            ctx.beginPath();
            ctx.moveTo(particles[i].x, particles[i].y);
            ctx.lineTo(particles[j].x, particles[j].y);
            ctx.stroke();
          }
        }
      }

      // Draw and move particles
      for (let i = 0; i < particles.length; i++) {
        const p = particles[i];
        p.x += p.vx;
        p.y += p.vy;

        if (p.x < 0) p.x = width;
        if (p.x > width) p.x = 0;
        if (p.y < 0) p.y = height;
        if (p.y > height) p.y = 0;

        ctx.fillStyle = p.color;
        ctx.globalAlpha = p.alpha;
        ctx.beginPath();
        ctx.arc(p.x, p.y, p.radius, 0, Math.PI * 2);
        ctx.fill();
        ctx.globalAlpha = 1;
      }

      requestAnimationFrame(animateParticles);
    }

    window.addEventListener('resize', resizeCanvas);
    window.addEventListener('mousemove', (e) => {
      mouse.x = e.clientX;
      mouse.y = e.clientY;
    });

    resizeCanvas();
    animateParticles();
  }

  // ==========================================================================
  // 3. CURSOR GLOW TRACKER
  // ==========================================================================
  const cursorGlow = document.getElementById('cursor-glow');
  if (cursorGlow) {
    window.addEventListener('mousemove', (e) => {
      cursorGlow.style.left = `${e.clientX}px`;
      cursorGlow.style.top = `${e.clientY}px`;
    });
  }

  // ==========================================================================
  // 4. TOAST NOTIFICATION SYSTEM
  // ==========================================================================
  function showToast(message, icon = '✓') {
    const container = document.getElementById('toast-container');
    if (!container) return;

    const toast = document.createElement('div');
    toast.className = 'toast';
    toast.innerHTML = `<span style="color: var(--cyan-primary); font-weight: bold;">${icon}</span> <span>${message}</span>`;
    container.appendChild(toast);

    setTimeout(() => {
      toast.style.transition = 'opacity 0.3s ease, transform 0.3s ease';
      toast.style.opacity = '0';
      toast.style.transform = 'translateY(10px)';
      setTimeout(() => toast.remove(), 320);
    }, 2800);
  }

  // ==========================================================================
  // 5. COPY TERMINAL COMMANDS
  // ==========================================================================
  const copyQuickCmd = document.getElementById('copy-quick-cmd');
  if (copyQuickCmd) {
    copyQuickCmd.addEventListener('click', () => {
      const text = document.getElementById('quick-cmd').innerText.trim();
      navigator.clipboard.writeText(text).then(() => {
        sfx.click();
        showToast('Install command copied to clipboard!');
      });
    });
  }

  document.querySelectorAll('.copy-term-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const text = btn.getAttribute('data-copy');
      if (text) {
        navigator.clipboard.writeText(text).then(() => {
          sfx.click();
          showToast('Code block copied!');
          const orig = btn.innerText;
          btn.innerText = 'Copied!';
          setTimeout(() => btn.innerText = orig, 2000);
        });
      }
    });
  });

  // ==========================================================================
  // 6. INTERACTIVE 2D TILING SIMULATOR ENGINE
  // ==========================================================================
  const sim = {
    viewport: document.getElementById('sim-viewport'),
    canvas: document.getElementById('sim-canvas'),
    coordsDisplay: document.getElementById('sim-coords-display'),
    modeTag: document.getElementById('sim-mode-tag'),
    countBadge: document.getElementById('sim-count-badge'),
    overviewHud: document.getElementById('overview-hud'),

    // State
    isOverview: false,
    isTabbedStack: false,
    currentCol: 1,
    currentRow: 0,
    activeCount: 5,
    activeTopology: 'single',
    activeDesktop: 1,
    widthPresetIndex: 0,
    widthPresets: [440, 560, 360],

    // Default 5 Core Windows
    defaultCells: [
      { id: 'win-editor', col: 0, row: 0, baseWidth: 440, baseHeight: 320, title: 'crates/core/spring.rs — Neovim', icon: '🦀', theme: 'code-theme', type: 'code' },
      { id: 'win-terminal', col: 1, row: 0, baseWidth: 440, baseHeight: 320, title: 'PowerShell (Elevated High Integrity) — cargo run', icon: '⚡', theme: 'term-theme', type: 'term' },
      { id: 'win-browser', col: 2, row: 0, baseWidth: 440, baseHeight: 320, title: 'WinScroller 2D — Architecture & DWM Docs', icon: '🌐', theme: 'web-theme', type: 'web' },
      { id: 'win-chat', col: 1, row: 1, baseWidth: 440, baseHeight: 320, title: '#dev-chat — WinScroller 2D Community', icon: '💬', theme: 'chat-theme', type: 'chat' },
      { id: 'win-monitor', col: 0, row: 1, baseWidth: 440, baseHeight: 320, title: 'System Resource & Spring Oscilloscope', icon: '📊', theme: 'monitor-theme', type: 'monitor' }
    ],

    cells: [],

    // Target Camera and Physics
    camX: 0,
    camY: 0,
    scale: 1.0,

    init() {
      if (!this.viewport || !this.canvas) return;

      this.cells = [...this.defaultCells];
      this.renderCellsDOM();
      this.updateLayout();
      this.setupEventListeners();
    },

    setWindowCount(count) {
      this.activeCount = count;
      sfx.snap();

      if (count === 5) {
        this.cells = [...this.defaultCells];
      } else if (count === 25) {
        // 5x5 Grid (-2..2, -2..2)
        this.cells = [];
        const apps = [
          { title: 'Neovim — core/layout.rs', icon: '🦀', theme: 'code-theme', type: 'code' },
          { title: 'PowerShell — cargo test', icon: '⚡', theme: 'term-theme', type: 'term' },
          { title: 'Arc Browser — DWM API Specs', icon: '🌐', theme: 'web-theme', type: 'web' },
          { title: 'Discord — #winscroller', icon: '💬', theme: 'chat-theme', type: 'chat' },
          { title: 'System Monitor — 120Hz Oscilloscope', icon: '📊', theme: 'monitor-theme', type: 'monitor' }
        ];

        let idx = 0;
        for (let r = -2; r <= 2; r++) {
          for (let c = -2; c <= 2; c++) {
            const app = apps[idx % apps.length];
            this.cells.push({
              id: `win-dyn-${c}-${r}`,
              col: c,
              row: r,
              baseWidth: 440,
              baseHeight: 320,
              title: `${app.title} (${c}, ${r})`,
              icon: app.icon,
              theme: app.theme,
              type: app.type
            });
            idx++;
          }
        }
      } else if (count === 50) {
        // 10x5 Grid (-4..5, -2..2)
        this.cells = [];
        const apps = [
          { title: 'Neovim — spring.rs', icon: '🦀', theme: 'code-theme', type: 'code' },
          { title: 'PowerShell — release build', icon: '⚡', theme: 'term-theme', type: 'term' },
          { title: 'Edge — GitHub PR Review', icon: '🌐', theme: 'web-theme', type: 'web' },
          { title: 'Slack — Core Contributors', icon: '💬', theme: 'chat-theme', type: 'chat' },
          { title: 'DWMWA Diagnostics', icon: '📊', theme: 'monitor-theme', type: 'monitor' }
        ];

        let idx = 0;
        for (let r = -2; r <= 2; r++) {
          for (let c = -4; c <= 5; c++) {
            const app = apps[idx % apps.length];
            this.cells.push({
              id: `win-dyn-${c}-${r}`,
              col: c,
              row: r,
              baseWidth: 440,
              baseHeight: 320,
              title: `${app.title} (${c}, ${r})`,
              icon: app.icon,
              theme: app.theme,
              type: app.type
            });
            idx++;
          }
        }
      } else if (count === 100) {
        // 10x10 Matrix (-4..5, -4..5)
        this.cells = [];
        const apps = [
          { title: 'Neovim — winscroller2d', icon: '🦀', theme: 'code-theme', type: 'code' },
          { title: 'Admin Terminal — VSync locked', icon: '⚡', theme: 'term-theme', type: 'term' },
          { title: 'Chrome — Win32 Hooks Guide', icon: '🌐', theme: 'web-theme', type: 'web' },
          { title: 'Matrix Chat — Developer Stream', icon: '💬', theme: 'chat-theme', type: 'chat' },
          { title: 'DWM Telemetry HUD', icon: '📊', theme: 'monitor-theme', type: 'monitor' },
          { title: 'Docker — Local Registry', icon: '🐳', theme: 'term-theme', type: 'term' },
          { title: 'K9s — Cluster Watcher', icon: '☸️', theme: 'term-theme', type: 'term' }
        ];

        let idx = 0;
        for (let r = -4; r <= 5; r++) {
          for (let c = -4; c <= 5; c++) {
            const app = apps[idx % apps.length];
            this.cells.push({
              id: `win-dyn-${c}-${r}`,
              col: c,
              row: r,
              baseWidth: 440,
              baseHeight: 320,
              title: `${app.title} (${c}, ${r})`,
              icon: app.icon,
              theme: app.theme,
              type: app.type
            });
            idx++;
          }
        }
      }

      // Center camera on a valid cell
      this.currentCol = 0;
      this.currentRow = 0;
      this.renderCellsDOM();
      this.updateLayout();

      const cloaked = Math.max(0, count - 4);
      if (this.countBadge) {
        this.countBadge.innerText = `Active: ${count} Windows | ${cloaked} Cloaked`;
      }
      showToast(`Scale Stress Test: Loaded ${count} managed windows in 2D sparse matrix!`, '⚡');
    },

    setTopology(topo) {
      this.activeTopology = topo;
      sfx.click();

      this.viewport.classList.remove('dual-mon-mode');

      if (topo === 'dual') {
        this.viewport.classList.add('dual-mon-mode');
        showToast('Multi-Monitor: Dual 4K Extended Mode active with strict 0px boundary seam!', '🖥️');
      } else if (topo === 'triple') {
        showToast('Multi-Monitor: Triple Display Surround mode active (Vertical + Dual 4K)!', '🖥️');
      } else {
        showToast('Switched to Single 4K Monitor canvas.', '🖥️');
      }
    },

    setVirtualDesktop(id) {
      this.activeDesktop = id;
      sfx.overview();
      const deskNames = {
        1: 'Rust Core Development 🦀',
        2: 'Research & Documentation 🌐',
        3: 'DevOps & Cluster ⚡',
        4: 'Communication & Media 💬'
      };
      showToast(`COM IVirtualDesktopManager: Switched to Desktop ${id} (${deskNames[id]})`, '🪟');
      // Gentle bounce camera animation
      this.camX += 40;
      this.updateLayout();
      setTimeout(() => {
        this.camX -= 40;
        this.updateLayout();
      }, 150);
    },

    renderCellsDOM() {
      // Clear canvas and render all active cells
      this.canvas.innerHTML = '';

      this.cells.forEach(cell => {
        const el = document.createElement('div');
        el.className = `sim-window-cell ${cell.col === this.currentCol && cell.row === this.currentRow ? 'active-focus' : ''}`;
        el.id = cell.id;
        el.dataset.col = cell.col;
        el.dataset.row = cell.row;
        el.setAttribute('draggable', 'true');

        let bodyHtml = '';
        if (cell.type === 'code') {
          bodyHtml = `
            <div class="sim-window-body code-theme">
              <pre><code><span class="tok-kw">pub struct</span> <span class="tok-type">SpringPhysics</span> {
    <span class="tok-kw">pub</span> stiffness: <span class="tok-type">f32</span>, <span class="tok-comment">// 380.0</span>
    <span class="tok-kw">pub</span> damping: <span class="tok-type">f32</span>,   <span class="tok-comment">// 24.0</span>
}
<span class="tok-kw">impl</span> <span class="tok-type">SpringPhysics</span> {
    <span class="tok-kw">pub fn</span> <span class="tok-fn">step</span>(&amp;<span class="tok-kw">mut self</span>, dt: <span class="tok-type">f32</span>) -&gt; <span class="tok-type">f32</span> {
        <span class="tok-kw">self</span>.pos += <span class="tok-kw">self</span>.vel * dt;
        <span class="tok-kw">self</span>.pos
    }
}</code></pre>
            </div>`;
        } else if (cell.type === 'term') {
          bodyHtml = `
            <div class="sim-window-body term-theme">
              <div class="term-line"><span class="term-green">   Compiling</span> winscroller2d-core v0.1.0</div>
              <div class="term-line"><span class="term-cyan">    Finished</span> release target(s) in 0.12s</div>
              <div class="term-line"><span class="term-yellow">[DWM]</span> 120Hz VSync timing locked</div>
              <div class="term-line"><span class="term-green">[SUCCESS]</span> Zero spillover 0px cloaking active</div>
              <div class="term-cursor-line"><span class="term-prompt">PS &gt;</span> <span class="term-cursor">_</span></div>
            </div>`;
        } else if (cell.type === 'web') {
          bodyHtml = `
            <div class="sim-window-body web-theme">
              <div class="browser-address-bar">
                <span class="ssl-lock">🔒</span>
                <span class="url-text">https://winscroller2d.local/docs/overview</span>
              </div>
              <div class="browser-content">
                <h4>Live DWM Thumbnails &amp; Acrylic</h4>
                <p>Fast spatial navigation without rendering lag across all managed windows.</p>
              </div>
            </div>`;
        } else if (cell.type === 'chat') {
          bodyHtml = `
            <div class="sim-window-body chat-theme">
              <div class="chat-msg">
                <span class="chat-user">@zammy:</span>
                <span class="chat-text">Handling 100 windows effortlessly with DWM cloaking!</span>
              </div>
              <div class="chat-msg">
                <span class="chat-user">@system:</span>
                <span class="chat-text">All 24 unit layout tests passing.</span>
              </div>
            </div>`;
        } else {
          bodyHtml = `
            <div class="sim-window-body monitor-theme">
              <div class="monitor-metric"><span>RAM Usage:</span><strong>24.2 MB (Native Rust)</strong></div>
              <div class="monitor-metric"><span>CPU Idle:</span><strong>&lt; 0.1% (Zero Polling)</strong></div>
              <div class="sim-oscilloscope-bar"><div class="wave-anim"></div></div>
            </div>`;
        }

        el.innerHTML = `
          <div class="sim-window-header">
            <div class="sim-window-title">
              <span class="win-app-icon">${cell.icon}</span>
              <span>${cell.title}</span>
            </div>
            <div class="sim-window-badges">
              <span class="win-coord-badge ${cell.col === this.currentCol && cell.row === this.currentRow ? 'active' : ''}">(${cell.col}, ${cell.row})</span>
            </div>
          </div>
          ${bodyHtml}
        `;

        // Click to focus or expand
        el.addEventListener('click', () => {
          if (this.isOverview) {
            this.currentCol = cell.col;
            this.currentRow = cell.row;
            this.isOverview = false;
            sfx.overview();
            const btn = document.getElementById('btn-toggle-overview');
            if (btn) btn.classList.remove('active');
            this.updateLayout();
          } else {
            this.focusCell(cell.col, cell.row);
          }
        });

        // Drag and drop snapping
        el.addEventListener('dragstart', (e) => {
          if (!this.isOverview) {
            e.preventDefault();
            return;
          }
          e.dataTransfer.setData('text/plain', cell.id);
          sfx.click();
        });

        el.addEventListener('dragover', (e) => {
          if (this.isOverview) {
            e.preventDefault();
            el.classList.add('snap-target-highlight');
          }
        });

        el.addEventListener('dragleave', () => {
          el.classList.remove('snap-target-highlight');
        });

        el.addEventListener('drop', (e) => {
          e.preventDefault();
          el.classList.remove('snap-target-highlight');
          const draggedId = e.dataTransfer.getData('text/plain');
          if (draggedId && draggedId !== cell.id) {
            sfx.snap();
            showToast(`Snapped ${draggedId} and ${cell.id} into 50/50 dual view!`, '🪟');
          }
        });

        this.canvas.appendChild(el);
      });
    },

    updateLayout() {
      const colSpacing = 480;
      const rowSpacing = 360;

      // Position each window cell
      this.cells.forEach(cell => {
        const el = document.getElementById(cell.id);
        if (el) {
          el.style.left = `${(cell.col) * colSpacing}px`;
          el.style.top = `${cell.row * rowSpacing}px`;
          el.style.width = `${cell.baseWidth}px`;
          el.style.height = `${cell.baseHeight}px`;

          // Focus state
          if (cell.col === this.currentCol && cell.row === this.currentRow) {
            el.classList.add('active-focus');
          } else {
            el.classList.remove('active-focus');
          }
        }
      });

      // Update camera position
      if (this.isOverview) {
        // Dynamic zoom scale depending on window density!
        if (this.activeCount === 100) {
          this.scale = 0.16;
        } else if (this.activeCount === 50) {
          this.scale = 0.24;
        } else if (this.activeCount === 25) {
          this.scale = 0.32;
        } else {
          this.scale = 0.42;
        }

        this.camX = 0;
        this.camY = 0;
        this.viewport.classList.add('overview-mode');
        this.modeTag.innerText = `Mode: WIN+O OVERVIEW (${this.activeCount} WINS)`;
        this.modeTag.style.color = '#c084fc';
        this.modeTag.style.borderColor = 'rgba(168, 85, 247, 0.4)';
      } else {
        // Focus active cell
        this.scale = 1.0;
        this.camX = -(this.currentCol) * colSpacing;
        this.camY = -this.currentRow * rowSpacing;
        this.viewport.classList.remove('overview-mode');
        this.modeTag.innerText = 'Mode: TILING CANVAS';
        this.modeTag.style.color = '#00f5d4';
        this.modeTag.style.borderColor = 'rgba(0, 245, 212, 0.3)';
      }

      this.canvas.style.transform = `translate(calc(-50% + ${this.camX}px), calc(-50% + ${this.camY}px)) scale(${this.scale})`;
      this.coordsDisplay.innerHTML = `Camera Focus: <strong>Col: ${this.currentCol}, Row: ${this.currentRow}</strong>`;
    },

    focusCell(col, row) {
      // Find if cell exists
      const exists = this.cells.some(c => c.col === col && c.row === row);
      if (exists) {
        this.currentCol = col;
        this.currentRow = row;
        sfx.nav();
        this.updateLayout();
      } else {
        // Elastic bounce bump on boundary
        sfx.playTone(220, 'sine', 0.05, 0.03);
      }
    },

    toggleOverview() {
      this.isOverview = !this.isOverview;
      sfx.overview();
      const btn = document.getElementById('btn-toggle-overview');
      if (btn) btn.classList.toggle('active', this.isOverview);
      this.updateLayout();
    },

    toggleSplit() {
      this.isTabbedStack = !this.isTabbedStack;
      sfx.snap();
      const activeCellEl = document.getElementById(`win-dyn-${this.currentCol}-${this.currentRow}`) || document.getElementById('win-terminal');
      if (activeCellEl) {
        if (this.isTabbedStack) {
          activeCellEl.style.borderStyle = 'dashed';
          showToast('Cell mode: Tabbed Stack active (Win+W)');
        } else {
          activeCellEl.style.borderStyle = 'solid';
          showToast('Cell mode: Vertical Split active (Win+W)');
        }
      }
    },

    consumeWindow() {
      sfx.snap();
      showToast(`Consumed adjacent window into Cell (${this.currentCol}, ${this.currentRow}) stack!`);
    },

    cycleWidth() {
      this.widthPresetIndex = (this.widthPresetIndex + 1) % this.widthPresets.length;
      const newWidth = this.widthPresets[this.widthPresetIndex];
      const activeCell = this.cells.find(c => c.col === this.currentCol && c.row === this.currentRow);
      if (activeCell) {
        activeCell.baseWidth = newWidth;
        sfx.snap();
        this.updateLayout();
        showToast(`Cell width stepped to ${newWidth}px (Win+=)`);
      }
    },

    centerCamera() {
      sfx.nav();
      this.isOverview = false;
      const btn = document.getElementById('btn-toggle-overview');
      if (btn) btn.classList.remove('active');
      this.currentCol = 0;
      this.currentRow = 0;
      this.updateLayout();
      showToast('Camera centered on default focus (0, 0)');
    },

    setupEventListeners() {
      // Density selector buttons
      document.querySelectorAll('.density-btn').forEach(btn => {
        btn.addEventListener('click', () => {
          document.querySelectorAll('.density-btn').forEach(b => b.classList.remove('active'));
          btn.classList.add('active');
          const count = parseInt(btn.dataset.count, 10);
          this.setWindowCount(count);
        });
      });

      // Topology buttons
      document.querySelectorAll('.topology-btn').forEach(btn => {
        btn.addEventListener('click', () => {
          document.querySelectorAll('.topology-btn').forEach(b => b.classList.remove('active'));
          btn.classList.add('active');
          this.setTopology(btn.dataset.topo);
        });
      });

      // Virtual Desktop buttons
      document.querySelectorAll('.vdesk-btn').forEach(btn => {
        btn.addEventListener('click', () => {
          document.querySelectorAll('.vdesk-btn').forEach(b => b.classList.remove('active'));
          btn.classList.add('active');
          this.setVirtualDesktop(parseInt(btn.dataset.desk, 10));
        });
      });

      // Action Buttons
      document.getElementById('btn-toggle-overview')?.addEventListener('click', () => this.toggleOverview());
      document.getElementById('btn-toggle-split')?.addEventListener('click', () => this.toggleSplit());
      document.getElementById('btn-consume-window')?.addEventListener('click', () => this.consumeWindow());
      document.getElementById('btn-resize-step')?.addEventListener('click', () => this.cycleWidth());
      document.getElementById('btn-reset-cam')?.addEventListener('click', () => this.centerCamera());

      // Touch D-Pad
      document.getElementById('dpad-up')?.addEventListener('click', () => this.focusCell(this.currentCol, this.currentRow - 1));
      document.getElementById('dpad-down')?.addEventListener('click', () => this.focusCell(this.currentCol, this.currentRow + 1));
      document.getElementById('dpad-left')?.addEventListener('click', () => this.focusCell(this.currentCol - 1, this.currentRow));
      document.getElementById('dpad-right')?.addEventListener('click', () => this.focusCell(this.currentCol + 1, this.currentRow));

      // Global Keyboard Controls
      window.addEventListener('keydown', (e) => {
        if (['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement.tagName)) return;

        if (e.key === 'ArrowUp') {
          e.preventDefault();
          this.focusCell(this.currentCol, this.currentRow - 1);
        } else if (e.key === 'ArrowDown') {
          e.preventDefault();
          this.focusCell(this.currentCol, this.currentRow + 1);
        } else if (e.key === 'ArrowLeft') {
          e.preventDefault();
          this.focusCell(this.currentCol - 1, this.currentRow);
        } else if (e.key === 'ArrowRight') {
          e.preventDefault();
          this.focusCell(this.currentCol + 1, this.currentRow);
        } else if (e.key === 'o' || e.key === 'O') {
          this.toggleOverview();
        } else if (e.key === 'w' || e.key === 'W') {
          this.toggleSplit();
        } else if (e.key === 'c' || e.key === 'C') {
          this.centerCamera();
        }
      });

      document.getElementById('hud-close-btn')?.addEventListener('click', () => {
        if (this.overviewHud) this.overviewHud.style.display = 'none';
      });
    }
  };

  sim.init();

  // ==========================================================================
  // 7. 120HZ SPRING PHYSICS LAB & OSCILLOSCOPE
  // ==========================================================================
  const physicsLab = {
    canvas: document.getElementById('physics-canvas'),
    stiffnessInput: document.getElementById('input-stiffness'),
    dampingInput: document.getElementById('input-damping'),
    massInput: document.getElementById('input-mass'),
    valStiffness: document.getElementById('val-stiffness'),
    valDamping: document.getElementById('val-damping'),
    valMass: document.getElementById('val-mass'),
    settleTimeDisplay: document.getElementById('stat-settle-time'),
    dampingRatioDisplay: document.getElementById('stat-damping-ratio'),
    testWindow: document.getElementById('spring-test-window'),
    btnFire: document.getElementById('btn-fire-impulse'),
    btnReset: document.getElementById('btn-reset-physics'),

    init() {
      if (!this.canvas) return;
      this.ctx = this.canvas.getContext('2d');
      this.setupListeners();
      this.render();
    },

    setupListeners() {
      const update = () => {
        this.valStiffness.innerText = this.stiffnessInput.value;
        this.valDamping.innerText = this.dampingInput.value;
        this.valMass.innerText = Number(this.massInput.value).toFixed(1);
        this.render();
      };

      this.stiffnessInput?.addEventListener('input', update);
      this.dampingInput?.addEventListener('input', update);
      this.massInput?.addEventListener('input', update);

      this.btnFire?.addEventListener('click', () => this.fireImpulse());
      this.btnReset?.addEventListener('click', () => {
        this.stiffnessInput.value = 380;
        this.dampingInput.value = 24;
        this.massInput.value = 1.0;
        update();
        sfx.click();
      });
    },

    fireImpulse() {
      if (!this.testWindow) return;
      sfx.snap();

      const k = parseFloat(this.stiffnessInput.value);
      const c = parseFloat(this.dampingInput.value);
      const m = parseFloat(this.massInput.value);

      // Simple real-time spring animation loop for test window
      let x = 20;
      let v = 550; // impulse velocity
      const target = 160;
      let lastTime = performance.now();

      const step = (time) => {
        const dt = Math.min((time - lastTime) / 1000, 0.032);
        lastTime = time;

        const fSpring = -k * (x - target);
        const fDamper = -c * v;
        const a = (fSpring + fDamper) / m;

        v += a * dt;
        x += v * dt;

        this.testWindow.style.left = `${Math.round(x)}px`;

        if (Math.abs(x - target) > 0.5 || Math.abs(v) > 2) {
          requestAnimationFrame(step);
        } else {
          this.testWindow.style.left = `${target}px`;
        }
      };

      requestAnimationFrame(step);
    },

    render() {
      const w = this.canvas.width = this.canvas.clientWidth;
      const h = this.canvas.height = this.canvas.clientHeight;
      const ctx = this.ctx;

      const k = parseFloat(this.stiffnessInput.value);
      const c = parseFloat(this.dampingInput.value);
      const m = parseFloat(this.massInput.value);

      // Calculations: zeta = c / (2 * sqrt(m * k))
      const omegaN = Math.sqrt(k / m);
      const zeta = c / (2 * Math.sqrt(m * k));
      const settleTime = zeta > 0 ? (4 / (zeta * omegaN)) * 1000 : 999;

      this.settleTimeDisplay.innerText = `${Math.round(settleTime)} ms`;
      let type = 'Underdamped';
      if (zeta >= 1.0) type = 'Overdamped';
      else if (Math.abs(zeta - 1.0) < 0.05) type = 'Critically Damped';
      this.dampingRatioDisplay.innerText = `${zeta.toFixed(2)} (${type})`;

      ctx.clearRect(0, 0, w, h);

      // Target centerline
      const targetY = h * 0.45;
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
      ctx.setLineDash([4, 4]);
      ctx.beginPath();
      ctx.moveTo(0, targetY);
      ctx.lineTo(w, targetY);
      ctx.stroke();
      ctx.setLineDash([]);

      // Draw Linear Lerp Reference Curve (Standard WM)
      ctx.strokeStyle = '#64748b';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      const lerpDuration = w * 0.5;
      ctx.moveTo(0, h * 0.85);
      ctx.lineTo(lerpDuration, targetY);
      ctx.lineTo(w, targetY);
      ctx.stroke();

      // Draw WinScroller 2D Damped Harmonic Wave
      ctx.strokeStyle = '#00f5d4';
      ctx.lineWidth = 2.5;
      ctx.shadowColor = 'rgba(0, 245, 212, 0.4)';
      ctx.shadowBlur = 8;
      ctx.beginPath();

      const startY = h * 0.85;
      const amplitude = startY - targetY;
      const omegaD = omegaN * Math.sqrt(Math.max(0, 1 - zeta * zeta));

      for (let px = 0; px < w; px++) {
        const t = (px / w) * 0.6; // 600ms time horizon
        let val;
        if (zeta < 1.0) {
          val = targetY + amplitude * Math.exp(-zeta * omegaN * t) * Math.cos(omegaD * t);
        } else {
          val = targetY + amplitude * Math.exp(-zeta * omegaN * t);
        }
        if (px === 0) ctx.moveTo(px, val);
        else ctx.lineTo(px, val);
      }
      ctx.stroke();
      ctx.shadowBlur = 0;
    }
  };

  physicsLab.init();

  // ==========================================================================
  // 8. VISUAL MEDIA GALLERY TABS & LIGHTBOX
  // ==========================================================================
  const galleryTabs = document.querySelectorAll('.gallery-tab');
  const galleryPanels = document.querySelectorAll('.gallery-panel');

  galleryTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      sfx.click();
      galleryTabs.forEach(t => t.classList.remove('active'));
      galleryPanels.forEach(p => p.classList.remove('active'));

      tab.classList.add('active');
      const targetId = tab.getAttribute('data-tab');
      document.getElementById(targetId)?.classList.add('active');
    });
  });

  // Lightbox
  const lightboxModal = document.getElementById('lightbox-modal');
  const lightboxImg = document.getElementById('lightbox-img');
  const lightboxClose = document.getElementById('lightbox-close');

  document.querySelectorAll('[data-lightbox]').forEach(wrapper => {
    wrapper.addEventListener('click', () => {
      const src = wrapper.getAttribute('data-lightbox');
      if (src && lightboxModal && lightboxImg) {
        sfx.click();
        lightboxImg.src = src;
        lightboxModal.classList.add('active');
      }
    });
  });

  function closeLightbox() {
    if (lightboxModal) lightboxModal.classList.remove('active');
  }

  lightboxClose?.addEventListener('click', closeLightbox);
  lightboxModal?.querySelector('.lightbox-backdrop')?.addEventListener('click', closeLightbox);
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') closeLightbox();
  });

  // ==========================================================================
  // 9. INTERACTIVE KEYBINDINGS STUDIO & VIRTUAL KEYBOARD
  // ==========================================================================
  const kbSearchInput = document.getElementById('kb-search-input');
  const kbCatButtons = document.querySelectorAll('.kb-cat-btn');
  const kbCards = document.querySelectorAll('.kb-card');
  const virtualKbKeys = document.querySelectorAll('.vk-key');

  function filterCards() {
    const query = (kbSearchInput?.value || '').toLowerCase().trim();
    const activeCatBtn = document.querySelector('.kb-cat-btn.active');
    const category = activeCatBtn ? activeCatBtn.getAttribute('data-category') : 'all';

    kbCards.forEach(card => {
      const cardCat = card.getAttribute('data-cat');
      const title = card.querySelector('.kb-title')?.innerText.toLowerCase() || '';
      const desc = card.querySelector('.kb-desc')?.innerText.toLowerCase() || '';
      const keys = card.getAttribute('data-keys')?.toLowerCase() || '';

      const matchCat = category === 'all' || cardCat === category;
      const matchQuery = !query || title.includes(query) || desc.includes(query) || keys.includes(query);

      card.style.display = matchCat && matchQuery ? 'block' : 'none';
    });
  }

  kbSearchInput?.addEventListener('input', filterCards);

  kbCatButtons.forEach(btn => {
    btn.addEventListener('click', () => {
      sfx.click();
      kbCatButtons.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      filterCards();
    });
  });

  // Highlight virtual keys when hovering / clicking a keybinding card
  kbCards.forEach(card => {
    card.addEventListener('mouseenter', () => {
      const rawKeys = card.getAttribute('data-keys') || '';
      const keyList = rawKeys.split(' ');
      virtualKbKeys.forEach(vk => {
        const vkData = vk.getAttribute('data-key');
        if (keyList.includes(vkData)) {
          vk.classList.add('highlight');
        } else {
          vk.classList.remove('highlight');
        }
      });
    });

    card.addEventListener('mouseleave', () => {
      virtualKbKeys.forEach(vk => vk.classList.remove('highlight'));
    });

    card.addEventListener('click', () => {
      sfx.key();
      const title = card.querySelector('.kb-title')?.innerText || '';
      showToast(`Selected shortcut: ${title}`, '⌨️');
    });
  });

  // Highlight physical keypress on virtual keyboard
  window.addEventListener('keydown', (e) => {
    virtualKbKeys.forEach(vk => {
      if (vk.getAttribute('data-key')?.toLowerCase() === e.key.toLowerCase() ||
          vk.getAttribute('data-key')?.toLowerCase() === e.code.toLowerCase()) {
        vk.classList.add('highlight');
        setTimeout(() => vk.classList.remove('highlight'), 250);
      }
    });
  });

  // ==========================================================================
  // 10. LIVE CONFIGURATION STUDIO (winscroller2d.toml)
  // ==========================================================================
  const configStudio = {
    stiffness: document.getElementById('cfg-stiffness'),
    damping: document.getElementById('cfg-damping'),
    peekX: document.getElementById('cfg-peek-x'),
    peekY: document.getElementById('cfg-peek-y'),
    gap: document.getElementById('cfg-gap'),
    blurMode: document.getElementById('cfg-blur-mode'),
    borderColor: document.getElementById('cfg-border-color'),
    zeroSpill: document.getElementById('cfg-zero-spill'),

    valStiffness: document.getElementById('cfg-val-stiffness'),
    valDamping: document.getElementById('cfg-val-damping'),
    valPeekX: document.getElementById('cfg-val-peek-x'),
    valPeekY: document.getElementById('cfg-val-peek-y'),
    valGap: document.getElementById('cfg-val-gap'),
    colorCode: document.getElementById('cfg-color-code'),
    liveCode: document.getElementById('live-toml-code'),

    presets: {
      balanced: { stiffness: 380, damping: 24, peekX: 64, peekY: 48, gap: 8, blur: 'acrylic_clear', color: '#00f5d4', zero: true },
      gaming: { stiffness: 680, damping: 36, peekX: 0, peekY: 0, gap: 4, blur: 'disabled', color: '#ff5722', zero: true },
      ultrawide: { stiffness: 320, damping: 22, peekX: 120, peekY: 60, gap: 12, blur: 'pure_blur', color: '#a855f7', zero: true },
      minimal: { stiffness: 420, damping: 28, peekX: 20, peekY: 20, gap: 0, blur: 'pure_blur', color: '#ffffff', zero: true }
    },

    init() {
      if (!this.liveCode) return;
      this.setupListeners();
      this.updateToml();
    },

    setupListeners() {
      const inputs = [this.stiffness, this.damping, this.peekX, this.peekY, this.gap, this.blurMode, this.borderColor, this.zeroSpill];
      inputs.forEach(input => {
        input?.addEventListener('input', () => this.updateToml());
        input?.addEventListener('change', () => this.updateToml());
      });

      // Preset buttons
      document.querySelectorAll('.preset-btn').forEach(btn => {
        btn.addEventListener('click', () => {
          sfx.click();
          document.querySelectorAll('.preset-btn').forEach(b => b.classList.remove('active'));
          btn.classList.add('active');
          const p = this.presets[btn.getAttribute('data-preset')];
          if (p) this.applyPreset(p);
        });
      });

      // Copy TOML
      document.getElementById('btn-copy-toml')?.addEventListener('click', () => {
        sfx.click();
        navigator.clipboard.writeText(this.liveCode.innerText).then(() => {
          showToast('Configuration copied to clipboard!');
        });
      });

      // Download TOML
      document.getElementById('btn-download-toml')?.addEventListener('click', () => {
        sfx.click();
        const blob = new Blob([this.liveCode.innerText], { type: 'text/toml;charset=utf-8' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'winscroller2d.toml';
        a.click();
        URL.revokeObjectURL(url);
        showToast('Downloaded winscroller2d.toml!');
      });
    },

    applyPreset(p) {
      this.stiffness.value = p.stiffness;
      this.damping.value = p.damping;
      this.peekX.value = p.peekX;
      this.peekY.value = p.peekY;
      this.gap.value = p.gap;
      this.blurMode.value = p.blur;
      this.borderColor.value = p.color;
      this.zeroSpill.checked = p.zero;
      this.updateToml();
    },

    updateToml() {
      this.valStiffness.innerText = this.stiffness.value;
      this.valDamping.innerText = this.damping.value;
      this.valPeekX.innerText = `${this.peekX.value}px`;
      this.valPeekY.innerText = `${this.peekY.value}px`;
      this.valGap.innerText = `${this.gap.value}px`;
      this.colorCode.innerText = this.borderColor.value;

      const toml = `# Generated by WinScroller 2D Config Studio
version = "0.1.0"

[physics]
stiffness = ${parseFloat(this.stiffness.value).toFixed(1)}
damping = ${parseFloat(this.damping.value).toFixed(1)}
mass = 1.0
fps_target = 120

[layout]
peek_x = ${this.peekX.value}
peek_y = ${this.peekY.value}
gap = ${this.gap.value}
zero_spillover = ${this.zeroSpill.checked}
spawn_mode = "smart_nearest"

[overview]
blur_mode = "${this.blurMode.value}"
drag_to_split = true
show_dwm_thumbnails = true
badge_icons = true

[theme]
border_color = "${this.borderColor.value}"
border_width = 2
inactive_border_color = "#334155"`;

      this.liveCode.innerText = toml;
    }
  };

  configStudio.init();

  // ==========================================================================
  // 11. FAQ ACCORDION
  // ==========================================================================
  document.querySelectorAll('.faq-question').forEach(btn => {
    btn.addEventListener('click', () => {
      sfx.click();
      const item = btn.closest('.faq-item');
      item?.classList.toggle('active');
    });
  });

  // ==========================================================================
  // 12. 3D TILT EFFECT ON CARDS
  // ==========================================================================
  const tiltCards = document.querySelectorAll('.tilt-target, #hero-tilt-card');
  tiltCards.forEach(card => {
    card.addEventListener('mousemove', (e) => {
      const rect = card.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      const centerX = rect.width / 2;
      const centerY = rect.height / 2;
      const rotateX = ((y - centerY) / centerY) * -7;
      const rotateY = ((x - centerX) / centerX) * 7;

      card.style.transform = `perspective(1000px) rotateX(${rotateX.toFixed(2)}deg) rotateY(${rotateY.toFixed(2)}deg) translateY(-4px)`;
    });

    card.addEventListener('mouseleave', () => {
      card.style.transform = 'perspective(1000px) rotateX(0deg) rotateY(0deg) translateY(0px)';
    });
  });

  // ==========================================================================
  // 13. STAT COUNTERS ANIMATION ON SCROLL
  // ==========================================================================
  const counters = document.querySelectorAll('.counter');
  let animated = false;

  function runCounters() {
    if (animated) return;
    const heroStats = document.querySelector('.hero-stats-row');
    if (!heroStats) return;

    const rect = heroStats.getBoundingClientRect();
    if (rect.top < window.innerHeight && rect.bottom >= 0) {
      animated = true;
      counters.forEach(counter => {
        const target = +counter.getAttribute('data-target');
        let count = 0;
        const inc = Math.max(1, Math.ceil(target / 40));
        const timer = setInterval(() => {
          count += inc;
          if (count >= target) {
            counter.innerText = target;
            clearInterval(timer);
          } else {
            counter.innerText = count;
          }
        }, 30);
      });
    }
  }

  window.addEventListener('scroll', runCounters);
  runCounters();

  // ==========================================================================
  // 14. SOUND & PERFORMANCE TOGGLE BUTTONS
  // ==========================================================================
  const soundBtn = document.getElementById('sound-toggle');
  if (soundBtn) {
    soundBtn.addEventListener('click', () => {
      sfx.enabled = !sfx.enabled;
      soundBtn.querySelector('.btn-text').innerText = sfx.enabled ? 'SFX: ON' : 'SFX: OFF';
      soundBtn.querySelector('.icon-speaker').innerText = sfx.enabled ? '🔊' : '🔇';
      if (sfx.enabled) sfx.click();
    });
  }

  const perfBtn = document.getElementById('perf-toggle');
  if (perfBtn) {
    let mode120 = true;
    perfBtn.addEventListener('click', () => {
      mode120 = !mode120;
      sfx.click();
      perfBtn.querySelector('.btn-text').innerText = mode120 ? '120Hz Spring' : '60Hz Eco';
      showToast(mode120 ? '120Hz physics sampling active' : '60Hz battery saver active');
    });
  }

  // Mobile menu toggle
  const mobileToggle = document.getElementById('mobile-toggle');
  const navLinks = document.getElementById('nav-links');
  if (mobileToggle && navLinks) {
    mobileToggle.addEventListener('click', () => {
      sfx.click();
      const open = navLinks.style.display === 'flex';
      navLinks.style.display = open ? 'none' : 'flex';
      if (!open) {
        navLinks.style.flexDirection = 'column';
        navLinks.style.position = 'absolute';
        navLinks.style.top = '100%';
        navLinks.style.left = '0';
        navLinks.style.width = '100%';
        navLinks.style.background = '#07090e';
        navLinks.style.padding = '20px';
        navLinks.style.borderBottom = '1px solid rgba(255,255,255,0.1)';
      }
    });
  }

})();
