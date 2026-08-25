import crypto from 'node:crypto';
import fs from 'node:fs';

/**
 * Extract IP addresses from the OpenSSL rendering of a subject alternative name extension.
 *
 * The value is a single string such as "IP Address:1.2.3.4, DNS:example.com", so entries are
 * split out rather than substring-matched: searching the raw string for "1.2.3.4" would also
 * match inside "11.2.3.44" and inside a DNS entry.
 *
 * @param {string|undefined} subjectAltName
 * @return {string[]}
 */
export function parseIpAddresses(subjectAltName) {
  if (!subjectAltName) {
    return [];
  }

  return subjectAltName
    .split(',')
    .map((entry) => entry.trim())
    .filter((entry) => entry.startsWith('IP Address:'))
    .map((entry) => entry.slice('IP Address:'.length));
}

/**
 * Read the server certificate from a PEM bundle.
 *
 * The server certificate is expected first, but an operator supplying their own bundle can
 * order it the other way round, so the first block is only accepted when it is not a CA.
 * Comparing a served certificate against an intermediate would report a permanent mismatch.
 *
 * The fingerprint is the same uppercase colon-separated SHA-256 that a TLS peer certificate
 * reports, so the two can be compared directly.
 *
 * @param {string} filePath
 * @return {{fingerprint256: string, validFrom: Date, validTo: Date, subject: string,
 *   issuer: string, ipAddresses: string[]}|null} null when the file is missing or unparseable
 */
export default function readCertificateBundle(filePath) {
  let pem;

  try {
    pem = fs.readFileSync(filePath, 'utf8');
  } catch {
    return null;
  }

  const blocks = pem.match(/-----BEGIN CERTIFICATE-----[\s\S]*?-----END CERTIFICATE-----/g) ?? [];

  for (const block of blocks) {
    let certificate;

    try {
      certificate = new crypto.X509Certificate(block);
    } catch {
      // Skip a block we cannot parse rather than failing the whole bundle
      continue;
    }

    if (certificate.ca) {
      continue;
    }

    return {
      fingerprint256: certificate.fingerprint256,
      validFrom: new Date(certificate.validFrom),
      validTo: new Date(certificate.validTo),
      subject: certificate.subject,
      issuer: certificate.issuer,
      ipAddresses: parseIpAddresses(certificate.subjectAltName),
    };
  }

  return null;
}
