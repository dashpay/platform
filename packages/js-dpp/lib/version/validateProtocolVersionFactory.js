const UnsupportedProtocolVersionError = require('../errors/consensus/basic/UnsupportedProtocolVersionError');
const CompatibleProtocolVersionIsNotDefinedError = require('../errors/CompatibleProtocolVersionIsNotDefinedError');
const ValidationResult = require('../validation/ValidationResult');

const IncompatibleProtocolVersionError = require('../errors/consensus/basic/IncompatibleProtocolVersionError');
const { latestVersion } = require('./protocolVersion');

/**
 * @param {DashPlatformProtocol} dpp
 * @param versionCompatibilityMap
 * @returns {validateProtocolVersion}
 */
function validateProtocolVersionFactory(dpp, versionCompatibilityMap) {
  /**
   * @typedef {validateProtocolVersion}
   * @param {number} protocolVersion
   * @returns {ValidationResult}
   */
  function validateProtocolVersion(protocolVersion) {
    const result = new ValidationResult();

    // Parsed protocol version must be equal or lower than latest protocol version
    if (protocolVersion > latestVersion) {
      result.addError(
        new UnsupportedProtocolVersionError(
          protocolVersion,
          latestVersion,
        ),
      );

      return result;
    }

    // The highest version should be used for the compatibility map
    // to get minimal compatible version
    const maxProtocolVersion = Math.max(protocolVersion, dpp.getProtocolVersion());

    // The lowest version should be used to compare with the minimal compatible version
    const minProtocolVersion = Math.min(protocolVersion, dpp.getProtocolVersion());

    if (!Object.prototype.hasOwnProperty.call(versionCompatibilityMap, maxProtocolVersion)) {
      throw new CompatibleProtocolVersionIsNotDefinedError(maxProtocolVersion);
    }

    const minimalCompatibleProtocolVersion = versionCompatibilityMap[maxProtocolVersion];

    // Parsed protocol version (or current network protocol version) must higher
    // or equal to the minimum compatible version
    if (minProtocolVersion < minimalCompatibleProtocolVersion) {
      result.addError(
        new IncompatibleProtocolVersionError(
          protocolVersion,
          minimalCompatibleProtocolVersion,
        ),
      );

      return result;
    }

    return result;
  }

  return validateProtocolVersion;
}

module.exports = validateProtocolVersionFactory;
