import hideString from './hideString.js';

/**
 * Words dashmate writes about itself, which must survive masking even when the
 * operator's account happens to be named after one of them.
 *
 * On a package-installed node the service account is called `dashmate`, so the
 * username and the product name are the same string. Replacing it blanks the
 * subject out of every sentence dashmate writes - "******** could not find the
 * certificate bundle" - and mangles the directories dashmate itself creates,
 * while hiding nothing: the home directory below is what actually discloses who
 * is running it, and that is removed regardless.
 */
const PRODUCT_WORDS = ['dashmate'];

/**
 * A name is only masked where it stands alone. Substring replacement turns
 * every word that happens to contain it into nonsense, and the collisions are
 * not rare - short account names are common.
 *
 * @param {string} name
 * @return {RegExp}
 */
function wholeWord(name) {
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  return new RegExp(`(?<![A-Za-z0-9])${escaped}(?![A-Za-z0-9])`, 'g');
}

/**
 * Remove the operator's identity from text that may leave the machine.
 *
 * The home directory becomes `~`, so the part of a path that makes a problem
 * actionable - which file, under which config - survives, and the whole path
 * still resolves when pasted. A report is what an operator hands to whoever is
 * helping them, and a path they cannot use is a problem they cannot act on.
 *
 * @param {*} value - returned unchanged unless it is a string
 * @param {Object} identity
 * @param {string|null} identity.username
 * @param {string|null} identity.homePath
 * @return {*}
 */
export default function maskOperatorIdentity(value, { username, homePath }) {
  if (typeof value !== 'string') {
    return value;
  }

  let masked = value;

  if (homePath) {
    // Written home-relative rather than blanked out. A masked segment removes
    // the name but also removes the operator's ability to paste the path back;
    // `~` removes the name and still resolves.
    masked = masked.replaceAll(homePath, '~');
  }

  if (username && !PRODUCT_WORDS.includes(username)) {
    masked = masked.replace(wholeWord(username), hideString(username));
  }

  return masked;
}
