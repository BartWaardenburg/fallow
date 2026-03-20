#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, statSync, rmSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import os from 'node:os';

const __dirname = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(__dirname, '..');
const args = process.argv.slice(2);
const hasFilter = args.includes('--synthetic') || args.includes('--real-world');
const runSynthetic = args.includes('--synthetic') || !hasFilter;
const runRealWorld = args.includes('--real-world') || !hasFilter;
const RUNS = parseInt(args.find(a => a.startsWith('--runs='))?.split('=')[1] ?? '5');
const WARMUP = parseInt(args.find(a => a.startsWith('--warmup='))?.split('=')[1] ?? '2');

console.log('Building fallow (release)...');
const buildResult = spawnSync('cargo', ['build', '--release'], { cwd: rootDir, stdio: 'pipe', timeout: 300000 });
if (buildResult.status !== 0) { console.error('Build failed:', buildResult.stderr?.toString()); process.exit(1); }
const fallowBin = join(rootDir, 'target', 'release', 'fallow');
const knipBin = join(__dirname, 'node_modules', '.bin', 'knip');
const knip6Bin = join(__dirname, 'knip6', 'node_modules', '.bin', 'knip');
if (!existsSync(knipBin)) { console.error('knip v5 not found. Run: cd benchmarks && npm install'); process.exit(1); }
const hasKnip6 = existsSync(knip6Bin);
if (!hasKnip6) { console.warn('knip v6 not found. Run: cd benchmarks/knip6 && npm install knip@6'); }

const fallowVersion = spawnSync(fallowBin, ['--version'], { stdio: 'pipe' }).stdout?.toString().trim();
const knipVersion = spawnSync(knipBin, ['--version'], { stdio: 'pipe' }).stdout?.toString().trim();
const knip6Version = hasKnip6 ? spawnSync(knip6Bin, ['--version'], { stdio: 'pipe' }).stdout?.toString().trim() : null;
const rustVersion = spawnSync('rustc', ['--version'], { stdio: 'pipe' }).stdout?.toString().trim();

console.log(`\n=== Fallow vs Knip Benchmark Suite ===\n`);
printEnvironment();
console.log(`Tools:\n  fallow   ${fallowVersion}\n  knip v5  ${knipVersion}${knip6Version ? `\n  knip v6  ${knip6Version}` : ''}\nConfig: ${RUNS} runs, ${WARMUP} warmup\n`);

function printEnvironment() {
  const cpus = os.cpus();
  console.log('Environment:');
  console.log(`  CPU:     ${cpus[0].model.trim()} (${cpus.length} logical cores)`);
  console.log(`  RAM:     ${(os.totalmem() / 1024 / 1024 / 1024).toFixed(1)} GB`);
  console.log(`  OS:      ${os.platform()} ${os.release()} ${os.arch()}`);
  console.log(`  Node:    ${process.version}`);
  console.log(`  Rust:    ${rustVersion}`);
  console.log('');
}

function countSourceFiles(dir) {
  let count = 0;
  const walk = d => { try { for (const e of readdirSync(d)) { if (['node_modules','.git','dist'].includes(e)) continue; const f = join(d, e); try { const s = statSync(f); if (s.isDirectory()) walk(f); else if (/\.(ts|tsx|js|jsx|mjs|cjs)$/.test(e)) count++; } catch {} } } catch {} };
  walk(dir); return count;
}

function timeRun(cmd, cmdArgs, cwd) {
  const start = performance.now();
  const result = spawnSync(cmd, cmdArgs, { cwd, stdio: 'pipe', timeout: 300000, maxBuffer: 50*1024*1024, env: { ...process.env, NO_COLOR: '1', FORCE_COLOR: '0' } });
  return { elapsed: performance.now() - start, status: result.status, stdout: result.stdout?.toString() ?? '', stderr: result.stderr?.toString() ?? '' };
}

function timeRunWithMemory(cmd, cmdArgs, cwd) {
  const isLinux = process.platform === 'linux';
  const timeBin = '/usr/bin/time';
  const timeArgs = isLinux ? ['-v', cmd, ...cmdArgs] : ['-l', cmd, ...cmdArgs];

  const start = performance.now();
  const result = spawnSync(timeBin, timeArgs, { cwd, stdio: 'pipe', timeout: 300000, maxBuffer: 50*1024*1024, env: { ...process.env, NO_COLOR: '1', FORCE_COLOR: '0' } });
  const elapsed = performance.now() - start;
  const stderr = result.stderr?.toString() ?? '';

  let peakRssBytes = 0;
  if (isLinux) {
    const match = stderr.match(/Maximum resident set size \(kbytes\): (\d+)/);
    if (match) peakRssBytes = parseInt(match[1]) * 1024;
  } else {
    // macOS: reports in bytes
    const match = stderr.match(/(\d+)\s+maximum resident set size/);
    if (match) peakRssBytes = parseInt(match[1]);
  }

  // stdout for fallow comes from the time wrapper's child process — it's on stdout
  const stdout = result.stdout?.toString() ?? '';

  return { elapsed, status: result.status, stdout, stderr, peakRssBytes };
}

function parseIssueCount(stdout) {
  try { const data = JSON.parse(stdout); let count = 0; for (const v of Object.values(data)) { if (Array.isArray(v)) count += v.length; } return count; } catch { return '?'; }
}

function stats(times) {
  const sorted = [...times].sort((a,b) => a-b);
  const mid = Math.floor(sorted.length / 2);
  const median = sorted.length % 2 === 0 ? (sorted[mid - 1] + sorted[mid]) / 2 : sorted[mid];
  return { min: sorted[0], max: sorted.at(-1), mean: sorted.reduce((a,b)=>a+b,0)/sorted.length, median };
}

function fmt(ms) { return ms < 1000 ? `${ms.toFixed(0)}ms` : `${(ms/1000).toFixed(2)}s`; }
function fmtMem(bytes) { if (bytes === 0) return '?'; const mb = bytes / 1024 / 1024; return mb < 1024 ? `${mb.toFixed(1)} MB` : `${(mb/1024).toFixed(2)} GB`; }

function clearFallowCache(dir) {
  const cacheDir = join(dir, '.fallow');
  if (existsSync(cacheDir)) rmSync(cacheDir, { recursive: true });
}

function benchmarkProject(name, dir) {
  const files = countSourceFiles(dir);
  console.log(`### ${name} (${files} source files)\n`);

  // --- Cold cache (no cache) ---
  const fArgsCold = ['check', '--quiet', '--format', 'json', '--no-cache'];
  const kArgs = ['--reporter', 'json'];
  for (let i = 0; i < WARMUP; i++) {
    timeRun(fallowBin, fArgsCold, dir);
    timeRun(knipBin, kArgs, dir);
    if (hasKnip6) timeRun(knip6Bin, kArgs, dir);
  }

  const fTimesCold = [], kTimes = [], k6Times = [];
  let fIssues = '?', kIssues = '?', k6Issues = '?', fPeakRss = 0, kPeakRss = 0, k6PeakRss = 0;

  for (let i = 0; i < RUNS; i++) {
    const fr = timeRunWithMemory(fallowBin, fArgsCold, dir);
    fTimesCold.push(fr.elapsed);
    if (i === 0) { fIssues = parseIssueCount(fr.stdout); fPeakRss = fr.peakRssBytes; }
    const kr = timeRunWithMemory(knipBin, kArgs, dir);
    kTimes.push(kr.elapsed);
    if (i === 0) { kIssues = parseIssueCount(kr.stdout); kPeakRss = kr.peakRssBytes; }
    if (hasKnip6) {
      const k6r = timeRunWithMemory(knip6Bin, kArgs, dir);
      k6Times.push(k6r.elapsed);
      if (i === 0) { k6Issues = parseIssueCount(k6r.stdout); k6PeakRss = k6r.peakRssBytes; }
    }
  }

  // --- Warm cache ---
  clearFallowCache(dir);
  const fArgsWarm = ['check', '--quiet', '--format', 'json'];
  // Populate cache
  timeRun(fallowBin, fArgsWarm, dir);
  // Benchmark warm runs
  const fTimesWarm = [];
  for (let i = 0; i < RUNS; i++) {
    const fr = timeRun(fallowBin, fArgsWarm, dir);
    fTimesWarm.push(fr.elapsed);
  }
  clearFallowCache(dir);

  const fsCold = stats(fTimesCold), fsWarm = stats(fTimesWarm), ks = stats(kTimes);
  const k6s = hasKnip6 ? stats(k6Times) : null;
  const speedupColdV5 = ks.median / fsCold.median;
  const speedupWarmV5 = ks.median / fsWarm.median;
  const speedupColdV6 = k6s ? k6s.median / fsCold.median : null;
  const speedupWarmV6 = k6s ? k6s.median / fsWarm.median : null;
  const cacheSpeedup = fsCold.median / fsWarm.median;

  const rows = [
    { Tool: 'fallow (cold)', Min: fmt(fsCold.min), Mean: fmt(fsCold.mean), Median: fmt(fsCold.median), Max: fmt(fsCold.max), 'vs knip v5': `${speedupColdV5.toFixed(1)}x`, ...(k6s ? { 'vs knip v6': `${speedupColdV6.toFixed(1)}x` } : {}), Memory: fmtMem(fPeakRss), Issues: fIssues },
    { Tool: 'fallow (warm)', Min: fmt(fsWarm.min), Mean: fmt(fsWarm.mean), Median: fmt(fsWarm.median), Max: fmt(fsWarm.max), 'vs knip v5': `${speedupWarmV5.toFixed(1)}x`, ...(k6s ? { 'vs knip v6': `${speedupWarmV6.toFixed(1)}x` } : {}), Memory: '-', Issues: fIssues },
    { Tool: 'knip v5',       Min: fmt(ks.min),     Mean: fmt(ks.mean),     Median: fmt(ks.median),     Max: fmt(ks.max),     'vs knip v5': '1.0x',                           ...(k6s ? { 'vs knip v6': '-' } : {}),                          Memory: fmtMem(kPeakRss), Issues: kIssues },
  ];
  if (k6s) {
    rows.push({ Tool: 'knip v6', Min: fmt(k6s.min), Mean: fmt(k6s.mean), Median: fmt(k6s.median), Max: fmt(k6s.max), 'vs knip v5': `${(ks.median / k6s.median).toFixed(1)}x`, 'vs knip v6': '1.0x', Memory: fmtMem(k6PeakRss), Issues: k6Issues });
  }
  console.table(rows);
  console.log(`  Cache speedup: ${cacheSpeedup.toFixed(2)}x (warm vs cold)`);
  console.log(`  fallow cold: [${fTimesCold.map(t=>t.toFixed(0)).join(', ')}]`);
  console.log(`  fallow warm: [${fTimesWarm.map(t=>t.toFixed(0)).join(', ')}]`);
  console.log(`  knip v5:     [${kTimes.map(t=>t.toFixed(0)).join(', ')}]`);
  if (k6s) console.log(`  knip v6:     [${k6Times.map(t=>t.toFixed(0)).join(', ')}]`);
  console.log('');

  return { name, files, fallowCold: fsCold, fallowWarm: fsWarm, knip: ks, knip6: k6s, speedupColdV5, speedupWarmV5, speedupColdV6, speedupWarmV6, cacheSpeedup, fIssues, kIssues, k6Issues, fPeakRss, kPeakRss, k6PeakRss };
}

const results = [];
if (runSynthetic) {
  const d = join(__dirname, 'fixtures', 'synthetic');
  if (!existsSync(d)) { console.log('No synthetic fixtures. Run: npm run generate\n'); }
  else {
    console.log('--- Synthetic Projects ---\n');
    const order = ['tiny','small','medium','large','xlarge'];
    for (const p of readdirSync(d).filter(x => existsSync(join(d,x,'package.json'))).sort((a,b) => order.indexOf(a)-order.indexOf(b)))
      results.push(benchmarkProject(p, join(d, p)));
  }
}
if (runRealWorld) {
  const d = join(__dirname, 'fixtures', 'real-world');
  if (!existsSync(d)) { console.log('No real-world fixtures. Run: npm run download-fixtures\n'); }
  else {
    console.log('--- Real-World Projects ---\n');
    for (const p of readdirSync(d).filter(x => existsSync(join(d,x,'package.json'))).sort())
      results.push(benchmarkProject(p, join(d, p)));
  }
}
if (results.length > 0) {
  console.log('\n=== Summary ===\n');
  console.table(results.map(r => ({
    Project: r.name,
    Files: r.files,
    'Cold (median)': fmt(r.fallowCold.median),
    'Warm (median)': fmt(r.fallowWarm.median),
    'Knip v5 (median)': fmt(r.knip.median),
    ...(r.knip6 ? { 'Knip v6 (median)': fmt(r.knip6.median) } : {}),
    'vs v5 (cold)': `${r.speedupColdV5.toFixed(1)}x`,
    ...(r.speedupColdV6 != null ? { 'vs v6 (cold)': `${r.speedupColdV6.toFixed(1)}x` } : {}),
    'Cache effect': `${r.cacheSpeedup.toFixed(2)}x`,
    'Fallow RSS': fmtMem(r.fPeakRss),
    'Knip v5 RSS': fmtMem(r.kPeakRss),
    ...(r.knip6 ? { 'Knip v6 RSS': fmtMem(r.k6PeakRss) } : {}),
  })));
  console.log(`Average speedup vs knip v5 (cold): ${(results.reduce((s,r) => s+r.speedupColdV5, 0)/results.length).toFixed(1)}x`);
  console.log(`Average speedup vs knip v5 (warm): ${(results.reduce((s,r) => s+r.speedupWarmV5, 0)/results.length).toFixed(1)}x`);
  if (hasKnip6) {
    console.log(`Average speedup vs knip v6 (cold): ${(results.reduce((s,r) => s+(r.speedupColdV6??0), 0)/results.length).toFixed(1)}x`);
    console.log(`Average speedup vs knip v6 (warm): ${(results.reduce((s,r) => s+(r.speedupWarmV6??0), 0)/results.length).toFixed(1)}x`);
  }
  console.log(`Average cache effect:              ${(results.reduce((s,r) => s+r.cacheSpeedup, 0)/results.length).toFixed(2)}x\n`);
}
