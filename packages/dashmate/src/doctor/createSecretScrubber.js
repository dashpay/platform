import { getConfigSecretValues } from '../config/obfuscateConfig.js';
import maskOperatorIdentity, { getOperatorIdentity } from '../util/maskOperatorIdentity.js';

/**
 * Config secrets shorter than this are not searched for in the samples.
 *
 * A value that short turns up in ordinary text by chance - a word in a log
 * line, digits in a number - as readily as it marks the secret, and masking it
 * there would mangle the logs the report exists to carry. The secrets dashmate
 * generates and its default passwords are all longer, and the values of the
 * container environment, where a config value of any length would otherwise be
 * copied, are left out when samples are collected.
 */
const MIN_SCRUBBED_SECRET_LENGTH = 8;

/**
 * A percent-escape, also as it is written once its `%` is escaped again
 * (`%2f` as `%252f`): a URL in a query value is encoded once more, its own
 * escapes included
 */
const PERCENT_ESCAPE = /%(?:25)*[0-9A-Fa-f]{2}/g;

/**
 * @param {string} text
 * @param {string[]} needles
 * @return {[number, number][]} what each needle covers in the text, every
 *   occurrence included. Occurrences of one needle that overlap are merged as
 *   they are found, so a long run of them is one range rather than one per
 *   position.
 */
function findRanges(text, needles) {
  const ranges = [];

  for (const needle of needles) {
    let current = null;

    for (let index = text.indexOf(needle); index !== -1; index = text.indexOf(needle, index + 1)) {
      if (current && index <= current[1]) {
        current[1] = index + needle.length;
      } else {
        current = [index, index + needle.length];
        ranges.push(current);
      }
    }
  }

  return ranges;
}

/**
 * Mask every character any range covers. Ranges are found in the text as it
 * was and merged where they overlap, so masking one secret never hides part
 * of another from the search.
 *
 * @param {string} text
 * @param {[number, number][]} ranges
 * @return {string}
 */
function maskRanges(text, ranges) {
  if (ranges.length === 0) {
    return text;
  }

  const sorted = [...ranges].sort(([a], [b]) => a - b);

  let masked = '';
  let cursor = 0;
  let [start, end] = sorted[0];

  for (const [rangeStart, rangeEnd] of sorted.slice(1)) {
    if (rangeStart <= end) {
      end = Math.max(end, rangeEnd);
    } else {
      masked += text.slice(cursor, start) + '*'.repeat(end - start);
      cursor = end;
      [start, end] = [rangeStart, rangeEnd];
    }
  }

  return masked + text.slice(cursor, start) + '*'.repeat(end - start) + text.slice(end);
}

/**
 * Mask every config secret long enough to search for wherever it occurs in a
 * string.
 *
 * The config sample has its secret fields hidden already, but the same values
 * reach the collected samples on their own: a service can log one, an RPC
 * response can echo one. They are all known while samples are collected, so
 * they are taken out of collected text before the operator's identity is
 * masked in it - which could rewrite part of a secret, a home directory at
 * its start, say - and out of every sample again when the archive is
 * written.
 *
 * @param {Config} config - as stored, not obfuscated
 * @param {string[]} [extraValues] - secrets known from elsewhere, such as the
 *   option values of a container's command
 * @return {function(string): string}
 */
export default function createSecretScrubber(config, extraValues = []) {
  const identity = getOperatorIdentity();

  const needles = [...new Set(
    [...getConfigSecretValues(config), ...extraValues]
      .filter((value) => value.length >= MIN_SCRUBBED_SECRET_LENGTH)
      // Samples have the operator's name masked out as they are collected, so
      // a secret that contains the name is found there in that form
      .flatMap((value) => [value, maskOperatorIdentity(value, identity)])
      // Text that embeds JSON, such as a structured log line, carries a value
      // with a quote or a backslash in it escaped
      .flatMap((value) => [value, JSON.stringify(value).slice(1, -1)])
      // Masking the name can leave too little of a secret to search for
      .filter((value) => value.length >= MIN_SCRUBBED_SECRET_LENGTH)
      // A percent-escape stands for the same character whatever the case of
      // its hex digits, and text can quote a value escaped either way
      .flatMap((value) => [
        value,
        value.replace(PERCENT_ESCAPE, (escape) => escape.toUpperCase()),
        value.replace(PERCENT_ESCAPE, (escape) => escape.toLowerCase()),
      ]),
  )];

  return (text) => maskRanges(text, findRanges(text, needles));
}
