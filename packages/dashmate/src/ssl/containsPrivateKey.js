/**
 * Any private key block, whatever its type or form, an OpenPGP one ("PRIVATE
 * KEY BLOCK") included. Unanchored, so a marker on a mangled line counts too.
 */
const BEGIN_PRIVATE_KEY = /-----BEGIN [A-Z0-9 ]*PRIVATE KEY[A-Z0-9 ]*-----/;

/**
 * Whether a file's text holds a private key block
 *
 * @param {string} text
 * @return {boolean}
 */
export default function containsPrivateKey(text) {
  return BEGIN_PRIVATE_KEY.test(text);
}
