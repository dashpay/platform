import semver from 'semver';
import { UNPRINTABLE } from './sanitizeRemoteText.js';

/**
 * Handling of bytes a remote host chose: bounded, validated, and safe to print.
 *
 * Everything here defends the same boundary from two directions. A version string from a
 * registry or a release API is printed to a terminal, included in JSON output and handed to a
 * package manager, where npm accepts `git+https://…`, `file:…`, `https://….tgz` and `npm:`
 * aliases wherever it accepts a version, so only a strict version shape may pass. A response
 * body is read into memory, so its size is bounded while it is read, and error messages quote
 * the payload that failed to parse, so remote bytes reach the terminal through diagnostics even
 * when the value itself is rejected.
 *
 * It lives here rather than in either caller because the same defences are needed by the status
 * providers and by update-candidate resolution, and two copies of a validation regex are two
 * copies that can drift apart — with the weaker one deciding what gets installed.
 */

const MAX_LOG_LENGTH = 200;

export const MAX_RESPONSE_SIZE = 1 * 1024 * 1024; // 1 MB

/**
 * The only version shape accepted from a remote host.
 *
 * The anchors and character classes leave no room for control or ANSI escape characters, nor
 * for the specifiers a package manager would treat as a location to install from.
 */
export const VERSION_REGEX = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/;

/**
 * How a body read ended.
 */
export const BODY_STATUS = {
  OK: 'ok',
  TOO_LARGE: 'too-large',
  UNREADABLE: 'unreadable',
};

/**
 * Make text received from a remote host safe to print.
 *
 * Both the dangerous characters and the length are bounded here, and a value that is not text
 * is reported as such rather than coerced: coercing an object whose `toString` is not a
 * function throws, and that throw escapes through the diagnostic that was meant to explain a
 * rejection.
 *
 * @param {*} text
 * @return {string}
 */
export function sanitizeForLog(text) {
  return typeof text === 'string'
    ? text.replace(UNPRINTABLE, '').slice(0, MAX_LOG_LENGTH)
    : '[unprintable]';
}

/**
 * Accept a version string only if it is a version.
 *
 * The shape check runs first and the semver parse second: the regex refuses what semver would
 * accept in a package-manager context (a `git+https://` specifier is not a version but is a
 * perfectly good argument to `npm install`), and semver refuses what the shape allows but
 * cannot order, such as the leading zeroes in `01.2.3`. The normalised result drops build
 * metadata, which comparison ignores and a package manager would refuse to resolve.
 *
 * @param {*} version
 * @return {string|null} the normalised version, or null when it is not one
 */
export function parseVersion(version) {
  if (typeof version !== 'string' || !VERSION_REGEX.test(version)) {
    return null;
  }

  return semver.valid(version);
}

/**
 * Read a response body, giving up as soon as it exceeds the size limit.
 *
 * The declared content length is only a hint, so the body is measured again while it is read:
 * an implementation that reads the whole body and measures afterwards does exactly what the
 * limit exists to prevent. Neither limit leaves a body hanging - a declared oversize is
 * cancelled explicitly, and stopping mid-stream cancels through the async iterator - because
 * until a body is released its connection stays checked out of the pool.
 *
 * @param {Response} response
 * @return {Promise<{status: string, text?: string, message?: string, declaredSize?: number|null}>}
 */
export async function readCappedBody(response) {
  const declaredSize = Number(response.headers.get('content-length'));

  if (Number.isFinite(declaredSize) && declaredSize > MAX_RESPONSE_SIZE) {
    // This body is never read, and until it is cancelled the connection stays checked out
    // of the pool
    response.body?.cancel().catch(() => {});

    return { status: BODY_STATUS.TOO_LARGE, declaredSize };
  }

  if (!response.body) {
    return { status: BODY_STATUS.UNREADABLE, message: 'the response carried no body' };
  }

  const chunks = [];
  let size = 0;

  try {
    for await (const chunk of response.body) {
      size += chunk.length;

      if (size > MAX_RESPONSE_SIZE) {
        // Leaving the loop here releases the async iterator, which cancels the stream, so
        // the connection is not held by a body this returns without
        return { status: BODY_STATUS.TOO_LARGE, declaredSize: null };
      }

      chunks.push(chunk);
    }
  } catch (e) {
    // The connection can still drop after the headers arrived, leaving a truncated body
    return { status: BODY_STATUS.UNREADABLE, message: e?.message };
  }

  return { status: BODY_STATUS.OK, text: Buffer.concat(chunks).toString('utf8') };
}
