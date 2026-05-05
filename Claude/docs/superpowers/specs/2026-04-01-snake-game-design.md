# Snake Game Design Document

**Date:** 2026-04-01
**Status:** Approved
**Version:** 1.1

## Overview

A modern, feature-rich Snake Game implemented as a standalone web application using vanilla HTML, CSS, and JavaScript. The game features multiple difficulty levels, various food types with power-ups, comprehensive audio system, and support for keyboard, touch, and gesture controls.

## Requirements

### Core Features
- Classic snake gameplay with modern enhancements
- Score tracking with high score persistence (LocalStorage)
- Three difficulty levels: Easy, Medium, Hard
- Multiple food types with different point values and effects
- Power-up system with temporary abilities
- Comprehensive audio system (SFX + multiple BGM tracks)
- Cross-platform controls: keyboard, on-screen buttons, swipe gestures
- Modern, minimalist visual design

### Success Criteria
- Game runs smoothly at 60fps
- Responsive design works on mobile, tablet, and desktop
- All controls are responsive and reliable
- High scores persist across sessions
- Audio can be fully customized or muted

## Architecture

### Component Structure

```
snake-game/
├── index.html           # Main HTML file
├── css/
│   └── style.css        # All styling
├── js/
│   ├── main.js          # Entry point, initialization
│   ├── game.js          # Core game engine
│   ├── snake.js         # Snake class
│   ├── food.js          # Food & power-up system
│   ├── renderer.js      # Canvas rendering
│   ├── input.js         # Input handling (keyboard, touch, swipe)
│   ├── audio.js         # Audio manager
│   ├── storage.js       # High score & settings persistence
│   └── ui.js            # UI management
├── assets/
│   ├── sounds/          # Sound effects and BGM
│   └── fonts/           # Custom game font
└── README.md            # Setup & controls guide
```

### Core Modules

**Game Engine (`game.js`)**
- Game loop management (requestAnimationFrame)
- Collision detection (walls, self, food)
- State management (single source of truth)
- Score tracking and validation
- Difficulty level handling

**Snake (`snake.js`)**
- Snake entity management
- Movement with smooth interpolation
- Growth mechanics
- Power-up state tracking

**Food System (`food.js`)**
- Food spawning logic
- Different food types with unique properties
- Power-up effect management
- Despawn timers

**Renderer (`renderer.js`)**
- Canvas-based rendering (60fps)
- Grid drawing
- Snake, food, and UI rendering
- Animations (particles, glow effects)

**Input Handler (`input.js`)**
- Unified input system
- Keyboard event handling
- Touch button management
- Swipe gesture detection
- Input normalization and buffering

**Audio Manager (`audio.js`)**
- Sound effect library
- Background music tracks
- Volume controls (SFX/Music independent)
- Mute functionality
- Track selection

**Storage Manager (`storage.js`)**
- LocalStorage wrapper
- High score persistence
- Settings persistence
- Data validation

**UI Manager (`ui.js`)**
- Screen management (start, pause, game over)
- HUD updates
- Settings panel
- Responsive layout

## Game Mechanics

### Grid Specifications
- Base grid: 20x20 cells
- Base cell size: 25px (scales responsively)
- Canvas size: 500x500px (scales to fit viewport with aspect ratio)
- Minimum cell size: 15px (mobile)
- Maximum cell size: 40px (large desktop)

### Movement
- Grid-based movement with smooth visual interpolation
- Configurable speed per difficulty:
  - Easy: 150ms per tick
  - Medium: 100ms per tick
  - Hard: 60ms per tick
- Input buffer holds up to 2 inputs; oldest dropped when full
- Buffer expires inputs after 500ms if not processed
- Direction reverse prevention (can't go left when moving right)

### Food Spawning Logic

**Spawn Location:**
- Random empty cell (not occupied by snake or other food)
- Safe zone: Cannot spawn within 2 cells of snake head (prevents cheap deaths)
- Maximum 3 food items on screen simultaneously

**Spawn Rate:**
- New food spawns every 3 seconds (if below max)
- Hard mode: New food spawns every 2 seconds

**Spawn Probabilities:**
| Food Type | Spawn Chance |
|-----------|--------------|
| Regular | 70% |
| Golden | 10% |
| Purple | 10% |
| Blue | 5% |
| Ghost | 5% |

**Food Lifetimes:**
- Regular: No expiration
- Golden: Expires after 8 seconds
- Purple: Expires after 8 seconds
- Blue: Expires after 10 seconds
- Ghost: Expires after 6 seconds

### Food Types

| Type | Appearance | Points | Effect |
|------|------------|--------|--------|
| Regular | Red circle | 10 | Grow by 1 |
| Golden | Gold circle with glow | 50 | Grow by 1 + 5 sec speed boost |
| Purple | Purple circle with shield icon | 30 | Grow by 1 + 5 sec invincibility |
| Blue | Blue circle with shrink icon | 20 | Shrink by 1 (risky food) |
| Ghost | Translucent white with trail effect | 100 | Pass through walls/self for 5 sec |

### Difficulty Levels

**Easy:**
- Speed: 150ms per tick
- Wall collision: Wraps around (snake appears on opposite side)
- Self collision: Kills
- Food spawn rate: Every 3 seconds

**Medium:**
- Speed: 100ms per tick
- Wall collision: Kills
- Self collision: Kills
- Food spawn rate: Every 3 seconds

**Hard:**
- Speed: 60ms per tick
- Wall collision: Kills
- Self collision: Kills
- Food spawn rate: Every 2 seconds

### Scoring System

**Base Points:** Per food type (see table above)

**Speed Multiplier:**
- 1.5x if food eaten within 2 seconds of previous food
- Resets after 2 seconds of not eating

**Combo Bonus:**
- +10 points per consecutive same-type food eaten
- Resets after 5 seconds or different food type eaten
- Maximum combo: 5 (for +50 bonus)

**Final Score Formula:**
```
score = (base_points * speed_multiplier) + combo_bonus
```

**Example:**
- Eat Golden food (50 points) quickly: 50 * 1.5 = 75
- With 3x Golden combo: 75 + 30 = 105 points

### Power-up Visual Feedback

**Active Indicators:**
- Snake body changes color during power-up:
  - Speed boost: Gold glow
  - Invincibility: Purple shield icon on head
  - Ghost mode: Translucent/white with trail
- Countdown timer displayed next to score (e.g., "⚡ 4.2s")
- Pulsing glow effect at edge of canvas:
  - Gold pulsing for speed boost
  - Purple pulsing for invincibility
  - White pulsing for ghost mode

**Power-up Stacking:**
- Only one power-up active at a time
- New power-up replaces current (timer resets)
- Cannot pick up same power-up while active

## Visual Design

### Color Palette
- Background: `#1a1a2e` (dark navy)
- Snake head: `#4ecca3` (teal)
- Snake body: `#45b393` to `#38ada9` gradient
- Grid lines: `#16213e` (subtle, 1px opacity 0.3)
- UI panel: `#0f3460`
- Accent: `#e94560`
- Power-up glow colors: Gold `#ffd700`, Purple `#9b59b6`, White `#ffffff`

### Layout
```
┌─────────────────────────────────────┐
│  🐍 SNAKE GAME    Score: 150        │
│  High: 2500      Difficulty: Med ●  │
│  ⚡ Speed Boost: 3.2s                │
├─────────────────────────────────────┤
│                                     │
│                                     │
│          [Game Canvas]              │
│         (500x500, responsive)       │
│                                     │
│                                     │
├─────────────────────────────────────┤
│        [◀] [▲] [▼] [▶]             │
│     (Mobile: shown, Desktop: hidden)│
└─────────────────────────────────────┘
```

### Animations
- Snake movement: Smooth lerp interpolation between grid cells (60fps visual, game tick at difficulty rate)
- Food spawn: Scale from 0 to 1 with fade in (300ms ease-out)
- Power-up activation: Pulsing glow effect (1s cycle, scale 1.0-1.2)
- Death: Particle explosion (20 particles, 1 second, fade out)
- Food expire: Shrink and fade (500ms)
- UI transitions: Slide/fade (200ms ease-in-out)

### Particle System (Death Animation)
- 20 particles spawned at head position
- Each particle: Random velocity (-100 to 100 px/s), random angle
- Particles fade out over 1 second
- Color: Same as snake head color

## Audio System

### Sound Effects
| Event | Sound | Duration |
|-------|-------|----------|
| Eat (regular) | High-pitch "bloop" | 100ms |
| Eat (golden) | Chime + sparkle | 300ms |
| Eat (purple) | Low hum + shield | 400ms |
| Eat (blue) | Downward slide | 200ms |
| Eat (ghost) | Whoosh | 250ms |
| Die | Crash/crumble | 500ms |
| Game Over | Sad melody (descending) | 2s |
| New High Score | Victory fanfare | 1.5s |

### Background Music
1. **Ambient Chill** - Lo-fi beats (Easy mode)
2. **Retro Synth** - Upbeat electronic (Medium mode)
3. **Intense** - Fast-paced (Hard mode)

### Audio Features
- Independent SFX and Music volume controls (0-100%)
- Mute toggles for each
- BGM track selector in settings
- Crossfade duration: 1.5 seconds on track change
- BGM switches automatically when difficulty changes (with crossfade)
- Handles autoplay restrictions: Audio starts on first user interaction

### Audio Transitions
- Difficulty change: Crossfade to new BGM track (1.5s)
- Power-up end: No BGM change (power-ups don't affect music)
- Game over: Fade out BGM over 2s, then stop

## Controls

### Keyboard
- Arrow keys or WASD for direction
- Space or P to pause
- M to toggle audio
- ESC to return to menu
- 1, 2, 3 to select difficulty (from menu)

### Touch Buttons
- On-screen D-pad for mobile/tablet
- Shown only on touch-capable devices (media query: `@media (hover: none)`)
- Visual feedback: Button highlights on press (100ms)
- Button size: Minimum 44x44px (iOS guideline)

### Swipe Gestures
- Detect swipe direction for movement
- Minimum velocity: 300 pixels/second
- Maximum duration: 500ms
- Visual indicator: Arrow overlay in swipe direction (200ms fade)
- Touch end required to trigger (prevent accidental swipes)

## Data Flow

### Game Loop
```
RequestAnimationFrame (60fps target)
    ↓
Process Input (from buffer, max 1 per game tick)
    ↓
Update Game State (at difficulty tick rate)
    ↓
Render Frame (always 60fps)
    ↓
Repeat
```

### State Management
- Single `gameState` object in `game.js` with structure:
```javascript
gameState = {
  snake: { body: [], direction: '', speedBoost: false, invincible: false, ghost: false },
  food: [],
  score: 0,
  combo: 0,
  lastEatTime: 0,
  difficulty: 'medium',
  paused: false,
  gameOver: false,
  powerUpTimer: 0
}
```
- State changes trigger UI updates via `notifyObservers()` method
- Debounced persistence to LocalStorage (500ms delay)

### Input Normalization
- All inputs converted to `{type, value}` format
  - `{type: 'direction', value: 'up'}`
  - `{type: 'pause', value: true}`
- Input throttling: Minimum 10ms between inputs
- Direction validation: Cannot reverse direction

### Observer Pattern API
```javascript
// Register observer
gameState.addObserver(callback)

// State change notification
gameState.notifyObservers()

// Observer receives: {property, value, timestamp}
```

## Error Handling

### Input Protection
- Input buffer: Holds up to 2 inputs, oldest dropped when full
- Buffer expiry: Inputs expire after 500ms if not processed
- Swipe gesture: Velocity threshold 300px/s, max duration 500ms
- Multi-touch: Ignored during active gameplay (only first touch processed)

### State Validation
- Direction reverse: Prevented (e.g., can't go left when moving right)
- Collision: Pre-validated before each move
- Score: Range validation (0 - 999,999), clamped if exceeded
- Power-up timer: Clamped to 0-5 seconds

### Storage Safety
- LocalStorage availability check on init
- Data validation on load (schema version check)
- Migration handling: Version field in stored data
- Graceful degradation: Game works without storage (high scores not saved)
- Quota exceeded: Show notification, continue without saving

### Browser Compatibility
- Canvas: Feature detection, show message if unsupported
- Touch: Modern (`touchstart`/`touchend`) and legacy APIs
- AudioContext: Handle autoplay restrictions (start on first interaction)
- Resize: Debounced canvas resize (200ms delay)

### Audio Error Handling
- File load failure: Show notification, continue without audio
- Playback error: Retry once, then give up
- Volume clamped to 0-100 range
- Missing files: Graceful skip, log to console

## Testing Checklist

### Gameplay
- [ ] All difficulty levels functional
- [ ] Snake grows correctly on food consumption
- [ ] Death triggers correctly (wall/self collision per difficulty)
- [ ] Power-ups activate and expire properly
- [ ] Score updates accurately with multipliers and combos
- [ ] Food spawns in valid locations (not on snake, not too close to head)
- [ ] Food expires at correct times
- [ ] Wall wrapping works on Easy mode
- [ ] Power-up stacking prevented (only one active)

### Controls
- [ ] Keyboard controls responsive in all directions
- [ ] Reverse direction prevented
- [ ] Touch buttons work on mobile
- [ ] Swipe gestures detected correctly (300px/s threshold)
- [ ] Input buffer handles rapid inputs correctly
- [ ] Multi-touch properly ignored

### Persistence
- [ ] High scores save to LocalStorage
- [ ] High scores load correctly on restart
- [ ] Settings persist across sessions
- [ ] Data validation prevents corrupted saves
- [ ] Graceful degradation when LocalStorage unavailable
- [ ] Quota exceeded handled gracefully

### Audio
- [ ] All sound effects play correctly
- [ ] BGM tracks loop correctly
- [ ] Volume controls work (0-100%)
- [ ] Mute toggles function
- [ ] Crossfade works on track change (1.5s)
- [ ] Audio starts on first user interaction (autoplay handling)
- [ ] Audio errors handled gracefully

### Visual
- [ ] Game renders at 60fps
- [ ] Animations play smoothly
- [ ] Responsive layout on all devices
- [ ] Canvas scales correctly with aspect ratio
- [ ] Particle explosion displays correctly
- [ ] Power-up visual indicators show (color changes, timer)
- [ ] Grid lines render subtly

### Edge Cases
- [ ] Power-up stacking behavior (replace active, no stacking)
- [ ] Rapid difficulty changes during gameplay
- [ ] Audio interruption handling (phone call, tab switch)
- [ ] Browser resize during gameplay
- [ ] Long-press on touch buttons (prevent repeat)

## Deployment

The game is designed as a standalone web application with no build process:
1. Open `index.html` directly in any modern browser
2. Host on any static server (GitHub Pages, Netlify, etc.)
3. No external dependencies required

### Browser Requirements
- HTML5 Canvas support
- ES6 JavaScript support
- LocalStorage (optional - graceful degradation)
- Web Audio API (optional - graceful degradation)

### File Sizes (Estimated)
- HTML: ~5KB
- CSS: ~10KB
- JavaScript: ~30KB
- Audio assets: ~500KB (compressed)
- Total: ~550KB

## Future Enhancements (Out of Scope)

- Multiplayer support
- Level editor
- Custom skin/theme system
- Cloud-based leaderboard
- Achievements system
- Tutorial mode
- Obstacles system (removed from v1.1 due to complexity)
