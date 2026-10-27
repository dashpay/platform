import tls from 'node:tls';

/**
 * How long the whole probe may take, matching the timeout used for the port checks.
 *
 * This is an absolute budget covering the TCP connect and the TLS handshake together. The
 * socket's own timeout option cannot serve as one: it is an inactivity timer that resets on
 * every byte received and does not close the socket, so a peer trickling data keeps a probe
 * alive indefinitely.
 */
export const PROBE_TIMEOUT_MS = 5000;

export const STATE = {
  SERVED: 'served',
  UNREACHABLE: 'unreachable',
  SKIPPED: 'skipped',
};

/**
 * Flatten a TLS peer certificate into plain values.
 *
 * The peer certificate must never be stored as it is: a chain that verifies ends at a
 * self-signed root whose issuerCertificate points back at itself, and that cycle makes both
 * JSON serialisation and the sample obfuscation pass fail. Its raw and pubkey fields are also
 * buffers that serialise into thousands of numbers.
 *
 * @param {Object} peerCertificate
 * @return {Object}
 */
function flattenCertificate(peerCertificate) {
  return {
    fingerprint256: peerCertificate.fingerprint256,
    validFrom: peerCertificate.valid_from,
    validTo: peerCertificate.valid_to,
    subject: peerCertificate.subject?.CN ?? null,
    issuer: peerCertificate.issuer?.CN ?? null,
    subjectAltName: peerCertificate.subjectaltname ?? null,
    serialNumber: peerCertificate.serialNumber ?? null,
  };
}

/**
 * Connect to the gateway and report the certificate it actually serves.
 *
 * Every other certificate check reads a file or asks the provider's API. A certificate that was
 * renewed on disk but never reached the gateway is indistinguishable from a healthy one to all
 * of them, so this opens a real connection and looks at what the gateway presents.
 *
 * @param {Object} options
 * @param {string} options.host - address the gateway listener is reachable on
 * @param {number} options.port
 * @param {string} options.externalIp - the address clients use, which the certificate must name
 * @param {number} [options.timeout]
 * @return {Promise<Object>} never rejects
 */
export default async function probeServedCertificate({
  host,
  port,
  externalIp,
  timeout = PROBE_TIMEOUT_MS,
}) {
  return new Promise((resolve) => {
    let settled = false;
    let deadline;

    const settle = (result) => {
      if (settled) {
        return;
      }

      settled = true;

      clearTimeout(deadline);

      resolve(result);
    };

    let socket;

    const fail = (reason) => {
      socket?.destroy();

      settle({
        state: STATE.UNREACHABLE,
        reason,
      });
    };

    deadline = setTimeout(() => fail('ETIMEDOUT'), timeout);

    try {
      socket = tls.connect({
        host,
        port,
        // A certificate identifying a node by IP address cannot be requested by name: SNI must
        // not carry an IP literal, and the gateway selects its filter chain without it.
        servername: undefined,
        // The handshake has to complete even when the certificate is expired or untrusted,
        // otherwise the probe learns nothing in the cases it exists for. Verification still
        // runs and its verdict is read from the socket below. Nothing here grants trust: the
        // result is reported, never used to authorise a connection.
        rejectUnauthorized: false,
        // Node would otherwise check the certificate against the address being dialled, which
        // is the local address the gateway happens to be reachable on rather than the one
        // clients use, and a correct certificate would fail that on every healthy node. The
        // check is done separately below, against the address that matters.
        checkServerIdentity: () => undefined,
      });
    } catch (e) {
      fail(e.code ?? 'CONNECT_FAILED');

      return;
    }

    // Stays attached for the socket's lifetime. A connection can fail after the certificate has
    // already been read, and settle() ignores anything that arrives once a result is decided.
    socket.on('error', (e) => fail(e.code ?? 'CONNECT_FAILED'));

    socket.on('timeout', () => fail('ETIMEDOUT'));

    socket.on('secureConnect', () => {
      const peerCertificate = socket.getPeerCertificate(true);

      // An absent peer certificate is reported as an empty object, which would otherwise be
      // taken for a served certificate with no fields and compared against the one on disk.
      if (!peerCertificate?.fingerprint256) {
        fail('NO_PEER_CERTIFICATE');

        return;
      }

      const { authorized, authorizationError } = socket;

      // Identity is checked separately from the chain because the socket reports only one
      // error: an expired certificate that also names the wrong address reports just the
      // expiry, so a single verdict would hide the second fault until the first was fixed.
      // Delegating to Node handles the common-name fallback and address normalisation that a
      // hand-rolled comparison against the alternative names gets wrong.
      const identityError = externalIp
        ? tls.checkServerIdentity(externalIp, peerCertificate)
        : undefined;

      socket.destroy();

      settle({
        state: STATE.SERVED,
        certificate: flattenCertificate(peerCertificate),
        chainVerified: authorized,
        chainError: authorized ? null : (authorizationError?.code ?? String(authorizationError)),
        identityVerified: externalIp ? identityError === undefined : null,
        identityError: identityError ? identityError.message : null,
      });
    });
  });
}
