#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { spawn, spawnSync } = require('child_process');

const repoRoot = process.env.PLATFORM_REPO_DIR || process.cwd();
const runId = process.env.LATEST_CORE_TESTNET_RUN_ID
  || now().replace(/[:.]/g, '-');
const runDir = process.env.LATEST_CORE_TESTNET_SYNC_RUN_DIR
  || path.join(repoRoot, '.latest-core-testnet-sync', runId);
const metadataPath = path.join(runDir, 'run-metadata.json');

const STATUS_CONTEXT = 'Latest public Core testnet sync';

const STATUS_LABELS = {
  sync_passed: 'Sync Passed',
  build_failed: 'Build Failed',
  sync_failed: 'Sync Failed',
};

const DEFAULT_RESOLVE_TIMEOUT_MINUTES = 30;
const DEFAULT_PHASE_TIMEOUT_MINUTES = 1440;
const GITHUB_FETCH_TIMEOUT_MS = parsePositiveInteger(
  process.env.LATEST_CORE_TESTNET_GITHUB_FETCH_TIMEOUT_MS,
  60_000,
  'LATEST_CORE_TESTNET_GITHUB_FETCH_TIMEOUT_MS',
);
const STATUS_PUBLISH_ATTEMPTS = parsePositiveInteger(
  process.env.LATEST_CORE_TESTNET_STATUS_PUBLISH_ATTEMPTS,
  3,
  'LATEST_CORE_TESTNET_STATUS_PUBLISH_ATTEMPTS',
);

function now() {
  return new Date().toISOString();
}

function ensureRunDir() {
  fs.mkdirSync(runDir, { recursive: true });
}

function appendLog(fileName, message) {
  fs.appendFileSync(path.join(runDir, fileName), message);
}

function writeMetadata(metadata) {
  fs.writeFileSync(metadataPath, `${JSON.stringify(metadata, null, 2)}\n`);
}

function firstLine(value) {
  return value.split(/\r?\n/).map((line) => line.trim()).find(Boolean);
}

function sleep(ms) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

function parsePositiveMinutes(rawValue, fallback, name) {
  if (rawValue === undefined || rawValue === null || String(rawValue).trim() === '') {
    return fallback;
  }

  const value = Number(rawValue);
  if (!Number.isFinite(value) || value <= 0) {
    throw new Error(`${name} must be a positive number, got: ${rawValue}`);
  }

  return value;
}

function parsePositiveInteger(rawValue, fallback, name) {
  const value = parsePositiveMinutes(rawValue, fallback, name);
  if (!Number.isInteger(value)) {
    throw new Error(`${name} must be a positive integer, got: ${rawValue}`);
  }

  return value;
}

function sanitizePhaseEnv(env) {
  const sanitized = { ...env };
  delete sanitized.GITHUB_TOKEN;
  delete sanitized.GH_TOKEN;
  return sanitized;
}

function killProcessGroup(child, signal) {
  try {
    process.kill(-child.pid, signal);
  } catch (error) {
    if (error.code !== 'ESRCH') {
      throw error;
    }
  }
}

async function fetchWithTimeout(url, options = {}) {
  const controller = new AbortController();
  const timeout = setTimeout(() => {
    controller.abort();
  }, GITHUB_FETCH_TIMEOUT_MS);

  try {
    return await fetch(url, {
      ...options,
      signal: controller.signal,
    });
  } finally {
    clearTimeout(timeout);
  }
}

function gitOutput(args) {
  const result = spawnSync('git', args, {
    cwd: repoRoot,
    encoding: 'utf8',
  });

  if (result.status !== 0) {
    throw new Error(`git ${args.join(' ')} failed: ${result.stderr.trim()}`);
  }

  return result.stdout.trim();
}

function resolveTargetSha() {
  return process.env.TARGET_SHA
    || process.env.GITHUB_SHA
    || gitOutput(['rev-parse', 'HEAD']);
}

async function runCommand(name, command, env, timeoutMinutes, options = {}) {
  if (!command || !command.trim()) {
    throw new Error(`Missing command for phase: ${name}`);
  }

  const { captureStdout = false } = options;
  const logFileName = `${name}.log`;
  appendLog(logFileName, `$ ${command}\n\n`);

  const timeoutMs = timeoutMinutes * 60 * 1000;
  const startedAt = Date.now();

  return new Promise((resolve, reject) => {
    const child = spawn('bash', ['-c', `set -euo pipefail\n${command}`], {
      cwd: repoRoot,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
      detached: true,
    });

    let timedOut = false;
    let settled = false;
    let killTimeout;
    let stdout = '';

    const timeout = setTimeout(() => {
      timedOut = true;
      killProcessGroup(child, 'SIGTERM');
      killTimeout = setTimeout(() => {
        if (!settled) {
          killProcessGroup(child, 'SIGKILL');
        }
      }, 30_000);
    }, timeoutMs);

    child.stdout.on('data', (chunk) => {
      process.stdout.write(chunk);
      if (captureStdout) {
        stdout += chunk.toString();
      }
      appendLog(logFileName, chunk.toString());
    });

    child.stderr.on('data', (chunk) => {
      process.stderr.write(chunk);
      appendLog(logFileName, chunk.toString());
    });

    child.on('error', (error) => {
      settled = true;
      clearTimeout(timeout);
      clearTimeout(killTimeout);
      reject(error);
    });

    child.on('close', (code, signal) => {
      settled = true;
      clearTimeout(timeout);
      clearTimeout(killTimeout);
      const durationSeconds = Math.round((Date.now() - startedAt) / 1000);
      appendLog(logFileName, `\nexit_code=${code} signal=${signal || ''} duration_seconds=${durationSeconds}\n`);

      if (timedOut) {
        reject(new Error(`${name} timed out after ${timeoutMinutes} minutes`));
        return;
      }

      if (code === 0) {
        resolve({ stdout, durationSeconds });
        return;
      }

      reject(new Error(`${name} exited with code ${code}${signal ? ` (${signal})` : ''}`));
    });
  });
}

async function prepareWorkspace(metadata, timeoutMinutes) {
  if (process.env.LATEST_CORE_TESTNET_SKIP_WORKSPACE_PREP === '1') {
    return;
  }

  const branch = process.env.PLATFORM_BRANCH || 'v3.1-dev';
  metadata.phase = 'workspace-prepare';
  writeMetadata(metadata);

  await runCommand(
    'workspace-prepare',
    [
      `git fetch --prune origin ${JSON.stringify(branch)}`,
      `git checkout ${JSON.stringify(branch)}`,
      `git reset --hard origin/${JSON.stringify(branch)}`,
      'git clean -ffdx -e .latest-core-testnet-sync/',
    ].join('\n'),
    sanitizePhaseEnv(process.env),
    timeoutMinutes,
  );

  const targetSha = resolveTargetSha();
  metadata.target_sha = targetSha;
  metadata.platform_sha = targetSha;
  process.env.TARGET_SHA = targetSha;
  writeMetadata(metadata);
}

async function resolveLatestCoreVersion(metadata) {
  if (process.env.LATEST_CORE_TESTNET_CORE_VERSION) {
    return process.env.LATEST_CORE_TESTNET_CORE_VERSION;
  }

  if (process.env.LATEST_CORE_TESTNET_CORE_VERSION_COMMAND) {
    const result = await runCommand(
      'resolve-core-version',
      process.env.LATEST_CORE_TESTNET_CORE_VERSION_COMMAND,
      sanitizePhaseEnv(process.env),
      parsePositiveMinutes(
        process.env.LATEST_CORE_TESTNET_RESOLVE_TIMEOUT_MINUTES,
        DEFAULT_RESOLVE_TIMEOUT_MINUTES,
        'LATEST_CORE_TESTNET_RESOLVE_TIMEOUT_MINUTES',
      ),
      { captureStdout: true },
    );
    const version = firstLine(result.stdout);
    if (!version) {
      throw new Error('Core version command did not print a version');
    }
    return version;
  }

  const releaseRepo = process.env.LATEST_CORE_RELEASE_REPO || 'dashpay/dash';
  const response = await fetchWithTimeout(`https://api.github.com/repos/${releaseRepo}/releases`, {
    headers: {
      Accept: 'application/vnd.github+json',
      'User-Agent': 'dash-platform-latest-core-testnet-sync',
      ...(process.env.GITHUB_TOKEN ? { Authorization: `Bearer ${process.env.GITHUB_TOKEN}` } : {}),
    },
  });

  if (!response.ok) {
    const errorBody = await response.text();
    throw new Error(`Unable to fetch releases for ${releaseRepo}: HTTP ${response.status} ${errorBody}`);
  }

  const releases = await response.json();
  const latestPublicRelease = releases.find((release) => !release.draft && !release.prerelease);
  if (!latestPublicRelease) {
    throw new Error(`No public non-prerelease releases found for ${releaseRepo}`);
  }

  metadata.core_release_url = latestPublicRelease.html_url;
  return latestPublicRelease.tag_name;
}

function statusForFailurePhase(phase) {
  if (phase === 'platform-build') {
    return 'build_failed';
  }

  return 'sync_failed';
}

function statusDescription(status, metadata) {
  const details = [
    metadata.core_version,
    metadata.platform_sha ? metadata.platform_sha.slice(0, 12) : null,
    metadata.completed_at,
  ].filter(Boolean).join(' ');

  return `${STATUS_LABELS[status]}${details ? ` - ${details}` : ''}`.slice(0, 140);
}

async function publishCommitStatusOnce(status, metadata) {
  if (process.env.SKIP_GITHUB_STATUS === '1') {
    console.log(`Skipping GitHub status publish: ${STATUS_LABELS[status]}`);
    return 'skipped';
  }

  const githubToken = process.env.GITHUB_TOKEN || process.env.GH_TOKEN;
  if (!githubToken) {
    throw new Error('GITHUB_TOKEN or GH_TOKEN is required to publish commit status');
  }

  const repository = process.env.GITHUB_REPOSITORY || 'dashpay/platform';
  if (!repository) {
    throw new Error('GITHUB_REPOSITORY is required to publish commit status');
  }

  const targetUrl = metadata.target_url
    || process.env.LATEST_CORE_TESTNET_TARGET_URL
    || (process.env.GITHUB_RUN_ID
      ? `${process.env.GITHUB_SERVER_URL || 'https://github.com'}/${repository}/actions/runs/${process.env.GITHUB_RUN_ID}`
      : undefined);

  const body = {
    state: status === 'sync_passed' ? 'success' : 'failure',
    context: STATUS_CONTEXT,
    description: statusDescription(status, metadata),
  };

  if (targetUrl) {
    body.target_url = targetUrl;
  }

  const response = await fetchWithTimeout(`https://api.github.com/repos/${repository}/statuses/${metadata.target_sha}`, {
    method: 'POST',
    headers: {
      Accept: 'application/vnd.github+json',
      Authorization: `Bearer ${githubToken}`,
      'Content-Type': 'application/json',
      'User-Agent': 'dash-platform-latest-core-testnet-sync',
    },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const errorBody = await response.text();
    throw new Error(`Unable to publish commit status: HTTP ${response.status} ${errorBody}`);
  }

  return 'published';
}

async function publishCommitStatus(status, metadata) {
  let lastError;
  for (let attempt = 1; attempt <= STATUS_PUBLISH_ATTEMPTS; attempt += 1) {
    try {
      return await publishCommitStatusOnce(status, metadata);
    } catch (error) {
      lastError = error;
      console.error(`Commit status publish attempt ${attempt} failed: ${error.message}`);
      if (attempt < STATUS_PUBLISH_ATTEMPTS) {
        await sleep(2 ** attempt * 1000);
      }
    }
  }

  throw lastError;
}

async function main() {
  ensureRunDir();

  const timeoutMinutes = parsePositiveMinutes(
    process.env.LATEST_CORE_TESTNET_PHASE_TIMEOUT_MINUTES,
    DEFAULT_PHASE_TIMEOUT_MINUTES,
    'LATEST_CORE_TESTNET_PHASE_TIMEOUT_MINUTES',
  );
  const targetSha = resolveTargetSha();
  const metadata = {
    status: 'running',
    started_at: now(),
    target_sha: targetSha,
    platform_sha: targetSha,
    run_dir: runDir,
    run_id: process.env.GITHUB_RUN_ID || null,
    run_attempt: process.env.GITHUB_RUN_ATTEMPT || null,
  };

  writeMetadata(metadata);

  let finalStatus = 'sync_passed';

  try {
    await prepareWorkspace(metadata, timeoutMinutes);

    metadata.phase = 'resolve-core-version';
    writeMetadata(metadata);
    metadata.core_version = await resolveLatestCoreVersion(metadata);
    writeMetadata(metadata);

    const phaseEnv = {
      ...process.env,
      LATEST_CORE_VERSION: metadata.core_version,
      PLATFORM_SHA: metadata.platform_sha,
      LATEST_CORE_TESTNET_SYNC_RUN_DIR: runDir,
    };
    delete phaseEnv.GITHUB_TOKEN;
    delete phaseEnv.GH_TOKEN;

    metadata.phase = 'core-sync';
    writeMetadata(metadata);
    await runCommand(
      'core-sync',
      process.env.LATEST_CORE_TESTNET_CORE_SYNC_COMMAND,
      phaseEnv,
      timeoutMinutes,
    );

    metadata.phase = 'platform-build';
    writeMetadata(metadata);
    await runCommand(
      'platform-build',
      process.env.LATEST_CORE_TESTNET_PLATFORM_BUILD_COMMAND,
      phaseEnv,
      timeoutMinutes,
    );

    metadata.phase = 'platform-sync';
    writeMetadata(metadata);
    await runCommand(
      'platform-sync',
      process.env.LATEST_CORE_TESTNET_PLATFORM_SYNC_COMMAND,
      phaseEnv,
      timeoutMinutes,
    );

    metadata.phase = 'complete';
  } catch (error) {
    metadata.failure_phase = metadata.phase || 'resolve-core-version';
    metadata.failure_summary = error.message;
    finalStatus = statusForFailurePhase(metadata.failure_phase);
    console.error(error.stack || error.message);
  } finally {
    metadata.status = finalStatus;
    metadata.completed_at = now();
    writeMetadata(metadata);
    try {
      const publishResult = await publishCommitStatus(finalStatus, metadata);
      if (publishResult === 'skipped') {
        metadata.status_publish_skipped_at = now();
      } else {
        metadata.status_published_at = now();
      }
      delete metadata.status_publish_error;
      delete metadata.status_publish_failed_at;
    } catch (error) {
      metadata.status_publish_error = error.message;
      metadata.status_publish_failed_at = now();
      writeMetadata(metadata);
      throw error;
    }
    writeMetadata(metadata);
  }

  console.log(`${STATUS_CONTEXT}: ${STATUS_LABELS[finalStatus]}`);

  if (finalStatus !== 'sync_passed') {
    process.exitCode = 1;
  }
}

main().catch((error) => {
  console.error(error.stack || error.message);
  process.exitCode = 1;
});
