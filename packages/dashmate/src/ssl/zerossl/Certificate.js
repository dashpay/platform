import convertDate from './convertDate.js';

export default class Certificate {
  id;

  type;

  /**
   * @type {'draft' | 'pending_validation' | 'issued' | 'cancelled' | 'revoked' | 'expired'}
   */
  status;

  /**
   * @type {Date}
   */
  created;

  /**
   * @type {Date}
   */
  expires;

  /**
   * @type {string}
   */
  // eslint-disable-next-line camelcase
  common_name;

  static EXPIRATION_LIMIT_DAYS = 3;

  /**
   * @param {Object} object
   * @param {string} object.id - The internal certificate ID.
   * @param {number} object.type - The numeric ID of the certificate type.
   * @param {string} object.common_name - The common name of the certificate.
   * @param {string} object.additional_domains - Any additional domains in the certificate.
   * @param {string} object.created - The exact time the certificate was created.
   * @param {string} object.expires - The exact time the certificate will expire.
   * @param {string} object.status - The current certificate status.
   * @param {string|null} object.validation_type - The selected verification type, or null
   * if not initiated.
   * @param {string|null} object.validation_emails - The selected verification emails, or null
   * if not chosen.
   * @param {string} object.replacement_for - The ID of the existing certificate this
   * one is replacing.
   * @param {string|null} object.fingerprint_sha1 - The SHA-1 fingerprint of the certificate,
   * or null for older certificates.
   * @param {boolean|null} object.brand_validation - True if the domain has to be manually reviewed,
   * usually null or false.
   * @param {Object} object.validation - A series of sub-objects related to domain verification.
   * @param {Array<string>} object.validation.email_validation - An array of eligible domain
   * verification emails.
   * @param {Object} object.validation.other_methods - Sub-objects containing alternative
   * verification methods.
   * @param {string} object.validation.other_methods.*.file_validation_url_http - The URL for
   * HTTP verification file upload.
   * @param {string} object.validation.other_methods.*.file_validation_url_https - The URL for
   * HTTPS verification file upload.
   * @param {Array<string>} object.validation.other_methods.*.file_validation_content - The
   * content for the verification file.
   * @param {string} object.validation.other_methods.*.cname_validation_p1 - The host-part of the
   * CNAME-record for domain verification.
   * @param {string} object.validation.other_methods.*.cname_validation_p2 - The value-part of
   * the CNAME-record for domain verification.
   */
  constructor(object) {
    const expires = convertDate(object.expires);
    const created = convertDate(object.created);

    Object.assign(this, {
      ...object,
      created,
      expires,
    });
  }

  /**
   * Is certificate issued?
   *
   * @returns {boolean}
   */
  isValid() {
    return this.status === 'issued';
  }

  /**
   * Is certificate require validation?
   *
   * @returns {boolean}
   */
  isPendingValidation() {
    return this.status === 'pending_validation';
  }

  /**
   * Is certificate draft yet?
   */
  isDraft() {
    return this.status === 'draft';
  }

  /**
   * Is certificate expired in N days?
   *
   * @param {number} days
   */
  isExpiredInDays(days) {
    const expiresInDays = new Date(this.expires);
    expiresInDays.setDate(expiresInDays.getDate() - days);

    return expiresInDays.getTime() <= Date.now();
  }

  /**
   * Is certificate expired less than in 3 days?
   */
  isExpiredSoon() {
    return this.isExpiredInDays(Certificate.EXPIRATION_LIMIT_DAYS);
  }
}
