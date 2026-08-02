#!/usr/bin/env node

/**
 * Validate the gateway's Envoy configuration against one or more Envoy images.
 *
 * The gateway config is rendered from `templates/platform/gateway/envoy.yaml.dot`,
 * which branches on the TLS provider, the rate limiter, metrics/admin and the
 * access-log settings. This script renders the template for a matrix of config
 * variants covering every branch and feeds each rendering to Envoy's built-in
 * config validator (`envoy --mode validate`) inside the requested image.
 *
 * The validator builds the whole config graph — listeners, filter chains,
 * clusters, TLS contexts, the overload manager — and then exits, so it reports
 * both hard errors (unknown/removed fields, type mismatches) and warnings about
 * fields that are deprecated in that Envoy version. That makes it the cheapest
 * gate to run before bumping the pinned gateway image.
 *
 * Usage:
 *   yarn node scripts/validate-envoy-config.js                      # pinned version
 *   yarn node scripts/validate-envoy-config.js v1.39.0              # candidate
 *   yarn node scripts/validate-envoy-config.js v1.35.11 v1.39.0     # side by side
 *   yarn node scripts/validate-envoy-config.js dashpay/envoy:1.39.0-impr.1
 *
 * Options:
 *   --image=<repo>     image repository for bare tags (default: envoyproxy/envoy)
 *   --variant=<name>   only run the named variant (repeatable)
 *   --out=<dir>        keep renderings, logs and results.json in <dir>
 *   --smoke            additionally boot the gateway against the pinned rate
 *                      limiter and assert the over-limit replies (see below)
 *   --list             print the variant matrix and exit
 *
 * With no image reference the baseline is taken from the image pinned in
 * `configs/defaults/getBaseConfigFactory.js`, so it tracks the pin.
 *
 * `--mode validate` proves the config loads, but two things about this gateway
 * only show up at runtime: the rate limit service wire protocol (Envoy speaks
 * RLS v3 to the pinned `envoyproxy/ratelimit` build) and the `local_reply_config`
 * mapper that turns the grpc-web over-limit reply into `grpc-status: 8`, which
 * depends on how Envoy orders local replies through encoder filters. `--smoke`
 * covers both by running the real thing on a throwaway Docker network.
 *
 * Exits non-zero when any validation or smoke check fails.
 */

import { spawnSync } from 'child_process';
import fs from 'fs';
import os from 'os';
import path from 'path';

import getBaseConfigFactory from '../configs/defaults/getBaseConfigFactory.js';
import HomeDir from '../src/config/HomeDir.js';
import renderServiceTemplatesFactory from '../src/templates/renderServiceTemplatesFactory.js';
import renderTemplateFactory from '../src/templates/renderTemplateFactory.js';

const DEFAULT_IMAGE_REPO = 'envoyproxy/envoy';
const ENVOY_CONFIG_TEMPLATE = 'platform/gateway/envoy.yaml';
const RATE_LIMITER_CONFIG_TEMPLATE = 'platform/gateway/rate_limiter/rate_limiter.yaml';
const DOCKER_TIMEOUT_MS = 180000;

// Same storage image docker-compose.rate_limiter.yml uses for the rate limiter.
const REDIS_IMAGE = 'redis:alpine';
// The smoke checks exercise the shipped defaults, where the rate limiter is on.
const SMOKE_VARIANT = 'default';
const SMOKE_NETWORK = 'dashmate-envoy-validate';
const SMOKE_CONTAINERS = {
  redis: 'dashmate-envoy-validate-redis',
  rateLimiter: 'dashmate-envoy-validate-rate-limiter',
  gateway: 'dashmate-envoy-validate-gateway',
};

/**
 * Config variants covering every branch of envoy.yaml.dot.
 *
 * Each variant mutates a fresh copy of the default config, so the renderings
 * differ only in the settings named by the variant.
 */
const VARIANTS = [
  {
    name: 'default',
    description: 'dashmate defaults: rate limiter on, stdout text access log, no metrics/admin',
    apply: () => {},
  },
  {
    name: 'ssl-self-signed',
    description: 'self-signed TLS: tls_inspector listener filter + raw_buffer/tls filter chains',
    apply: (config) => {
      config.set('platform.gateway.ssl.enabled', true);
      config.set('platform.gateway.ssl.provider', 'self-signed');
    },
  },
  {
    name: 'metrics-and-admin',
    description: 'Prometheus listener + admin cluster, admin bound to 0.0.0.0',
    apply: (config) => {
      config.set('platform.gateway.metrics.enabled', true);
      config.set('platform.gateway.admin.enabled', true);
    },
  },
  {
    name: 'metrics-only',
    description: 'Prometheus listener with the admin interface kept on loopback',
    apply: (config) => {
      config.set('platform.gateway.metrics.enabled', true);
      config.set('platform.gateway.admin.enabled', false);
    },
  },
  {
    name: 'no-rate-limiter',
    description: 'rate limiter off: no ratelimit filter, cluster, rate_limits or local_reply_config',
    apply: (config) => {
      config.set('platform.gateway.rateLimiter.enabled', false);
    },
  },
  {
    name: 'access-log-json',
    description: 'JSON access logs, default and custom field dictionaries',
    apply: (config) => {
      config.set('platform.gateway.log.accessLogs', [
        { type: 'stdout', format: 'json', template: null },
        { type: 'stderr', format: 'json', template: { start: '%START_TIME%', code: '%RESPONSE_CODE%' } },
      ]);
    },
  },
  {
    name: 'access-log-file',
    description: 'file access logs, text and JSON, alongside a custom text template',
    apply: (config) => {
      config.set('platform.gateway.log.accessLogs', [
        { type: 'file', format: 'text', path: '/var/log/dashmate/gateway/access.log', template: null },
        {
          type: 'file',
          format: 'json',
          path: '/var/log/dashmate/gateway/access_json.log',
          template: { start: '%START_TIME%', flags: '%RESPONSE_FLAGS%' },
        },
        { type: 'stdout', format: 'text', template: '[%START_TIME%] %RESPONSE_CODE%' },
      ]);
    },
  },
  {
    name: 'all-features',
    description: 'every optional feature at once: self-signed TLS, metrics, admin, rate limiter, JSON logs',
    apply: (config) => {
      config.set('platform.gateway.ssl.enabled', true);
      config.set('platform.gateway.ssl.provider', 'self-signed');
      config.set('platform.gateway.metrics.enabled', true);
      config.set('platform.gateway.admin.enabled', true);
      config.set('platform.gateway.rateLimiter.enabled', true);
      config.set('platform.gateway.log.accessLogs', [
        { type: 'stdout', format: 'json', template: null },
      ]);
    },
  },
];

/**
 * @param {string[]} argv
 * @return {{imageRepo: string, refs: string[], variants: string[], out: string|null,
 *   list: boolean, smoke: boolean}}
 */
function parseArgs(argv) {
  const options = {
    imageRepo: DEFAULT_IMAGE_REPO, refs: [], variants: [], out: null, list: false, smoke: false,
  };

  for (const arg of argv) {
    if (arg === '--list') {
      options.list = true;
    } else if (arg === '--smoke') {
      options.smoke = true;
    } else if (arg.startsWith('--image=')) {
      options.imageRepo = arg.slice('--image='.length);
    } else if (arg.startsWith('--variant=')) {
      options.variants.push(arg.slice('--variant='.length));
    } else if (arg.startsWith('--out=')) {
      options.out = path.resolve(arg.slice('--out='.length));
    } else if (arg.startsWith('-')) {
      throw new Error(`Unknown option ${arg}`);
    } else {
      options.refs.push(arg);
    }
  }

  return options;
}

/**
 * The pinned gateway image is a custom build of a stock Envoy release
 * (`dashpay/envoy:1.39.0-impr.1` wraps `envoyproxy/envoy:v1.39.0`), so the
 * baseline tag is the pinned version without the build suffix.
 *
 * @param {Config} config
 * @return {string} full image reference
 */
function resolveBaselineRef(config) {
  const pinnedImage = config.get('platform.gateway.docker.image');
  const pinnedTag = pinnedImage.slice(pinnedImage.lastIndexOf(':') + 1);
  const version = pinnedTag.replace(/-.*$/, '');

  return `${DEFAULT_IMAGE_REPO}:v${version}`;
}

/**
 * @param {string} ref - either `tag` or `repo/name:tag`
 * @param {string} imageRepo
 * @return {string}
 */
function resolveRef(ref, imageRepo) {
  return ref.includes(':') ? ref : `${imageRepo}:${ref}`;
}

/**
 * Certificate material for the TLS transport socket. The validator loads the
 * files named by the config, so they have to exist and hold a usable keypair.
 *
 * @param {string} dir
 */
function generateTlsMaterial(dir) {
  fs.mkdirSync(dir, { recursive: true });

  const result = spawnSync('openssl', [
    'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-days', '1',
    '-subj', '/CN=dashmate-envoy-validate',
    '-keyout', path.join(dir, 'private.key'),
    '-out', path.join(dir, 'bundle.crt'),
  ], { encoding: 'utf8' });

  if (result.status !== 0) {
    throw new Error(`openssl failed to generate TLS material: ${result.stderr || result.error}`);
  }
}

/**
 * @param {{name: string, apply: Function}} variant
 * @return {{config: Config, renderedConfigs: Object<string,string>}}
 */
function renderVariant(variant) {
  const getBaseConfig = getBaseConfigFactory(HomeDir.createTemp());
  const config = getBaseConfig();

  variant.apply(config);

  const renderServiceTemplates = renderServiceTemplatesFactory(renderTemplateFactory());

  return { config, renderedConfigs: renderServiceTemplates(config) };
}

/**
 * Pull the image up front so its progress output stays out of the validator log.
 *
 * @param {string} imageRef
 */
function ensureImage(imageRef) {
  const inspect = spawnSync('docker', ['image', 'inspect', imageRef], { encoding: 'utf8' });

  if (inspect.status === 0) {
    return;
  }

  const pull = spawnSync('docker', ['pull', '--quiet', imageRef], {
    encoding: 'utf8',
    stdio: ['ignore', 'inherit', 'inherit'],
    timeout: DOCKER_TIMEOUT_MS,
  });

  if (pull.status !== 0) {
    throw new Error(`Failed to pull ${imageRef}`);
  }
}

/**
 * The config file is mounted as a single file, exactly as docker-compose.yml
 * mounts it, so that the rest of /etc/envoy in the image stays visible — the
 * custom `dashpay/envoy` image keeps its hot-restart supervisor there.
 *
 * The entrypoint is overridden for the same reason: the custom image's
 * entrypoint is the supervisor, which does not forward Envoy's own flags.
 *
 * @param {string} imageRef
 * @param {string} configPath - rendered envoy.yaml on the host
 * @param {string} tlsDir
 * @param {string} logDir - mounted at /var/log for file access logs
 * @return {{status: number, output: string}}
 */
function runValidator(imageRef, configPath, tlsDir, logDir) {
  const result = spawnSync('docker', [
    'run', '--rm',
    '--entrypoint', 'envoy',
    '-v', `${configPath}:/etc/envoy/envoy.yaml:ro`,
    '-v', `${path.join(tlsDir, 'bundle.crt')}:/etc/ssl/bundle.crt:ro`,
    '-v', `${path.join(tlsDir, 'private.key')}:/etc/ssl/private.key:ro`,
    '-v', `${logDir}:/var/log`,
    imageRef,
    '--mode', 'validate',
    '-c', '/etc/envoy/envoy.yaml',
  ], { encoding: 'utf8', timeout: DOCKER_TIMEOUT_MS });

  if (result.error) {
    return { status: -1, output: `docker run failed: ${result.error.message}` };
  }

  return {
    status: result.status,
    output: `${result.stdout || ''}${result.stderr || ''}`,
  };
}

/**
 * @param {string} output
 * @return {{deprecations: string[], warnings: string[], errors: string[]}}
 */
function classifyOutput(output) {
  const lines = output.split('\n').map((line) => line.trim()).filter(Boolean);

  return {
    deprecations: lines.filter((line) => /deprecat/i.test(line)),
    warnings: lines.filter((line) => /\[warning]/.test(line) && !/deprecat/i.test(line)),
    errors: lines.filter((line) => /\[(error|critical)]/.test(line)),
  };
}

/**
 * @param {string[]} args
 * @param {boolean} [check=true] - throw when docker exits non-zero
 * @return {string} trimmed stdout
 */
function docker(args, check = true) {
  const result = spawnSync('docker', args, { encoding: 'utf8', timeout: DOCKER_TIMEOUT_MS });

  if (check && result.status !== 0) {
    throw new Error(`docker ${args.join(' ')} failed: ${result.stderr || result.stdout}`);
  }

  return (result.stdout || '').trim();
}

/**
 * Envoy writes its log to both stdout and stderr, so both streams are collected.
 *
 * @param {string} name - container name
 * @return {string}
 */
function containerLogs(name) {
  const result = spawnSync('docker', ['logs', name], { encoding: 'utf8' });

  return `${result.stdout || ''}${result.stderr || ''}`;
}

/**
 * Block the calling thread — this script drives docker synchronously throughout.
 *
 * @param {number} ms
 */
function sleepSync(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

/**
 * @param {string} url
 * @param {number} times
 * @param {string[]} [headers=[]]
 * @return {string[]} HTTP status code per request
 */
function httpStatuses(url, times, headers = []) {
  const headerArgs = headers.flatMap((header) => ['-H', header]);
  const result = spawnSync('curl', [
    // The listener terminates TLS with the throwaway certificate generated above.
    '--insecure', '--silent', '--output', '/dev/null',
    '--write-out', '%{http_code}\n',
    ...headerArgs,
    ...Array(times).fill(url),
  ], { encoding: 'utf8', timeout: DOCKER_TIMEOUT_MS });

  return (result.stdout || '').split('\n').filter(Boolean);
}

/**
 * @param {string} url
 * @param {string[]} [headers=[]]
 * @return {string} response status line and headers
 */
function httpResponseHeaders(url, headers = []) {
  const headerArgs = headers.flatMap((header) => ['-H', header]);
  const result = spawnSync('curl', [
    '--insecure', '--silent', '--output', '/dev/null', '--dump-header', '-',
    ...headerArgs, url,
  ], { encoding: 'utf8', timeout: DOCKER_TIMEOUT_MS });

  return result.stdout || '';
}

function removeSmokeContainers() {
  docker(['rm', '--force', ...Object.values(SMOKE_CONTAINERS)], false);
  docker(['network', 'rm', SMOKE_NETWORK], false);
}

/**
 * Boot the gateway with the rate limiter behind it and assert the over-limit
 * behaviour browser and native gRPC clients depend on.
 *
 * The rate limiter environment mirrors docker-compose.rate_limiter.yml; keep the
 * two in sync when that file changes.
 *
 * @param {string} imageRef - Envoy image to boot
 * @param {Config} config - the config the passed renderings came from
 * @param {string} envoyConfigPath
 * @param {string} rateLimiterConfigPath
 * @param {string} tlsDir
 * @return {{name: string, ok: boolean, detail: string}[]}
 */
function runSmokeChecks(imageRef, config, envoyConfigPath, rateLimiterConfigPath, tlsDir) {
  const checks = [];
  const record = (name, ok, detail = '') => checks.push({ name, ok, detail });

  removeSmokeContainers();
  docker(['network', 'create', SMOKE_NETWORK]);

  try {
    // The upstream aliases only make the STRICT_DNS clusters resolvable, so that
    // an unexpected warning in the gateway log is a real finding.
    docker(['run', '--detach', '--name', SMOKE_CONTAINERS.redis,
      '--network', SMOKE_NETWORK,
      '--network-alias', 'gateway_rate_limiter_redis',
      '--network-alias', 'rs_dapi',
      '--network-alias', 'drive_abci',
      REDIS_IMAGE]);

    docker(['run', '--detach', '--name', SMOKE_CONTAINERS.rateLimiter,
      '--network', SMOKE_NETWORK,
      '--network-alias', 'gateway_rate_limiter',
      '-v', `${rateLimiterConfigPath}:/data/ratelimit/config/config.yaml:ro`,
      '-e', 'LOG_LEVEL=info', '-e', 'LOG_FORMAT=text',
      '-e', 'BACKEND_TYPE=redis', '-e', 'REDIS_SOCKET_TYPE=tcp',
      '-e', 'REDIS_URL=gateway_rate_limiter_redis:6379',
      '-e', 'RUNTIME_ROOT=/data', '-e', 'RUNTIME_SUBDIRECTORY=ratelimit',
      '-e', 'RUNTIME_WATCH_ROOT=false', '-e', 'DISABLE_STATS=true',
      '-e', 'CONFIG_TYPE=FILE', '-e', 'GRPC_PORT=8081',
      '-e', 'GRPC_MAX_CONNECTION_AGE=1h', '-e', 'GRPC_MAX_CONNECTION_AGE_GRACE=10m',
      '-e', 'LIMIT_RESPONSE_HEADERS_ENABLED=true',
      config.get('platform.gateway.rateLimiter.docker.image'),
      '/bin/ratelimit']);

    docker(['run', '--detach', '--name', SMOKE_CONTAINERS.gateway,
      '--network', SMOKE_NETWORK,
      '--publish', '0:10000',
      '-v', `${envoyConfigPath}:/etc/envoy/envoy.yaml:ro`,
      '-v', `${path.join(tlsDir, 'bundle.crt')}:/etc/ssl/bundle.crt:ro`,
      '-v', `${path.join(tlsDir, 'private.key')}:/etc/ssl/private.key:ro`,
      imageRef, '-c', '/etc/envoy/envoy.yaml']);

    const deadline = Date.now() + 60000;
    let gatewayLog = containerLogs(SMOKE_CONTAINERS.gateway);

    while (!gatewayLog.includes('starting workers') && Date.now() < deadline) {
      sleepSync(500);
      gatewayLog = containerLogs(SMOKE_CONTAINERS.gateway);
    }

    if (!gatewayLog.includes('starting workers')) {
      record('gateway starts', false, gatewayLog.split('\n').slice(-5).join(' | '));

      return checks;
    }

    record('gateway starts', true);

    const noisyLines = gatewayLog.split('\n')
      .filter((line) => /\[(warning|error|critical)]/.test(line));

    record('gateway log is free of warnings', noisyLines.length === 0, noisyLines.join(' | '));

    const port = docker(['port', SMOKE_CONTAINERS.gateway, '10000/tcp'])
      .split('\n')[0].split(':').pop();
    const url = `https://127.0.0.1:${port}/`;
    const grpcUrl = `https://127.0.0.1:${port}/org.dash.platform.dapi.v0.Platform/getIdentity`;

    // The limiter allows exactly requestsPerUnit requests per remote address,
    // and the request after that must be rejected.
    const requestsPerUnit = config.get('platform.gateway.rateLimiter.requestsPerUnit');
    const statuses = httpStatuses(url, requestsPerUnit);
    const allowed = statuses.filter((status) => status !== '429').length;

    record(
      `first ${requestsPerUnit} requests pass the limiter`,
      allowed === requestsPerUnit,
      `allowed ${allowed}/${requestsPerUnit}`,
    );

    const overLimit = httpStatuses(url, 1)[0];
    record(`request ${requestsPerUnit + 1} is rate limited`, overLimit === '429', `got ${overLimit}`);

    // grpc-web over-limit reply: HTTP 200 carrying grpc-status 8 and the reset
    // hint in the same header map, which is what browser clients read.
    const grpcWebReply = httpResponseHeaders(grpcUrl, [
      'x-grpc-web: 1', 'content-type: application/grpc-web+proto',
    ]);

    record(
      'grpc-web over-limit reply is 200 + grpc-status 8 + ratelimit-reset',
      /HTTP\/2 200/.test(grpcWebReply)
        && /^grpc-status: 8/im.test(grpcWebReply)
        && /^ratelimit-reset:/im.test(grpcWebReply),
      grpcWebReply.split('\n').map((line) => line.trim()).filter(Boolean).join(' | '),
    );

    const grpcReply = httpResponseHeaders(grpcUrl, ['content-type: application/grpc']);

    record(
      'native gRPC over-limit reply is 200 + grpc-status 8',
      /HTTP\/2 200/.test(grpcReply) && /^grpc-status: 8/im.test(grpcReply),
      grpcReply.split('\n').map((line) => line.trim()).filter(Boolean).join(' | '),
    );

    return checks;
  } finally {
    removeSmokeContainers();
  }
}

function main() {
  const options = parseArgs(process.argv.slice(2));

  const variants = options.variants.length > 0
    ? VARIANTS.filter(({ name }) => options.variants.includes(name))
    : VARIANTS;

  if (variants.length === 0) {
    throw new Error(`No variant matched ${options.variants.join(', ')}`);
  }

  if (options.list) {
    for (const variant of VARIANTS) {
      process.stdout.write(`${variant.name.padEnd(20)} ${variant.description}\n`);
    }

    return 0;
  }

  const outDir = options.out ?? fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-envoy-validate-'));
  const renderedDir = path.join(outDir, 'rendered');
  const logsDir = path.join(outDir, 'logs');
  const tlsDir = path.join(outDir, 'tls');
  // Envoy runs as an unprivileged user inside the image; the file access-log
  // variant needs to create log files under the mounted /var/log.
  const logDir = path.join(outDir, 'var-log');

  for (const dir of [renderedDir, logsDir, logDir]) {
    fs.mkdirSync(dir, { recursive: true });
  }
  fs.chmodSync(logDir, 0o777);

  generateTlsMaterial(tlsDir);

  // Render first so a template error is reported before any image is pulled.
  const renderings = variants.map((variant) => {
    const { config, renderedConfigs } = renderVariant(variant);
    const configPath = path.join(renderedDir, `envoy.${variant.name}.yaml`);

    fs.writeFileSync(configPath, renderedConfigs[ENVOY_CONFIG_TEMPLATE]);

    if (config.get('platform.gateway.rateLimiter.enabled')) {
      fs.writeFileSync(
        path.join(renderedDir, `rate_limiter.${variant.name}.yaml`),
        renderedConfigs[RATE_LIMITER_CONFIG_TEMPLATE],
      );
    }

    return { variant, configPath };
  });

  const refs = (options.refs.length > 0 ? options.refs : [null])
    .map((ref) => (ref === null
      ? resolveBaselineRef(getBaseConfigFactory(HomeDir.createTemp())())
      : resolveRef(ref, options.imageRepo)));

  process.stdout.write(`Work directory: ${outDir}\n`);
  process.stdout.write(`Images: ${refs.join(', ')}\n\n`);

  const results = [];
  let failed = 0;

  for (const ref of refs) {
    ensureImage(ref);

    process.stdout.write(`=== ${ref} ===\n`);

    for (const { variant, configPath } of renderings) {
      const { status, output } = runValidator(ref, configPath, tlsDir, logDir);
      const classified = classifyOutput(output);
      const ok = status === 0 && /configuration '.*' OK/.test(output);

      const logName = `${ref.replace(/[^\w.-]/g, '_')}__${variant.name}.log`;
      fs.writeFileSync(path.join(logsDir, logName), output);

      results.push({
        image: ref,
        variant: variant.name,
        ok,
        exitCode: status,
        ...classified,
        log: path.join(logsDir, logName),
      });

      if (!ok) {
        failed += 1;
      }

      const notes = [
        classified.deprecations.length > 0 ? `${classified.deprecations.length} deprecation` : null,
        classified.warnings.length > 0 ? `${classified.warnings.length} warning` : null,
        classified.errors.length > 0 ? `${classified.errors.length} error` : null,
      ].filter(Boolean).join(', ');

      process.stdout.write(`${ok ? 'OK    ' : 'FAILED'}  ${variant.name.padEnd(20)}${notes}\n`);

      for (const line of [...classified.errors, ...classified.deprecations]) {
        process.stdout.write(`          ${line}\n`);
      }
    }

    process.stdout.write('\n');
  }

  const smokeResults = [];

  if (options.smoke) {
    const smokeVariant = VARIANTS.find(({ name }) => name === SMOKE_VARIANT);
    const { config, renderedConfigs } = renderVariant(smokeVariant);
    const envoyConfigPath = path.join(renderedDir, `envoy.${SMOKE_VARIANT}.yaml`);
    const rateLimiterConfigPath = path.join(renderedDir, `rate_limiter.${SMOKE_VARIANT}.yaml`);

    fs.writeFileSync(envoyConfigPath, renderedConfigs[ENVOY_CONFIG_TEMPLATE]);
    fs.writeFileSync(rateLimiterConfigPath, renderedConfigs[RATE_LIMITER_CONFIG_TEMPLATE]);

    ensureImage(config.get('platform.gateway.rateLimiter.docker.image'));
    ensureImage(REDIS_IMAGE);

    for (const ref of refs) {
      process.stdout.write(`=== ${ref} — smoke (${SMOKE_VARIANT} variant) ===\n`);

      const checks = runSmokeChecks(
        ref,
        config,
        envoyConfigPath,
        rateLimiterConfigPath,
        tlsDir,
      );

      for (const check of checks) {
        process.stdout.write(`${check.ok ? 'OK    ' : 'FAILED'}  ${check.name}\n`);

        if (!check.ok && check.detail) {
          process.stdout.write(`          ${check.detail}\n`);
        }
      }

      failed += checks.filter(({ ok }) => !ok).length;
      smokeResults.push({ image: ref, checks });
      process.stdout.write('\n');
    }
  }

  fs.writeFileSync(
    path.join(outDir, 'results.json'),
    `${JSON.stringify({ images: refs, results, smokeResults }, null, 2)}\n`,
  );

  const total = results.length + smokeResults.reduce((sum, { checks }) => sum + checks.length, 0);

  process.stdout.write(`${total - failed}/${total} checks passed\n`);
  process.stdout.write(`Logs and renderings: ${outDir}\n`);

  return failed === 0 ? 0 : 1;
}

process.exit(main());
