import LegoArtifactsMissingError from './errors/LegoArtifactsMissingError.js';
import LegoDidNotStartError from './errors/LegoDidNotStartError.js';
import LegoResultNotObservedError from './errors/LegoResultNotObservedError.js';

/**
 * Why a scheduled renewal did not produce a certificate.
 *
 * The vocabulary is closed and lives here rather than at the call sites,
 * because only the helper still holds the error: by the time a report reaches
 * whoever is helping an operator, the provider's own account of what happened
 * is gone. A reader that meets a code it does not know treats it as UNKNOWN,
 * so a newer helper can add one without silencing an older reader.
 */
export const RENEWAL_FAILURE_CODES = {
  PORT_80_UNREACHABLE: 'PORT_80_UNREACHABLE',
  PORT_80_WRONG_RESPONDER: 'PORT_80_WRONG_RESPONDER',
  PORT_80_IN_USE: 'PORT_80_IN_USE',
  RATE_LIMITED: 'RATE_LIMITED',
  PROVIDER_REJECTED: 'PROVIDER_REJECTED',
  HELPER_DID_NOT_START: 'HELPER_DID_NOT_START',
  CERTIFICATE_ISSUED_NOT_SAVED: 'CERTIFICATE_ISSUED_NOT_SAVED',
  RESULT_UNKNOWN: 'RESULT_UNKNOWN',
  QUOTA_EXHAUSTED: 'QUOTA_EXHAUSTED',
  PROVIDER_AUTH: 'PROVIDER_AUTH',
  PROVIDER_UNREACHABLE: 'PROVIDER_UNREACHABLE',
  CERTIFICATE_FILE_MISSING: 'CERTIFICATE_FILE_MISSING',
  RENEWAL_INTERRUPTED: 'RENEWAL_INTERRUPTED',
  UNKNOWN: 'UNKNOWN',
};

/**
 * What an operator should do, as a class rather than as prose.
 *
 * Every code carries one. Without it a code added later inherits whatever
 * ending the surrounding text happened to have, and the two endings that must
 * never be handed to the wrong cause are here: asking for another certificate
 * when the authority has already refused, and asking for one when an issuance
 * has been spent but never landed.
 */
export const REMEDY_CLASS = {
  /** Fix something on this machine; renewal retries by itself afterwards. */
  FIX_LOCALLY: 'FIX_LOCALLY',
  /** The certificate has to be requested again. */
  OBTAIN: 'OBTAIN',
  /** Nothing this provider can do; the operator chooses another. */
  SWITCH_PROVIDER: 'SWITCH_PROVIDER',
  /** Asking again makes it worse. Say so before anything else. */
  DO_NOT_RETRY: 'DO_NOT_RETRY',
  /** Wait - it is transient, or already in someone else's hands. */
  WAIT: 'WAIT',
  /** Nothing actionable was established. */
  SUPPORT: 'SUPPORT',
};

/**
 * One sentence per code, and the ending it is allowed to take.
 *
 * Shared by every operator-facing surface. Both `doctor` and `update` say the
 * same thing about a cause because they read the same entry; the commands
 * around it differ because those two surfaces render differently, and that is
 * the only part either is free to choose.
 */
const DESCRIPTIONS = {
  [RENEWAL_FAILURE_CODES.PORT_80_UNREACHABLE]: {
    sentence: 'the certificate authority could not reach this node on port 80',
    remedy: REMEDY_CLASS.FIX_LOCALLY,
  },
  [RENEWAL_FAILURE_CODES.PORT_80_WRONG_RESPONDER]: {
    sentence: "something answered on port 80, but not this node's certificate check",
    remedy: REMEDY_CLASS.FIX_LOCALLY,
  },
  [RENEWAL_FAILURE_CODES.PORT_80_IN_USE]: {
    sentence: 'something on this machine is already using port 80, so the certificate check could'
      + ' not start',
    remedy: REMEDY_CLASS.FIX_LOCALLY,
  },
  [RENEWAL_FAILURE_CODES.RATE_LIMITED]: {
    sentence: 'the certificate authority has temporarily refused this address',
    remedy: REMEDY_CLASS.DO_NOT_RETRY,
  },
  [RENEWAL_FAILURE_CODES.PROVIDER_REJECTED]: {
    sentence: 'the certificate authority refused the request',
    remedy: REMEDY_CLASS.SUPPORT,
  },
  [RENEWAL_FAILURE_CODES.HELPER_DID_NOT_START]: {
    sentence: 'dashmate could not start the certificate check on this machine, so nothing reached'
      + ' the certificate authority',
    remedy: REMEDY_CLASS.FIX_LOCALLY,
  },
  [RENEWAL_FAILURE_CODES.CERTIFICATE_ISSUED_NOT_SAVED]: {
    sentence: 'a certificate was issued but dashmate could not save it',
    remedy: REMEDY_CLASS.DO_NOT_RETRY,
  },
  [RENEWAL_FAILURE_CODES.RESULT_UNKNOWN]: {
    sentence: 'dashmate could not read how the certificate check finished, so it does not know'
      + ' whether a certificate was requested',
    remedy: REMEDY_CLASS.DO_NOT_RETRY,
  },
  [RENEWAL_FAILURE_CODES.QUOTA_EXHAUSTED]: {
    sentence: "this node's free ZeroSSL account has used all three of its certificates, so ZeroSSL"
      + ' will not issue another one',
    remedy: REMEDY_CLASS.SWITCH_PROVIDER,
  },
  [RENEWAL_FAILURE_CODES.PROVIDER_AUTH]: {
    sentence: "ZeroSSL rejected this node's account details, so it will not issue a certificate",
    remedy: REMEDY_CLASS.SWITCH_PROVIDER,
  },
  [RENEWAL_FAILURE_CODES.PROVIDER_UNREACHABLE]: {
    sentence: 'dashmate could not reach the certificate provider',
    remedy: REMEDY_CLASS.WAIT,
  },
  [RENEWAL_FAILURE_CODES.CERTIFICATE_FILE_MISSING]: {
    sentence: "this node's certificate file is missing, so there is nothing to renew",
    remedy: REMEDY_CLASS.OBTAIN,
  },
  [RENEWAL_FAILURE_CODES.RENEWAL_INTERRUPTED]: {
    sentence: 'another dashmate command was changing configuration, so renewal stopped part way',
    remedy: REMEDY_CLASS.WAIT,
  },
  [RENEWAL_FAILURE_CODES.UNKNOWN]: {
    sentence: 'dashmate could not work out why',
    remedy: REMEDY_CLASS.SUPPORT,
  },
};

/**
 * The cause and the ending it may take, for anything an operator reads.
 *
 * A code this build does not know is described as unknown rather than passed
 * through: an identifier an operator cannot look up is worse than an admission.
 *
 * @param {string} code
 * @return {{sentence: string, remedy: string}}
 */
export function describeRenewalFailure(code) {
  return DESCRIPTIONS[code] ?? DESCRIPTIONS[RENEWAL_FAILURE_CODES.UNKNOWN];
}

/**
 * How much of a provider's account of a failure is examined.
 *
 * lego writes single lines of unbounded length, and the patterns below run on
 * the helper's event loop - the same loop the configuration lock's lease
 * refresh lives on. Bounding the input first keeps a hostile or merely verbose
 * line from stalling it.
 */
const MAX_EXAMINED_CHARS = 2048;

/**
 * How much of it is kept.
 *
 * A size control, not a secrecy one: this much text comfortably holds a key or
 * an account address, which is why the selection below is an allow-list rather
 * than a slice of whatever came back.
 */
export const MAX_DETAIL_CHARS = 200;

/**
 * The problem types RFC 8555 registers, as lego prints them.
 *
 * A registered vocabulary rather than prose, which is what makes it safe to
 * branch on: `ProblemDetails.Error()` prints the type verbatim, and the type
 * is the authority's own classification rather than dashmate's reading of it.
 */
const ACME_PROBLEM_PATTERN = /urn:ietf:params:acme:error:([A-Za-z]+)/;

const ACME_PROBLEM_CODES = {
  connection: RENEWAL_FAILURE_CODES.PORT_80_UNREACHABLE,
  unauthorized: RENEWAL_FAILURE_CODES.PORT_80_WRONG_RESPONDER,
  rateLimited: RENEWAL_FAILURE_CODES.RATE_LIMITED,
};

/**
 * ZeroSSL's own numeric codes, which survive to here because the API client
 * copies them onto the error it throws.
 */
const ZEROSSL_QUOTA_CODES = [2817, 2839];
const ZEROSSL_AUTH_CODES = [101, 102, 2801, 2841];

/**
 * Docker's wording when a port cannot be bound.
 *
 * Matched only to separate an occupied port from every other reason the
 * certificate check might not start: the two have opposite repairs, and
 * confusing them sends an operator to open a port that is already open.
 */
const PORT_BIND_PATTERN = /port is already allocated|address already in use|bind for \S+ failed/i;

/**
 * Dashmate's own account of losing the configuration lock.
 *
 * Recognised so it does not fall through to "could not work out why" - dashmate
 * diagnosed this one itself, and saying otherwise would send an operator to
 * support over a condition that clears on its own.
 */
const LOCK_PATTERN = /Lost the configuration lock|Timed out waiting for configuration lock/;

/**
 * Terminal control sequences, removed wherever this text is stored or shown.
 *
 * `detail` is the first free text either operator surface prints verbatim, and
 * `dashmate doctor --samples` renders an archive that arrived from someone
 * else - so escape sequences in it would be interpreted by the terminal of
 * whoever is helping, and could rewrite what they see.
 */
// eslint-disable-next-line no-control-regex -- matching them is the point
const CONTROL_CHARACTERS = /[\u0000-\u001F\u007F]/g;

/**
 * @param {*} value
 * @return {string}
 */
function readMessage(value) {
  // Only the message, and never the error object. Both providers hang extra
  // fields off the errors they throw - ZeroSSL copies its whole response body
  // on, one field of which is named a single character away from this one, and
  // a listr error can carry the task context, which on the ZeroSSL path holds
  // the gateway's private key. None of that may reach disk.
  const message = value?.message;

  return typeof message === 'string' ? message : '';
}

/**
 * @param {string} text
 * @param {string|null} homeDirPath
 * @return {string}
 */
function collapseHomeDir(text, homeDirPath) {
  if (!homeDirPath) {
    return text;
  }

  // Done here rather than where the report is assembled. The reader's masking
  // matches the home directory only where it ends, so a value cut to length
  // partway through the operator's name matches nothing and would survive.
  return text.split(homeDirPath).join('~');
}

/**
 * @param {string} text
 * @return {string}
 */
function redact(text) {
  return text
    // The host says which certificate authority answered, which is worth
    // keeping; everything after it identifies the account, the order or the
    // authorization, and identifies the operator with it.
    .replace(/(https?:\/\/[^/\s]+)\/\S*/g, '$1/...')
    .replace(/[^\s:/@]+@[^\s:/@]+\.[^\s:/@]+/g, '[email]');
}

/**
 * The line that carries the evidence, or nothing.
 *
 * An allow-list rather than a position. Taking the first or last line instead
 * would mean storing an arbitrary slice of dashmate's own guidance, which is
 * the part of these errors that carries absolute paths and Docker's raw
 * output - and which the surface reading this is about to write for itself.
 *
 * @param {string} message
 * @param {number|null} providerCode
 * @return {string|null}
 */
function selectEvidence(message, providerCode) {
  const lines = message.split('\n').map((line) => line.trim()).filter(Boolean);

  const acmeLine = lines.find((line) => ACME_PROBLEM_PATTERN.test(line));

  if (acmeLine) {
    return acmeLine;
  }

  // ZeroSSL answers with a code, and the message beside it is the provider's
  // own description of that code rather than anything dashmate composed.
  if (providerCode !== null && lines.length > 0) {
    return lines[0];
  }

  return null;
}

/**
 * @param {*} error
 * @return {number|null}
 */
function readProviderCode(error) {
  const code = error?.code;

  return typeof code === 'number' ? code : null;
}

/**
 * @param {*} error
 * @return {string}
 */
function classifyCode(error, message) {
  // The typed errors describe how far the attempt got, which no amount of
  // reading the text can establish: whether the certificate check ever ran,
  // and whether an issuance was spent. They arrive as the cause because the
  // task that raises them replaces them with guidance written for a terminal.
  const cause = error?.cause;

  if (cause instanceof LegoArtifactsMissingError) {
    return RENEWAL_FAILURE_CODES.CERTIFICATE_ISSUED_NOT_SAVED;
  }

  if (cause instanceof LegoResultNotObservedError) {
    return RENEWAL_FAILURE_CODES.RESULT_UNKNOWN;
  }

  if (cause instanceof LegoDidNotStartError) {
    // Bounded like everything else: this one comes from the Docker daemon,
    // which is the only message here that is not dashmate's or a certificate
    // authority's, and the pattern below is the one that is not linear.
    return PORT_BIND_PATTERN.test(readMessage(cause.cause).slice(0, MAX_EXAMINED_CHARS))
      ? RENEWAL_FAILURE_CODES.PORT_80_IN_USE
      : RENEWAL_FAILURE_CODES.HELPER_DID_NOT_START;
  }

  const acmeProblem = message.match(ACME_PROBLEM_PATTERN);

  if (acmeProblem) {
    return ACME_PROBLEM_CODES[acmeProblem[1]] ?? RENEWAL_FAILURE_CODES.PROVIDER_REJECTED;
  }

  const providerCode = readProviderCode(error);

  if (providerCode !== null) {
    if (ZEROSSL_QUOTA_CODES.includes(providerCode)) {
      return RENEWAL_FAILURE_CODES.QUOTA_EXHAUSTED;
    }

    if (ZEROSSL_AUTH_CODES.includes(providerCode)) {
      return RENEWAL_FAILURE_CODES.PROVIDER_AUTH;
    }

    return RENEWAL_FAILURE_CODES.PROVIDER_REJECTED;
  }

  // Only an absent file. The same read also throws for a permission denial and
  // for a corrupt certificate, and telling an operator to obtain a new one
  // spends an issuance against a weekly limit on a problem a new certificate
  // cannot fix.
  if (error?.code === 'ENOENT') {
    return RENEWAL_FAILURE_CODES.CERTIFICATE_FILE_MISSING;
  }

  if (LOCK_PATTERN.test(message)) {
    return RENEWAL_FAILURE_CODES.RENEWAL_INTERRUPTED;
  }

  // The verification server binds port 80 on this machine before ZeroSSL is
  // asked to look at it, so a server that never answered is a local condition
  // rather than anything the provider said.
  if (message.includes('Verification server is not responding')) {
    return RENEWAL_FAILURE_CODES.PORT_80_IN_USE;
  }

  if (message.includes('Invalid ZeroSSL API response')
    || message.includes('fetch failed')) {
    return RENEWAL_FAILURE_CODES.PROVIDER_UNREACHABLE;
  }

  return RENEWAL_FAILURE_CODES.UNKNOWN;
}

/**
 * Remove control sequences and flatten to a single line.
 *
 * Applied where this text is written and again where it is read: a record can
 * be edited by hand, and a report can arrive from someone else entirely.
 *
 * @param {string} text
 * @return {string}
 */
export function sanitizeDetail(text) {
  return String(text ?? '')
    .replace(CONTROL_CHARACTERS, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

/**
 * Name what stopped a renewal, and keep a bounded account of it for a person.
 *
 * @param {*} error - whatever the renewal threw; not necessarily an Error
 * @param {Object} [options]
 * @param {string|null} [options.homeDirPath] - collapsed out of the excerpt
 * @return {{code: string, detail: string|null}}
 */
export default function classifyRenewalFailure(error, { homeDirPath = null } = {}) {
  // Bounded once, before anything examines it. lego writes single lines of
  // unbounded length and these patterns run on the helper's event loop - the
  // same loop that refreshes the configuration lock's lease, so a stall here
  // is a lease that stops being renewed while the helper still looks alive.
  const examined = readMessage(error).slice(0, MAX_EXAMINED_CHARS);

  const code = classifyCode(error, examined);
  const evidence = selectEvidence(
    collapseHomeDir(examined, homeDirPath),
    readProviderCode(error),
  );

  if (evidence === null) {
    return { code, detail: null };
  }

  return {
    code,
    detail: sanitizeDetail(redact(evidence)).slice(0, MAX_DETAIL_CHARS),
  };
}
