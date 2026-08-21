import fs from 'node:fs';
import path from 'node:path';

import { SSL_PROVIDERS } from '../constants.js';
import { parseIpAddresses } from './readCertificateBundle.js';
import isCertificatePairInstalled from './letsencrypt/isCertificatePairInstalled.js';
import selectLeafCertificate, { LEAF_SELECTION_ERRORS } from './selectLeafCertificate.js';

export const CERTIFICATE_STATUS = {
  // Deliberately not VALID. This function runs a fixed list of local checks; it
  // does not validate the chain to a public root, does not check revocation and
  // never opens a connection, so it cannot establish that a certificate is
  // trusted, usable, or that the node is reachable. Passing means exactly that
  // the checks below found no problem.
  CHECKS_PASSED: 'CHECKS_PASSED',
  WARN: 'WARN',
  INVALID: 'INVALID',
};

export const CERTIFICATE_REASONS = {
  BUNDLE_MISSING: 'BUNDLE_MISSING',
  NOT_YET_VALID: 'NOT_YET_VALID',
  BUNDLE_UNREADABLE: 'BUNDLE_UNREADABLE',
  BUNDLE_ORDER: 'BUNDLE_ORDER',
  KEY_MISSING: 'KEY_MISSING',
  KEY_UNUSABLE: 'KEY_UNUSABLE',
  KEY_MISMATCH: 'KEY_MISMATCH',
  EXPIRED: 'EXPIRED',
  EXPIRING_SOON: 'EXPIRING_SOON',
  SELF_SIGNED: 'SELF_SIGNED',
  IP_MISMATCH: 'IP_MISMATCH',
  SWITCH_INCOMPLETE: 'SWITCH_INCOMPLETE',
  PROVIDER_MISMATCH: 'PROVIDER_MISMATCH',
  SSL_UNMANAGED: 'SSL_UNMANAGED',
  SSL_DISABLED: 'SSL_DISABLED',
};

const DAY_MS = 24 * 60 * 60 * 1000;

/**
 * A certificate this close to expiry has already passed the point where the
 * helper renews it, so anything further out is a window renewal clears by
 * itself.
 */
const EXPIRING_SOON_DAYS = 1;

/**
 * Which provider the issuer of an installed leaf points at.
 *
 * Only issuers dashmate can obtain from are recognised. An unrecognised issuer
 * returns null rather than a guess, so a certificate from a paid CA is never
 * reported as disagreeing with anything.
 *
 * @param {X509Certificate} leaf
 * @param {boolean} isSelfSigned
 * @return {string|null}
 */
function identifyIssuer(leaf, isSelfSigned) {
  if (isSelfSigned) {
    return SSL_PROVIDERS.SELF_SIGNED;
  }

  const issuer = (leaf.issuer ?? '').toLowerCase();

  if (issuer.includes("let's encrypt") || issuer.includes('letsencrypt') || issuer.includes('isrg')) {
    return SSL_PROVIDERS.LETSENCRYPT;
  }

  if (issuer.includes('zerossl') || issuer.includes('sectigo')) {
    return SSL_PROVIDERS.ZEROSSL;
  }

  return null;
}

/**
 * @param {HomeDir} homeDir
 * @return {checkGatewayCertificate}
 */
export default function checkGatewayCertificateFactory(homeDir) {
  /**
   * Judge the certificate bundle installed for the gateway.
   *
   * The verdict is derived from the files the gateway loads rather than from
   * the configured provider, so it is provider-independent and works offline.
   * Switching on the provider instead would mean calling ZeroSSL's REST API -
   * which fails for exactly the free-tier operators this check exists for - and
   * would miss a self-signed bundle installed under any other provider.
   *
   * No side effects, no prompts, no network.
   *
   * @typedef {checkGatewayCertificate}
   * @param {Config} config
   * @return {{status: string, reasons: Object[], warnings: Object[], skipped: string[],
   *   provider: string, installed: Object|null, expiresInDays: number|null,
   *   bundleFilePath: string, privateKeyFilePath: string}}
   */
  function checkGatewayCertificate(config) {
    const provider = config.get('platform.gateway.ssl.provider');
    const externalIp = config.get('externalIp');

    const sslDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
    const bundleFilePath = path.join(sslDir, 'bundle.crt');
    const privateKeyFilePath = path.join(sslDir, 'private.key');

    const reasons = [];
    const warnings = [];
    const skipped = [];

    /**
     * @param {Object|null} installed
     * @param {number|null} expiresInDays
     * @return {Object}
     */
    const verdict = (installed = null, expiresInDays = null) => {
      // A certificate the checks cleared is not a certificate dashmate is
      // managing, so an unmanaged node is reported either way - as a warning on
      // its own, and as part of the failure when something else is wrong too.
      if (config.get('platform.gateway.ssl.enabled') === false) {
        (reasons.length > 0 ? reasons : warnings).push(reasons.length > 0
          ? {
            code: CERTIFICATE_REASONS.SSL_DISABLED,
            message: 'Dashmate is not managing this certificate, so nothing will renew it',
          }
          : {
            code: CERTIFICATE_REASONS.SSL_UNMANAGED,
            message: 'Dashmate is not managing this certificate. It will not be renewed automatically',
          });
      }

      let status = CERTIFICATE_STATUS.CHECKS_PASSED;

      if (reasons.length > 0) {
        status = CERTIFICATE_STATUS.INVALID;
      } else if (warnings.length > 0) {
        status = CERTIFICATE_STATUS.WARN;
      }

      return {
        status,
        reasons,
        warnings,
        skipped,
        provider,
        installed,
        expiresInDays,
        bundleFilePath,
        privateKeyFilePath,
      };
    };

    let bundlePem;
    try {
      bundlePem = fs.readFileSync(bundleFilePath, 'utf8');
    } catch (e) {
      reasons.push(e.code === 'ENOENT'
        ? {
          code: CERTIFICATE_REASONS.BUNDLE_MISSING,
          message: `dashmate could not find the certificate bundle at ${bundleFilePath}`,
        }
        : {
          code: CERTIFICATE_REASONS.BUNDLE_UNREADABLE,
          message: `dashmate could not read the certificate bundle at ${bundleFilePath}: ${e.message}`,
        });
    }

    let privateKeyPem;
    try {
      privateKeyPem = fs.readFileSync(privateKeyFilePath, 'utf8');
    } catch (e) {
      reasons.push(e.code === 'ENOENT'
        ? {
          code: CERTIFICATE_REASONS.KEY_MISSING,
          message: `dashmate could not find the private key at ${privateKeyFilePath}`,
        }
        : {
          code: CERTIFICATE_REASONS.KEY_UNUSABLE,
          message: `dashmate could not read the private key at ${privateKeyFilePath}: ${e.message}`,
        });
    }

    if (bundlePem === undefined || privateKeyPem === undefined) {
      return verdict();
    }

    // The gateway is handed the key file with no password or passphrase field
    // anywhere in its configuration, so a key dashmate cannot load is a key the
    // gateway cannot load either and the node serves no TLS at all. Warning
    // here would pass a node that is already dark.
    const { leaf, error, detail } = selectLeafCertificate(bundlePem, privateKeyPem);

    if (error === LEAF_SELECTION_ERRORS.KEY_UNUSABLE) {
      reasons.push({
        code: CERTIFICATE_REASONS.KEY_UNUSABLE,
        message: `dashmate could not read the private key at ${privateKeyFilePath}: ${detail}`,
      });

      return verdict();
    }

    if (error === LEAF_SELECTION_ERRORS.BUNDLE_UNREADABLE) {
      reasons.push({
        code: CERTIFICATE_REASONS.BUNDLE_UNREADABLE,
        message: `dashmate could not read ${bundleFilePath}: ${detail}`,
      });

      return verdict();
    }

    if (error === LEAF_SELECTION_ERRORS.BUNDLE_ORDER) {
      reasons.push({
        code: CERTIFICATE_REASONS.BUNDLE_ORDER,
        message: `The certificates in ${bundleFilePath} are in the wrong order: ${detail}`,
      });

      return verdict();
    }

    if (error === LEAF_SELECTION_ERRORS.KEY_MISMATCH) {
      reasons.push({
        code: CERTIFICATE_REASONS.KEY_MISMATCH,
        message: `No certificate in ${bundleFilePath} belongs to the private key`
          + ` at ${privateKeyFilePath}`,
      });

      return verdict();
    }

    // A certificate that verifies under its own public key is self-signed by
    // definition. Subject equal to issuer is only a naming convention: a
    // self-signed certificate may name any issuer it likes, and a private-CA
    // certificate can render the two identically.
    let isSelfSigned = false;
    try {
      isSelfSigned = leaf.verify(leaf.publicKey);
    } catch {
      isSelfSigned = false;
    }

    const validTo = new Date(leaf.validTo);
    const expiresInDays = (validTo.getTime() - Date.now()) / DAY_MS;

    const installed = {
      subject: leaf.subject,
      issuer: leaf.issuer,
      validFrom: new Date(leaf.validFrom),
      validTo,
      ipAddresses: parseIpAddresses(leaf.subjectAltName),
      fingerprint256: leaf.fingerprint256,
      selfSigned: isSelfSigned,
    };

    if (isSelfSigned) {
      const message = 'The installed certificate is self-signed. Self-signed TLS is not'
        + ' publicly trusted and standards-compliant clients will reject it';

      // Dashmate's own setup wizard offers self-signed to a mainnet evolution
      // fullnode, so blocking it unconditionally would break update for a
      // configuration dashmate created. Enforcement is scoped to registered
      // masternodes, which is the population the wizard's own rule scopes to.
      const isEnforced = config.get('core.masternode.enable') === true;

      (isEnforced ? reasons : warnings).push({
        code: CERTIFICATE_REASONS.SELF_SIGNED,
        message,
      });
    }

    // Unservable in exactly the way an expired certificate is - clients reject
    // it on the same field. Nothing is concluded about the clock here: a fast
    // local clock and a genuinely future start date look identical from disk,
    // so this says only that the certificate cannot be served as it stands.
    if (installed.validFrom.getTime() > Date.now()) {
      reasons.push({
        code: CERTIFICATE_REASONS.NOT_YET_VALID,
        message: 'The installed certificate is not valid until'
          + ` ${installed.validFrom.toISOString().slice(0, 10)}, so clients reject it`,
      });
    }

    if (expiresInDays <= 0) {
      reasons.push({
        code: CERTIFICATE_REASONS.EXPIRED,
        message: `The installed certificate expired on ${validTo.toISOString().slice(0, 10)}`
          + ` - ${Math.floor(-expiresInDays)} days ago`,
      });
    } else if (expiresInDays < EXPIRING_SOON_DAYS) {
      warnings.push({
        code: CERTIFICATE_REASONS.EXPIRING_SOON,
        message: `The installed certificate expires on ${validTo.toISOString().slice(0, 10)}`
          + ` - in less than ${EXPIRING_SOON_DAYS} day`,
      });
    }

    if (!externalIp) {
      skipped.push('IDENTITY');
    } else {
      // Only the subject alternative name counts. Node's own
      // tls.checkServerIdentity does not consult the common name for an IP
      // identifier and neither do browsers, so a certificate carrying the
      // address only in its subject is one every client rejects - accepting it
      // would pass a node that nothing can connect to.
      if (!installed.ipAddresses.includes(externalIp)) {
        const detail = installed.ipAddresses.length > 0
          ? `it names ${installed.ipAddresses.join(', ')} instead`
          : 'it carries no IP address at all';

        reasons.push({
          code: CERTIFICATE_REASONS.IP_MISMATCH,
          message: "The installed certificate does not carry this node's address"
            + ` ${externalIp} in its subject alternative name - ${detail}`,
        });
      }
    }

    const legoDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'lego');
    const isLegoPairInstalled = Boolean(externalIp) && isCertificatePairInstalled(
      path.join(legoDir, 'certificates', `${externalIp}.crt`),
      path.join(legoDir, 'certificates', `${externalIp}.key`),
      bundleFilePath,
      privateKeyFilePath,
    );

    if (isLegoPairInstalled && provider !== SSL_PROVIDERS.LETSENCRYPT) {
      // The pair is installed before the provider is written, so a kill between
      // the two leaves exactly this. Left as a warning the helper keeps
      // renewing the old provider while the installed certificate runs out and
      // the state never repairs itself.
      reasons.push({
        code: CERTIFICATE_REASONS.SWITCH_INCOMPLETE,
        message: "A Let's Encrypt certificate is installed for the gateway, but the"
          + ` configuration still names ${provider}. A switch was interrupted before it finished`,
      });
    } else if (provider !== SSL_PROVIDERS.FILE) {
      // A certificate the operator supplied themselves can come from any
      // authority, so its issuer says nothing about the configuration.
      const issuedBy = identifyIssuer(leaf, isSelfSigned);

      if (issuedBy !== null && issuedBy !== provider) {
        warnings.push({
          code: CERTIFICATE_REASONS.PROVIDER_MISMATCH,
          message: `The installed certificate was issued by ${issuedBy}, but the configuration`
            + ` names ${provider}`,
        });
      }
    }

    return verdict(installed, expiresInDays);
  }

  return checkGatewayCertificate;
}
