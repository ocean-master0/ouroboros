// Project Ouroboros — Dashboard Controller
const MAX_POINTS = 60;
const labels = Array.from({ length: MAX_POINTS }, (_, i) => `${MAX_POINTS - 1 - i}s`);
const rpsData = Array(MAX_POINTS).fill(0);
const totalRpsData = Array(MAX_POINTS).fill(0);
const errorData = Array(MAX_POINTS).fill(0);

let rpsChart = null;
let errorChart = null;

function createGradient(ctx, color1, color2) {
  const gradient = ctx.createLinearGradient(0, 0, 0, 260);
  gradient.addColorStop(0, color1);
  gradient.addColorStop(1, color2);
  return gradient;
}

function initCharts() {
  if (typeof Chart === 'undefined') {
    console.warn('Chart.js not loaded, charts disabled.');
    return;
  }

  // Configure Chart.js global defaults
  Chart.defaults.color = '#94a3b8';
  Chart.defaults.font.family = "'JetBrains Mono', monospace";
  Chart.defaults.font.size = 11;

  const rpsCanvas = document.getElementById('rpsChart');
  if (rpsCanvas) {
    const ctx = rpsCanvas.getContext('2d');
    const cyanGrad = createGradient(ctx, 'rgba(6, 182, 212, 0.25)', 'rgba(6, 182, 212, 0.0)');
    const emeraldGrad = createGradient(ctx, 'rgba(16, 185, 129, 0.15)', 'rgba(16, 185, 129, 0.0)');

    rpsChart = new Chart(rpsCanvas, {
      type: 'line',
      data: {
        labels: labels,
        datasets: [
          {
            label: 'Ping RPS (Hot Path)',
            data: rpsData,
            borderColor: '#06b6d4',
            backgroundColor: cyanGrad,
            borderWidth: 2.5,
            tension: 0.3,
            fill: true,
            pointRadius: 0,
            pointHoverRadius: 4,
            pointHoverBackgroundColor: '#06b6d4'
          },
          {
            label: 'Total RPS (Engine)',
            data: totalRpsData,
            borderColor: '#10b981',
            backgroundColor: emeraldGrad,
            borderWidth: 2,
            tension: 0.3,
            fill: false,
            pointRadius: 0,
            pointHoverRadius: 4,
            pointHoverBackgroundColor: '#10b981'
          }
        ]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        animation: false,
        interaction: {
          mode: 'index',
          intersect: false
        },
        plugins: {
          legend: {
            position: 'top',
            align: 'end',
            labels: {
              boxWidth: 10,
              boxHeight: 10,
              usePointStyle: true,
              pointStyle: 'circle',
              color: '#94a3b8',
              padding: 15
            }
          },
          tooltip: {
            backgroundColor: 'rgba(13, 19, 33, 0.95)',
            titleColor: '#f8fafc',
            bodyColor: '#94a3b8',
            borderColor: 'rgba(255, 255, 255, 0.1)',
            borderWidth: 1,
            padding: 10,
            cornerRadius: 8,
            displayColors: true
          }
        },
        scales: {
          x: {
            display: false,
            grid: { color: 'rgba(255,255,255,0.03)' }
          },
          y: {
            beginAtZero: true,
            grid: { color: 'rgba(255,255,255,0.04)' },
            ticks: {
              color: '#64748b',
              callback: (val) => Number(val).toLocaleString()
            }
          }
        }
      }
    });
  }

  const errorCanvas = document.getElementById('errorChart');
  if (errorCanvas) {
    const ctx = errorCanvas.getContext('2d');
    const roseGrad = createGradient(ctx, 'rgba(244, 63, 94, 0.25)', 'rgba(244, 63, 94, 0.0)');

    errorChart = new Chart(errorCanvas, {
      type: 'line',
      data: {
        labels: labels,
        datasets: [{
          label: 'Error %',
          data: errorData,
          borderColor: '#f43f5e',
          backgroundColor: roseGrad,
          borderWidth: 2,
          tension: 0.3,
          fill: true,
          pointRadius: 0
        }]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        animation: false,
        plugins: {
          legend: {
            position: 'top',
            align: 'end',
            labels: {
              boxWidth: 10,
              boxHeight: 10,
              usePointStyle: true,
              color: '#94a3b8'
            }
          },
          tooltip: {
            backgroundColor: 'rgba(13, 19, 33, 0.95)',
            titleColor: '#f8fafc',
            bodyColor: '#f43f5e',
            borderColor: 'rgba(244, 63, 94, 0.3)',
            borderWidth: 1,
            padding: 10,
            cornerRadius: 8
          }
        },
        scales: {
          x: { display: false },
          y: {
            beginAtZero: true,
            max: 100,
            grid: { color: 'rgba(255,255,255,0.04)' },
            ticks: {
              color: '#64748b',
              callback: (val) => val + '%'
            }
          }
        }
      }
    });
  }
}

function updateCharts(m) {
  rpsData.push(m.ping_rps || 0);
  rpsData.shift();

  totalRpsData.push(m.total_rps || 0);
  totalRpsData.shift();

  errorData.push(m.error_rate_pct || 0);
  errorData.shift();

  if (rpsChart) rpsChart.update('none');
  if (errorChart) errorChart.update('none');
}

// WebSocket Connection with auto-reconnect
function connect() {
  const wsStatus = document.getElementById('ws-status');
  const wsStatusText = document.getElementById('ws-status-text');
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const wsUrl = `${protocol}//${location.host}/ws/metrics`;

  const ws = new WebSocket(wsUrl);

  ws.onopen = () => {
    if (wsStatus) {
      wsStatus.className = 'pill-badge pill-ws';
      if (wsStatusText) wsStatusText.textContent = 'LIVE WS';
    }
  };

  ws.onmessage = (e) => {
    try {
      const m = JSON.parse(e.data);
      updateCharts(m);

      const rpsEl = document.getElementById('rps');
      if (rpsEl) rpsEl.textContent = Number(m.ping_rps || 0).toLocaleString();

      const totalRpsEl = document.getElementById('total-rps');
      if (totalRpsEl) totalRpsEl.textContent = Number(m.total_rps || 0).toLocaleString();

      const errorEl = document.getElementById('error-rate');
      if (errorEl) errorEl.textContent = (m.error_rate_pct || 0).toFixed(2) + '%';

      const connsEl = document.getElementById('active-conns');
      if (connsEl) connsEl.textContent = Number(m.active_conns || 0).toLocaleString();

      const targetEl = document.getElementById('target');
      if (targetEl) {
        if (m.ping_rps >= 1000000) {
          targetEl.className = 'pill-badge reached';
          targetEl.innerHTML = `<span class="pulse-dot"></span><span>🎯 TARGET REACHED</span>`;
        } else {
          targetEl.className = 'pill-badge';
          targetEl.innerHTML = `<span>⏳ Building...</span>`;
        }
      }
    } catch (err) {
      console.error('Failed to parse metric snapshot:', err);
    }
  };

  ws.onclose = () => {
    if (wsStatus) {
      wsStatus.className = 'pill-badge pill-ws disconnected';
      if (wsStatusText) wsStatusText.textContent = 'OFFLINE';
    }
    setTimeout(connect, 1000); // DR05 — auto-reconnect
  };

  ws.onerror = () => {
    ws.close();
  };
}

// Quick Presets
window.applyPreset = function(mode, concurrency, duration) {
  const modeEl = document.getElementById('bench-mode');
  const concEl = document.getElementById('bench-concurrency');
  const durEl = document.getElementById('bench-duration');
  if (modeEl) modeEl.value = mode;
  if (concEl) concEl.value = concurrency;
  if (durEl) durEl.value = duration;
};

// Live Ping Latency Probe
window.runLivePing = async function() {
  const badge = document.getElementById('ping-latency-badge');
  const btn = document.getElementById('quick-ping-btn');
  if (!badge) return;

  const t0 = performance.now();
  badge.textContent = 'PROBING...';
  badge.style.color = '#06b6d4';

  try {
    const res = await fetch('/api/v1/ping');
    const t1 = performance.now();
    const rtt = (t1 - t0).toFixed(1);
    if (res.ok) {
      badge.textContent = `${rtt} ms (HTTP 200)`;
      badge.style.color = '#10b981';
      badge.style.borderColor = 'rgba(16, 185, 129, 0.4)';
    } else {
      badge.textContent = `HTTP ${res.status}`;
      badge.style.color = '#f43f5e';
    }
  } catch (err) {
    badge.textContent = 'FAILED';
    badge.style.color = '#f43f5e';
  }
};

// Copy Endpoint Path helper
window.copyEndpoint = function(path, btn) {
  navigator.clipboard.writeText(path).then(() => {
    const originalHTML = btn.innerHTML;
    btn.innerHTML = `<span style="color: #10b981; font-size: 11px;">✓ Copied</span>`;
    setTimeout(() => {
      btn.innerHTML = originalHTML;
    }, 1500);
  });
};

// Bench Controls Init
document.addEventListener('DOMContentLoaded', () => {
  initCharts();
  connect();

  const startBtn = document.getElementById('bench-start-btn');
  const stopBtn = document.getElementById('bench-stop-btn');
  const statusEl = document.getElementById('bench-status');

  if (startBtn) {
    startBtn.addEventListener('click', async () => {
      const mode = document.getElementById('bench-mode')?.value || 'direct';
      const concurrency = parseInt(document.getElementById('bench-concurrency')?.value || '100', 10);
      const duration_secs = parseInt(document.getElementById('bench-duration')?.value || '10', 10);

      try {
        const res = await fetch('/internal/bench/start', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ mode, concurrency, duration_secs })
        });
        if (res.ok) {
          if (statusEl) {
            statusEl.innerHTML = `<span class="pulse-dot" style="background: var(--cyan); box-shadow: 0 0 10px var(--cyan);"></span> Status: Running (${mode}, ${concurrency} workers, ${duration_secs}s)`;
            statusEl.style.color = 'var(--cyan)';
          }
        } else {
          const err = await res.json();
          alert(`Bench start failed: ${err.error || res.statusText}`);
        }
      } catch (e) {
        alert(`Bench request error: ${e.message}`);
      }
    });
  }

  if (stopBtn) {
    stopBtn.addEventListener('click', async () => {
      try {
        const res = await fetch('/internal/bench/stop', { method: 'POST' });
        if (res.ok && statusEl) {
          statusEl.innerHTML = `<span class="pulse-dot" style="background: var(--rose); box-shadow: 0 0 10px var(--rose);"></span> Status: Stopped`;
          statusEl.style.color = 'var(--rose)';
          setTimeout(() => {
            if (statusEl) {
              statusEl.innerHTML = `<span class="pulse-dot" style="background: var(--text-muted); box-shadow: none;"></span> Status: Idle`;
              statusEl.style.color = 'var(--text-secondary)';
            }
          }, 3000);
        }
      } catch (e) {
        console.error('Failed to stop bench:', e);
      }
    });
  }
});
