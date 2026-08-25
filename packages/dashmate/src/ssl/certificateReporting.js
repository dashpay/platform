import renderCertificateGuidance from './renderCertificateGuidance.js';

/**
 * Everything the certificate check needs to say to an operator, kept out of the
 * update command itself.
 *
 * The certificate work is a mitigation that rides along with `dashmate update`
 * rather than part of updating a node, so it lives here and the command calls
 * into it. Removing the mitigation later should be deleting this file and its
 * call sites, not unpicking it from the update flow.
 */

/**
 * One machine-readable line about the certificate, on stderr.
 *
 * Kept off stdout because that stream is the command's own output and, under
 * JSON format, has to stay exactly one parseable document.
 *
 * @param {Object} verdict
 * @param {Config} config
 * @param {Object} [extra] - merged in, for fields only one caller has
 */
export function writeDiagnostics(verdict, config, extra = {}) {
  process.stderr.write(`${JSON.stringify({
    status: verdict.status,
    reasons: verdict.reasons.map(({ code }) => code),
    warnings: verdict.warnings.map(({ code }) => code),
    // What could not be established is as decisive as what failed: a check
    // that never ran is invisible to an unattended operator otherwise.
    skipped: verdict.skipped ?? [],
    provider: verdict.provider,
    config: config.getName(),
    expiresAt: verdict.installed ? verdict.installed.validTo.toISOString() : null,
    ...extra,
  })}\n`);
}

/**
 * The remediation an operator reads when the certificate did not pass.
 *
 * @param {Object} options
 * @param {Config} options.config
 * @param {Object} options.verdict
 * @param {Object} options.dockerCompose
 * @param {Object|null} options.pull
 * @param {boolean} [options.obtainAttemptFailed]
 * @return {Promise<void>}
 */
export async function reportUnresolved({
  config,
  verdict,
  dockerCompose,
  pull,
  obtainAttemptFailed = false,
}) {
  // Left null when it cannot be established. Docker being unavailable, or this
  // caller not being permitted to ask it, says nothing about whether the node
  // is up - and the guidance says nothing about it either rather than
  // defaulting to stopped, which would tell an operator with a running node the
  // opposite of the truth.
  let isNodeRunning = null;
  try {
    isNodeRunning = await dockerCompose.isServiceRunning(config, 'gateway');
  } catch {
    // Says nothing about the certificate either, so the verdict stands.
  }

  process.stderr.write(renderCertificateGuidance({
    config,
    verdict,
    isNodeRunning,
    pull,
    obtainAttemptFailed,
  }));
}
