const blessed = require('blessed');
const axios = require('axios');

// Configuration
const CONFIG = {
  API_HOST: 'http://localhost:6789', // Default agent API
  MESH_NAME: 'clawparty'      // Default mesh
};

// State
let state = {
  currentMesh: null,
  currentAgent: null,
  chats: [],
  activeChat: null,
  messages: [],
  users: [],
  openclawAgents: [],
  isLoading: false,
  inputValue: ''
};

// Create screen
const screen = blessed.screen({
  smartCSR: true,
  title: 'ClawParty TUI',
  fullUnicode: true
});

// Create layout
const chatList = blessed.list({
  top: 0,
  left: 0,
  width: '30%',
  height: '80%',
  label: 'Chats',
  keys: true,
  vi: true,
  tags: true,
  border: { type: 'line' },
  style: {
    selected: { bg: 'blue' },
    item: { hover: { bg: 'gray' } }
  }
});

// Main chat area
const chatMessages = blessed.box({
  top: 0,
  left: '30%',
  width: '70%',
  height: '70%',
  label: 'Messages',
  tags: true,
  border: { type: 'line' },
  scrollable: true,
  alwaysScroll: true,
  scrollbar: { ch: ' ', inverse: true }
});

// Input area
const chatInput = blessed.textbox({
  top: '70%',
  left: '30%',
  width: '70%',
  height: '10%',
  label: 'Message',
  inputOnFocus: true,
  keys: true,
  border: { type: 'line' },
  height: 3
});

// Status bar
const statusBar = blessed.box({
  top: '80%',
  left: 0,
  width: '100%',
  height: '20%',
  label: 'Status',
  height: 3,
  border: { type: 'line' },
  style: { bg: 'black', fg: 'white' }
});

// Tabs for switching views
const tabs = blessed.tabbed({
  top: 0,
  left: 0,
  width: '100%',
  height: '5%',
  label: 'Navigation',
  border: { type: 'line' },
  tabs: ['Chats', 'Agents'],
  style: {
    bg: 'black',
    fg: 'white',
    focus: { bg: 'blue' }
  }
});

// Agent list (hidden initially)
const agentList = blessed.list({
  top: '5%',
  left: 0,
  width: '30%',
  height: '75%',
  label: 'OpenClaw Agents',
  keys: true,
  vi: true,
  tags: true,
  border: { type: 'line' },
  hidden: true,
  style: {
    selected: { bg: 'blue' },
    item: { hover: { bg: 'gray' } }
  }
});

// Append all elements to screen
screen.append([chatList, chatMessages, chatInput, statusBar, tabs, agentList]);

// Initialize HTTP client
const api = axios.create({
  baseURL: CONFIG.API_HOST,
  timeout: 5000
});

// Fetch data functions
async function fetchMeshes() {
  try {
    const response = await api.get('/api/meshes');
    const meshes = response.data;
    if (meshes.length > 0 && !state.currentMesh) {
      state.currentMesh = meshes[0].name;
      await fetchChats();
      await fetchUsers();
      await fetchOpenclawAgents();
    }
    updateStatusBar();
  } catch (err) {
    updateStatus(`Error fetching meshes: ${err.message}`);
  }
}

async function fetchChats() {
  if (!state.currentMesh) return;
  
  try {
    state.isLoading = true;
    updateStatusBar();
    const response = await api.get(`/meshes/${state.currentMesh}/apps/ztm/chat/api/chats`);
    state.chats = response.data || [];
    renderChatList();
  } catch (err) {
    updateStatus(`Error fetching chats: ${err.message}`);
  } finally {
    state.isLoading = false;
    updateStatusBar();
  }
}

async function fetchUsers() {
  if (!state.currentMesh) return;
  
  try {
    const response = await api.get(`/meshes/${state.currentMesh}/endpoints?limit=500`);
    state.users = response.data || [];
  } catch (err) {
    updateStatus(`Error fetching users: ${err.message}`);
  }
}

async function fetchOpenclawAgents() {
  try {
    // This would need to be adapted based on actual openclaw service endpoints
    state.openclawAgents = [
      { id: 'agent1', name: 'OpenClaw Agent 1', emoji: '🤖' },
      { id: 'agent2', name: 'OpenClaw Agent 2', emoji: '🤖' }
    ];
    renderAgentList();
  } catch (err) {
    updateStatus(`Error fetching agents: ${err.message}`);
  }
}

async function sendMessage(text) {
  if (!text.trim() || !state.activeChat || state.isLoading) return;
  
  try {
    state.isLoading = true;
    updateStatusBar();
    
    // Add user message to UI immediately
    const userMsg = {
      text: text,
      time: new Date().toLocaleTimeString(),
      sender: 'You',
      isSent: true
    };
    state.messages.push(userMsg);
    renderMessages();
    
    // Clear input
    chatInput.clearValue();
    screen.render();
    
    // Simulate receiving a response after delay
    setTimeout(() => {
      const botMsg = {
        text: `This is a simulated response to: "${text}"`,
        time: new Date().toLocaleTimeString(),
        sender: 'Bot',
        isSent: false
      };
      state.messages.push(botMsg);
      renderMessages();
      state.isLoading = false;
      updateStatusBar();
    }, 1500);
    
  } catch (err) {
    updateStatus(`Error sending message: ${err.message}`);
    state.isLoading = false;
    updateStatusBar();
  }
}

// Render functions
function renderChatList() {
  chatList.clearItems();
  state.chats.forEach((chat, index) => {
    const label = chat.name || `Chat ${index}`;
    chatList.addItem(label);
  });
  
  if (state.activeChat !== null && state.chats[state.activeChat]) {
    chatList.select(state.activeChat);
  }
  
  screen.render();
}

function renderAgentList() {
  agentList.clearItems();
  state.openclawAgents.forEach((agent, index) => {
    const label = `${agent.emoji} ${agent.name}`;
    agentList.addItem(label);
  });
  
  screen.render();
}

function renderMessages() {
  chatMessages.clear();
  
  if (state.messages.length === 0) {
    chatMessages.setContent('No messages yet. Start a conversation!');
    screen.render();
    return;
  }
  
  const messagesText = state.messages.map(msg => {
    const timeStr = `[${msg.time}] `;
    const senderStr = `${msg.sender}: `;
    return `${timeStr}${senderStr}${msg.text}`;
  }).join('\n\n');
  
  chatMessages.setContent(messagesText);
  chatMessages.scrollTo(state.messages.length - 1);
  screen.render();
}

function updateStatus(message = '') {
  if (message) {
    statusBar.setContent(message);
  } else {
    const meshInfo = state.currentMesh ? `Mesh: ${state.currentMesh}` : 'No mesh';
    const chatInfo = state.activeChat !== null && state.chats[state.activeChat] 
      ? `Chat: ${state.chats[state.activeChat].name || 'Unknown'}` 
      : 'No chat selected';
    statusBar.setContent(`${meshInfo} | ${chatInfo}`);
  }
  screen.render();
}

function updateStatusBar() {
  if (state.isLoading) {
    statusBar.setContent('Loading...');
  } else {
    updateStatus();
  }
  screen.render();
}

// Event handlers
chatList.on('select', (item, index) => {
  state.activeChat = index;
  state.messages = []; // Clear messages for new chat
  // In a real implementation, we would fetch messages for this chat
  renderMessages();
  updateStatusBar();
});

agentList.on('select', (item, index) => {
  // Handle agent selection
  updateStatus(`Selected agent: ${state.openclawAgents[index].name}`);
});

chatInput.on('submit', (value) => {
  sendMessage(value);
});

tabs.on('select', (title) => {
  if (title === 'Chats') {
    chatList.show();
    agentList.hide();
  } else if (title === 'Agents') {
    chatList.hide();
    agentList.show();
  }
  screen.render();
});

// Key bindings
screen.key(['escape', 'q', 'C-c'], function(ch, key) {
  return process.exit(0);
});

screen.key(['tab'], function(ch, key) {
  // Focus next element
  screen.focusNext();
});

// Initialize
async function init() {
  updateStatus('Initializing...');
  await fetchMeshes();
  updateStatus('Ready');
}

// Start application
init().then(() => {
  screen.render();
}).catch(err => {
  updateStatus(`Fatal error: ${err.message}`);
  screen.render();
});