const { invoke, event } = window.__TAURI__;

// ── State ──
const state = {
  sessionId: null, sessionName: '', messages: [],
  totalInputTokens: 0, totalOutputTokens: 0, totalCost: 0,
  generating: false, streamBuffer: '',
  inputHistory: [], historyIdx: -1, inputDraft: '',
  config: null, searchMatches: [], searchIdx: 0,
  modelPrices: { input: 2, output: 8 },
};

const $ = (s) => document.querySelector(s);
const chatMessages = $('#chat-messages');
const inputBox = $('#input-box');

// ── Init ──
document.addEventListener('DOMContentLoaded', async () => {
  marked.setOptions({
    highlight: (code, lang) => {
      if (lang && hljs.getLanguage(lang)) return hljs.highlight(code, { language: lang }).value;
      return hljs.highlightAuto(code).value;
    },
    breaks: true, gfm: true,
  });
  await loadConfig();
  await loadSessions();
  setupEvents();
  setupStreamListeners();
  showWelcome();
});

// ── Config ──
async function loadConfig() {
  try {
    state.config = await invoke('get_config');
    const prices = await invoke('get_model_prices');
    state.modelPrices = { input: prices[0], output: prices[1] };
    updateStatusBar();
    applyTheme(state.config.theme);
  } catch (e) { console.error('配置加载失败:', e); }
}
function updateStatusBar() {
  if (!state.config) return;
  $('#status-provider').textContent = state.config.provider;
  $('#status-model').textContent = state.config.model;
  updateTokenDisplay();
}
function updateTokenDisplay() {
  const t = state.totalInputTokens + state.totalOutputTokens;
  if (t > 0) {
    $('#status-tokens').textContent = `↓${(state.totalInputTokens/1000).toFixed(1)}K ↑${(state.totalOutputTokens/1000).toFixed(1)}K`;
    $('#status-cost').textContent = `¥${state.totalCost.toFixed(4)}`;
  } else { $('#status-tokens').textContent = ''; $('#status-cost').textContent = ''; }
}
function applyTheme(name) {
  if (name && name !== 'tokyo-night') document.documentElement.setAttribute('data-theme', name);
  else document.documentElement.removeAttribute('data-theme');
  const map = { 'tokyo-night': 'tokyo-night-dark', 'nord': 'nord', 'catppuccin': 'catppuccin-mocha' };
  const link = document.querySelector('link[href*="lib/"]');
  if (link) link.href = `lib/${map[name] || 'tokyo-night-dark'}.min.css`;
  // Update theme cards
  document.querySelectorAll('.theme-card').forEach(c => {
    c.classList.toggle('active', c.dataset.theme === name);
  });
}

// ── Sessions ──
async function loadSessions() {
  try {
    const sessions = await invoke('list_sessions');
    renderSessionList(sessions);
    if (sessions.length > 0 && !state.sessionId) await switchSession(sessions[0].id);
  } catch (e) { console.error(e); }
}
function renderSessionList(sessions) {
  const el = $('#session-list');
  el.innerHTML = '';
  sessions.forEach(s => {
    const div = document.createElement('div');
    div.className = `session-item${s.id === state.sessionId ? ' active' : ''}`;
    div.innerHTML = `<div class="session-item-name">${escHtml(s.name)}</div><div class="session-item-meta">${s.msg_count} 条 · ${s.age}</div>`;
    div.onclick = () => switchSession(s.id);
    div.ondblclick = () => renameSession(s);
    div.oncontextmenu = (e) => {
      e.preventDefault();
      showConfirm(`删除会话 "${s.name}"？`, () => invoke('delete_session', { sessionId: s.id }).then(loadSessions));
    };
    el.appendChild(div);
  });
}
function renameSession(s) {
  const nameEl = event.currentTarget.querySelector('.session-item-name');
  const input = document.createElement('input');
  input.type = 'text'; input.value = s.name;
  input.style.cssText = 'width:100%;background:var(--bg-input);border:1px solid var(--accent);border-radius:4px;color:var(--text-primary);font-size:13px;padding:2px 6px;outline:none;';
  nameEl.innerHTML = ''; nameEl.appendChild(input); input.focus(); input.select();
  const finish = () => {
    const newName = input.value.trim();
    if (newName && newName !== s.name) {
      s.name = newName;
      if (state.sessionId === s.id) { state.sessionName = newName; $('#session-title').textContent = newName; }
      saveCurrentSession();
    }
    loadSessions();
  };
  input.onblur = finish;
  input.onkeydown = e => { if (e.key === 'Enter') { e.preventDefault(); input.blur(); } if (e.key === 'Escape') { input.value = s.name; input.blur(); } };
}
async function switchSession(id) {
  try {
    const d = await invoke('load_session', { sessionId: id });
    Object.assign(state, { sessionId: d.id, sessionName: d.name, messages: d.messages,
      totalInputTokens: d.total_input_tokens, totalOutputTokens: d.total_output_tokens,
      totalCost: d.total_cost, streamBuffer: '' });
    $('#session-title').textContent = d.name;
    updateTokenDisplay(); renderMessages();
    renderSessionList(await invoke('list_sessions'));
  } catch (e) { console.error(e); }
}
async function newSession() {
  await saveCurrentSession();
  const d = await invoke('create_session');
  Object.assign(state, { sessionId: d.id, sessionName: d.name, messages: [],
    totalInputTokens: 0, totalOutputTokens: 0, totalCost: 0, streamBuffer: '' });
  $('#session-title').textContent = d.name;
  updateTokenDisplay(); renderMessages();
  renderSessionList(await invoke('list_sessions'));
}
async function saveCurrentSession() {
  if (!state.sessionId || state.messages.length === 0) return;
  try {
    await invoke('save_session_data', {
      sessionId: state.sessionId, name: state.sessionName, messages: state.messages,
      totalInputTokens: state.totalInputTokens, totalOutputTokens: state.totalOutputTokens,
      totalCost: state.totalCost,
    });
  } catch (e) { console.error(e); }
}

// ── Messages ──
function renderMessages() {
  chatMessages.innerHTML = '';
  if (state.messages.length === 0 && !state.generating) { showWelcome(); return; }
  state.messages.forEach(m => appendMessageEl(m.role, m.content));
  scrollToBottom();
}
function appendMessageEl(role, content) {
  const div = document.createElement('div');
  div.className = `message ${role}`;
  const label = role === 'user' ? 'You' : role === 'assistant' ? 'Assistant' : 'System';
  div.innerHTML = `<div class="message-role">${label}</div><div class="message-content">${renderContent(content)}</div>`;
  addCopyButtons(div);
  chatMessages.appendChild(div);
}
function addCopyButtons(container) {
  container.querySelectorAll('pre').forEach(pre => {
    if (pre.parentNode.classList?.contains('code-header')) return;
    const header = document.createElement('div');
    header.className = 'code-header';
    header.innerHTML = `<span>${escHtml(pre.getAttribute('data-lang') || '')}</span>`;
    const btn = document.createElement('button');
    btn.className = 'btn-copy-code'; btn.textContent = '复制';
    btn.onclick = () => {
      const code = pre.querySelector('code');
      navigator.clipboard.writeText(code ? code.textContent : pre.textContent);
      btn.textContent = '已复制'; setTimeout(() => btn.textContent = '复制', 1500);
    };
    header.appendChild(btn);
    pre.parentNode.insertBefore(header, pre);
  });
}
function renderContent(text) { if (!text) return ''; try { return marked.parse(text); } catch { return escHtml(text); } }
function scrollToBottom() { requestAnimationFrame(() => chatMessages.scrollTop = chatMessages.scrollHeight); }
function showWelcome() {
  chatMessages.innerHTML = `<div class="welcome">
    <h1>MiMo-OPT</h1>
    <p>多 Provider AI 聊天工具 · 桌面端</p>
    <p style="margin-top:16px;font-size:13px;">
      输入消息开始对话 · <kbd>/</kbd> 查看命令<br>
      <kbd>Ctrl+N</kbd> 新会话 · <kbd>Ctrl+F</kbd> 搜索 · <kbd>Ctrl+Z</kbd> 撤回
    </p>
  </div>`;
}

// ── Send ──
async function sendMessage() {
  const text = inputBox.value.trim();
  if (!text || state.generating) return;
  if (text.startsWith('/')) { handleCommand(text); inputBox.value = ''; autoResize(); return; }

  state.messages.push({ role: 'user', content: text });
  state.inputHistory.push(text); state.historyIdx = -1;
  inputBox.value = ''; autoResize(); $('#token-estimate').textContent = '';
  appendMessageEl('user', text); scrollToBottom();

  state.generating = true; state.streamBuffer = '';
  $('#btn-send').classList.add('hidden'); $('#btn-stop').classList.remove('hidden');

  const ph = document.createElement('div');
  ph.className = 'message assistant'; ph.id = 'streaming-msg';
  ph.innerHTML = `<div class="message-role">Assistant</div><div class="message-content"><span class="streaming-cursor">思考中</span></div>`;
  chatMessages.appendChild(ph); scrollToBottom();

  try { await invoke('send_message', { messages: state.messages }); }
  catch (e) { finishStreaming(null, e.toString()); }
}
function stopGeneration() {
  state.generating = false;
  $('#btn-send').classList.remove('hidden'); $('#btn-stop').classList.add('hidden');
  invoke('stop_generation');
  if (state.streamBuffer) state.messages.push({ role: 'assistant', content: state.streamBuffer });
  state.streamBuffer = ''; saveCurrentSession();
}

// ── Stream ──
function setupStreamListeners() {
  event.listen('stream_token', e => { state.streamBuffer += e.payload.token; updateStreamingMessage(); });
  event.listen('stream_done', e => finishStreaming(e.payload, null));
  event.listen('stream_error', e => finishStreaming(null, e.payload.error));
}
function updateStreamingMessage() {
  const el = $('#streaming-msg'); if (!el) return;
  const c = el.querySelector('.message-content');
  c.innerHTML = renderContent(state.streamBuffer);
  addCopyButtons(el);
  scrollToBottom();
}
function finishStreaming(tokens, error) {
  state.generating = false;
  $('#btn-send').classList.remove('hidden'); $('#btn-stop').classList.add('hidden');
  const el = $('#streaming-msg'); if (el) el.removeAttribute('id');

  if (error) {
    if (state.streamBuffer) state.messages.push({ role: 'assistant', content: state.streamBuffer });
    state.messages.push({ role: 'error', content: `错误: ${error}` });
    appendMessageEl('error', `错误: ${error}`);
  } else if (state.streamBuffer) {
    state.messages.push({ role: 'assistant', content: state.streamBuffer });
    if (tokens) {
      state.totalInputTokens += tokens.input_tokens || 0;
      state.totalOutputTokens += tokens.output_tokens || 0;
      updateCost(); updateTokenDisplay();
    }
  }
  state.streamBuffer = ''; scrollToBottom(); saveCurrentSession();
}
function updateCost() {
  const p = state.modelPrices;
  state.totalCost = (state.totalInputTokens/1e6)*p.input + (state.totalOutputTokens/1e6)*p.output;
}

// ── File upload ──
function handleFileUpload(file) {
  if (!file) return;
  const reader = new FileReader();
  reader.onload = () => {
    const content = reader.result;
    const name = file.name;
    // Insert as /read content into chat
    const display = `📄 已上传: ${name} (${content.split('\n').length} 行, ${file.size} B)`;
    const fileContent = `\`\`\`\n${content}\n\`\`\``;
    state.messages.push({ role: 'user', content: `请分析文件 ${name} 的内容:\n${fileContent}` });
    appendMessageEl('user', `${display}\n\n${fileContent}`);
    scrollToBottom();
    // Auto-send for analysis
    state.generating = true; state.streamBuffer = '';
    $('#btn-send').classList.add('hidden'); $('#btn-stop').classList.remove('hidden');
    const ph = document.createElement('div');
    ph.className = 'message assistant'; ph.id = 'streaming-msg';
    ph.innerHTML = `<div class="message-role">Assistant</div><div class="message-content"><span class="streaming-cursor">分析中</span></div>`;
    chatMessages.appendChild(ph); scrollToBottom();
    invoke('send_message', { messages: state.messages }).catch(e => finishStreaming(null, e.toString()));
  };
  reader.readAsText(file);
}

// ── Commands ──
function handleCommand(text) {
  const parts = text.slice(1).split(/\s+/);
  const cmd = parts[0], args = parts.slice(1).join(' ');
  switch (cmd) {
    case 'help':
      appendMessageEl('assistant', `可用命令:
/ \`help\` — 帮助 · /\`clear\` — 清空 · /\`search <关键词>\` — 搜索
/ \`model [name]\` — 模型 · /\`provider [name]\` — 提供商
/ \`theme [name]\` — 主题 · /\`read <path>\` — 读文件 · /\`export\` — 导出`); break;
    case 'clear': doClear(); break;
    case 'search': args ? doSearch(args) : appendMessageEl('error', '用法: /search <关键词>'); break;
    case 'model':
      if (!args) { invoke('get_models').then(ms => {
        appendMessageEl('assistant', '模型:\n' + ms.map(m => `  ${m.name}${m.is_current?' ✓':''}  ${m.desc}  ¥${m.input_price}/M ↑¥${m.output_price}/M`).join('\n'));
      }); } else { invoke('update_config', { model: args }).then(() => { state.config.model = args; updateStatusBar(); appendMessageEl('assistant', `✓ ${args}`); }); }
      break;
    case 'provider':
      if (!args) { invoke('get_providers').then(ps => {
        appendMessageEl('assistant', 'Provider:\n' + ps.map(p => `  ${p.name} — ${p.default_model}`).join('\n'));
      }); } else { invoke('update_config', { provider: args }).then(loadConfig).then(() => appendMessageEl('assistant', `✓ ${args}`)); }
      break;
    case 'theme':
      if (!args) { invoke('get_themes').then(ts => appendMessageEl('assistant', '主题:\n' + ts.map(t => `  ${t}${t===state.config?.theme?' ✓':''}`).join('\n'))); }
      else { invoke('update_config', { theme: args }).then(() => { state.config.theme = args; applyTheme(args); appendMessageEl('assistant', `✓ ${args}`); }); }
      break;
    case 'read':
      if (!args) appendMessageEl('error', '用法: /read <path>');
      else invoke('read_file', { path: args, startLine: null, endLine: null })
        .then(c => appendMessageEl('assistant', `📄 ${args}:\n\`\`\`\n${c}\n\`\`\``))
        .catch(e => appendMessageEl('error', e));
      break;
    case 'export': doExport(); break;
    default: appendMessageEl('error', `未知: /${cmd}。输入 /help`);
  }
}
async function doSearch(query) {
  appendMessageEl('assistant', `🔍 ${query}...`);
  try {
    const r = await invoke('web_search', { query });
    state.messages.push({ role: 'user', content: `搜索: ${query}\n\n${r}` });
    await invoke('send_message', { messages: state.messages });
  } catch (e) { appendMessageEl('error', `搜索失败: ${e}`); }
}
function doClear() {
  if (state.messages.length === 0) return;
  showConfirm(`清空 ${state.messages.length} 条对话？`, () => {
    state.messages = []; state.totalInputTokens = 0; state.totalOutputTokens = 0; state.totalCost = 0;
    updateTokenDisplay(); renderMessages(); saveCurrentSession();
  });
}
async function doExport() {
  if (!state.sessionId) return;
  try {
    const { save } = window.__TAURI__.dialog;
    const path = await save({
      defaultPath: `${state.sessionName || 'session'}.md`,
      filters: [{ name: 'Markdown', extensions: ['md'] }],
    });
    if (path) {
      const msg = await invoke('export_session', { sessionId: state.sessionId, outputPath: path });
      appendMessageEl('assistant', `✓ ${msg}`);
    }
  } catch (e) { appendMessageEl('error', e); }
}

// ── Chat search ──
function openSearch() { $('#search-overlay').classList.remove('hidden'); $('#search-input').focus(); }
function closeSearch() { $('#search-overlay').classList.add('hidden'); state.searchMatches = []; state.searchIdx = 0; }
function doChatSearch(query) {
  state.searchMatches = []; state.searchIdx = 0;
  if (!query) { $('#search-count').textContent = ''; return; }
  const l = query.toLowerCase();
  document.querySelectorAll('.message-content').forEach((el, i) => {
    if (el.textContent.toLowerCase().includes(l)) state.searchMatches.push(i);
  });
  $('#search-count').textContent = state.searchMatches.length ? `1/${state.searchMatches.length}` : '无';
  scrollToSearch();
}
function scrollToSearch() {
  if (!state.searchMatches.length) return;
  const idx = state.searchMatches[state.searchIdx];
  const msgs = document.querySelectorAll('.message');
  if (msgs[idx]) { msgs[idx].scrollIntoView({ behavior: 'smooth', block: 'center' }); msgs[idx].style.outline = '2px solid var(--accent)'; setTimeout(() => msgs[idx].style.outline = '', 1500); }
  $('#search-count').textContent = `${state.searchIdx+1}/${state.searchMatches.length}`;
}

// ── Confirm dialog ──
let confirmCallback = null;
function showConfirm(msg, onOk) {
  $('#confirm-msg').textContent = msg;
  confirmCallback = onOk;
  $('#confirm-overlay').classList.remove('hidden');
}
function closeConfirm(ok) {
  $('#confirm-overlay').classList.add('hidden');
  if (ok && confirmCallback) confirmCallback();
  confirmCallback = null;
}

// ── Settings ──
async function openSettings() {
  const overlay = $('#settings-overlay');
  overlay.classList.remove('hidden');

  // Providers
  const providers = await invoke('get_providers');
  $('#cfg-provider').innerHTML = providers.map(p => `<option value="${p.name}"${p.name===state.config?.provider?' selected':''}>${p.name}</option>`).join('');
  await updateModelSelect();

  // Fill fields
  $('#cfg-apikey').value = ''; $('#cfg-apikey').type = 'password';
  $('#cfg-baseurl').value = state.config?.base_url || '';
  $('#cfg-maxtokens').value = state.config?.max_tokens || 4096;
  $('#cfg-temperature').value = state.config?.temperature ?? '';
  $('#cfg-topp').value = state.config?.top_p ?? '';
  $('#cfg-proxy').value = state.config?.proxy_url || '';

  // Theme cards
  applyTheme(state.config?.theme || 'tokyo-night');

  // Load skills
  await loadSkills();

  // Activate first tab
  activateTab('tab-basic');
}
async function updateModelSelect() {
  try {
    const ms = await invoke('get_models');
    $('#cfg-model').innerHTML = ms.map(m => `<option value="${m.name}"${m.is_current?' selected':''}>${m.name} — ${m.desc}</option>`).join('');
  } catch {}
}
function closeSettings() { $('#settings-overlay').classList.add('hidden'); }

async function loadSkills() {
  try {
    const skills = await invoke('get_skills');
    const list = $('#skills-list');
    list.innerHTML = '';
    Object.entries(skills).forEach(([name, desc]) => {
      const div = document.createElement('div');
      div.className = 'skill-item';
      div.innerHTML = `<div class="skill-item-info"><div class="skill-item-name">${escHtml(name)}</div><div class="skill-item-cmd">${escHtml(desc)}</div></div>`;
      const btn = document.createElement('button');
      btn.className = 'btn-remove-skill'; btn.textContent = '✕'; btn.title = '删除';
      btn.onclick = () => invoke('remove_skill', { name }).then(loadSkills);
      div.appendChild(btn);
      list.appendChild(div);
    });
  } catch {}
}
function addSkill() {
  const name = $('#skill-name').value.trim();
  const cmd = $('#skill-cmd').value.trim();
  const desc = $('#skill-desc').value.trim() || null;
  if (!name || !cmd) return;
  invoke('add_skill', { name, cmd, desc }).then(() => {
    $('#skill-name').value = ''; $('#skill-cmd').value = ''; $('#skill-desc').value = '';
    loadSkills();
  });
}

async function saveConfig() {
  const updates = {};
  const apikey = $('#cfg-apikey').value.trim();
  if (apikey) updates.api_key = apikey;
  const provider = $('#cfg-provider').value;
  if (provider !== state.config?.provider) updates.provider = provider;
  const model = $('#cfg-model').value;
  if (model !== state.config?.model) updates.model = model;
  const baseurl = $('#cfg-baseurl').value.trim();
  if (baseurl !== state.config?.base_url) updates.base_url = baseurl;
  const maxtokens = parseInt($('#cfg-maxtokens').value);
  if (maxtokens !== state.config?.max_tokens) updates.max_tokens = maxtokens;
  // Get theme from active card
  const activeTheme = document.querySelector('.theme-card.active');
  if (activeTheme) {
    const theme = activeTheme.dataset.theme;
    if (theme !== state.config?.theme) updates.theme = theme;
  }
  const proxy = $('#cfg-proxy').value.trim();
  if (proxy !== (state.config?.proxy_url || '')) updates.proxy_url = proxy;
  updates.temperature = isNaN(parseFloat($('#cfg-temperature').value)) ? null : parseFloat($('#cfg-temperature').value);
  updates.top_p = isNaN(parseFloat($('#cfg-topp').value)) ? null : parseFloat($('#cfg-topp').value);

  try { await invoke('update_config', updates); await loadConfig(); closeSettings(); }
  catch (e) { alert('保存失败: ' + e); }
}

function activateTab(tabId) {
  document.querySelectorAll('.tab-btn').forEach(b => b.classList.toggle('active', b.dataset.tab === tabId));
  document.querySelectorAll('.tab-panel').forEach(p => p.classList.toggle('active', p.id === tabId));
}

// ── Token estimate ──
function updateEstimate() {
  const text = inputBox.value;
  if (!text.trim()) { $('#token-estimate').textContent = ''; $('#token-estimate').className = 'token-estimate'; return; }
  invoke('estimate_tokens', { text }).then(t => {
    const c = (t/1e6)*state.modelPrices.input;
    const el = $('#token-estimate');
    el.textContent = `~${t} tok · ~¥${c<0.01?c.toFixed(4):c.toFixed(2)}`;
    el.className = 'token-estimate' + (t>20000?' danger':t>5000?' warn':'');
  });
}
function updateHints() {
  const text = inputBox.value;
  if (!text.startsWith('/')) { $('#hint-bar').innerHTML = ''; return; }
  const rest = text.slice(1);
  const cmds = ['help','clear','search','model','provider','theme','read','export'];
  const hints = cmds.filter(c => c.startsWith(rest) && c !== rest);
  $('#hint-bar').innerHTML = hints.map(h => `<span class="hint-item" data-cmd="/${h}">${h}</span>`).join('');
  $('#hint-bar').querySelectorAll('.hint-item').forEach(el => {
    el.onclick = () => { inputBox.value = el.dataset.cmd+' '; inputBox.focus(); $('#hint-bar').innerHTML = ''; };
  });
}
function autoResize() { inputBox.style.height = 'auto'; inputBox.style.height = Math.min(inputBox.scrollHeight, 200) + 'px'; }

// ── History ──
function navigateHistory(dir) {
  if (!state.inputHistory.length) return;
  if (dir < 0) {
    if (state.historyIdx === -1) { state.inputDraft = inputBox.value; state.historyIdx = state.inputHistory.length-1; }
    else if (state.historyIdx > 0) state.historyIdx--;
    else return;
    inputBox.value = state.inputHistory[state.historyIdx];
  } else {
    if (state.historyIdx === -1) return;
    if (state.historyIdx < state.inputHistory.length-1) { state.historyIdx++; inputBox.value = state.inputHistory[state.historyIdx]; }
    else { state.historyIdx = -1; inputBox.value = state.inputDraft; }
  }
  autoResize();
}

// ── Events ──
function setupEvents() {
  // Send / Stop
  $('#btn-send').onclick = sendMessage;
  $('#btn-stop').onclick = stopGeneration;

  // Input
  inputBox.addEventListener('input', () => { autoResize(); updateEstimate(); updateHints(); });
  inputBox.addEventListener('keydown', e => {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendMessage(); }
    if (e.key === 'ArrowUp' && inputBox.selectionStart === 0) { e.preventDefault(); navigateHistory(-1); }
    if (e.key === 'ArrowDown' && inputBox.selectionStart === inputBox.value.length) { e.preventDefault(); navigateHistory(1); }
  });

  // Sidebar
  $('#btn-toggle-sidebar').onclick = () => $('#sidebar').classList.toggle('collapsed');
  $('#btn-new-session').onclick = newSession;
  $('#btn-tree').onclick = async () => {
    const panel = $('#project-tree-panel');
    const list = $('#session-list');
    if (panel.style.display === 'none') {
      try {
        const tree = await invoke('get_project_tree');
        $('#project-tree-content').textContent = tree;
      } catch {}
      panel.style.display = 'block'; panel.classList.remove('hidden');
      list.style.display = 'none';
    } else {
      panel.style.display = 'none'; panel.classList.add('hidden');
      list.style.display = 'block';
    }
  };

  // Top bar actions
  $('#btn-search').onclick = openSearch;
  $('#btn-web-search').onclick = () => {
    const q = inputBox.value.trim();
    if (q) { doSearch(q); inputBox.value = ''; autoResize(); }
    else { appendMessageEl('error', '请先输入搜索关键词'); }
  };
  $('#btn-upload').onclick = () => $('#file-upload').click();
  $('#btn-upload-input').onclick = () => $('#file-upload').click();
  $('#btn-export').onclick = doExport;
  $('#btn-clear').onclick = doClear;
  $('#btn-settings').onclick = openSettings;

  // File upload
  $('#file-upload').onchange = (e) => {
    const file = e.target.files[0];
    if (file) handleFileUpload(file);
    e.target.value = '';
  };

  // Drag-and-drop file upload
  const chatContainer = $('#chat-container');
  chatContainer.addEventListener('dragover', e => { e.preventDefault(); chatContainer.style.outline = '2px dashed var(--accent)'; });
  chatContainer.addEventListener('dragleave', () => { chatContainer.style.outline = ''; });
  chatContainer.addEventListener('drop', e => {
    e.preventDefault(); chatContainer.style.outline = '';
    const file = e.dataTransfer.files[0];
    if (file) handleFileUpload(file);
  });

  // Settings
  $('#btn-close-settings').onclick = closeSettings;
  $('#btn-save-config').onclick = saveConfig;
  $('#btn-toggle-key').onclick = () => { const i = $('#cfg-apikey'); i.type = i.type === 'password' ? 'text' : 'password'; };
  $('#cfg-provider').onchange = async () => {
    // Just update model list for preview; actual config change happens on save
    const provider = $('#cfg-provider').value;
    const models = await invoke('get_models_for_provider', { provider });
    $('#cfg-model').innerHTML = models.map(m => `<option value="${m.name}"${m.is_current?' selected':''}>${m.name} — ${m.desc}</option>`).join('');
  };
  $('#settings-overlay').onclick = e => { if (e.target === $('#settings-overlay')) closeSettings(); };

  // Theme cards
  document.querySelectorAll('.theme-card').forEach(card => {
    card.onclick = () => {
      document.querySelectorAll('.theme-card').forEach(c => c.classList.remove('active'));
      card.classList.add('active');
      applyTheme(card.dataset.theme);
    };
  });

  // Tabs
  document.querySelectorAll('.tab-btn').forEach(btn => {
    btn.onclick = () => activateTab(btn.dataset.tab);
  });

  // Skills
  $('#btn-add-skill').onclick = addSkill;

  // Search
  $('#search-input').addEventListener('input', e => doChatSearch(e.target.value));
  $('#search-input').addEventListener('keydown', e => {
    if (e.key === 'Enter') { e.preventDefault(); state.searchIdx = (state.searchIdx + (e.shiftKey?-1:1) + state.searchMatches.length) % state.searchMatches.length; scrollToSearch(); }
    if (e.key === 'Escape') closeSearch();
  });
  $('#btn-search-close').onclick = closeSearch;
  $('#btn-search-prev').onclick = () => { state.searchIdx = (state.searchIdx-1+state.searchMatches.length)%state.searchMatches.length; scrollToSearch(); };
  $('#btn-search-next').onclick = () => { state.searchIdx = (state.searchIdx+1)%state.searchMatches.length; scrollToSearch(); };

  // Confirm
  $('#btn-confirm-ok').onclick = () => closeConfirm(true);
  $('#btn-confirm-cancel').onclick = () => closeConfirm(false);

  // Global shortcuts
  document.addEventListener('keydown', e => {
    if (e.ctrlKey && e.key === 'n') { e.preventDefault(); newSession(); }
    if (e.ctrlKey && e.key === 'b') { e.preventDefault(); $('#sidebar').classList.toggle('collapsed'); }
    if (e.ctrlKey && e.key === 'f') { e.preventDefault(); openSearch(); }
    if (e.key === 'Escape') {
      if (!$('#search-overlay').classList.contains('hidden')) closeSearch();
      else if (!$('#settings-overlay').classList.contains('hidden')) closeSettings();
      else if (!$('#confirm-overlay').classList.contains('hidden')) closeConfirm(false);
      else if (state.generating) stopGeneration();
    }
    if (e.ctrlKey && e.key === 'z' && !state.generating && state.messages.length >= 2) {
      const l = state.messages[state.messages.length-1], p = state.messages[state.messages.length-2];
      if (p.role === 'user' && l.role === 'assistant') {
        e.preventDefault(); inputBox.value = p.content; state.messages.pop(); state.messages.pop();
        renderMessages(); autoResize();
      }
    }
  });
}

function escHtml(s) { const d = document.createElement('div'); d.textContent = s; return d.innerHTML; }
window.addEventListener('beforeunload', () => saveCurrentSession());
