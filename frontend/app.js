/**
 * SSHTerm — 前端主逻辑
 * Tauri 2 + xterm.js 桌面 SSH/SFTP 客户端
 */
import { invoke } from '@tauri-apps/api/core';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { WebglAddon } from 'xterm-addon-webgl';
import { SearchAddon } from 'xterm-addon-search';

// ─── 状态 ────────────────────────────────────────────────────────────────

const state = {
  hosts: [],
  groups: [],
  sessions: {},        // { sessionId: { host, terminal, fitAddon, ws } }
  activeSession: null,
  sftp: {
    remotePath: '/',
    localPath: '',
    remoteFiles: [],
    localFiles: [],
    selectedRemote: null,
    selectedLocal: null,
  },
};

let ws = null; // Terminal WebSocket

// ─── 初始化 ──────────────────────────────────────────────────────────────

async function init() {
  // 获取本地默认路径
  state.sftp.localPath = '/home';

  await loadHosts();
  setupEventListeners();
  showWelcome();
}

// ─── 主机管理 ────────────────────────────────────────────────────────────

async function loadHosts() {
  try {
    state.hosts = await invoke('list_hosts');
    state.groups = await invoke('get_groups');
    renderHostList();
  } catch (e) {
    console.error('Failed to load hosts:', e);
  }
}

function renderHostList() {
  const container = document.getElementById('host-list');
  const search = (document.getElementById('search-hosts').value || '').toLowerCase();

  const filtered = state.hosts.filter(h =>
    h.name.toLowerCase().includes(search) ||
    h.hostname.toLowerCase().includes(search)
  );

  const groups = {};
  filtered.forEach(h => {
    if (!groups[h.group]) groups[h.group] = [];
    groups[h.group].push(h);
  });

  let html = '';
  for (const [groupName, hosts] of Object.entries(groups)) {
    html += `<div class="group-header">${escapeHtml(groupName)} (${hosts.length})</div>`;
    hosts.forEach(h => {
      const isActive = state.activeSession === h.id;
      html += `
        <div class="host-item ${isActive ? 'active' : ''}" data-id="${h.id}">
          <span class="dot" style="background:${h.color}"></span>
          <div class="info" onclick="window.connectHost('${h.id}')">
            <div class="name">${escapeHtml(h.name)}</div>
            <div class="addr">${escapeHtml(h.username)}@${escapeHtml(h.hostname)}:${h.port}</div>
          </div>
          <div class="actions">
            <button onclick="window.editHost('${h.id}')" title="编辑">✏️</button>
            <button onclick="window.deleteHost('${h.id}')" title="删除">🗑️</button>
          </div>
        </div>`;
    });
  }

  if (!html) {
    html = '<div class="group-header" style="text-align:center;padding:20px;color:var(--text-dim)">暂无主机</div>';
  }

  container.innerHTML = html;
}

function escapeHtml(s) {
  if (!s) return '';
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

// ─── 主机 CRUD 对话框 ──────────────────────────────────────────────────

function openAddHostDialog() {
  document.getElementById('dialog-title').textContent = '添加主机';
  document.getElementById('host-form').reset();
  document.getElementById('host-id').value = '';
  document.getElementById('host-port').value = '22';
  document.getElementById('host-username').value = 'root';
  document.getElementById('host-color').value = '#667eea';
  document.getElementById('host-group').value = 'Default';
  toggleAuthFields();
  document.getElementById('host-dialog').classList.remove('hidden');
}

function openEditHostDialog(id) {
  const host = state.hosts.find(h => h.id === id);
  if (!host) return;

  document.getElementById('dialog-title').textContent = '编辑主机';
  document.getElementById('host-id').value = host.id;
  document.getElementById('host-name').value = host.name;
  document.getElementById('host-hostname').value = host.hostname;
  document.getElementById('host-port').value = host.port;
  document.getElementById('host-username').value = host.username;
  document.getElementById('host-auth-type').value = host.auth_type;
  document.getElementById('host-password').value = host.password || '';
  document.getElementById('host-key-path').value = host.key_path || '';
  document.getElementById('host-key-passphrase').value = host.key_passphrase || '';
  document.getElementById('host-group').value = host.group;
  document.getElementById('host-color').value = host.color;
  document.getElementById('host-notes').value = host.notes;
  toggleAuthFields();
  document.getElementById('host-dialog').classList.remove('hidden');
}

function closeHostDialog() {
  document.getElementById('host-dialog').classList.add('hidden');
}

function toggleAuthFields() {
  const type = document.getElementById('host-auth-type').value;
  document.getElementById('password-row').classList.toggle('hidden', type !== 'password');
  document.getElementById('key-row').classList.toggle('hidden', type !== 'key');
}

async function saveHost(e) {
  e.preventDefault();
  const now = new Date().toISOString();
  const id = document.getElementById('host-id').value || crypto.randomUUID();

  const host = {
    id,
    name: document.getElementById('host-name').value,
    hostname: document.getElementById('host-hostname').value,
    port: parseInt(document.getElementById('host-port').value) || 22,
    username: document.getElementById('host-username').value,
    auth_type: document.getElementById('host-auth-type').value,
    password: document.getElementById('host-password').value || null,
    key_path: document.getElementById('host-key-path').value || null,
    key_passphrase: document.getElementById('host-key-passphrase').value || null,
    group: document.getElementById('host-group').value || 'Default',
    color: document.getElementById('host-color').value,
    notes: document.getElementById('host-notes').value,
    created_at: document.getElementById('host-id').value ? state.hosts.find(h => h.id === id)?.created_at || now : now,
    updated_at: now,
  };

  try {
    if (document.getElementById('host-id').value) {
      await invoke('update_host', { host });
    } else {
      await invoke('add_host', { host });
    }
    closeHostDialog();
    await loadHosts();
  } catch (e) {
    alert('保存失败: ' + e);
  }
}

async function deleteHost(id) {
  if (!confirm('确定要删除这个主机吗？')) return;
  try {
    await invoke('delete_host', { id });
    await loadHosts();
  } catch (e) {
    alert('删除失败: ' + e);
  }
}

window.openAddHostDialog = openAddHostDialog;
window.editHost = openEditHostDialog;
window.deleteHost = deleteHost;
window.closeHostDialog = closeHostDialog;

// ─── SSH 连接 ────────────────────────────────────────────────────────────

class TerminalSession {
  constructor(host, termContainer, tabElement) {
    this.host = host;
    this.sessionId = crypto.randomUUID();
    this.termContainer = termContainer;
    this.tabElement = tabElement;
    this.connected = false;

    // 创建 xterm.js 终端
    this.terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: 'block',
      fontSize: 14,
      fontFamily: "'JetBrains Mono', 'SF Mono', 'Fira Code', 'Cascadia Code', monospace",
      rendererType: 'canvas',
      allowTransparency: false,
      theme: {
        background: '#000000',
        foreground: '#c0caf5',
        cursor: '#7aa2f7',
        selectionBackground: '#3b3d5c',
        black: '#1d202f',
        red: '#f7768e',
        green: '#9ece6a',
        yellow: '#e0af68',
        blue: '#7aa2f7',
        magenta: '#bb9af7',
        cyan: '#7dcfff',
        white: '#a9b1d6',
        brightBlack: '#414868',
        brightRed: '#f7768e',
        brightGreen: '#9ece6a',
        brightYellow: '#e0af68',
        brightBlue: '#7aa2f7',
        brightMagenta: '#bb9af7',
        brightCyan: '#7dcfff',
        brightWhite: '#c0caf5',
      },
    });

    this.fitAddon = new FitAddon();
    this.terminal.loadAddon(this.fitAddon);
    this.terminal.loadAddon(new SearchAddon());

    // 尝试 WebGL 渲染
    try {
      this.terminal.loadAddon(new WebglAddon());
    } catch (e) {
      // fallback to canvas
    }

    this.terminal.open(termContainer);

    // 延迟 resize
    setTimeout(() => this.fitAddon.fit(), 100);

    // 键盘输入
    this.terminal.onData(data => {
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        this.ws.send(data);
      }
    });

    // 窗口 resize
    this.resizeHandler = () => {
      this.fitAddon.fit();
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        const dims = this.fitAddon.proposeDimensions();
        if (dims) {
          this.ws.send(JSON.stringify({
            type: 'resize',
            cols: dims.cols,
            rows: dims.rows,
          }));
        }
      }
    };
    window.addEventListener('resize', this.resizeHandler);

    // 显示输入栏
    document.getElementById('terminal-input-bar').classList.remove('hidden');
    document.getElementById('quick-command').onkeydown = e => {
      if (e.key === 'Enter') {
        const cmd = e.target.value;
        if (cmd && this.connected) {
          this.terminal.write(cmd + '\n');
          e.target.value = '';
        }
      }
    };
  }

  async connect() {
    try {
      // 通过 Tauri 后端建立 SSH 连接
      const config = {
        hostname: this.host.hostname,
        port: this.host.port,
        username: this.host.username,
        auth_type: this.host.auth_type,
        password: this.host.password,
        key_path: this.host.key_path,
        key_passphrase: this.host.key_passphrase,
      };

      await invoke('ssh_connect', {
        sessionId: this.sessionId,
        config,
      });

      // 启动 WebSocket 终端桥
      // 注意：Tauri 2 中我们可以通过 Rust 后端直接读写 SSH Channel
      // 但真正的实时终端流需要通过 WebSocket 或 Tauri event 机制
      // 这里我们使用 Tauri event 来实现双向流

      this.connected = true;
      this.terminal.focus();
      this.fitAddon.fit();

      // 这里简化为通过 execute 命令方式
      // 完整的 PTY shell 需要 Rust 后端通过 events 推送数据
      // 实际实现中需要 Tauri event 监听 + SSH channel 循环读取
      // 我们先用 exec 模式展示

      this.terminal.write('\r\n\x1b[1;32m✓ 已连接到 ' + this.host.name + '\x1b[0m\r\n');
      this.terminal.write('\x1b[1;33m注意: 实时终端流需通过 Rust WebSocket 桥实现\x1b[0m\r\n');
      this.terminal.write('\x1b[1;33m当前为命令执行模式，在下方输入命令执行\x1b[0m\r\n\r\n');

      // 示例：执行 uname
      try {
        const result = await invoke('ssh_exec', {
          sessionId: this.sessionId,
          command: 'uname -a',
        });
        this.terminal.write(result + '\r\n');
      } catch (e) {
        this.terminal.write('\r\n\x1b[1;31m执行失败: ' + e + '\x1b[0m\r\n');
      }

    } catch (e) {
      this.terminal.write('\r\n\x1b[1;31m连接失败: ' + e + '\x1b[0m\r\n');
    }
  }

  async executeCommand(cmd) {
    if (!this.connected) return;
    try {
      this.terminal.write('\r\n$ ' + cmd + '\r\n');
      const result = await invoke('ssh_exec', {
        sessionId: this.sessionId,
        command: cmd,
      });
      this.terminal.write(result + '\r\n');
    } catch (e) {
      this.terminal.write('\x1b[1;31m' + e + '\x1b[0m\r\n');
    }
  }

  destroy() {
    window.removeEventListener('resize', this.resizeHandler);
    try {
      invoke('ssh_disconnect', { sessionId: this.sessionId });
    } catch (e) {}
    this.terminal.dispose();
  }
}

// ─── 连接主机入口 ──────────────────────────────────────────────────────

function connectHost(hostId) {
  const host = state.hosts.find(h => h.id === hostId);
  if (!host) return;

  // 检查是否已有该主机的标签
  const existing = document.getElementById(`tab-${host.id}`);
  if (existing) {
    switchTab(host.id);
    return;
  }

  // 隐藏欢迎页
  document.getElementById('welcome').classList.remove('active');
  document.getElementById('terminal-panel').classList.add('active');
  document.getElementById('sftp-panel').classList.add('hidden');

  // 创建标签
  const tabs = document.getElementById('tabs');
  const tab = document.createElement('div');
  tab.className = 'tab active';
  tab.id = `tab-${host.id}`;
  tab.innerHTML = `<span style="display:inline-block;width:8px;height:8px;border-radius:50%;background:${host.color};margin-right:4px"></span>${escapeHtml(host.name)}<button class="close-btn" onclick="closeTab('${host.id}')">×</button>`;
  tab.onclick = () => switchTab(host.id);
  tabs.appendChild(tab);

  // 清除旧终端
  const container = document.getElementById('terminal-container');
  container.innerHTML = '';

  // 创建会话
  const session = new TerminalSession(host, container, tab);
  state.sessions[host.id] = session;
  state.activeSession = host.id;

  // 更新侧边栏
  renderHostList();

  // 连接
  session.connect();
}

function switchTab(hostId) {
  // 切换标签高亮
  document.querySelectorAll('#tabs .tab').forEach(t => t.classList.remove('active'));
  const tab = document.getElementById(`tab-${hostId}`);
  if (tab) tab.classList.add('active');

  state.activeSession = hostId;
  renderHostList();

  // 显示终端面板
  document.getElementById('welcome').classList.remove('active');
  document.getElementById('terminal-panel').classList.add('active');
  document.getElementById('sftp-panel').classList.add('hidden');
}

function closeTab(hostId) {
  const session = state.sessions[hostId];
  if (session) {
    session.destroy();
    delete state.sessions[hostId];
  }

  const tab = document.getElementById(`tab-${hostId}`);
  if (tab) tab.remove();

  if (state.activeSession === hostId) {
    state.activeSession = null;
    // 切换到其他标签或欢迎页
    const remaining = Object.keys(state.sessions);
    if (remaining.length > 0) {
      switchTab(remaining[0]);
    } else {
      document.getElementById('welcome').classList.add('active');
      document.getElementById('terminal-panel').classList.remove('active');
      document.getElementById('sftp-panel').classList.add('hidden');
      document.getElementById('terminal-input-bar').classList.add('hidden');
    }
  }

  renderHostList();
}

window.connectHost = connectHost;
window.closeTab = closeTab;

// ─── SFTP 文件管理 ──────────────────────────────────────────────────────

async function openSftp(hostId) {
  const host = state.hosts.find(h => h.id === hostId);
  if (!host) return;

  // 确保 SSH 已连接
  if (!state.sessions[hostId]) {
    connectHost(hostId);
    return;
  }

  // 切换到 SFTP 面板
  document.getElementById('terminal-panel').classList.remove('active');
  document.getElementById('sftp-panel').classList.remove('hidden');

  // 创建一个 SFTP 标签
  const tabs = document.getElementById('tabs');
  const tabId = `sftp-tab-${hostId}`;
  if (!document.getElementById(tabId)) {
    const tab = document.createElement('div');
    tab.className = 'tab active';
    tab.id = tabId;
    tab.innerHTML = `📂 ${escapeHtml(host.name)} (SFTP)<button class="close-btn" onclick="closeTab('${hostId}')">×</button>`;
    tab.onclick = () => switchTab(hostId);
    tabs.appendChild(tab);
  }

  await loadRemoteDir(hostId, '/');
}

async function loadRemoteDir(sessionId, path) {
  try {
    const files = await invoke('sftp_list_dir', { sessionId, path });
    state.sftp.remotePath = path;
    state.sftp.remoteFiles = files;
    renderRemoteFiles();
  } catch (e) {
    console.error('Failed to list remote dir:', e);
  }
}

function renderRemoteFiles() {
  const container = document.getElementById('remote-files');
  document.getElementById('remote-path').textContent = state.sftp.remotePath;

  let html = '';
  // 向上
  if (state.sftp.remotePath !== '/') {
    html += `<div class="sftp-entry" onclick="sftpGoUp()">
      <span class="icon">📂</span>
      <span class="name">.. (上级目录)</span>
      <span class="size"></span>
    </div>`;
  }

  state.sftp.remoteFiles.forEach(f => {
    const icon = f.is_dir ? '📁' : getFileIcon(f.name);
    const size = f.is_dir ? '' : formatSize(f.size);
    html += `<div class="sftp-entry" data-path="${escapeHtml(f.path)}" onclick="sftpClickEntry(this, '${f.path}', ${f.is_dir})">
      <span class="icon">${icon}</span>
      <span class="name">${escapeHtml(f.name)}</span>
      <span class="size">${size}</span>
    </div>`;
  });

  container.innerHTML = html;
}

function sftpGoUp() {
  const path = state.sftp.remotePath;
  const parent = path === '/' ? '/' : path.split('/').slice(0, -1).join('/') || '/';
  loadRemoteDir(state.activeSession, parent);
}

function sftpClickEntry(el, path, isDir) {
  if (isDir) {
    loadRemoteDir(state.activeSession, path);
  } else {
    // 选中文件
    document.querySelectorAll('#remote-files .sftp-entry').forEach(e => e.classList.remove('selected'));
    el.classList.add('selected');
    state.sftp.selectedRemote = path;
  }
}

function getFileIcon(name) {
  const ext = name.split('.').pop().toLowerCase();
  const icons = {
    pdf: '📕', zip: '📦', rar: '📦', '7z': '📦',
    mp3: '🎵', wav: '🎵', mp4: '🎬', avi: '🎬',
    jpg: '🖼', jpeg: '🖼', png: '🖼', gif: '🖼',
    txt: '📄', md: '📄', json: '📋', xml: '📋',
    js: '⚙️', ts: '⚙️', py: '⚙️', go: '⚙️', rs: '⚙️',
    html: '🌐', css: '🎨',
  };
  return icons[ext] || '📄';
}

function formatSize(bytes) {
  if (!bytes) return '-';
  const units = ['B', 'KB', 'MB', 'GB'];
  let i = 0;
  let size = bytes;
  while (size >= 1024 && i < 3) { size /= 1024; i++; }
  return size.toFixed(i === 0 ? 0 : 1) + ' ' + units[i];
}

window.sftpGoUp = sftpGoUp;
window.sftpClickEntry = sftpClickEntry;

// ─── 事件绑定 ──────────────────────────────────────────────────────────

function setupEventListeners() {
  // 添加主机按钮
  document.getElementById('btn-add-host').onclick = openAddHostDialog;

  // 主机表单提交
  document.getElementById('host-form').onsubmit = saveHost;

  // 认证方式切换
  document.getElementById('host-auth-type').onchange = toggleAuthFields;

  // 搜索
  document.getElementById('search-hosts').oninput = () => renderHostList();

  // SFTP 按钮
  document.getElementById('btn-remote-refresh').onclick = () => {
    if (state.activeSession) loadRemoteDir(state.activeSession, state.sftp.remotePath);
  };
}

function showWelcome() {
  document.getElementById('welcome').classList.add('active');
  document.getElementById('terminal-panel').classList.remove('active');
  document.getElementById('sftp-panel').classList.add('hidden');
}

// ─── 启动 ──────────────────────────────────────────────────────────────

document.addEventListener('DOMContentLoaded', init);
