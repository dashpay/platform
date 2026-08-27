import { MAX_DETAIL_CHARS, sanitizeDetail } from '../renewal-failure.js';

/**
 * What the helper recorded about the last renewal for one config.
 *
 * The questions a reader actually asks - did it fail, does it still describe
 * this node, what should the operator be told - are answered here rather than
 * at each call site. Both operator surfaces asked them separately before, which
 * is how the two came to disagree about whether a record applied.
 */
export default class RenewalRecord {
  /**
   * The shape this build writes.
   *
   * Recorded for a person opening the file, and for a future reader that has a
   * reason to care. Nothing gates on it: a reader that refused an unfamiliar
   * version would go silent on a node that is actively failing, which is worse
   * than reporting nothing at all. Fields are validated one at a time instead,
   * so an unfamiliar shape degrades to the parts that are recognisable.
   */
  static FORMAT_VERSION = 1;

  /**
   * Leaves room for anything a reader derives from a stored instant while
   * staying well inside what a Date can represent and format.
   */
  static #MAX_SAFE_INSTANT_MS = 8.64e15 - 86400000;

  static OUTCOMES = {
    SUCCEEDED: 'succeeded',
    FAILED: 'failed',
  };

  #provider;

  #outcome;

  #code;

  #detail;

  #attemptedAt;

  #lastSuccessAt;

  #consecutiveFailures;

  #issuanceSpentAt;

  #issuanceUncertainAt;

  #gatewayReloadFailedAt;

  #storageWritable;

  /**
   * @param {Object} properties - already validated by fromObject
   */
  constructor(properties) {
    this.#provider = properties.provider;
    this.#outcome = properties.outcome;
    this.#code = properties.code;
    this.#detail = properties.detail;
    this.#attemptedAt = properties.attemptedAt;
    this.#lastSuccessAt = properties.lastSuccessAt;
    this.#consecutiveFailures = properties.consecutiveFailures;
    this.#issuanceSpentAt = properties.issuanceSpentAt;
    this.#issuanceUncertainAt = properties.issuanceUncertainAt;
    this.#gatewayReloadFailedAt = properties.gatewayReloadFailedAt;
    this.#storageWritable = properties.storageWritable;
  }

  /**
   * @param {*} value
   * @return {Date|null}
   */
  static #readDate(value) {
    if (typeof value !== 'string') {
      return null;
    }

    const parsed = new Date(value);

    if (Number.isNaN(parsed.getTime())) {
      return null;
    }

    // A date near the edge of the representable range is valid on its own and
    // still unusable: readers derive instants from it - the next attempt is
    // this plus an hour - and formatting the result throws, which would take
    // the whole diagnosis down rather than one field. An archive can carry
    // such a value, so it is rejected where it enters.
    return Math.abs(parsed.getTime()) > RenewalRecord.#MAX_SAFE_INSTANT_MS ? null : parsed;
  }

  /**
   * Build a record from whatever was on disk, or from a collected sample.
   *
   * Taken field by field so an unfamiliar or damaged value costs only itself
   * rather than discarding an account of a failure that is otherwise sound. A
   * record with no verdict and no moment to judge it against says nothing at
   * all, and is rejected outright.
   *
   * @param {*} raw
   * @return {RenewalRecord|null}
   */
  static fromObject(raw) {
    if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
      return null;
    }

    const outcome = Object.values(RenewalRecord.OUTCOMES).includes(raw.outcome)
      ? raw.outcome
      : null;
    const attemptedAt = RenewalRecord.#readDate(raw.attemptedAt);

    if (outcome === null || attemptedAt === null) {
      return null;
    }

    return new RenewalRecord({
      provider: typeof raw.provider === 'string' ? raw.provider : null,
      outcome,
      code: typeof raw.code === 'string' ? raw.code : null,
      // Sanitised on the way in as well as on the way out. The file is editable
      // by hand and a collected report can arrive from someone else, so what
      // was safe when written is not established at the point it is read.
      // Bounded before it is scanned, not after. An archived report is read
      // straight into a sample without passing through this repository's own
      // write path, so the value can be any length at all.
      detail: typeof raw.detail === 'string'
        ? sanitizeDetail(raw.detail.slice(0, MAX_DETAIL_CHARS)) || null
        : null,
      attemptedAt,
      lastSuccessAt: RenewalRecord.#readDate(raw.lastSuccessAt),
      consecutiveFailures: Number.isInteger(raw.consecutiveFailures) && raw.consecutiveFailures >= 0
        ? raw.consecutiveFailures
        : 0,
      issuanceSpentAt: RenewalRecord.#readDate(raw.issuanceSpentAt),
      issuanceUncertainAt: RenewalRecord.#readDate(raw.issuanceUncertainAt),
      gatewayReloadFailedAt: RenewalRecord.#readDate(raw.gatewayReloadFailedAt),
      // Absent means it was never asked, which is not the same as "writable".
      // An older record, or one from a build that did not check, must not be
      // read as an assurance it never gave.
      storageWritable: typeof raw.storageWritable === 'boolean' ? raw.storageWritable : null,
    });
  }

  /**
   * @return {Object} what gets written, and what a sample carries
   */
  toObject() {
    return {
      formatVersion: RenewalRecord.FORMAT_VERSION,
      provider: this.#provider,
      outcome: this.#outcome,
      code: this.#code,
      detail: this.#detail,
      attemptedAt: this.#attemptedAt.toISOString(),
      lastSuccessAt: this.#lastSuccessAt ? this.#lastSuccessAt.toISOString() : null,
      consecutiveFailures: this.#consecutiveFailures,
      issuanceSpentAt: this.#issuanceSpentAt ? this.#issuanceSpentAt.toISOString() : null,
      issuanceUncertainAt: this.#issuanceUncertainAt
        ? this.#issuanceUncertainAt.toISOString()
        : null,
      gatewayReloadFailedAt: this.#gatewayReloadFailedAt
        ? this.#gatewayReloadFailedAt.toISOString()
        : null,
      storageWritable: this.#storageWritable,
    };
  }

  /**
   * @return {string|null}
   */
  getProvider() {
    return this.#provider;
  }

  /**
   * @return {string|null}
   */
  getCode() {
    return this.#code;
  }

  /**
   * @return {string|null}
   */
  getDetail() {
    return this.#detail;
  }

  /**
   * @return {Date}
   */
  getAttemptedAt() {
    return this.#attemptedAt;
  }

  /**
   * @return {Date|null}
   */
  getLastSuccessAt() {
    return this.#lastSuccessAt;
  }

  /**
   * @return {number}
   */
  getConsecutiveFailures() {
    return this.#consecutiveFailures;
  }

  /**
   * @return {Date|null}
   */
  getGatewayReloadFailedAt() {
    return this.#gatewayReloadFailedAt;
  }

  /**
   * Whether a certificate was issued and never landed.
   *
   * Outlives the failure that produced it, because that issuance is spent
   * against a weekly limit whether or not it arrived - so it still forbids
   * asking again once a later, different failure has replaced the cause.
   *
   * @return {boolean}
   */
  isIssuanceSpent() {
    return this.#issuanceSpentAt !== null;
  }

  /**
   * Whether a certificate may have been issued without dashmate seeing it.
   *
   * The certificate helper ran and its result was never read, so a request may
   * have reached the authority and counted against this node's allowance. Like
   * a confirmed spend this outlives the failure that produced it, because the
   * next attempt an hour later records an ordinary cause whose advice is to
   * ask again - and asking again is the one thing that must not happen while
   * it is unknown whether the last request succeeded.
   *
   * @return {boolean}
   */
  isIssuanceUncertain() {
    return this.#issuanceUncertainAt !== null;
  }

  /**
   * Whether asking the authority again may cost something already spent.
   *
   * @return {boolean}
   */
  isIssuanceOutstanding() {
    return this.isIssuanceSpent() || this.isIssuanceUncertain();
  }

  /**
   * Whether a certificate obtained now could be saved.
   *
   * Three answers, not two: `null` means nothing was established, and is what
   * a record written before this check existed carries. Only an outright
   * `false` withholds anything.
   *
   * @return {boolean|null}
   */
  getStorageWritable() {
    return this.#storageWritable;
  }

  /**
   * @return {boolean}
   */
  isFailed() {
    return this.#outcome === RenewalRecord.OUTCOMES.FAILED;
  }

  /**
   * Whether this record still describes the certificate the node is using.
   *
   * Two ways it stops doing so. A provider switch leaves the previous
   * provider's account behind, and it says nothing about the one now in use.
   * And a certificate obtained by hand after a failure overtakes that failure
   * completely - the helper cannot notice, because it stops watching the
   * configuration while it waits to retry and the values it watches do not
   * change when a certificate is installed. Without this an operator who has
   * just repaired their node is told renewal is failing, at the exact moment
   * they run the command to check their work.
   *
   * @param {Object} options
   * @param {string} options.provider - the configured provider
   * @param {*} [options.certificateValidFrom] - when the installed certificate
   *   was issued; an unusable value is treated as unknown rather than as older
   *   than everything, which would suppress every problem without a signal
   * @return {boolean}
   */
  appliesTo({ provider, certificateValidFrom = null }) {
    if (this.#provider !== provider) {
      return false;
    }

    const issuedAt = RenewalRecord.#readDate(
      certificateValidFrom instanceof Date
        ? certificateValidFrom.toISOString()
        : certificateValidFrom,
    );

    if (!this.isFailed() || issuedAt === null) {
      return true;
    }

    return this.#attemptedAt.getTime() > issuedAt.getTime();
  }
}
