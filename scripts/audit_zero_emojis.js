#!/usr/bin/env node
/**
 * Zero-Emoji Audit Script
 * Validates that core UI files and desktop source code do not contain emoji characters.
 */

const fs = require('fs');
const path = require('path');

const emojiRegex = /[\u{1F300}-\u{1F9FF}]|[\u{2600}-\u{26FF}]|[\u{2700}-\u{27BF}]|[\u{1F1E6}-\u{1F1FF}]|[\u{1F600}-\u{1F64F}]|[\u{1F680}-\u{1F6FF}]/u;

const scanDirs = [
  path.join(__dirname, '..', 'core'),
  path.join(__dirname, '..', 'desktop', 'src')
];

let hasErrors = false;

function scanDir(dir) {
  if (!fs.existsSync(dir)) return;
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      scanDir(fullPath);
    } else if (entry.isFile() && (entry.name.endsWith('.js') || entry.name.endsWith('.html') || entry.name.endsWith('.css') || entry.name.endsWith('.rs'))) {
      const content = fs.readFileSync(fullPath, 'utf-8');
      const lines = content.split('\n');
      lines.forEach((line, idx) => {
        if (emojiRegex.test(line)) {
          console.error(`[EMOJI AUDIT FAILURE] ${path.relative(path.join(__dirname, '..'), fullPath)}:${idx + 1}: ${line.trim()}`);
          hasErrors = true;
        }
      });
    }
  }
}

for (const dir of scanDirs) {
  scanDir(dir);
}

if (hasErrors) {
  console.error('\nAudit failed: emoji characters found in UI/desktop code.');
  process.exit(1);
} else {
  console.log('Audit passed: zero emojis detected in core and desktop UI sources.');
}
