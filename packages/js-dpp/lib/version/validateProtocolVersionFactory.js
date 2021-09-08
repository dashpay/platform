const UnsupportedProtocolVersionError = require('../errors/consensus/basic/UnsupportedProtocolVersionError');
const CompatibleProtocolVersionIsNotDefinedError = require('../errors/CompatibleProtocolVersionIsNotDefinedError');
const IncompatibleProtocolVersionError = require('../errors/consensus/basic/IncompatibleProtocolVersionError');
const ValidationResult = require('../validation/ValidationResult');

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

    // Parsed protocol version must be equal or lower than current version
    if (protocolVersion > dpp.getProtocolVersion()) {
      result.addError(
        new UnsupportedProtocolVersionError(
          protocolVersion,
          dpp.getProtocolVersion(),
        ),
      );

      return result;
    }

    if (!Object.prototype.hasOwnProperty.call(versionCompatibilityMap, dpp.getProtocolVersion())) {
      throw new CompatibleProtocolVersionIsNotDefinedError(dpp.getProtocolVersion());
    }

    const minimalProtocolVersion = versionCompatibilityMap[dpp.getProtocolVersion()];

    // Parsed protocol version must higher or equal to the minimum compatible version
    if (protocolVersion < minimalProtocolVersion) {
      result.addError(
        new IncompatibleProtocolVersionError(
          protocolVersion,
          minimalProtocolVersion,
        ),
      );
    }

    return result;
  }

  return validateProtocolVersion;
}

module.exports = validateProtocolVersionFactory;
