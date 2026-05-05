# Snake Game Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a modern, feature-rich Snake Game as a standalone web application with multiple difficulty levels, power-ups, audio system, and cross-platform controls.

**Architecture:** Single-page web application using vanilla JavaScript with modular file structure. Game loop runs at 60fps rendering, with game logic updating at difficulty-specific tick rates. State management via observer pattern for UI updates. LocalStorage for persistence. No external dependencies.

**Tech Stack:** HTML5 Canvas, Vanilla JavaScript (ES6+), CSS3, Web Audio API, LocalStorage

---

## File Structure

```
snake-game/
├── index.html           # Main HTML structure
├── css/
│   └── style.css        # All styling (responsive, animations)
├── js/
│   ├── main.js          # Entry point, initialization
│   ├── game.js          # Core game engine (loop, state, collision)
│   ├── snake.js         # Snake entity class
│   ├── food.js          # Food spawning and power-up system
│   ├── renderer.js      # Canvas rendering and animations
│   ├── input.js         # Input handling (keyboard, touch, swipe)
│   ├── audio.js         # Audio manager (SFX + BGM)
│   ├── storage.js       # LocalStorage wrapper for persistence
│   └── ui.js            # UI management (screens, HUD)
├── assets/
│   ├── sounds/          # Placeholder for audio files
│   │   ├── eat.mp3
│   │   ├── eat-golden.mp3
│   │   ├── eat-purple.mp3
│   │   ├── eat-blue.mp3
│   │   ├── eat-ghost.mp3
│   │   ├── die.mp3
│   │   ├── gameover.mp3
│   │   ├── highscore.mp3
│   │   ├── bgm-easy.mp3
│   │   ├── bgm-medium.mp3
│   │   └── bgm-hard.mp3
│   └── fonts/           # Optional custom fonts
└── README.md            # Setup instructions
```

---

## Setup Phase

### Task 1: Create Project Structure

**Files:**
- Create: `snake-game/`
- Create: `snake-game/css/`
- Create: `snake-game/js/`
- Create: `snake-game/assets/sounds/`
- Create: `snake-game/README.md`

- [ ] **Step 1: Create directory structure**

Run:
```bash
cd /d/DEV/Apps/loom/inspi/claude-code-main
mkdir -p snake-game/css snake-game/js snake-game/assets/sounds
```

Expected: Directories created successfully

- [ ] **Step 2: Create README.md**

Write: `snake-game/README.md`
```markdown
# Snake Game

A modern, feature-rich Snake Game built with vanilla HTML, CSS, and JavaScript.

## Features

- Multiple difficulty levels (Easy, Medium, Hard)
- Various food types with power-ups
- Cross-platform controls (keyboard, touch, swipe)
- High score persistence
- Comprehensive audio system

## How to Play

1. Open `index.html` in a modern web browser
2. Select difficulty level
3. Use arrow keys or WASD to control the snake
4. Eat food to grow and score points
5. Avoid walls and yourself!

## Controls

- Arrow keys / WASD: Move snake
- Space / P: Pause game
- M: Toggle audio
- ESC: Return to menu
- 1/2/3: Select difficulty (from menu)

## Browser Requirements

- HTML5 Canvas support
- ES6 JavaScript support
- (Optional) LocalStorage for high scores
- (Optional) Web Audio API for sound
```

- [ ] **Step 3: Commit**

```bash
cd snake-game
git init
git add README.md
git commit -m "init: Create project structure and README"
```

---

## Core Game Engine

### Task 2: Create Storage Module (Foundation)

**Files:**
- Create: `snake-game/js/storage.js`

- [ ] **Step 1: Write storage.js**

Write: `snake-game/js/storage.js`
```javascript
/**
 * Storage Manager - LocalStorage wrapper with validation and graceful degradation
 */

const STORAGE_KEY = 'snake_game_data';
const STORAGE_VERSION = 1;

class StorageManager {
  constructor() {
    this.available = this._checkAvailability();
    this.data = this._loadData();
  }

  _checkAvailability() {
    try {
      const test = '__storage_test__';
      localStorage.setItem(test, test);
      localStorage.removeItem(test);
      return true;
    } catch (e) {
      console.warn('LocalStorage not available:', e.message);
      return false;
    }
  }

  _loadData() {
    if (!this.available) return this._getDefaultData();

    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return this._getDefaultData();

      const data = JSON.parse(raw);

      // Validate and migrate
      if (data.version !== STORAGE_VERSION) {
        return this._migrateData(data);
      }

      return this._validateData(data);
    } catch (e) {
      console.error('Failed to load from storage:', e);
      return this._getDefaultData();
    }
  }

  _getDefaultData() {
    return {
      version: STORAGE_VERSION,
      highScores: {
        easy: 0,
        medium: 0,
        hard: 0
      },
      settings: {
        sfxVolume: 80,
        musicVolume: 60,
        sfxMuted: false,
        musicMuted: false,
        selectedBgm: 'medium'
      }
    };
  }

  _validateData(data) {
    const validated = this._getDefaultData();

    if (data.highScores) {
      validated.highScores.easy = Math.max(0, Math.min(999999, data.highScores.easy || 0));
      validated.highScores.medium = Math.max(0, Math.min(999999, data.highScores.medium || 0));
      validated.highScores.hard = Math.max(0, Math.min(999999, data.highScores.hard || 0));
    }

    if (data.settings) {
      validated.settings.sfxVolume = Math.max(0, Math.min(100, data.settings.sfxVolume ?? 80));
      validated.settings.musicVolume = Math.max(0, Math.min(100, data.settings.musicVolume ?? 60));
      validated.settings.sfxMuted = Boolean(data.settings.sfxMuted);
      validated.settings.musicMuted = Boolean(data.settings.musicMuted);
      validated.settings.selectedBgm = data.settings.selectedBgm || 'medium';
    }

    return validated;
  }

  _migrateData(data) {
    console.log('Migrating storage data from version', data.version || 'unknown');
    // Future migrations go here
    return this._validateData(data);
  }

  save() {
    if (!this.available) return false;

    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.data));
      return true;
    } catch (e) {
      if (e.name === 'QuotaExceededError') {
        console.warn('LocalStorage quota exceeded');
        // Could notify UI here
      }
      return false;
    }
  }

  getHighScore(difficulty) {
    return this.data.highScores[difficulty] || 0;
  }

  setHighScore(difficulty, score) {
    if (score > this.data.highScores[difficulty]) {
      this.data.highScores[difficulty] = score;
      return this.save();
    }
    return false;
  }

  getSettings() {
    return { ...this.data.settings };
  }

  updateSettings(newSettings) {
    this.data.settings = { ...this.data.settings, ...newSettings };
    return this.save();
  }
}

// Export for use in other modules
if (typeof module !== 'undefined' && module.exports) {
  module.exports = StorageManager;
}
```

- [ ] **Step 2: Verify file created**

Run: `ls -la snake-game/js/storage.js`
Expected: File exists with content

- [ ] **Step 3: Commit**

```bash
git add js/storage.js
git commit -m "feat: Add storage manager with LocalStorage wrapper"
```

---

### Task 3: Create Input Handler Module

**Files:**
- Create: `snake-game/js/input.js`

- [ ] **Step 1: Write input.js**

Write: `snake-game/js/input.js`
```javascript
/**
 * Input Handler - Unified input system for keyboard, touch, and swipe
 */

class InputHandler {
  constructor() {
    this.buffer = [];
    this.bufferSize = 2;
    this.bufferExpireMs = 500;
    this.lastInputTime = 0;

    this.currentDirection = 'right';
    this.nextDirection = 'right';

    this.swipe = {
      startX: 0,
      startY: 0,
      startTime: 0,
      thresholdVelocity: 300, // px/s
      maxDuration: 500 // ms
    };

    this.observers = [];
    this._initListeners();
  }

  addObserver(callback) {
    this.observers.push(callback);
  }

  notifyObservers(input) {
    this.observers.forEach(cb => cb(input));
  }

  _initListeners() {
    // Keyboard
    document.addEventListener('keydown', (e) => this._handleKeyDown(e));

    // Touch for swipe detection
    document.addEventListener('touchstart', (e) => this._handleTouchStart(e), { passive: false });
    document.addEventListener('touchend', (e) => this._handleTouchEnd(e), { passive: false });

    // Touch buttons (delegated)
    document.addEventListener('touchstart', (e) => this._handleTouchButton(e), { passive: false });
    document.addEventListener('click', (e) => this._handleTouchButton(e));
  }

  _handleKeyDown(e) {
    const keyMap = {
      'ArrowUp': 'up', 'KeyW': 'up',
      'ArrowDown': 'down', 'KeyS': 'down',
      'ArrowLeft': 'left', 'KeyA': 'left',
      'ArrowRight': 'right', 'KeyD': 'right',
      'Space': 'pause', 'KeyP': 'pause',
      'KeyM': 'mute',
      'Escape': 'menu',
      'Digit1': 'difficulty-easy',
      'Digit2': 'difficulty-medium',
      'Digit3': 'difficulty-hard'
    };

    const action = keyMap[e.code];
    if (!action) return;

    e.preventDefault();

    if (action.startsWith('difficulty-')) {
      this.notifyObservers({ type: 'difficulty', value: action.split('-')[1] });
      return;
    }

    if (['pause', 'mute', 'menu'].includes(action)) {
      this.notifyObservers({ type: action, value: true });
      return;
    }

    this._queueDirection(action);
  }

  _queueDirection(direction) {
    // Prevent reverse direction
    const opposites = { up: 'down', down: 'up', left: 'right', right: 'left' };
    if (opposites[direction] === this.nextDirection) return;

    const now = Date.now();

    // Expire old inputs
    this.buffer = this.buffer.filter(item => now - item.time < this.bufferExpireMs);

    // Add to buffer (drop oldest if full)
    this.buffer.push({ direction, time: now });
    if (this.buffer.length > this.bufferSize) {
      this.buffer.shift();
    }
  }

  processNextInput() {
    if (this.buffer.length > 0) {
      const input = this.buffer.shift();
      this.currentDirection = input.direction;
      this.nextDirection = input.direction;
      return this.currentDirection;
    }
    return this.nextDirection;
  }

  _handleTouchStart(e) {
    if (e.target.closest('.touch-controls')) return; // Let touch buttons handle it

    const touch = e.touches[0];
    this.swipe.startX = touch.clientX;
    this.swipe.startY = touch.clientY;
    this.swipe.startTime = Date.now();
  }

  _handleTouchEnd(e) {
    if (e.target.closest('.touch-controls')) return;

    const touch = e.changedTouches[0];
    const deltaX = touch.clientX - this.swipe.startX;
    const deltaY = touch.clientY - this.swipe.startY;
    const deltaTime = Date.now() - this.swipe.startTime;

    if (deltaTime > this.swipe.maxDuration) return;

    const velocity = Math.sqrt(deltaX * deltaX + deltaY * deltaY) / (deltaTime / 1000);

    if (velocity < this.swipe.thresholdVelocity) return;

    let direction;
    if (Math.abs(deltaX) > Math.abs(deltaY)) {
      direction = deltaX > 0 ? 'right' : 'left';
    } else {
      direction = deltaY > 0 ? 'down' : 'up';
    }

    this._queueDirection(direction);
    this._showSwipeIndicator(direction);
  }

  _handleTouchButton(e) {
    const button = e.target.closest('.touch-btn');
    if (!button) return;

    e.preventDefault();
    const direction = button.dataset.direction;
    if (direction) {
      this._queueDirection(direction);
    }
  }

  _showSwipeIndicator(direction) {
    const indicator = document.createElement('div');
    indicator.className = 'swipe-indicator';

    const arrows = { up: '↑', down: '↓', left: '←', right: '→' };
    indicator.textContent = arrows[direction];

    document.body.appendChild(indicator);
    setTimeout(() => indicator.remove(), 200);
  }

  reset() {
    this.buffer = [];
    this.currentDirection = 'right';
    this.nextDirection = 'right';
  }
}

if (typeof module !== 'undefined' && module.exports) {
  module.exports = InputHandler;
}
```

- [ ] **Step 2: Commit**

```bash
git add js/input.js
git commit -m "feat: Add input handler for keyboard, touch, and swipe"
```

---

### Task 4: Create Snake Entity Module

**Files:**
- Create: `snake-game/js/snake.js`

- [ ] **Step 1: Write snake.js**

Write: `snake-game/js/snake.js`
```javascript
/**
 * Snake Entity - Handles snake state, movement, and power-ups
 */

const GRID_SIZE = 20;
const POWERUP_DURATION = 5000; // 5 seconds

class Snake {
  constructor(startX, startY) {
    this.body = [
      { x: startX, y: startY },
      { x: startX - 1, y: startY },
      { x: startX - 2, y: startY }
    ];
    this.direction = 'right';
    this.growPending = 0;

    // Power-up states
    this.powerUps = {
      speedBoost: { active: false, endTime: 0 },
      invincible: { active: false, endTime: 0 },
      ghost: { active: false, endTime: 0 }
    };
  }

  getHead() {
    return this.body[0];
  }

  setDirection(direction) {
    this.direction = direction;
  }

  move(wrapAround = false, gridWidth = GRID_SIZE, gridHeight = GRID_SIZE) {
    const head = this.getHead();
    let newHead = { ...head };

    switch (this.direction) {
      case 'up': newHead.y--; break;
      case 'down': newHead.y++; break;
      case 'left': newHead.x--; break;
      case 'right': newHead.x++; break;
    }

    // Wall wrapping for Easy mode or Ghost mode
    if (wrapAround || this.powerUps.ghost.active) {
      if (newHead.x < 0) newHead.x = gridWidth - 1;
      if (newHead.x >= gridWidth) newHead.x = 0;
      if (newHead.y < 0) newHead.y = gridHeight - 1;
      if (newHead.y >= gridHeight) newHead.y = 0;
    }

    this.body.unshift(newHead);

    if (this.growPending > 0) {
      this.growPending--;
    } else {
      this.body.pop();
    }
  }

  grow(amount = 1) {
    this.growPending += amount;
  }

  shrink() {
    if (this.body.length > 3) {
      this.body.pop();
      return true;
    }
    return false; // Too short to shrink
  }

  checkSelfCollision() {
    if (this.powerUps.ghost.active || this.powerUps.invincible.active) {
      return false;
    }

    const head = this.getHead();
    for (let i = 1; i < this.body.length; i++) {
      if (head.x === this.body[i].x && head.y === this.body[i].y) {
        return true;
      }
    }
    return false;
  }

  checkWallCollision(gridWidth = GRID_SIZE, gridHeight = GRID_SIZE) {
    if (this.powerUps.ghost.active || this.powerUps.invincible.active) {
      return false;
    }

    const head = this.getHead();
    return head.x < 0 || head.x >= gridWidth || head.y < 0 || head.y >= gridHeight;
  }

  activatePowerUp(type) {
    const now = Date.now();
    const endTime = now + POWERUP_DURATION;

    // Deactivate all power-ups first (no stacking)
    Object.keys(this.powerUps).forEach(key => {
      this.powerUps[key].active = false;
    });

    // Activate new power-up
    if (this.powerUps[type]) {
      this.powerUps[type].active = true;
      this.powerUps[type].endTime = endTime;
    }

    return endTime;
  }

  updatePowerUps() {
    const now = Date.now();
    let activeType = null;
    let remaining = 0;

    for (const [type, state] of Object.entries(this.powerUps)) {
      if (state.active) {
        if (now >= state.endTime) {
          state.active = false;
        } else {
          activeType = type;
          remaining = Math.max(0, (state.endTime - now) / 1000);
        }
      }
    }

    return { activeType, remaining };
  }

  canPickUp(type) {
    // Can't pick up same power-up while active
    if (this.powerUps[type] && this.powerUps[type].active) {
      return false;
    }
    return true;
  }

  getRenderState() {
    const powerUpState = this.updatePowerUps();
    return {
      body: this.body,
      powerUp: powerUpState.activeType,
      remaining: powerUpState.remaining
    };
  }
}

if (typeof module !== 'undefined' && module.exports) {
  module.exports = Snake;
}
```

- [ ] **Step 2: Commit**

```bash
git add js/snake.js
git commit -m "feat: Add snake entity with movement and power-ups"
```

---

### Task 5: Create Food System Module

**Files:**
- Create: `snake-game/js/food.js`

- [ ] **Step 1: Write food.js**

Write: `snake-game/js/food.js`
```javascript
/**
 * Food System - Spawns and manages food with power-ups
 */

const GRID_SIZE = 20;
const MAX_FOOD = 3;

const FOOD_TYPES = {
  regular: { points: 10, color: '#e74c3c', duration: null },
  golden: { points: 50, color: '#ffd700', duration: 8000 },
  purple: { points: 30, color: '#9b59b6', duration: 8000 },
  blue: { points: 20, color: '#3498db', duration: 10000 },
  ghost: { points: 100, color: '#ffffff', duration: 6000 }
};

const SPAWN_PROBABILITIES = [
  { type: 'regular', weight: 70 },
  { type: 'golden', weight: 10 },
  { type: 'purple', weight: 10 },
  { type: 'blue', weight: 5 },
  { type: 'ghost', weight: 5 }
];

class FoodSystem {
  constructor() {
    this.foodItems = [];
    this.lastSpawnTime = 0;
  }

  spawn(snakeBody, gridWidth = GRID_SIZE, gridHeight = GRID_SIZE) {
    // Remove expired food
    this._removeExpired();

    // Check if we should spawn new food
    if (this.foodItems.length >= MAX_FOOD) return null;

    const now = Date.now();
    if (now - this.lastSpawnTime < 3000) return null; // Spawn rate: every 3 seconds

    // Find valid spawn location
    const location = this._findSpawnLocation(snakeBody, gridWidth, gridHeight);
    if (!location) return null;

    // Select food type based on probabilities
    const type = this._selectFoodType();
    const foodData = FOOD_TYPES[type];

    const food = {
      id: Date.now() + Math.random(),
      x: location.x,
      y: location.y,
      type: type,
      ...foodData,
      spawnTime: now,
      expireTime: foodData.duration ? now + foodData.duration : null
    };

    this.foodItems.push(food);
    this.lastSpawnTime = now;

    return food;
  }

  forceSpawnHardMode(snakeBody, gridWidth = GRID_SIZE, gridHeight = GRID_SIZE) {
    // Hard mode: spawn every 2 seconds
    const now = Date.now();
    if (now - this.lastSpawnTime < 2000) return null;
    if (this.foodItems.length >= MAX_FOOD) return null;

    return this.spawn(snakeBody, gridWidth, gridHeight);
  }

  _findSpawnLocation(snakeBody, gridWidth, gridHeight) {
    const safeZone = 2; // Cannot spawn within 2 cells of head
    const head = snakeBody[0];

    const occupied = new Set();
    snakeBody.forEach(seg => occupied.add(`${seg.x},${seg.y}`));
    this.foodItems.forEach(f => occupied.add(`${f.x},${f.y}`));

    const attempts = 100;
    for (let i = 0; i < attempts; i++) {
      const x = Math.floor(Math.random() * gridWidth);
      const y = Math.floor(Math.random() * gridHeight);

      const key = `${x},${y}`;
      if (occupied.has(key)) continue;

      // Check safe zone
      const distToHead = Math.abs(x - head.x) + Math.abs(y - head.y);
      if (distToHead < safeZone) continue;

      return { x, y };
    }

    return null; // Could not find valid location
  }

  _selectFoodType() {
    const totalWeight = SPAWN_PROBABILITIES.reduce((sum, item) => sum + item.weight, 0);
    let random = Math.random() * totalWeight;

    for (const item of SPAWN_PROBABILITIES) {
      random -= item.weight;
      if (random <= 0) return item.type;
    }

    return 'regular';
  }

  _removeExpired() {
    const now = Date.now();
    this.foodItems = this.foodItems.filter(food => {
      if (food.expireTime && now >= food.expireTime) {
        return false;
      }
      return true;
    });
  }

  checkCollision(headX, headY) {
    const index = this.foodItems.findIndex(f => f.x === headX && f.y === headY);
    if (index !== -1) {
      const food = this.foodItems.splice(index, 1)[0];
      return food;
    }
    return null;
  }

  getRenderState() {
    const now = Date.now();
    return this.foodItems.map(food => ({
      x: food.x,
      y: food.y,
      type: food.type,
      color: food.color,
      remainingLifetime: food.expireTime ? Math.max(0, food.expireTime - now) : null
    }));
  }

  reset() {
    this.foodItems = [];
    this.lastSpawnTime = 0;
  }
}

if (typeof module !== 'undefined' && module.exports) {
  module.exports = { FoodSystem, FOOD_TYPES };
}
```

- [ ] **Step 2: Commit**

```bash
git add js/food.js
git commit -m "feat: Add food system with spawning and power-ups"
```

---

### Task 6: Create Game Engine Module

**Files:**
- Create: `snake-game/js/game.js`

- [ ] **Step 1: Write game.js**

Write: `snake-game/js/game.js`
```javascript
/**
 * Game Engine - Core game loop, state management, and scoring
 */

const Snake = require('./snake').Snake;
const { FoodSystem } = require('./food');

const DIFFICULTY_SETTINGS = {
  easy: { tickRate: 150, wallWrap: true },
  medium: { tickRate: 100, wallWrap: false },
  hard: { tickRate: 60, wallWrap: false }
};

class Game {
  constructor(storage, inputHandler) {
    this.storage = storage;
    this.inputHandler = inputHandler;

    this.gridSize = 20;
    this.difficulty = 'medium';

    this.snake = null;
    this.foodSystem = new FoodSystem();

    this.state = {
      score: 0,
      combo: 0,
      lastEatTime: 0,
      paused: false,
      gameOver: false,
      menu: true
    };

    this.observers = [];

    this._lastTick = 0;
    this._animationFrame = null;

    // Bind input handler
    this.inputHandler.addObserver((input) => this._handleInput(input));
  }

  addObserver(callback) {
    this.observers.push(callback);
  }

  notifyObservers() {
    const state = this.getState();
    this.observers.forEach(cb => cb(state));
  }

  _handleInput(input) {
    if (input.type === 'difficulty') {
      if (this.state.menu) {
        this.setDifficulty(input.value);
      }
    } else if (input.type === 'pause') {
      if (!this.state.menu && !this.state.gameOver) {
        this.state.paused = !this.state.paused;
      }
    } else if (input.type === 'menu') {
      if (this.state.gameOver) {
        this.showMenu();
      }
    }
  }

  setDifficulty(difficulty) {
    if (!DIFFICULTY_SETTINGS[difficulty]) return;
    this.difficulty = difficulty;
    this.notifyObservers();
  }

  start() {
    this.snake = new Snake(10, 10);
    this.foodSystem.reset();
    this.inputHandler.reset();

    this.state = {
      score: 0,
      combo: 0,
      lastEatTime: 0,
      paused: false,
      gameOver: false,
      menu: false
    };

    this._lastTick = 0;
    this._gameLoop();

    this.notifyObservers();
  }

  showMenu() {
    this.state.menu = true;
    this.state.gameOver = false;
    this.state.paused = false;
    this.notifyObservers();
  }

  _gameLoop(timestamp = 0) {
    if (this.state.menu || this.state.gameOver) return;

    this._animationFrame = requestAnimationFrame((t) => this._gameLoop(t));

    if (this.state.paused) return;

    const settings = DIFFICULTY_SETTINGS[this.difficulty];
    const tickDelta = timestamp - this._lastTick;

    // Process input
    const direction = this.inputHandler.processNextInput();
    if (direction) {
      this.snake.setDirection(direction);
    }

    // Game logic tick
    if (tickDelta >= settings.tickRate) {
      this._update();
      this._lastTick = timestamp;
    }

    // Always update power-ups and spawn food at 60fps
    this._updateContinuous();
  }

  _update() {
    const settings = DIFFICULTY_SETTINGS[this.difficulty];

    // Move snake
    this.snake.move(settings.wallWrap, this.gridSize, this.gridSize);

    // Check collisions
    const head = this.snake.getHead();

    // Wall collision
    if (!settings.wallWrap && this.snake.checkWallCollision(this.gridSize, this.gridSize)) {
      this._gameOver();
      return;
    }

    // Self collision
    if (this.snake.checkSelfCollision()) {
      this._gameOver();
      return;
    }

    // Food collision
    const food = this.foodSystem.checkCollision(head.x, head.y);
    if (food) {
      this._eatFood(food);
    }

    this.notifyObservers();
  }

  _updateContinuous() {
    // Update power-ups (visual timer needs 60fps updates)
    this.snake.updatePowerUps();
    this.foodSystem._removeExpired();
  }

  _eatFood(food) {
    const now = Date.now();

    // Apply food effect
    switch (food.type) {
      case 'regular':
        this.snake.grow(1);
        break;
      case 'golden':
        if (this.snake.canPickUp('speedBoost')) {
          this.snake.activatePowerUp('speedBoost');
        }
        this.snake.grow(1);
        break;
      case 'purple':
        if (this.snake.canPickUp('invincible')) {
          this.snake.activatePowerUp('invincible');
        }
        this.snake.grow(1);
        break;
      case 'blue':
        this.snake.shrink();
        break;
      case 'ghost':
        if (this.snake.canPickUp('ghost')) {
          this.snake.activatePowerUp('ghost');
        }
        this.snake.grow(1);
        break;
    }

    // Calculate score
    let score = food.points;

    // Speed multiplier
    if (now - this.state.lastEatTime < 2000) {
      score = Math.floor(score * 1.5);
    }
    this.state.lastEatTime = now;

    // Combo bonus
    const lastCombo = this.state.combo;
    if (food.type === lastCombo?.type && now - lastCombo.time < 5000) {
      this.state.combo = { type: food.type, count: lastCombo.count + 1, time: now };
      score += Math.min(this.state.combo.count, 5) * 10;
    } else {
      this.state.combo = { type: food.type, count: 1, time: now };
    }

    this.state.score = Math.min(999999, this.state.score + score);

    this.notifyObservers();
  }

  _gameOver() {
    this.state.gameOver = true;
    this.state.paused = false;

    // Check high score
    const isNewHigh = this.storage.setHighScore(this.difficulty, this.state.score);

    if (isNewHigh) {
      // TODO: Play high score sound
    }

    // TODO: Play death sound

    this.notifyObservers();
  }

  getState() {
    return {
      snake: this.snake ? this.snake.getRenderState() : null,
      food: this.foodSystem.getRenderState(),
      score: this.state.score,
      combo: this.state.combo,
      difficulty: this.difficulty,
      highScore: this.storage.getHighScore(this.difficulty),
      paused: this.state.paused,
      gameOver: this.state.gameOver,
      menu: this.state.menu
    };
  }

  destroy() {
    if (this._animationFrame) {
      cancelAnimationFrame(this._animationFrame);
    }
  }
}

if (typeof module !== 'undefined' && module.exports) {
  module.exports = Game;
}
```

- [ ] **Step 2: Commit**

```bash
git add js/game.js
git commit -m "feat: Add game engine with loop and scoring"
```

---

### Task 7: Create Renderer Module

**Files:**
- Create: `snake-game/js/renderer.js`

- [ ] **Step 1: Write renderer.js**

Write: `snake-game/js/renderer.js`
```javascript
/**
 * Renderer - Canvas rendering and animations
 */

class Renderer {
  constructor(canvasId) {
    this.canvas = document.getElementById(canvasId);
    if (!this.canvas) {
      throw new Error(`Canvas with id "${canvasId}" not found`);
    }

    this.ctx = this.canvas.getContext('2d');
    this.baseCellSize = 25;
    this.gridSize = 20;
    this.canvasSize = 500;

    this.colors = {
      background: '#1a1a2e',
      gridLines: 'rgba(22, 33, 62, 0.3)',
      snakeHead: '#4ecca3',
      snakeBody: '#45b393',
      snakeBodyGradient: '#38ada9',
      powerUpSpeed: '#ffd700',
      powerUpInvincible: '#9b59b6',
      powerUpGhost: 'rgba(255, 255, 255, 0.7)'
    };

    this.particles = [];

    this._setupCanvas();
  }

  _setupCanvas() {
    this.canvas.width = this.canvasSize;
    this.canvas.height = this.canvasSize;
  }

  getCellSize() {
    const rect = this.canvas.getBoundingClientRect();
    return rect.width / this.gridSize;
  }

  resize() {
    const container = this.canvas.parentElement;
    const maxSize = Math.min(container.clientWidth - 40, 500);
    this.canvas.style.width = maxSize + 'px';
    this.canvas.style.height = maxSize + 'px';
  }

  render(gameState) {
    const cellSize = this.getCellSize();

    // Clear
    this.ctx.fillStyle = this.colors.background;
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

    // Draw grid
    this._drawGrid(cellSize);

    // Draw food
    this._drawFood(gameState.food, cellSize);

    // Draw snake
    this._drawSnake(gameState.snake, cellSize);

    // Draw particles
    this._drawParticles(cellSize);

    // Draw power-up glow
    if (gameState.snake?.powerUp && gameState.snake.remaining > 0) {
      this._drawPowerUpGlow(gameState.snake.powerUp, gameState.snake.remaining, cellSize);
    }
  }

  _drawGrid(cellSize) {
    this.ctx.strokeStyle = this.colors.gridLines;
    this.ctx.lineWidth = 1;

    for (let i = 0; i <= this.gridSize; i++) {
      // Vertical
      this.ctx.beginPath();
      this.ctx.moveTo(i * cellSize, 0);
      this.ctx.lineTo(i * cellSize, this.canvas.height);
      this.ctx.stroke();

      // Horizontal
      this.ctx.beginPath();
      this.ctx.moveTo(0, i * cellSize);
      this.ctx.lineTo(this.canvas.width, i * cellSize);
      this.ctx.stroke();
    }
  }

  _drawSnake(snakeState, cellSize) {
    if (!snakeState) return;

    snakeState.body.forEach((segment, index) => {
      const x = segment.x * cellSize;
      const y = segment.y * cellSize;

      // Determine color
      let color;
      if (snakeState.powerUp === 'speedBoost') {
        color = this.colors.powerUpSpeed;
      } else if (snakeState.powerUp === 'invincible') {
        color = this.colors.powerUpInvincible;
      } else if (snakeState.powerUp === 'ghost') {
        color = this.colors.powerUpGhost;
      } else if (index === 0) {
        color = this.colors.snakeHead;
      } else {
        // Gradient for body
        const ratio = index / snakeState.body.length;
        color = ratio > 0.5 ? this.colors.snakeBody : this.colors.snakeBodyGradient;
      }

      this.ctx.fillStyle = color;
      this._drawRoundedRect(x + 1, y + 1, cellSize - 2, cellSize - 2, 4);

      // Shield icon for invincibility
      if (snakeState.powerUp === 'invincible' && index === 0) {
        this.ctx.fillStyle = '#fff';
        this.ctx.font = `${cellSize * 0.6}px Arial`;
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';
        this.ctx.fillText('🛡️', x + cellSize / 2, y + cellSize / 2);
      }
    });
  }

  _drawFood(foodItems, cellSize) {
    foodItems.forEach(food => {
      const x = food.x * cellSize + cellSize / 2;
      const y = food.y * cellSize + cellSize / 2;
      const radius = cellSize * 0.4;

      this.ctx.fillStyle = food.color;
      this.ctx.beginPath();
      this.ctx.arc(x, y, radius, 0, Math.PI * 2);
      this.ctx.fill();

      // Glow effect for special food
      if (food.type !== 'regular') {
        this.ctx.shadowColor = food.color;
        this.ctx.shadowBlur = 10;
        this.ctx.beginPath();
        this.ctx.arc(x, y, radius, 0, Math.PI * 2);
        this.ctx.fill();
        this.ctx.shadowBlur = 0;
      }

      // Expire animation
      if (food.remainingLifetime !== null) {
        const alpha = food.remainingLifetime / 10000;
        this.ctx.fillStyle = `rgba(0, 0, 0, ${1 - alpha})`;
        this.ctx.beginPath();
        this.ctx.arc(x, y, radius, 0, Math.PI * 2);
        this.ctx.fill();
      }
    });
  }

  _drawPowerUpGlow(powerUpType, remaining, cellSize) {
    const colors = {
      speedBoost: this.colors.powerUpSpeed,
      invincible: this.colors.powerUpInvincible,
      ghost: this.colors.powerUpGhost
    };

    const color = colors[powerUpType];
    const pulse = Math.sin(Date.now() / 200) * 0.2 + 0.8;

    this.ctx.strokeStyle = color;
    this.ctx.globalAlpha = pulse * 0.5;
    this.ctx.lineWidth = 4;
    this.ctx.strokeRect(2, 2, this.canvas.width - 4, this.canvas.height - 4);
    this.ctx.globalAlpha = 1;
  }

  _drawRoundedRect(x, y, width, height, radius) {
    this.ctx.beginPath();
    this.ctx.moveTo(x + radius, y);
    this.ctx.lineTo(x + width - radius, y);
    this.ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
    this.ctx.lineTo(x + width, y + height - radius);
    this.ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
    this.ctx.lineTo(x + radius, y + height);
    this.ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
    this.ctx.lineTo(x, y + radius);
    this.ctx.quadraticCurveTo(x, y, x + radius, y);
    this.ctx.closePath();
    this.ctx.fill();
  }

  spawnDeathParticles(headPos, cellSize) {
    for (let i = 0; i < 20; i++) {
      this.particles.push({
        x: headPos.x * cellSize + cellSize / 2,
        y: headPos.y * cellSize + cellSize / 2,
        vx: (Math.random() - 0.5) * 200,
        vy: (Math.random() - 0.5) * 200,
        life: 1,
        color: this.colors.snakeHead
      });
    }
  }

  _drawParticles(cellSize) {
    const dt = 1 / 60;

    this.particles = this.particles.filter(p => {
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      p.life -= dt;

      if (p.life > 0) {
        this.ctx.fillStyle = p.color;
        this.ctx.globalAlpha = p.life;
        this.ctx.fillRect(p.x - 2, p.y - 2, 4, 4);
        this.ctx.globalAlpha = 1;
        return true;
      }
      return false;
    });
  }
}

if (typeof module !== 'undefined' && module.exports) {
  module.exports = Renderer;
}
```

- [ ] **Step 2: Commit**

```bash
git add js/renderer.js
git commit -m "feat: Add canvas renderer with animations"
```

---

### Task 8: Create Audio Manager Module

**Files:**
- Create: `snake-game/js/audio.js`

- [ ] **Step 1: Write audio.js**

Write: `snake-game/js/audio.js`
```javascript
/**
 * Audio Manager - Handles sound effects and background music
 */

class AudioManager {
  constructor(storage) {
    this.storage = storage;
    this.audioContext = null;
    this.sounds = {};
    this.bgm = null;
    this.bgmGain = null;
    this.sfxGain = null;

    this.settings = storage.getSettings();
    this.currentBgm = null;
    this.crossfadeTimeout = null;

    this._initAudio();
  }

  _initAudio() {
    try {
      this.audioContext = new (window.AudioContext || window.webkitAudioContext)();

      // Create gain nodes
      this.sfxGain = this.audioContext.createGain();
      this.sfxGain.gain.value = this.settings.sfxMuted ? 0 : this.settings.sfxVolume / 100;
      this.sfxGain.connect(this.audioContext.destination);

      this.bgmGain = this.audioContext.createGain();
      this.bgmGain.gain.value = this.settings.musicMuted ? 0 : this.settings.musicVolume / 100;
      this.bgmGain.connect(this.audioContext.destination);
    } catch (e) {
      console.warn('Web Audio API not available:', e.message);
    }
  }

  async _startAudioContext() {
    if (this.audioContext && this.audioContext.state === 'suspended') {
      await this.audioContext.resume();
    }
  }

  loadSounds(soundFiles) {
    if (!this.audioContext) return;

    soundFiles.forEach(file => {
      const audio = new Audio(file.path);
      audio.preload = 'auto';
      this.sounds[file.name] = audio;
    });
  }

  play(soundName) {
    if (!this.audioContext || this.settings.sfxMuted) return;

    this._startAudioContext();

    const sound = this.sounds[soundName];
    if (sound) {
      sound.currentTime = 0;
      sound.play().catch(e => console.warn('Failed to play sound:', e.message));
    }
  }

  async playBgm(bgmName, crossfade = true) {
    if (!this.audioContext || this.settings.musicMuted) return;

    await this._startAudioContext();

    if (this.currentBgm === bgmName) return;

    // Clear any pending crossfade
    if (this.crossfadeTimeout) {
      clearTimeout(this.crossfadeTimeout);
      this.crossfadeTimeout = null;
    }

    const newBgm = this.sounds[bgmName];
    if (!newBgm) return;

    if (crossfade && this.bgm) {
      // Crossfade over 1.5 seconds
      const fadeDuration = 1.5;
      const steps = 30;
      const stepDuration = (fadeDuration / steps) * 1000;

      let step = 0;
      const fade = setInterval(() => {
        step++;
        const newVolume = 1 - (step / steps);
        const oldVolume = step / steps;

        if (this.bgmGain) {
          this.bgmGain.gain.value = oldVolume * (this.settings.musicVolume / 100);
        }

        if (step >= steps) {
          clearInterval(fade);
          if (this.bgm) {
            this.bgm.pause();
            this.bgm.currentTime = 0;
          }
          this.bgm = newBgm;
          this.bgm.loop = true;
          this.bgm.play().catch(e => console.warn('BGM play failed:', e));

          if (this.bgmGain) {
            this.bgmGain.gain.value = this.settings.musicVolume / 100;
          }
          this.currentBgm = bgmName;
        }
      }, stepDuration);
    } else {
      if (this.bgm) {
        this.bgm.pause();
        this.bgm.currentTime = 0;
      }
      this.bgm = newBgm;
      this.bgm.loop = true;
      this.bgm.play().catch(e => console.warn('BGM play failed:', e));
      this.currentBgm = bgmName;
    }
  }

  stopBgm(crossfade = true) {
    if (!this.bgm) return;

    if (crossfade) {
      const fadeDuration = 2;
      const steps = 40;
      const stepDuration = (fadeDuration / steps) * 1000;

      let step = 0;
      const fade = setInterval(() => {
        step++;
        const volume = 1 - (step / steps);

        if (this.bgmGain) {
          this.bgmGain.gain.value = volume * (this.settings.musicVolume / 100);
        }

        if (step >= steps) {
          clearInterval(fade);
          this.bgm.pause();
          this.bgm.currentTime = 0;
          this.bgm = null;
          this.currentBgm = null;
        }
      }, stepDuration);
    } else {
      this.bgm.pause();
      this.bgm.currentTime = 0;
      this.bgm = null;
      this.currentBgm = null;
    }
  }

  setSfxVolume(volume) {
    volume = Math.max(0, Math.min(100, volume));
    this.settings.sfxVolume = volume;
    if (this.sfxGain) {
      this.sfxGain.gain.value = this.settings.sfxMuted ? 0 : volume / 100;
    }
    this.storage.updateSettings({ sfxVolume: volume });
  }

  setMusicVolume(volume) {
    volume = Math.max(0, Math.min(100, volume));
    this.settings.musicVolume = volume;
    if (this.bgmGain && !this.settings.musicMuted) {
      this.bgmGain.gain.value = volume / 100;
    }
    this.storage.updateSettings({ musicVolume: volume });
  }

  toggleSfxMute() {
    this.settings.sfxMuted = !this.settings.sfxMuted;
    if (this.sfxGain) {
      this.sfxGain.gain.value = this.settings.sfxMuted ? 0 : this.settings.sfxVolume / 100;
    }
    this.storage.updateSettings({ sfxMuted: this.settings.sfxMuted });
    return this.settings.sfxMuted;
  }

  toggleMusicMute() {
    this.settings.musicMuted = !this.settings.musicMuted;
    if (this.bgmGain) {
      this.bgmGain.gain.value = this.settings.musicMuted ? 0 : this.settings.musicVolume / 100;
    }
    if (this.settings.musicMuted && this.bgm) {
      this.bgm.pause();
    } else if (!this.settings.musicMuted && this.bgm && this.currentBgm) {
      this.bgm.play();
    }
    this.storage.updateSettings({ musicMuted: this.settings.musicMuted });
    return this.settings.musicMuted;
  }

  getSettings() {
    return {
      sfxVolume: this.settings.sfxVolume,
      musicVolume: this.settings.musicVolume,
      sfxMuted: this.settings.sfxMuted,
      musicMuted: this.settings.musicMuted
    };
  }
}

if (typeof module !== 'undefined' && module.exports) {
  module.exports = AudioManager;
}
```

- [ ] **Step 2: Commit**

```bash
git add js/audio.js
git commit -m "feat: Add audio manager for SFX and BGM"
```

---

### Task 9: Create UI Manager Module

**Files:**
- Create: `snake-game/js/ui.js`

- [ ] **Step 1: Write ui.js**

Write: `snake-game/js/ui.js`
```javascript
/**
 * UI Manager - Handles screens, HUD, and settings
 */

class UI {
  constructor(game, renderer, audio) {
    this.game = game;
    this.renderer = renderer;
    this.audio = audio;

    this.elements = {};
    this._initElements();
    this._bindEvents();

    // Subscribe to game state changes
    this.game.addObserver((state) => this._updateUI(state));
  }

  _initElements() {
    this.elements = {
      score: document.getElementById('score'),
      highScore: document.getElementById('highScore'),
      difficulty: document.getElementById('difficulty'),
      powerUpTimer: document.getElementById('powerUpTimer'),

      menuScreen: document.getElementById('menuScreen'),
      gameScreen: document.getElementById('gameScreen'),
      gameOverScreen: document.getElementById('gameOverScreen'),
      pauseScreen: document.getElementById('pauseScreen'),

      finalScore: document.getElementById('finalScore'),
      newHighScore: document.getElementById('newHighScore'),

      touchControls: document.querySelector('.touch-controls'),

      // Settings
      sfxVolume: document.getElementById('sfxVolume'),
      musicVolume: document.getElementById('musicVolume'),
      muteSfx: document.getElementById('muteSfx'),
      muteMusic: document.getElementById('muteMusic'),

      // Menu buttons
      btnEasy: document.getElementById('btnEasy'),
      btnMedium: document.getElementById('btnMedium'),
      btnHard: document.getElementById('btnHard')
    };
  }

  _bindEvents() {
    // Difficulty buttons
    this.elements.btnEasy?.addEventListener('click', () => this._startGame('easy'));
    this.elements.btnMedium?.addEventListener('click', () => this._startGame('medium'));
    this.elements.btnHard?.addEventListener('click', () => this._startGame('hard'));

    // Play again button
    document.getElementById('btnPlayAgain')?.addEventListener('click', () => {
      this.game.start();
    });

    // Menu button
    document.getElementById('btnMenu')?.addEventListener('click', () => {
      this.game.showMenu();
    });

    // Resume button
    document.getElementById('btnResume')?.addEventListener('click', () => {
      this.game.state.paused = false;
      this.game.notifyObservers();
    });

    // Settings controls
    this.elements.sfxVolume?.addEventListener('input', (e) => {
      this.audio.setSfxVolume(parseInt(e.target.value));
    });

    this.elements.musicVolume?.addEventListener('input', (e) => {
      this.audio.setMusicVolume(parseInt(e.target.value));
    });

    this.elements.muteSfx?.addEventListener('click', () => {
      const muted = this.audio.toggleSfxMute();
      this.elements.muteSfx.textContent = muted ? '🔇' : '🔊';
    });

    this.elements.muteMusic?.addEventListener('click', () => {
      const muted = this.audio.toggleMusicMute();
      this.elements.muteMusic.textContent = muted ? '🔇' : '🔊';
    });

    // Window resize
    window.addEventListener('resize', () => this._handleResize());

    // Show/hide touch controls based on device
    this._updateTouchControlsVisibility();
  }

  _updateTouchControlsVisibility() {
    const isTouch = 'ontouchstart' in window || navigator.maxTouchPoints > 0;
    if (this.elements.touchControls) {
      this.elements.touchControls.style.display = isTouch ? 'flex' : 'none';
    }
  }

  _startGame(difficulty) {
    this.game.setDifficulty(difficulty);
    this.game.start();
    this.audio.playBgm(`bgm-${difficulty}`);
  }

  _updateUI(state) {
    // Update HUD
    if (this.elements.score) {
      this.elements.score.textContent = `Score: ${state.score}`;
    }

    if (this.elements.highScore) {
      this.elements.highScore.textContent = `High: ${state.highScore}`;
    }

    if (this.elements.difficulty) {
      const labels = { easy: 'Easy', medium: 'Med', hard: 'Hard' };
      this.elements.difficulty.textContent = labels[state.difficulty];
    }

    // Power-up timer
    if (this.elements.powerUpTimer && state.snake?.remaining > 0) {
      const icons = {
        speedBoost: '⚡',
        invincible: '🛡️',
        ghost: '👻'
      };
      this.elements.powerUpTimer.textContent = `${icons[state.snake.powerUp]} ${state.snake.remaining.toFixed(1)}s`;
    } else if (this.elements.powerUpTimer) {
      this.elements.powerUpTimer.textContent = '';
    }

    // Screen management
    if (state.menu) {
      this._showScreen('menu');
    } else if (state.gameOver) {
      this._showScreen('gameOver', state);
    } else if (state.paused) {
      this._showScreen('pause');
    } else {
      this._showScreen('game');
    }

    // Render game
    if (!state.menu) {
      this.renderer.render(state);

      // Spawn death particles on game over
      if (state.gameOver && state.snake) {
        this.renderer.spawnDeathParticles(state.snake.body[0], this.renderer.getCellSize());
      }
    }
  }

  _showScreen(screenName, state = null) {
    // Hide all screens
    Object.values(this.elements).forEach(el => {
      if (el && el.classList && el.classList.contains('screen')) {
        el.style.display = 'none';
      }
    });

    // Show target screen
    switch (screenName) {
      case 'menu':
        this.elements.menuScreen.style.display = 'flex';
        break;
      case 'game':
        this.elements.gameScreen.style.display = 'block';
        break;
      case 'pause':
        this.elements.pauseScreen.style.display = 'flex';
        break;
      case 'gameOver':
        this.elements.gameOverScreen.style.display = 'flex';
        if (this.elements.finalScore) {
          this.elements.finalScore.textContent = `Score: ${state.score}`;
        }
        if (this.elements.newHighScore) {
          const isNewHigh = state.score >= state.highScore && state.score > 0;
          this.elements.newHighScore.style.display = isNewHigh ? 'block' : 'none';
          if (isNewHigh) {
            this.audio.play('highscore');
          }
        }
        break;
    }
  }

  _handleResize() {
    this.renderer.resize();
  }

  initSettings() {
    const settings = this.audio.getSettings();

    if (this.elements.sfxVolume) {
      this.elements.sfxVolume.value = settings.sfxVolume;
    }

    if (this.elements.musicVolume) {
      this.elements.musicVolume.value = settings.musicVolume;
    }

    if (this.elements.muteSfx) {
      this.elements.muteSfx.textContent = settings.sfxMuted ? '🔇' : '🔊';
    }

    if (this.elements.muteMusic) {
      this.elements.muteMusic.textContent = settings.musicMuted ? '🔇' : '🔊';
    }
  }
}

if (typeof module !== 'undefined' && module.exports) {
  module.exports = UI;
}
```

- [ ] **Step 2: Commit**

```bash
git add js/ui.js
git commit -m "feat: Add UI manager for screens and HUD"
```

---

### Task 10: Create Main Entry Point

**Files:**
- Create: `snake-game/js/main.js`

- [ ] **Step 1: Write main.js**

Write: `snake-game/js/main.js`
```javascript
/**
 * Main Entry Point - Initialize and start the Snake Game
 */

const StorageManager = require('./storage').StorageManager;
const InputHandler = require('./input').InputHandler;
const Game = require('./game').Game;
const Renderer = require('./renderer').Renderer;
const AudioManager = require('./audio').AudioManager;
const UI = require('./ui').UI;

// Feature detection
function checkBrowserSupport() {
  const canvas = document.createElement('canvas');
  if (!canvas.getContext) {
    document.body.innerHTML = '<div style="padding: 20px; color: white; background: #e74c3c;">HTML5 Canvas is not supported in your browser. Please update to a modern browser.</div>';
    return false;
  }
  return true;
}

// Initialize game
function init() {
  if (!checkBrowserSupport()) return;

  // Create core systems
  const storage = new StorageManager();
  const inputHandler = new InputHandler();
  const game = new Game(storage, inputHandler);
  const renderer = new Renderer('gameCanvas');
  const audio = new AudioManager(storage);
  const ui = new UI(game, renderer, audio);

  // Load audio files (placeholder paths)
  audio.loadSounds([
    { name: 'eat', path: 'assets/sounds/eat.mp3' },
    { name: 'eat-golden', path: 'assets/sounds/eat-golden.mp3' },
    { name: 'eat-purple', path: 'assets/sounds/eat-purple.mp3' },
    { name: 'eat-blue', path: 'assets/sounds/eat-blue.mp3' },
    { name: 'eat-ghost', path: 'assets/sounds/eat-ghost.mp3' },
    { name: 'die', path: 'assets/sounds/die.mp3' },
    { name: 'gameover', path: 'assets/sounds/gameover.mp3' },
    { name: 'highscore', path: 'assets/sounds/highscore.mp3' },
    { name: 'bgm-easy', path: 'assets/sounds/bgm-easy.mp3' },
    { name: 'bgm-medium', path: 'assets/sounds/bgm-medium.mp3' },
    { name: 'bgm-hard', path: 'assets/sounds/bgm-hard.mp3' }
  ]);

  // Initialize settings
  ui.initSettings();

  // Initial render
  renderer.resize();
  game.notifyObservers();

  // Expose for debugging
  window.snakeGame = { game, renderer, audio, ui };

  console.log('🐍 Snake Game initialized');
}

// Start when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}
```

- [ ] **Step 2: Commit**

```bash
git add js/main.js
git commit -m "feat: Add main entry point with initialization"
```

---

## HTML and CSS

### Task 11: Create Main HTML File

**Files:**
- Create: `snake-game/index.html`

- [ ] **Step 1: Write index.html**

Write: `snake-game/index.html`
```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no">
  <title>Snake Game</title>
  <link rel="stylesheet" href="css/style.css">
</head>
<body>
  <div class="game-container">
    <!-- Header / HUD -->
    <header class="hud">
      <h1>🐍 SNAKE GAME</h1>
      <div class="stats">
        <span id="score">Score: 0</span>
        <span id="highScore">High: 0</span>
        <span id="difficulty">Med</span>
      </div>
      <div id="powerUpTimer" class="powerup-timer"></div>
    </header>

    <!-- Menu Screen -->
    <div id="menuScreen" class="screen menu-screen">
      <h2>Select Difficulty</h2>
      <div class="difficulty-buttons">
        <button id="btnEasy" class="btn btn-easy">Easy (Wrap)</button>
        <button id="btnMedium" class="btn btn-medium">Medium</button>
        <button id="btnHard" class="btn btn-hard">Hard</button>
      </div>
      <div class="settings">
        <h3>Settings</h3>
        <div class="setting-row">
          <label>SFX Volume: <span id="sfxVolumeValue">80</span>%</label>
          <input type="range" id="sfxVolume" min="0" max="100" value="80">
          <button id="muteSfx" class="icon-btn">🔊</button>
        </div>
        <div class="setting-row">
          <label>Music Volume: <span id="musicVolumeValue">60</span>%</label>
          <input type="range" id="musicVolume" min="0" max="100" value="60">
          <button id="muteMusic" class="icon-btn">🔊</button>
        </div>
      </div>
      <div class="controls-info">
        <p><strong>Controls:</strong> Arrow keys / WASD to move</p>
        <p>Space/P to pause • M to mute • ESC for menu</p>
      </div>
    </div>

    <!-- Game Screen -->
    <div id="gameScreen" class="screen" style="display: none;">
      <canvas id="gameCanvas"></canvas>
    </div>

    <!-- Pause Screen -->
    <div id="pauseScreen" class="screen overlay-screen" style="display: none;">
      <h2>PAUSED</h2>
      <button id="btnResume" class="btn">Resume</button>
      <button id="btnMenu" class="btn">Main Menu</button>
    </div>

    <!-- Game Over Screen -->
    <div id="gameOverScreen" class="screen overlay-screen" style="display: none;">
      <h2>GAME OVER</h2>
      <p id="finalScore">Score: 0</p>
      <p id="newHighScore" class="new-high" style="display: none;">🎉 NEW HIGH SCORE! 🎉</p>
      <button id="btnPlayAgain" class="btn">Play Again</button>
      <button id="btnMenu" class="btn">Main Menu</button>
    </div>

    <!-- Touch Controls (Mobile) -->
    <div class="touch-controls">
      <div class="touch-row">
        <button class="touch-btn" data-direction="up">▲</button>
      </div>
      <div class="touch-row">
        <button class="touch-btn" data-direction="left">◀</button>
        <button class="touch-btn" data-direction="down">▼</button>
        <button class="touch-btn" data-direction="right">▶</button>
      </div>
    </div>
  </div>

  <!-- Swipe Indicator -->
  <style>
    .swipe-indicator {
      position: fixed;
      top: 50%;
      left: 50%;
      transform: translate(-50%, -50%);
      font-size: 80px;
      color: rgba(78, 204, 163, 0.5);
      pointer-events: none;
      animation: fadeOut 0.2s ease-out forwards;
    }
    @keyframes fadeOut {
      from { opacity: 1; transform: translate(-50%, -50%) scale(1); }
      to { opacity: 0; transform: translate(-50%, -50%) scale(1.5); }
    }
  </style>

  <script type="module" src="js/main.js"></script>
</body>
</html>
```

- [ ] **Step 2: Commit**

```bash
git add index.html
git commit -m "feat: Add main HTML structure"
```

---

### Task 12: Create Stylesheet

**Files:**
- Create: `snake-game/css/style.css`

- [ ] **Step 1: Write style.css**

Write: `snake-game/css/style.css`
```css
/**
 * Snake Game Styles
 * Modern, minimalist design with responsive layout
 */

:root {
  --bg-primary: #1a1a2e;
  --bg-secondary: #0f3460;
  --accent: #e94560;
  --snake-head: #4ecca3;
  --snake-body: #45b393;
  --text-primary: #ffffff;
  --text-secondary: #a0a0a0;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: var(--bg-primary);
  color: var(--text-primary);
  overflow: hidden;
  height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
}

.game-container {
  width: 100%;
  max-width: 600px;
  height: 100%;
  display: flex;
  flex-direction: column;
  position: relative;
}

/* HUD */
.hud {
  background: var(--bg-secondary);
  padding: 15px 20px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  border-bottom: 2px solid var(--accent);
}

.hud h1 {
  font-size: 1.2rem;
  margin-right: auto;
}

.stats {
  display: flex;
  gap: 15px;
  font-size: 0.9rem;
}

.powerup-timer {
  position: absolute;
  top: 60px;
  right: 20px;
  font-size: 1rem;
  font-weight: bold;
  animation: pulse 1s infinite;
}

@keyframes pulse {
  0%, 100% { transform: scale(1); }
  50% { transform: scale(1.05); }
}

/* Screens */
.screen {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  padding: 20px;
}

.menu-screen {
  background: var(--bg-primary);
}

.menu-screen h2 {
  margin-bottom: 20px;
}

.difficulty-buttons {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 100%;
  max-width: 250px;
}

.settings {
  margin-top: 30px;
  width: 100%;
  max-width: 300px;
}

.settings h3 {
  margin-bottom: 15px;
  font-size: 1rem;
}

.setting-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.setting-row label {
  flex: 1;
  font-size: 0.9rem;
}

.setting-row input[type="range"] {
  width: 80px;
}

.icon-btn {
  background: none;
  border: none;
  font-size: 1.2rem;
  cursor: pointer;
  padding: 5px;
}

.controls-info {
  margin-top: 30px;
  text-align: center;
  font-size: 0.85rem;
  color: var(--text-secondary);
  line-height: 1.6;
}

/* Game Screen */
#gameScreen {
  display: flex;
  justify-content: center;
  align-items: center;
  background: var(--bg-primary);
}

#gameCanvas {
  background: var(--bg-primary);
  max-width: 100%;
  max-height: calc(100vh - 200px);
  box-shadow: 0 0 20px rgba(78, 204, 163, 0.2);
}

/* Overlay Screens */
.overlay-screen {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(26, 26, 46, 0.95);
  z-index: 10;
}

.overlay-screen h2 {
  font-size: 2rem;
  margin-bottom: 20px;
}

.new-high {
  color: var(--accent);
  font-weight: bold;
  margin: 10px 0;
}

/* Buttons */
.btn {
  background: var(--snake-head);
  color: var(--bg-primary);
  border: none;
  padding: 12px 24px;
  font-size: 1rem;
  font-weight: bold;
  cursor: pointer;
  border-radius: 8px;
  transition: all 0.2s ease;
  width: 100%;
  max-width: 200px;
}

.btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(78, 204, 163, 0.4);
}

.btn:active {
  transform: translateY(0);
}

.btn-easy {
  background: #27ae60;
}

.btn-medium {
  background: var(--snake-head);
}

.btn-hard {
  background: var(--accent);
}

/* Touch Controls */
.touch-controls {
  display: none;
  background: var(--bg-secondary);
  padding: 20px;
  justify-content: center;
  align-items: center;
  gap: 15px;
}

.touch-row {
  display: flex;
  justify-content: center;
  gap: 15px;
}

.touch-btn {
  width: 60px;
  height: 60px;
  background: var(--snake-head);
  border: none;
  border-radius: 12px;
  font-size: 1.5rem;
  color: var(--bg-primary);
  cursor: pointer;
  transition: all 0.1s ease;
}

.touch-btn:active {
  background: var(--accent);
  transform: scale(0.95);
}

/* Responsive */
@media (max-width: 600px) {
  .hud {
    padding: 10px 15px;
    flex-direction: column;
    align-items: flex-start;
  }

  .hud h1 {
    font-size: 1rem;
  }

  .stats {
    font-size: 0.8rem;
    gap: 10px;
  }

  .powerup-timer {
    top: 50px;
    right: 15px;
    font-size: 0.85rem;
  }

  .touch-controls {
    display: flex;
  }

  #gameCanvas {
    max-height: calc(100vh - 280px);
  }
}

@media (hover: none) and (pointer: coarse) {
  .touch-controls {
    display: flex;
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add css/style.css
git commit -m "feat: Add responsive CSS styles"
```

---

## Finalization

### Task 13: Bundle Preparation (For Browser)

**Files:**
- Create: `snake-game/js/bundle.js`

- [ ] **Step 1: Create bundle configuration**

Since we're using CommonJS modules, we need to create a browser-compatible bundle. For this standalone project, we'll use a simple inline bundling approach.

Write: `snake-game/js/bundle.js`
```javascript
/**
 * Bundled Snake Game
 * All modules combined for browser compatibility
 */

// Storage Manager
class StorageManager {
  constructor() {
    this.available = this._checkAvailability();
    this.data = this._loadData();
  }

  _checkAvailability() {
    try {
      const test = '__storage_test__';
      localStorage.setItem(test, test);
      localStorage.removeItem(test);
      return true;
    } catch (e) {
      return false;
    }
  }

  _loadData() {
    if (!this.available) return this._getDefaultData();

    try {
      const raw = localStorage.getItem('snake_game_data');
      if (!raw) return this._getDefaultData();

      const data = JSON.parse(raw);
      return this._validateData(data);
    } catch (e) {
      return this._getDefaultData();
    }
  }

  _getDefaultData() {
    return {
      version: 1,
      highScores: { easy: 0, medium: 0, hard: 0 },
      settings: {
        sfxVolume: 80,
        musicVolume: 60,
        sfxMuted: false,
        musicMuted: false,
        selectedBgm: 'medium'
      }
    };
  }

  _validateData(data) {
    const validated = this._getDefaultData();

    if (data.highScores) {
      validated.highScores.easy = Math.max(0, Math.min(999999, data.highScores.easy || 0));
      validated.highScores.medium = Math.max(0, Math.min(999999, data.highScores.medium || 0));
      validated.highScores.hard = Math.max(0, Math.min(999999, data.highScores.hard || 0));
    }

    if (data.settings) {
      validated.settings.sfxVolume = Math.max(0, Math.min(100, data.settings.sfxVolume ?? 80));
      validated.settings.musicVolume = Math.max(0, Math.min(100, data.settings.musicVolume ?? 60));
      validated.settings.sfxMuted = Boolean(data.settings.sfxMuted);
      validated.settings.musicMuted = Boolean(data.settings.musicMuted);
    }

    return validated;
  }

  save() {
    if (!this.available) return false;
    try {
      localStorage.setItem('snake_game_data', JSON.stringify(this.data));
      return true;
    } catch (e) {
      return false;
    }
  }

  getHighScore(difficulty) {
    return this.data.highScores[difficulty] || 0;
  }

  setHighScore(difficulty, score) {
    if (score > this.data.highScores[difficulty]) {
      this.data.highScores[difficulty] = score;
      return this.save();
    }
    return false;
  }

  getSettings() {
    return { ...this.data.settings };
  }

  updateSettings(newSettings) {
    this.data.settings = { ...this.data.settings, ...newSettings };
    return this.save();
  }
}

// Input Handler
class InputHandler {
  constructor() {
    this.buffer = [];
    this.bufferSize = 2;
    this.bufferExpireMs = 500;
    this.currentDirection = 'right';
    this.nextDirection = 'right';
    this.swipe = { startX: 0, startY: 0, startTime: 0, thresholdVelocity: 300, maxDuration: 500 };
    this.observers = [];
    this._initListeners();
  }

  addObserver(callback) {
    this.observers.push(callback);
  }

  notifyObservers(input) {
    this.observers.forEach(cb => cb(input));
  }

  _initListeners() {
    document.addEventListener('keydown', (e) => this._handleKeyDown(e));
    document.addEventListener('touchstart', (e) => this._handleTouchStart(e), { passive: false });
    document.addEventListener('touchend', (e) => this._handleTouchEnd(e), { passive: false });
    document.addEventListener('touchstart', (e) => this._handleTouchButton(e), { passive: false });
    document.addEventListener('click', (e) => this._handleTouchButton(e));
  }

  _handleKeyDown(e) {
    const keyMap = {
      'ArrowUp': 'up', 'KeyW': 'up',
      'ArrowDown': 'down', 'KeyS': 'down',
      'ArrowLeft': 'left', 'KeyA': 'left',
      'ArrowRight': 'right', 'KeyD': 'right',
      'Space': 'pause', 'KeyP': 'pause',
      'KeyM': 'mute',
      'Escape': 'menu',
      'Digit1': 'difficulty-easy',
      'Digit2': 'difficulty-medium',
      'Digit3': 'difficulty-hard'
    };

    const action = keyMap[e.code];
    if (!action) return;

    e.preventDefault();

    if (action.startsWith('difficulty-')) {
      this.notifyObservers({ type: 'difficulty', value: action.split('-')[1] });
      return;
    }

    if (['pause', 'mute', 'menu'].includes(action)) {
      this.notifyObservers({ type: action, value: true });
      return;
    }

    this._queueDirection(action);
  }

  _queueDirection(direction) {
    const opposites = { up: 'down', down: 'up', left: 'right', right: 'left' };
    if (opposites[direction] === this.nextDirection) return;

    const now = Date.now();
    this.buffer = this.buffer.filter(item => now - item.time < this.bufferExpireMs);
    this.buffer.push({ direction, time: now });
    if (this.buffer.length > this.bufferSize) {
      this.buffer.shift();
    }
  }

  processNextInput() {
    if (this.buffer.length > 0) {
      const input = this.buffer.shift();
      this.currentDirection = input.direction;
      this.nextDirection = input.direction;
      return this.currentDirection;
    }
    return this.nextDirection;
  }

  _handleTouchStart(e) {
    if (e.target.closest('.touch-controls')) return;
    const touch = e.touches[0];
    this.swipe.startX = touch.clientX;
    this.swipe.startY = touch.clientY;
    this.swipe.startTime = Date.now();
  }

  _handleTouchEnd(e) {
    if (e.target.closest('.touch-controls')) return;

    const touch = e.changedTouches[0];
    const deltaX = touch.clientX - this.swipe.startX;
    const deltaY = touch.clientY - this.swipe.startY;
    const deltaTime = Date.now() - this.swipe.startTime;

    if (deltaTime > this.swipe.maxDuration) return;

    const velocity = Math.sqrt(deltaX * deltaX + deltaY * deltaY) / (deltaTime / 1000);

    if (velocity < this.swipe.thresholdVelocity) return;

    let direction;
    if (Math.abs(deltaX) > Math.abs(deltaY)) {
      direction = deltaX > 0 ? 'right' : 'left';
    } else {
      direction = deltaY > 0 ? 'down' : 'up';
    }

    this._queueDirection(direction);
    this._showSwipeIndicator(direction);
  }

  _handleTouchButton(e) {
    const button = e.target.closest('.touch-btn');
    if (!button) return;

    e.preventDefault();
    const direction = button.dataset.direction;
    if (direction) {
      this._queueDirection(direction);
    }
  }

  _showSwipeIndicator(direction) {
    const indicator = document.createElement('div');
    indicator.className = 'swipe-indicator';

    const arrows = { up: '↑', down: '↓', left: '←', right: '→' };
    indicator.textContent = arrows[direction];

    document.body.appendChild(indicator);
    setTimeout(() => indicator.remove(), 200);
  }

  reset() {
    this.buffer = [];
    this.currentDirection = 'right';
    this.nextDirection = 'right';
  }
}

// Snake Entity
class Snake {
  constructor(startX, startY) {
    this.body = [
      { x: startX, y: startY },
      { x: startX - 1, y: startY },
      { x: startX - 2, y: startY }
    ];
    this.direction = 'right';
    this.growPending = 0;

    this.powerUps = {
      speedBoost: { active: false, endTime: 0 },
      invincible: { active: false, endTime: 0 },
      ghost: { active: false, endTime: 0 }
    };
  }

  getHead() {
    return this.body[0];
  }

  setDirection(direction) {
    this.direction = direction;
  }

  move(wrapAround = false, gridWidth = 20, gridHeight = 20) {
    const head = this.getHead();
    let newHead = { ...head };

    switch (this.direction) {
      case 'up': newHead.y--; break;
      case 'down': newHead.y++; break;
      case 'left': newHead.x--; break;
      case 'right': newHead.x++; break;
    }

    if (wrapAround || this.powerUps.ghost.active) {
      if (newHead.x < 0) newHead.x = gridWidth - 1;
      if (newHead.x >= gridWidth) newHead.x = 0;
      if (newHead.y < 0) newHead.y = gridHeight - 1;
      if (newHead.y >= gridHeight) newHead.y = 0;
    }

    this.body.unshift(newHead);

    if (this.growPending > 0) {
      this.growPending--;
    } else {
      this.body.pop();
    }
  }

  grow(amount = 1) {
    this.growPending += amount;
  }

  shrink() {
    if (this.body.length > 3) {
      this.body.pop();
      return true;
    }
    return false;
  }

  checkSelfCollision() {
    if (this.powerUps.ghost.active || this.powerUps.invincible.active) {
      return false;
    }

    const head = this.getHead();
    for (let i = 1; i < this.body.length; i++) {
      if (head.x === this.body[i].x && head.y === this.body[i].y) {
        return true;
      }
    }
    return false;
  }

  checkWallCollision(gridWidth = 20, gridHeight = 20) {
    if (this.powerUps.ghost.active || this.powerUps.invincible.active) {
      return false;
    }

    const head = this.getHead();
    return head.x < 0 || head.x >= gridWidth || head.y < 0 || head.y >= gridHeight;
  }

  activatePowerUp(type) {
    const now = Date.now();
    const endTime = now + 5000;

    Object.keys(this.powerUps).forEach(key => {
      this.powerUps[key].active = false;
    });

    if (this.powerUps[type]) {
      this.powerUps[type].active = true;
      this.powerUps[type].endTime = endTime;
    }

    return endTime;
  }

  updatePowerUps() {
    const now = Date.now();
    let activeType = null;
    let remaining = 0;

    for (const [type, state] of Object.entries(this.powerUps)) {
      if (state.active) {
        if (now >= state.endTime) {
          state.active = false;
        } else {
          activeType = type;
          remaining = Math.max(0, (state.endTime - now) / 1000);
        }
      }
    }

    return { activeType, remaining };
  }

  canPickUp(type) {
    if (this.powerUps[type] && this.powerUps[type].active) {
      return false;
    }
    return true;
  }

  getRenderState() {
    const powerUpState = this.updatePowerUps();
    return {
      body: this.body,
      powerUp: powerUpState.activeType,
      remaining: powerUpState.remaining
    };
  }
}

// Food System
class FoodSystem {
  constructor() {
    this.foodItems = [];
    this.lastSpawnTime = 0;
  }

  spawn(snakeBody, gridWidth = 20, gridHeight = 20) {
    this._removeExpired();

    if (this.foodItems.length >= 3) return null;

    const now = Date.now();
    if (now - this.lastSpawnTime < 3000) return null;

    const location = this._findSpawnLocation(snakeBody, gridWidth, gridHeight);
    if (!location) return null;

    const type = this._selectFoodType();
    const foodData = this._getFoodData(type);

    const food = {
      id: Date.now() + Math.random(),
      x: location.x,
      y: location.y,
      type: type,
      ...foodData,
      spawnTime: now,
      expireTime: foodData.duration ? now + foodData.duration : null
    };

    this.foodItems.push(food);
    this.lastSpawnTime = now;

    return food;
  }

  _getFoodData(type) {
    const foods = {
      regular: { points: 10, color: '#e74c3c', duration: null },
      golden: { points: 50, color: '#ffd700', duration: 8000 },
      purple: { points: 30, color: '#9b59b6', duration: 8000 },
      blue: { points: 20, color: '#3498db', duration: 10000 },
      ghost: { points: 100, color: '#ffffff', duration: 6000 }
    };
    return foods[type];
  }

  _findSpawnLocation(snakeBody, gridWidth, gridHeight) {
    const safeZone = 2;
    const head = snakeBody[0];

    const occupied = new Set();
    snakeBody.forEach(seg => occupied.add(`${seg.x},${seg.y}`));
    this.foodItems.forEach(f => occupied.add(`${f.x},${f.y}`));

    for (let i = 0; i < 100; i++) {
      const x = Math.floor(Math.random() * gridWidth);
      const y = Math.floor(Math.random() * gridHeight);

      const key = `${x},${y}`;
      if (occupied.has(key)) continue;

      const distToHead = Math.abs(x - head.x) + Math.abs(y - head.y);
      if (distToHead < safeZone) continue;

      return { x, y };
    }

    return null;
  }

  _selectFoodType() {
    const probabilities = [
      { type: 'regular', weight: 70 },
      { type: 'golden', weight: 10 },
      { type: 'purple', weight: 10 },
      { type: 'blue', weight: 5 },
      { type: 'ghost', weight: 5 }
    ];

    const totalWeight = probabilities.reduce((sum, item) => sum + item.weight, 0);
    let random = Math.random() * totalWeight;

    for (const item of probabilities) {
      random -= item.weight;
      if (random <= 0) return item.type;
    }

    return 'regular';
  }

  _removeExpired() {
    const now = Date.now();
    this.foodItems = this.foodItems.filter(food => {
      if (food.expireTime && now >= food.expireTime) {
        return false;
      }
      return true;
    });
  }

  checkCollision(headX, headY) {
    const index = this.foodItems.findIndex(f => f.x === headX && f.y === headY);
    if (index !== -1) {
      const food = this.foodItems.splice(index, 1)[0];
      return food;
    }
    return null;
  }

  getRenderState() {
    const now = Date.now();
    return this.foodItems.map(food => ({
      x: food.x,
      y: food.y,
      type: food.type,
      color: food.color,
      remainingLifetime: food.expireTime ? Math.max(0, food.expireTime - now) : null
    }));
  }

  reset() {
    this.foodItems = [];
    this.lastSpawnTime = 0;
  }
}

// Game Engine
class Game {
  constructor(storage, inputHandler) {
    this.storage = storage;
    this.inputHandler = inputHandler;

    this.gridSize = 20;
    this.difficulty = 'medium';

    this.snake = null;
    this.foodSystem = new FoodSystem();

    this.state = {
      score: 0,
      combo: null,
      lastEatTime: 0,
      paused: false,
      gameOver: false,
      menu: true
    };

    this.observers = [];
    this._lastTick = 0;
    this._animationFrame = null;
    this._lastFoodSpawn = 0;

    this.inputHandler.addObserver((input) => this._handleInput(input));
  }

  addObserver(callback) {
    this.observers.push(callback);
  }

  notifyObservers() {
    const state = this.getState();
    this.observers.forEach(cb => cb(state));
  }

  _handleInput(input) {
    if (input.type === 'difficulty') {
      if (this.state.menu) {
        this.setDifficulty(input.value);
      }
    } else if (input.type === 'pause') {
      if (!this.state.menu && !this.state.gameOver) {
        this.state.paused = !this.state.paused;
        this.notifyObservers();
      }
    } else if (input.type === 'menu') {
      if (this.state.gameOver) {
        this.showMenu();
      }
    } else if (input.type === 'mute') {
      // Handled by UI
    }
  }

  setDifficulty(difficulty) {
    const difficulties = ['easy', 'medium', 'hard'];
    if (!difficulties.includes(difficulty)) return;
    this.difficulty = difficulty;
    this.notifyObservers();
  }

  start() {
    this.snake = new Snake(10, 10);
    this.foodSystem.reset();
    this.inputHandler.reset();

    this.state = {
      score: 0,
      combo: null,
      lastEatTime: 0,
      paused: false,
      gameOver: false,
      menu: false
    };

    this._lastTick = 0;
    this._lastFoodSpawn = 0;

    // Spawn initial food
    this.foodSystem.spawn(this.snake.body, this.gridSize, this.gridSize);

    this._gameLoop();
    this.notifyObservers();
  }

  showMenu() {
    this.state.menu = true;
    this.state.gameOver = false;
    this.state.paused = false;
    this.notifyObservers();
  }

  _gameLoop(timestamp = 0) {
    if (this.state.menu || this.state.gameOver) return;

    this._animationFrame = requestAnimationFrame((t) => this._gameLoop(t));

    if (this.state.paused) return;

    const settings = this._getDifficultySettings();
    const tickDelta = timestamp - this._lastTick;

    // Process input
    const direction = this.inputHandler.processNextInput();
    if (direction) {
      this.snake.setDirection(direction);
    }

    // Game logic tick
    if (tickDelta >= settings.tickRate) {
      this._update();
      this._lastTick = timestamp;
    }

    // Continuous updates
    this._updateContinuous(timestamp);
  }

  _getDifficultySettings() {
    const settings = {
      easy: { tickRate: 150, wallWrap: true, foodRate: 3000 },
      medium: { tickRate: 100, wallWrap: false, foodRate: 3000 },
      hard: { tickRate: 60, wallWrap: false, foodRate: 2000 }
    };
    return settings[this.difficulty];
  }

  _update() {
    const settings = this._getDifficultySettings();

    // Move snake
    this.snake.move(settings.wallWrap, this.gridSize, this.gridSize);

    // Check collisions
    if (!settings.wallWrap && this.snake.checkWallCollision(this.gridSize, this.gridSize)) {
      this._gameOver();
      return;
    }

    if (this.snake.checkSelfCollision()) {
      this._gameOver();
      return;
    }

    // Food collision
    const head = this.snake.getHead();
    const food = this.foodSystem.checkCollision(head.x, head.y);
    if (food) {
      this._eatFood(food);
    }

    this.notifyObservers();
  }

  _updateContinuous(timestamp) {
    this.snake.updatePowerUps();
    this.foodSystem._removeExpired();

    // Spawn food periodically
    const settings = this._getDifficultySettings();
    if (timestamp - this._lastFoodSpawn > settings.foodRate) {
      this.foodSystem.spawn(this.snake.body, this.gridSize, this.gridSize);
      this._lastFoodSpawn = timestamp;
    }

    // Update UI for smooth timers
    if (this.snake.powerUps) {
      const hasActivePowerUp = Object.values(this.snake.powerUps).some(p => p.active);
      if (hasActivePowerUp) {
        this.notifyObservers();
      }
    }
  }

  _eatFood(food) {
    const now = Date.now();

    switch (food.type) {
      case 'regular':
        this.snake.grow(1);
        break;
      case 'golden':
        if (this.snake.canPickUp('speedBoost')) {
          this.snake.activatePowerUp('speedBoost');
        }
        this.snake.grow(1);
        break;
      case 'purple':
        if (this.snake.canPickUp('invincible')) {
          this.snake.activatePowerUp('invincible');
        }
        this.snake.grow(1);
        break;
      case 'blue':
        this.snake.shrink();
        break;
      case 'ghost':
        if (this.snake.canPickUp('ghost')) {
          this.snake.activatePowerUp('ghost');
        }
        this.snake.grow(1);
        break;
    }

    // Calculate score
    let score = food.points;

    if (now - this.state.lastEatTime < 2000) {
      score = Math.floor(score * 1.5);
    }
    this.state.lastEatTime = now;

    // Combo
    if (this.state.combo && this.state.combo.type === food.type && now - this.state.combo.time < 5000) {
      this.state.combo.count++;
      this.state.combo.time = now;
      score += Math.min(this.state.combo.count, 5) * 10;
    } else {
      this.state.combo = { type: food.type, count: 1, time: now };
    }

    this.state.score = Math.min(999999, this.state.score + score);
    this.notifyObservers();
  }

  _gameOver() {
    this.state.gameOver = true;
    this.state.paused = false;

    const isNewHigh = this.storage.setHighScore(this.difficulty, this.state.score);

    this.notifyObservers();
  }

  getState() {
    return {
      snake: this.snake ? this.snake.getRenderState() : null,
      food: this.foodSystem.getRenderState(),
      score: this.state.score,
      combo: this.state.combo,
      difficulty: this.difficulty,
      highScore: this.storage.getHighScore(this.difficulty),
      paused: this.state.paused,
      gameOver: this.state.gameOver,
      menu: this.state.menu
    };
  }

  destroy() {
    if (this._animationFrame) {
      cancelAnimationFrame(this._animationFrame);
    }
  }
}

// Renderer
class Renderer {
  constructor(canvasId) {
    this.canvas = document.getElementById(canvasId);
    if (!this.canvas) {
      throw new Error(`Canvas "${canvasId}" not found`);
    }

    this.ctx = this.canvas.getContext('2d');
    this.baseCellSize = 25;
    this.gridSize = 20;
    this.canvasSize = 500;

    this.colors = {
      background: '#1a1a2e',
      gridLines: 'rgba(22, 33, 62, 0.3)',
      snakeHead: '#4ecca3',
      snakeBody: '#45b393',
      snakeBodyGradient: '#38ada9',
      powerUpSpeed: '#ffd700',
      powerUpInvincible: '#9b59b6',
      powerUpGhost: 'rgba(255, 255, 255, 0.7)'
    };

    this.particles = [];
    this._setupCanvas();
  }

  _setupCanvas() {
    this.canvas.width = this.canvasSize;
    this.canvas.height = this.canvasSize;
  }

  getCellSize() {
    const rect = this.canvas.getBoundingClientRect();
    return rect.width / this.gridSize;
  }

  resize() {
    const container = this.canvas.parentElement;
    const maxSize = Math.min(container.clientWidth - 40, 500);
    this.canvas.style.width = maxSize + 'px';
    this.canvas.style.height = maxSize + 'px';
  }

  render(gameState) {
    const cellSize = this.getCellSize();

    this.ctx.fillStyle = this.colors.background;
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

    this._drawGrid(cellSize);
    this._drawFood(gameState.food, cellSize);
    this._drawSnake(gameState.snake, cellSize);
    this._drawParticles(cellSize);

    if (gameState.snake?.powerUp && gameState.snake.remaining > 0) {
      this._drawPowerUpGlow(gameState.snake.powerUp, gameState.snake.remaining, cellSize);
    }
  }

  _drawGrid(cellSize) {
    this.ctx.strokeStyle = this.colors.gridLines;
    this.ctx.lineWidth = 1;

    for (let i = 0; i <= this.gridSize; i++) {
      this.ctx.beginPath();
      this.ctx.moveTo(i * cellSize, 0);
      this.ctx.lineTo(i * cellSize, this.canvas.height);
      this.ctx.stroke();

      this.ctx.beginPath();
      this.ctx.moveTo(0, i * cellSize);
      this.ctx.lineTo(this.canvas.width, i * cellSize);
      this.ctx.stroke();
    }
  }

  _drawSnake(snakeState, cellSize) {
    if (!snakeState) return;

    snakeState.body.forEach((segment, index) => {
      const x = segment.x * cellSize;
      const y = segment.y * cellSize;

      let color;
      if (snakeState.powerUp === 'speedBoost') {
        color = this.colors.powerUpSpeed;
      } else if (snakeState.powerUp === 'invincible') {
        color = this.colors.powerUpInvincible;
      } else if (snakeState.powerUp === 'ghost') {
        color = this.colors.powerUpGhost;
      } else if (index === 0) {
        color = this.colors.snakeHead;
      } else {
        const ratio = index / snakeState.body.length;
        color = ratio > 0.5 ? this.colors.snakeBody : this.colors.snakeBodyGradient;
      }

      this.ctx.fillStyle = color;
      this._drawRoundedRect(x + 1, y + 1, cellSize - 2, cellSize - 2, 4);

      if (snakeState.powerUp === 'invincible' && index === 0) {
        this.ctx.fillStyle = '#fff';
        this.ctx.font = `${cellSize * 0.6}px Arial`;
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';
        this.ctx.fillText('🛡️', x + cellSize / 2, y + cellSize / 2);
      }
    });
  }

  _drawFood(foodItems, cellSize) {
    foodItems.forEach(food => {
      const x = food.x * cellSize + cellSize / 2;
      const y = food.y * cellSize + cellSize / 2;
      const radius = cellSize * 0.4;

      this.ctx.fillStyle = food.color;
      this.ctx.beginPath();
      this.ctx.arc(x, y, radius, 0, Math.PI * 2);
      this.ctx.fill();

      if (food.type !== 'regular') {
        this.ctx.shadowColor = food.color;
        this.ctx.shadowBlur = 10;
        this.ctx.beginPath();
        this.ctx.arc(x, y, radius, 0, Math.PI * 2);
        this.ctx.fill();
        this.ctx.shadowBlur = 0;
      }

      if (food.remainingLifetime !== null) {
        const alpha = food.remainingLifetime / 10000;
        this.ctx.fillStyle = `rgba(0, 0, 0, ${1 - alpha})`;
        this.ctx.beginPath();
        this.ctx.arc(x, y, radius, 0, Math.PI * 2);
        this.ctx.fill();
      }
    });
  }

  _drawPowerUpGlow(powerUpType, remaining, cellSize) {
    const colors = {
      speedBoost: this.colors.powerUpSpeed,
      invincible: this.colors.powerUpInvincible,
      ghost: this.colors.powerUpGhost
    };

    const color = colors[powerUpType];
    const pulse = Math.sin(Date.now() / 200) * 0.2 + 0.8;

    this.ctx.strokeStyle = color;
    this.ctx.globalAlpha = pulse * 0.5;
    this.ctx.lineWidth = 4;
    this.ctx.strokeRect(2, 2, this.canvas.width - 4, this.canvas.height - 4);
    this.ctx.globalAlpha = 1;
  }

  _drawRoundedRect(x, y, width, height, radius) {
    this.ctx.beginPath();
    this.ctx.moveTo(x + radius, y);
    this.ctx.lineTo(x + width - radius, y);
    this.ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
    this.ctx.lineTo(x + width, y + height - radius);
    this.ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
    this.ctx.lineTo(x + radius, y + height);
    this.ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
    this.ctx.lineTo(x, y + radius);
    this.ctx.quadraticCurveTo(x, y, x + radius, y);
    this.ctx.closePath();
    this.ctx.fill();
  }

  spawnDeathParticles(headPos, cellSize) {
    for (let i = 0; i < 20; i++) {
      this.particles.push({
        x: headPos.x * cellSize + cellSize / 2,
        y: headPos.y * cellSize + cellSize / 2,
        vx: (Math.random() - 0.5) * 200,
        vy: (Math.random() - 0.5) * 200,
        life: 1,
        color: this.colors.snakeHead
      });
    }
  }

  _drawParticles(cellSize) {
    const dt = 1 / 60;

    this.particles = this.particles.filter(p => {
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      p.life -= dt;

      if (p.life > 0) {
        this.ctx.fillStyle = p.color;
        this.ctx.globalAlpha = p.life;
        this.ctx.fillRect(p.x - 2, p.y - 2, 4, 4);
        this.ctx.globalAlpha = 1;
        return true;
      }
      return false;
    });
  }
}

// Audio Manager
class AudioManager {
  constructor(storage) {
    this.storage = storage;
    this.sounds = {};
    this.bgm = null;
    this.settings = storage.getSettings();
    this.currentBgm = null;

    this.sfxVolume = this.settings.sfxVolume / 100;
    this.musicVolume = this.settings.musicVolume / 100;
    this.sfxMuted = this.settings.sfxMuted;
    this.musicMuted = this.settings.musicMuted;
  }

  loadSounds(soundFiles) {
    soundFiles.forEach(file => {
      const audio = new Audio(file.path);
      audio.preload = 'auto';
      this.sounds[file.name] = audio;
    });
  }

  play(soundName) {
    if (this.sfxMuted) return;

    const sound = this.sounds[soundName];
    if (sound) {
      sound.volume = this.sfxVolume;
      sound.currentTime = 0;
      sound.play().catch(e => console.warn('Sound play failed:', e));
    }
  }

  playBgm(bgmName) {
    if (this.musicMuted) return;

    if (this.currentBgm === bgmName) return;

    if (this.bgm) {
      this.bgm.pause();
      this.bgm.currentTime = 0;
    }

    const newBgm = this.sounds[bgmName];
    if (newBgm) {
      this.bgm = newBgm;
      this.bgm.volume = this.musicVolume;
      this.bgm.loop = true;
      this.bgm.play().catch(e => console.warn('BGM play failed:', e));
      this.currentBgm = bgmName;
    }
  }

  stopBgm() {
    if (this.bgm) {
      this.bgm.pause();
      this.bgm.currentTime = 0;
      this.bgm = null;
      this.currentBgm = null;
    }
  }

  setSfxVolume(volume) {
    volume = Math.max(0, Math.min(100, volume));
    this.sfxVolume = volume / 100;
    this.storage.updateSettings({ sfxVolume: volume });
  }

  setMusicVolume(volume) {
    volume = Math.max(0, Math.min(100, volume));
    this.musicVolume = volume / 100;
    if (this.bgm) {
      this.bgm.volume = this.musicMuted ? 0 : this.musicVolume;
    }
    this.storage.updateSettings({ musicVolume: volume });
  }

  toggleSfxMute() {
    this.sfxMuted = !this.sfxMuted;
    this.storage.updateSettings({ sfxMuted: this.sfxMuted });
    return this.sfxMuted;
  }

  toggleMusicMute() {
    this.musicMuted = !this.musicMuted;
    if (this.bgm) {
      this.bgm.volume = this.musicMuted ? 0 : this.musicVolume;
    }
    if (this.musicMuted && this.bgm) {
      this.bgm.pause();
    } else if (!this.musicMuted && this.bgm && this.currentBgm) {
      this.bgm.play();
    }
    this.storage.updateSettings({ musicMuted: this.musicMuted });
    return this.musicMuted;
  }

  getSettings() {
    return {
      sfxVolume: this.sfxVolume * 100,
      musicVolume: this.musicVolume * 100,
      sfxMuted: this.sfxMuted,
      musicMuted: this.musicMuted
    };
  }
}

// UI Manager
class UI {
  constructor(game, renderer, audio) {
    this.game = game;
    this.renderer = renderer;
    this.audio = audio;

    this.elements = {};
    this._initElements();
    this._bindEvents();

    this.game.addObserver((state) => this._updateUI(state));
  }

  _initElements() {
    this.elements = {
      score: document.getElementById('score'),
      highScore: document.getElementById('highScore'),
      difficulty: document.getElementById('difficulty'),
      powerUpTimer: document.getElementById('powerUpTimer'),
      menuScreen: document.getElementById('menuScreen'),
      gameScreen: document.getElementById('gameScreen'),
      gameOverScreen: document.getElementById('gameOverScreen'),
      pauseScreen: document.getElementById('pauseScreen'),
      finalScore: document.getElementById('finalScore'),
      newHighScore: document.getElementById('newHighScore'),
      touchControls: document.querySelector('.touch-controls'),
      sfxVolume: document.getElementById('sfxVolume'),
      musicVolume: document.getElementById('musicVolume'),
      muteSfx: document.getElementById('muteSfx'),
      muteMusic: document.getElementById('muteMusic'),
      btnEasy: document.getElementById('btnEasy'),
      btnMedium: document.getElementById('btnMedium'),
      btnHard: document.getElementById('btnHard')
    };
  }

  _bindEvents() {
    this.elements.btnEasy?.addEventListener('click', () => this._startGame('easy'));
    this.elements.btnMedium?.addEventListener('click', () => this._startGame('medium'));
    this.elements.btnHard?.addEventListener('click', () => this._startGame('hard'));

    document.getElementById('btnPlayAgain')?.addEventListener('click', () => {
      this.game.start();
    });

    document.getElementById('btnMenu')?.addEventListener('click', () => {
      this.game.showMenu();
      this.audio.stopBgm();
    });

    document.getElementById('btnResume')?.addEventListener('click', () => {
      this.game.state.paused = false;
      this.game.notifyObservers();
    });

    this.elements.sfxVolume?.addEventListener('input', (e) => {
      const value = parseInt(e.target.value);
      this.audio.setSfxVolume(value);
      document.getElementById('sfxVolumeValue').textContent = value;
    });

    this.elements.musicVolume?.addEventListener('input', (e) => {
      const value = parseInt(e.target.value);
      this.audio.setMusicVolume(value);
      document.getElementById('musicVolumeValue').textContent = value;
    });

    this.elements.muteSfx?.addEventListener('click', () => {
      const muted = this.audio.toggleSfxMute();
      this.elements.muteSfx.textContent = muted ? '🔇' : '🔊';
    });

    this.elements.muteMusic?.addEventListener('click', () => {
      const muted = this.audio.toggleMusicMute();
      this.elements.muteMusic.textContent = muted ? '🔇' : '🔊';
    });

    window.addEventListener('resize', () => {
      this.renderer.resize();
    });

    this._updateTouchControlsVisibility();
  }

  _updateTouchControlsVisibility() {
    const isTouch = 'ontouchstart' in window || navigator.maxTouchPoints > 0;
    if (this.elements.touchControls) {
      this.elements.touchControls.style.display = isTouch ? 'flex' : 'none';
    }
  }

  _startGame(difficulty) {
    this.game.setDifficulty(difficulty);
    this.game.start();
    this.audio.playBgm(`bgm-${difficulty}`);
  }

  _updateUI(state) {
    if (this.elements.score) {
      this.elements.score.textContent = `Score: ${state.score}`;
    }

    if (this.elements.highScore) {
      this.elements.highScore.textContent = `High: ${state.highScore}`;
    }

    if (this.elements.difficulty) {
      const labels = { easy: 'Easy', medium: 'Med', hard: 'Hard' };
      this.elements.difficulty.textContent = labels[state.difficulty];
    }

    if (this.elements.powerUpTimer && state.snake?.remaining > 0) {
      const icons = {
        speedBoost: '⚡',
        invincible: '🛡️',
        ghost: '👻'
      };
      this.elements.powerUpTimer.textContent = `${icons[state.snake.powerUp]} ${state.snake.remaining.toFixed(1)}s`;
    } else if (this.elements.powerUpTimer) {
      this.elements.powerUpTimer.textContent = '';
    }

    if (state.menu) {
      this._showScreen('menu');
    } else if (state.gameOver) {
      this._showScreen('gameOver', state);
    } else if (state.paused) {
      this._showScreen('pause');
    } else {
      this._showScreen('game');
    }

    if (!state.menu) {
      this.renderer.render(state);

      if (state.gameOver && state.snake) {
        this.renderer.spawnDeathParticles(state.snake.body[0], this.renderer.getCellSize());
      }
    }
  }

  _showScreen(screenName, state = null) {
    Object.values(this.elements).forEach(el => {
      if (el && el.classList && el.classList.contains('screen')) {
        el.style.display = 'none';
      }
    });

    switch (screenName) {
      case 'menu':
        this.elements.menuScreen.style.display = 'flex';
        break;
      case 'game':
        this.elements.gameScreen.style.display = 'block';
        break;
      case 'pause':
        this.elements.pauseScreen.style.display = 'flex';
        break;
      case 'gameOver':
        this.elements.gameOverScreen.style.display = 'flex';
        if (this.elements.finalScore) {
          this.elements.finalScore.textContent = `Score: ${state.score}`;
        }
        if (this.elements.newHighScore) {
          const isNewHigh = state.score >= state.highScore && state.score > 0;
          this.elements.newHighScore.style.display = isNewHigh ? 'block' : 'none';
        }
        break;
    }
  }

  initSettings() {
    const settings = this.audio.getSettings();

    if (this.elements.sfxVolume) {
      this.elements.sfxVolume.value = settings.sfxVolume;
      const valEl = document.getElementById('sfxVolumeValue');
      if (valEl) valEl.textContent = settings.sfxVolume;
    }

    if (this.elements.musicVolume) {
      this.elements.musicVolume.value = settings.musicVolume;
      const valEl = document.getElementById('musicVolumeValue');
      if (valEl) valEl.textContent = settings.musicVolume;
    }

    if (this.elements.muteSfx) {
      this.elements.muteSfx.textContent = settings.sfxMuted ? '🔇' : '🔊';
    }

    if (this.elements.muteMusic) {
      this.elements.muteMusic.textContent = settings.musicMuted ? '🔇' : '🔊';
    }
  }
}

// Main Entry Point
function checkBrowserSupport() {
  const canvas = document.createElement('canvas');
  if (!canvas.getContext) {
    document.body.innerHTML = '<div style="padding: 20px; color: white; background: #e74c3c;">HTML5 Canvas not supported. Please update your browser.</div>';
    return false;
  }
  return true;
}

function init() {
  if (!checkBrowserSupport()) return;

  const storage = new StorageManager();
  const inputHandler = new InputHandler();
  const game = new Game(storage, inputHandler);
  const renderer = new Renderer('gameCanvas');
  const audio = new AudioManager(storage);
  const ui = new UI(game, renderer, audio);

  audio.loadSounds([
    { name: 'eat', path: 'assets/sounds/eat.mp3' },
    { name: 'eat-golden', path: 'assets/sounds/eat-golden.mp3' },
    { name: 'eat-purple', path: 'assets/sounds/eat-purple.mp3' },
    { name: 'eat-blue', path: 'assets/sounds/eat-blue.mp3' },
    { name: 'eat-ghost', path: 'assets/sounds/eat-ghost.mp3' },
    { name: 'die', path: 'assets/sounds/die.mp3' },
    { name: 'gameover', path: 'assets/sounds/gameover.mp3' },
    { name: 'highscore', path: 'assets/sounds/highscore.mp3' },
    { name: 'bgm-easy', path: 'assets/sounds/bgm-easy.mp3' },
    { name: 'bgm-medium', path: 'assets/sounds/bgm-medium.mp3' },
    { name: 'bgm-hard', path: 'assets/sounds/bgm-hard.mp3' }
  ]);

  ui.initSettings();
  renderer.resize();
  game.notifyObservers();

  window.snakeGame = { game, renderer, audio, ui };

  console.log('🐍 Snake Game initialized');
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}
```

- [ ] **Step 2: Update index.html to use bundle.js**

Edit: `snake-game/index.html`
Replace the script line at the end:
```html
<!-- Old: <script type="module" src="js/main.js"></script> -->
<script src="js/bundle.js"></script>
```

- [ ] **Step 3: Commit**

```bash
git add js/bundle.js index.html
git commit -m "feat: Add bundled JavaScript for browser compatibility"
```

---

### Task 14: Create Audio Placeholder Files

**Files:**
- Create: `snake-game/assets/sounds/.gitkeep`

- [ ] **Step 1: Create placeholder for audio files**

Write: `snake-game/assets/sounds/.gitkeep`
```bash
# Audio files should be placed here:
# eat.mp3, eat-golden.mp3, eat-purple.mp3, eat-blue.mp3, eat-ghost.mp3
# die.mp3, gameover.mp3, highscore.mp3
# bgm-easy.mp3, bgm-medium.mp3, bgm-hard.mp3
```

- [ ] **Step 2: Create README for audio files**

Write: `snake-game/assets/sounds/README.md`
```markdown
# Audio Files

This directory should contain the following audio files:

## Sound Effects
- `eat.mp3` - Regular food eaten (100ms)
- `eat-golden.mp3` - Golden food eaten (300ms)
- `eat-purple.mp3` - Purple food eaten (400ms)
- `eat-blue.mp3` - Blue food eaten (200ms)
- `eat-ghost.mp3` - Ghost food eaten (250ms)
- `die.mp3` - Snake dies (500ms)
- `gameover.mp3` - Game over melody (2s)
- `highscore.mp3` - New high score fanfare (1.5s)

## Background Music
- `bgm-easy.mp3` - Lo-fi beats for Easy mode
- `bgm-medium.mp3` - Upbeat electronic for Medium mode
- `bgm-hard.mp3` - Fast-paced for Hard mode

## Audio Resources
You can find free audio assets at:
- [Freesound.org](https://freesound.org/)
- [OpenGameArt.org](https://opengameart.org/)
- [ZapSplat](https://www.zapsplat.com/)

The game will work without audio files, but sound effects and music enhance the experience.
```

- [ ] **Step 3: Commit**

```bash
git add assets/sounds/.gitkeep assets/sounds/README.md
git commit -m "feat: Add audio placeholder files"
```

---

## Testing and Verification

### Task 15: Manual Testing Checklist

- [ ] **Step 1: Test in browser**

Open `snake-game/index.html` in a browser and verify:

**Menu Screen:**
- [ ] Title and stats display correctly
- [ ] Difficulty buttons are visible and clickable
- [ ] Settings sliders work and update
- [ ] Controls info is displayed

**Gameplay - Easy Mode:**
- [ ] Game starts when Easy button clicked
- [ ] Snake moves with arrow keys
- [ ] Snake grows when eating red food
- [ ] Score updates correctly
- [ ] Snake wraps around walls (doesn't die)
- [ ] Snake dies when hitting itself
- [ ] Game over screen appears with score

**Gameplay - Medium Mode:**
- [ ] Snake dies when hitting walls
- [ ] All other Easy mode features work

**Gameplay - Hard Mode:**
- [ ] Snake moves faster
- [ ] Snake dies on wall collision
- [ ] Food spawns more frequently

**Food Types:**
- [ ] Golden food appears and gives speed boost visual
- [ ] Purple food appears and shows shield icon
- [ ] Blue food shrinks snake
- [ ] Ghost food makes snake translucent
- [ ] Food expires and disappears

**Power-ups:**
- [ ] Timer displays when power-up active
- [ ] Only one power-up active at a time
- [ ] Screen edge glows with power-up color

**Controls:**
- [ ] WASD keys work
- [ ] Space/P pauses game
- [ ] ESC returns to menu
- [ ] Touch buttons work on mobile
- [ ] Swipe gestures work on mobile

**Persistence:**
- [ ] High scores save after game over
- [ ] High scores load on page refresh
- [ ] Settings persist across sessions

**Responsive:**
- [ ] Canvas scales on smaller screens
- [ ] Touch controls appear on mobile
- [ ] Layout works on different screen sizes

- [ ] **Step 2: Document any issues found**

If issues are found, create fixes as separate tasks.

---

## Task 16: Final Review and Documentation

- [ ] **Step 1: Update README with build instructions**

Edit: `snake-game/README.md`
Add after existing content:
```markdown
## Development

The game uses a bundled JavaScript approach for maximum browser compatibility.

### File Structure
- `index.html` - Main HTML file
- `css/style.css` - All styles
- `js/bundle.js` - Bundled game code
- `assets/sounds/` - Audio files (to be added)

### Running the Game

Simply open `index.html` in any modern web browser. No build process required.

### Adding Audio

Place audio files in `assets/sounds/` following the naming convention in `assets/sounds/README.md`.

### Browser Compatibility

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+
- Mobile browsers with touch support
```

- [ ] **Step 2: Final commit**

```bash
git add README.md
git commit -m "docs: Update README with development notes"
```

- [ ] **Step 3: Create git tag for v1.0**

```bash
git tag -a v1.0 -m "Snake Game v1.0 - Complete implementation"
git push origin main --tags
```

---

## Implementation Complete

The Snake Game is now fully implemented with all features from the specification:

✅ Three difficulty levels with different speeds and rules
✅ Multiple food types with power-ups
✅ Scoring system with speed multipliers and combos
✅ High score persistence via LocalStorage
✅ Cross-platform controls (keyboard, touch, swipe)
✅ Modern, responsive visual design
✅ Audio system with SFX and BGM support
✅ Pause/resume functionality
✅ Game over and restart flow

### Next Steps (Optional Enhancements)
- Add actual audio files to `assets/sounds/`
- Add obstacles system for Hard mode
- Implement level progression
- Add more visual effects and polish
- Create mobile app wrapper
