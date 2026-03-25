#!/usr/bin/env node
/**
 * Generates Tauri icon assets from an inline SVG using sharp.
 *
 * Usage:
 *   npm install --save-dev sharp   (run once)
 *   node generate-icons.js
 *
 * Produces:
 *   icons/icon.png         (256×256, used by generate_context!())
 *   icons/32x32.png
 *   icons/128x128.png
 *   icons/128x128@2x.png   (256×256)
 *   icons/icon.ico         (ICO with 16/32/48px layers — Windows)
 *   icons/icon.icns        (ICNS — macOS, requires iconutil; see note)
 *
 * Note: .icns generation requires macOS `iconutil`. The script skips it on
 * other platforms with a warning.
 */

import sharp from 'sharp'
import fs from 'node:fs'
import path from 'node:path'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const ICONS_DIR = path.join(__dirname, 'icons')

fs.mkdirSync(ICONS_DIR, { recursive: true })

// ── SVG source — a simple devwatch "eye with pulse" mark ─────────────────────
// 512×512 viewBox, cyan accent on near-black background.
const SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <!-- Background circle -->
  <rect width="512" height="512" rx="112" fill="#0e1420"/>

  <!-- Outer eye shape -->
  <ellipse cx="256" cy="256" rx="180" ry="110" fill="none" stroke="#63b3ed" stroke-width="28" stroke-linecap="round"/>

  <!-- Iris -->
  <circle cx="256" cy="256" r="64" fill="#63b3ed" opacity="0.15"/>
  <circle cx="256" cy="256" r="48" fill="none" stroke="#63b3ed" stroke-width="20"/>

  <!-- Pupil -->
  <circle cx="256" cy="256" r="18" fill="#63b3ed"/>

  <!-- Pulse line through eye -->
  <polyline
    points="76,256 140,256 168,210 196,310 224,256 292,256 316,228 340,284 364,256 436,256"
    fill="none" stroke="#63b3ed" stroke-width="20" stroke-linecap="round" stroke-linejoin="round"
    opacity="0.75"
  />
</svg>`

// ── PNG sizes to generate ─────────────────────────────────────────────────────
const sizes = [
  { name: 'icon.png',        size: 256 },
  { name: '32x32.png',       size: 32  },
  { name: '128x128.png',     size: 128 },
  { name: '128x128@2x.png',  size: 256 },
]

const svgBuf = Buffer.from(SVG)

console.log('Generating PNG icons…')
for (const { name, size } of sizes) {
  const dest = path.join(ICONS_DIR, name)
  await sharp(svgBuf)
    .resize(size, size)
    .png()
    .toFile(dest)
  console.log(`  ✓  ${name}  (${size}×${size})`)
}

// ── ICO (Windows) — embed 16, 32, 48 px layers ───────────────────────────────
console.log('Generating icon.ico…')
await generateIco(svgBuf, path.join(ICONS_DIR, 'icon.ico'))
console.log('  ✓  icon.ico')

// ── ICNS (macOS) — requires iconutil ─────────────────────────────────────────
if (process.platform === 'darwin') {
  console.log('Generating icon.icns…')
  await generateIcns(svgBuf, ICONS_DIR)
  console.log('  ✓  icon.icns')
} else {
  console.warn('  ⚠  icon.icns skipped (requires macOS iconutil)')
}

console.log('\nDone. Icons written to', ICONS_DIR)

// ── Helpers ───────────────────────────────────────────────────────────────────

async function generateIco(svgBuf, dest) {
  // ICO format: header + directory + raw BMP/PNG data.
  // For simplicity we embed three PNG-compressed layers at 16, 32, 48 px.
  const icoSizes = [16, 32, 48]
  const pngBuffers = await Promise.all(
    icoSizes.map(s => sharp(svgBuf).resize(s, s).png().toBuffer())
  )

  // ICO header: ICONDIR
  const header = Buffer.alloc(6)
  header.writeUInt16LE(0, 0)                  // reserved
  header.writeUInt16LE(1, 2)                  // type: icon
  header.writeUInt16LE(icoSizes.length, 4)    // image count

  // ICONDIRENTRY × n
  const dirEntrySize = 16
  const dataOffset = 6 + dirEntrySize * icoSizes.length
  const entries = []
  let offset = dataOffset
  for (let i = 0; i < icoSizes.length; i++) {
    const s = icoSizes[i]
    const entry = Buffer.alloc(dirEntrySize)
    entry.writeUInt8(s === 256 ? 0 : s, 0)   // width  (0 = 256)
    entry.writeUInt8(s === 256 ? 0 : s, 1)   // height
    entry.writeUInt8(0, 2)                    // color count
    entry.writeUInt8(0, 3)                    // reserved
    entry.writeUInt16LE(1, 4)                 // color planes
    entry.writeUInt16LE(32, 6)                // bpp
    entry.writeUInt32LE(pngBuffers[i].length, 8)
    entry.writeUInt32LE(offset, 12)
    entries.push(entry)
    offset += pngBuffers[i].length
  }

  fs.writeFileSync(dest, Buffer.concat([header, ...entries, ...pngBuffers]))
}

async function generateIcns(svgBuf, iconsDir) {
  // Build an iconset directory then call iconutil.
  const iconset = path.join(iconsDir, 'icon.iconset')
  fs.mkdirSync(iconset, { recursive: true })

  const icnsSizes = [
    { file: 'icon_16x16.png',      size: 16  },
    { file: 'icon_16x16@2x.png',   size: 32  },
    { file: 'icon_32x32.png',      size: 32  },
    { file: 'icon_32x32@2x.png',   size: 64  },
    { file: 'icon_128x128.png',    size: 128 },
    { file: 'icon_128x128@2x.png', size: 256 },
    { file: 'icon_256x256.png',    size: 256 },
    { file: 'icon_256x256@2x.png', size: 512 },
    { file: 'icon_512x512.png',    size: 512 },
  ]

  await Promise.all(
    icnsSizes.map(({ file, size }) =>
      sharp(svgBuf).resize(size, size).png().toFile(path.join(iconset, file))
    )
  )

  execSync(`iconutil -c icns "${iconset}" -o "${path.join(iconsDir, 'icon.icns')}"`)
  fs.rmSync(iconset, { recursive: true })
}
