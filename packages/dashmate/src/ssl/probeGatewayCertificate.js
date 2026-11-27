import probeServedCertificate, { PROBE_TIMEOUT_MS, STATE } from './probeServedCertificate.js';
import readCertificateBundle from './readCertificateBundle.js';
import wait from '../util/wait.js';

/**
 * How long a reloaded gateway may take to serve the bundle on disk. The
 * signal starts a new Envoy, which loads its configuration and certificate
 * before it takes over the listener from the old one.
 */
export const RELOAD_TIMEOUT_MS = 30000;

export const RELOAD_POLL_INTERVAL_MS = 1000;

/**
 * Connect to a node's gateway and compare the certificate it serves with the
 * bundle on disk
 *
 * The gateway's listener is reached at the address it is published on, so
 * this is for a process running on the node.
 *
 * @param {Config} config
 * @param {string} bundleFilePath
 * @param {Object} [options]
 * @param {number} [options.timeout]
 * @return {Promise<Object>} the probe's result, with the port and, when a
 *  certificate was served, `onDisk` and `matchesOnDisk`; never rejects
 */
export default async function probeGatewayCertificate(config, bundleFilePath, {
  timeout = PROBE_TIMEOUT_MS,
} = {}) {
  const listenerHost = config.get('platform.gateway.listeners.dapiAndDrive.host');
  const port = config.get('platform.gateway.listeners.dapiAndDrive.port');

  const result = await probeServedCertificate({
    host: listenerHost === '0.0.0.0' ? '127.0.0.1' : listenerHost,
    port,
    externalIp: config.get('externalIp'),
    timeout,
  });

  result.port = port;

  if (result.state === STATE.SERVED) {
    // Read beside the probe: renewal replaces the file and signals the
    // gateway moments apart, and a bundle read at another time could be from
    // the other side of a renewal.
    const onDisk = readCertificateBundle(bundleFilePath);

    result.onDisk = onDisk && {
      fingerprint256: onDisk.fingerprint256,
      validTo: onDisk.validTo.toUTCString(),
    };

    result.matchesOnDisk = onDisk
      ? onDisk.fingerprint256 === result.certificate.fingerprint256
      : null;
  }

  return result;
}

/**
 * Probe a gateway that was just signalled until it serves the bundle on disk,
 * or until the reload has had time to complete
 *
 * Right after the signal the previous Envoy still answers, with the
 * certificate it loaded, so one probe cannot tell a slow reload from one that
 * loaded another file.
 *
 * @param {Config} config
 * @param {string} bundleFilePath
 * @param {Object} [options]
 * @param {number} [options.timeout]
 * @param {number} [options.interval]
 * @param {function(Config, string, Object): Promise<Object>} [options.probe]
 * @return {Promise<Object>} the last probe's result
 */
export async function waitForGatewayToServeBundle(config, bundleFilePath, {
  timeout = RELOAD_TIMEOUT_MS,
  interval = RELOAD_POLL_INTERVAL_MS,
  probe = probeGatewayCertificate,
} = {}) {
  const deadline = Date.now() + timeout;

  for (;;) {
    // A probe that would end past the deadline is cut short to it.
    const result = await probe(config, bundleFilePath, {
      timeout: Math.min(PROBE_TIMEOUT_MS, Math.max(deadline - Date.now(), 0)),
    });

    if (result.matchesOnDisk === true || Date.now() + interval >= deadline) {
      return result;
    }

    await wait(interval);
  }
}
