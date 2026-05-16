import { execSync } from 'node:child_process'
import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { createInterface } from 'node:readline'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const rl = createInterface({
  input: process.stdin,
  output: process.stdout
})

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)
const projectRoot = join(__dirname, '..')

const question = (query) => new Promise((resolve) => rl.question(query, resolve))

const C = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  dim: '\x1b[2m',
  cyan: '\x1b[36m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  magenta: '\x1b[35m',
  red: '\x1b[31m',
  bg: '\x1b[44m\x1b[37m'
}

const printHeader = () => {
  console.clear()
  console.log(`${C.cyan}${C.bright}
   █▀█ █▀█ █ █ ▀█▀ █▀▀ █▀▀ █▀█ █▀▄ █▀▀
   █▀▄ █▄█ █▄█  █  █▄▄ █▄▄ █▄█ █▄▀ ██▄  vRELEASE
  ${C.reset}`)
  console.log(`${C.dim}───────────────────────────────────────────${C.reset}\n`)
}

function getLatestTag() {
  try {
    const tag = execSync('git describe --tags --abbrev=0', { encoding: 'utf8', cwd: projectRoot }).trim()
    return tag.startsWith('v') ? tag.substring(1) : tag
  } catch (e) {
    return null
  }
}

function compareVersions(v1, v2) {
  if (!v1) return -1
  if (!v2) return 1
  const p1 = v1.split('.').map(Number)
  const p2 = v2.split('.').map(Number)
  for (let i = 0; i < 3; i++) {
    if (p1[i] > p2[i]) return 1
    if (p1[i] < p2[i]) return -1
  }
  return 0
}

function getNextVersion(current, type) {
  const parts = current.split('.').map(Number)
  if (parts.length !== 3) return current
  const [major, minor, patch] = parts
  if (type === 'patch') return `${major}.${minor}.${patch + 1}`
  if (type === 'minor') return `${major}.${minor + 1}.0`
  if (type === 'major') return `${major + 1}.0.0`
  return current
}

function getCargoVersion(path) {
    const content = readFileSync(join(projectRoot, path), 'utf8')
    const match = content.match(/version\s*=\s*"([^"]+)"/)
    return match ? match[1] : null
}

function updateCargoVersion(path, newVersion) {
    const fullPath = join(projectRoot, path)
    let content = readFileSync(fullPath, 'utf8')
    content = content.replace(/(version\s*=\s*")([^"]+)(")/, `$1${newVersion}$3`)
    writeFileSync(fullPath, content)
}

async function main() {
  try {
    printHeader()

    // 0. Build Check
    console.log(`${C.cyan}${C.bright}Running Workspace Build Check...${C.reset}\n`)
    try {
      execSync('cargo build --release --workspace', { stdio: 'inherit', cwd: projectRoot })
      console.log(`\n${C.green}✓${C.reset} Build successful!\n`)
    } catch (error) {
      console.log(`\n${C.red}${C.bright}Build Failed!${C.reset}`)
      console.log(`${C.yellow}${C.bright}Please Fix build errors before releasing.${C.reset}\n`)
      return
    }

    // 1. Fetch Version Info
    const localVersion = getCargoVersion('apps/cli/Cargo.toml')
    const latestTag = getLatestTag()
    
    const isLocalAhead = compareVersions(localVersion, latestTag) > 0
    const currentVersion = isLocalAhead ? localVersion : (latestTag || localVersion)

    printHeader()
    console.log(`${C.cyan}Local Version:  ${C.reset} ${localVersion}`)
    console.log(`${C.cyan}Latest Tag:     ${C.reset} ${latestTag ? 'v' + latestTag : 'None found'}`)
    
    if (isLocalAhead && latestTag) {
        console.log(`${C.yellow}${C.bright}⚠️  Notice: Local version is ahead of latest Git tag.${C.reset}`)
    }
    
    console.log(`${C.cyan}Base for bump:  ${C.reset} ${C.bright}${currentVersion}${C.reset}\n`)

    // 2. Selection Menu
    console.log(`${C.bright}SELECT RELEASE TYPE:${C.reset}`)
    console.log(`  ${C.green}(1)${C.reset} ${C.bright}Patch / Fix${C.reset}      ${C.dim}→ v${getNextVersion(currentVersion, 'patch')}${C.reset}`)
    console.log(`  ${C.green}(2)${C.reset} ${C.bright}Feature / Minor${C.reset}  ${C.dim}→ v${getNextVersion(currentVersion, 'minor')}${C.reset}`)
    console.log(`  ${C.green}(3)${C.reset} ${C.bright}Breaking / Major${C.reset} ${C.dim}→ v${getNextVersion(currentVersion, 'major')}${C.reset}`)
    console.log(`  ${C.yellow}(4)${C.reset} Custom Version`)
    console.log(`  ${C.red}(0)${C.reset} Cancel`)

    const vChoice = await question(`\n${C.cyan}${C.bright}» Choose [1-4, 0]:${C.reset} `)

    let newVersion = currentVersion
    if (vChoice === '1') newVersion = getNextVersion(currentVersion, 'patch')
    else if (vChoice === '2') newVersion = getNextVersion(currentVersion, 'minor')
    else if (vChoice === '3') newVersion = getNextVersion(currentVersion, 'major')
    else if (vChoice === '4') {
      newVersion = await question(`${C.cyan}${C.bright}» Enter Custom Version (x.y.z):${C.reset} `)
    } else {
      console.log(`\n${C.dim}Operation cancelled.${C.reset}`)
      return
    }

    if (!newVersion || !/^\d+\.\d+\.\d+$/.test(newVersion)) {
      console.log(`${C.red}Invalid version format.${C.reset}`)
      return
    }

    // 3. Apply Updates
    console.log(`\n${C.cyan}${C.bright}Bumping version to v${newVersion}...${C.reset}`)
    
    const cargoPaths = [
      'apps/cli/Cargo.toml',
      'libs/sdk/Cargo.toml'
    ]

    for (const relPath of cargoPaths) {
      if (existsSync(join(projectRoot, relPath))) {
        updateCargoVersion(relPath, newVersion)
        console.log(`${C.green}✓${C.reset} Updated ${relPath}`)
      }
    }

    // 4. Commit Decision
    console.log(`\n${C.cyan}${C.bright}--- CHANGES TO COMMIT ---${C.reset}`)
    execSync('git status -s', { stdio: 'inherit', cwd: projectRoot })

    const doCommit = await question(`\n${C.yellow}${C.bright}Commit these changes now? (y/n):${C.reset} `)
    
    if (doCommit.toLowerCase() === 'y') {
      execSync('git add .', { cwd: projectRoot })
      const msg = await question(`${C.cyan}${C.bright}» Commit Message (default: release v${newVersion}):${C.reset} `)
      const commitMsg = msg || `release v${newVersion}`
      execSync(`git commit -m "${commitMsg}"`, { cwd: projectRoot })
      console.log(`${C.green}✓${C.reset} Changes committed.`)
    }

    // 5. Final Confirmation
    console.log(`\n${C.bg} FINAL RELEASE PREPARATION ${C.reset}`)
    console.log(`${C.dim}Target: v${newVersion}${C.reset}`)
    
    const finalConfirm = await question(`\n${C.yellow}${C.bright}Is everything ready to go public? (y/n):${C.reset} `)
    if (finalConfirm.toLowerCase() !== 'y') {
      console.log(`${C.dim}Push cancelled.${C.reset}`)
      return
    }

    // 6. Push to Cloud
    try {
      console.log(`${C.dim}> Tagging v${newVersion}...${C.reset}`)
      execSync(`git tag -a v${newVersion} -m "v${newVersion} release"`, { cwd: projectRoot })
      
      console.log(`${C.dim}> Pushing main branch...${C.reset}`)
      execSync('git push origin main', { cwd: projectRoot })
      
      console.log(`${C.dim}> Pushing v${newVersion} tag...${C.reset}`)
      execSync(`git push origin v${newVersion}`, { cwd: projectRoot })
      
      console.log(`\n${C.green}${C.bright}SUCCESS! v${newVersion} is now live on GitHub. 🚀${C.reset}\n`)
    } catch (err) {
      console.error(`\n${C.red}${C.bright}PUSH FAILED:${C.reset} ${err.message}`)
    }

  } catch (error) {
    console.error(`\n${C.red}${C.bright}CRITICAL ERROR:${C.reset} ${error.message}`)
  } finally {
    rl.close()
  }
}

main()
