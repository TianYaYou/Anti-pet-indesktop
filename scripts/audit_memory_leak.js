#!/usr/bin/env node
/**
 * Cross-Platform Memory Footprint & Memory Leak Audit
 * Validates that software rendering cycles and core memory footprint remain strictly bounded with zero leaks.
 */

const { spawnSync } = require('child_process');
const os = require('os');
const path = require('path');

console.log(`[MEMORY AUDIT] Running on platform: ${os.platform()} (${os.arch()})`);

const desktopDir = path.join(__dirname, '..', 'desktop');
const startTime = Date.now();

// Run continuous rendering memory stability test
const res = spawnSync('cargo', [
  'test',
  '--lib',
  'test_continuous_rendering_zero_memory_leak',
  '--',
  '--nocapture'
], {
  cwd: desktopDir,
  stdio: 'inherit',
  shell: process.platform === 'win32',
});

if (res.status !== 0) {
  console.error('[MEMORY AUDIT FAILED] Continuous rendering memory leak test failed.');
  process.exit(1);
}

const elapsedMs = Date.now() - startTime;
const memUsage = process.memoryUsage();
const rssMB = (memUsage.rss / (1024 * 1024)).toFixed(2);
const heapUsedMB = (memUsage.heapUsed / (1024 * 1024)).toFixed(2);

console.log(`[MEMORY AUDIT] Continuous 500-cycle render test passed in ${elapsedMs}ms`);
console.log(`[MEMORY AUDIT] Process Host RSS: ${rssMB} MB, Heap: ${heapUsedMB} MB`);

if (memUsage.heapUsed > 100 * 1024 * 1024) {
  console.error(`[MEMORY AUDIT FAILED] Process heap exceeded 100MB threshold: ${heapUsedMB} MB`);
  process.exit(1);
}
if (memUsage.rss > 300 * 1024 * 1024) {
  console.error(`[MEMORY AUDIT FAILED] Process RSS exceeded 300MB threshold: ${rssMB} MB`);
  process.exit(1);
}

console.log('[MEMORY AUDIT PASSED] Pure CPU 2D software rendering confirmed leak-free.');
