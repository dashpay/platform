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
   * @type {string}
   */
  commonName;

  static EXPIRATION_LIMIT_DAYS = 2;

  /**
   * @param {Object} data
   * @param {Date} data.expires
   * @param {Date} data.created
   * @param {string} data.commonName
   */
  constructor(data) {
    this.expires = data.expires;
    this.created = data.created;
    this.commonName = data.commonName;
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

    return new LegoCertificate({
      expires: cert.validity.notAfter,
      created: cert.validity.notBefore,
      commonName: commonNameAttr ? commonNameAttr.value : null,
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
    return new Date(this.expires).getTime() > Date.now();
  }
}
