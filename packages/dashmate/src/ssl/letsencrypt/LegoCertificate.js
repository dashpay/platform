import fs from 'fs';
import forge from 'node-forge';

export default class LegoCertificate {
  /**
   * @type {Date}
   */
  expires;

  /**
   * @type {Date}
   */
  created;

  /**
   * @type {string|null}
   */
  commonName;

  /**
   * @type {string[]}
   */
  ipAddresses;

  static EXPIRATION_LIMIT_DAYS = 2;

  /**
   * @param {Object} data
   * @param {Date} data.expires
   * @param {Date} data.created
   * @param {string|null} data.commonName
   * @param {string[]} data.ipAddresses
   */
  constructor(data) {
    this.expires = data.expires;
    this.created = data.created;
    this.commonName = data.commonName;
    this.ipAddresses = data.ipAddresses || [];
  }

  /**
   * Parse certificate from PEM file
   *
   * @param {string} certPath - Path to certificate PEM file
   * @returns {LegoCertificate}
   */
  static fromFile(certPath) {
    const certPem = fs.readFileSync(certPath, 'utf8');
    return LegoCertificate.fromPem(certPem);
  }

  /**
   * Parse certificate from PEM string
   *
   * @param {string} certPem - PEM encoded certificate
   * @returns {LegoCertificate}
   */
  static fromPem(certPem) {
    const cert = forge.pki.certificateFromPem(certPem);

    const commonNameAttr = cert.subject.attributes.find(
      (attr) => attr.shortName === 'CN',
    );

    // Extract IP addresses from Subject Alternative Name extension
    const ipAddresses = [];
    const sanExtension = cert.getExtension('subjectAltName');
    if (sanExtension && sanExtension.altNames) {
      for (const altName of sanExtension.altNames) {
        // Type 7 is IP address in SAN
        if (altName.type === 7 && altName.ip) {
          ipAddresses.push(altName.ip);
        }
      }
    }

    return new LegoCertificate({
      expires: cert.validity.notAfter,
      created: cert.validity.notBefore,
      commonName: commonNameAttr ? commonNameAttr.value : null,
      ipAddresses,
    });
  }

  /**
   * Check if certificate file exists
   *
   * @param {string} certPath
   * @returns {boolean}
   */
  static exists(certPath) {
    return fs.existsSync(certPath);
  }

  /**
   * Is certificate expired in N days?
   *
   * @param {number} days
   * @returns {boolean}
   */
  isExpiredInDays(days) {
    const expiresInDays = new Date(this.expires);
    expiresInDays.setDate(expiresInDays.getDate() - days);

    return expiresInDays.getTime() <= Date.now();
  }

  /**
   * Is certificate expired less than in 2 days?
   *
   * @returns {boolean}
   */
  isExpiredSoon() {
    return this.isExpiredInDays(LegoCertificate.EXPIRATION_LIMIT_DAYS);
  }

  /**
   * Is certificate valid (not expired)?
   *
   * @returns {boolean}
   */
  isValid() {
    const now = Date.now();

    // Both ends of the window. A certificate whose validity has not started is
    // no more servable than an expired one, and the gateway checks reject it -
    // so judging it usable here would hand a rejected certificate back to a
    // repair meant to replace it.
    return new Date(this.created).getTime() <= now
      && new Date(this.expires).getTime() > now;
  }
}
