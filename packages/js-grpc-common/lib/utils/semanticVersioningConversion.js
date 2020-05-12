/**
 * Convert a sematic versioning string into an 32-bit integer.
 *
 * Make sure the input string is compatible with the standard found
 * at semver.org. Since this only uses 10-bit per major/minor/patch version,
 * the highest possible SemVer string would be 1023.1023.1023.
 *
 * @param  {string} version SemVer string
 * @return {number}         Numeric version
 */
function convertVersionToInt32(version) {
  // Split a given version string into three parts.
  const parts = version.split('.');

  // Check if we got exactly three parts, otherwise throw an error.
  if (parts.length !== 3) {
    throw new Error('Received invalid version string');
  }

  // Make sure that no part is larger than 1023 or else it
  // won't fit into a 32-bit integer.
  parts.forEach((part) => {
    if (part >= 1024) {
      throw new Error(`Version string invalid, ${part} is too large`);
    }
  });

  // Let's create a new number which we will return later on
  let numericVersion = 0;
  // Shift all parts either 0, 10 or 20 bits to the left.
  for (let i = 0; i < 3; i++) {
    // eslint-disable-next-line no-bitwise
    numericVersion |= parts[i] << i * 10;
  }

  return numericVersion;
}

/**
 * Converts a 32-bit integer into a semantic versioning (SemVer) compatible string.
 *
 * @param  {number} v Numeric version
 * @return {string}   SemVer string
 */
function convertInt32VersionToString(v) {
  // Works by shifting the numeric version to the right and then masking it
  // with 0b1111111111 (or 1023 in decimal).

  // eslint-disable-next-line no-bitwise, no-mixed-operators
  return `${v & 1023}.${v >> 10 & 1023}.${v >> 20 & 1023}`;
}

module.exports = {
  convertVersionToInt32,
  convertInt32VersionToString,
};
