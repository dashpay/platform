import fs from 'fs';

/**
 * Check whether the gateway certificate pair is the exact pair produced by lego.
 * File existence alone cannot distinguish a completed install from a partial
 * replacement, which would otherwise be treated as valid until certificate expiry.
 *
 * @param {string} legoCertificatePath
 * @param {string} legoKeyPath
 * @param {string} gatewayCertificatePath
 * @param {string} gatewayKeyPath
 * @return {boolean}
 */
export default function isCertificatePairInstalled(
  legoCertificatePath,
  legoKeyPath,
  gatewayCertificatePath,
  gatewayKeyPath,
) {
  try {
    return fs.readFileSync(legoCertificatePath).equals(fs.readFileSync(gatewayCertificatePath))
      && fs.readFileSync(legoKeyPath).equals(fs.readFileSync(gatewayKeyPath));
  } catch {
    return false;
  }
}
