const semver = require('semver');

const { Metadata } = require('grpc');

const VersionMismatchGrpcError = require('../error/VersionMismatchGrpcError');

const {
  convertVersionToInt32,
  convertInt32VersionToString,
} = require('../../utils/semanticVersioningConversion');

/**
 * Check version wrapper hanldler (factory)
 *
 * @param {string} serverProtocolVersionString
 *
 * @returns {checkVersionWrapper}
 */
function checkVersionWrapperFactory(serverProtocolVersionString) {
  /* Prepare server metadata on factory call */
  const serverProtocolVersionNumber = convertVersionToInt32(
    serverProtocolVersionString,
  );

  const serverMetadata = new Metadata();
  serverMetadata.set(
    'protocolVersion', serverProtocolVersionNumber,
  );

  /**
   *
   * @typedef checkVersionWrapper
   *
   * @param {Function(grpc.ServerWriteableStream, Function)} method
   *
   * @returns {internalHandler}
   */
  function checkVersionWrapper(method) {
    /**
     * @typedef internalHandler
     *
     * @param {grpc.ServerWriteableStream} call
     * @param {Function(Error|null, *|null)} [callback=undefined]
     *
     * @returns {Promise<void>}
     */
    async function handler(call, callback = undefined) {
      await call.sendMetadata(serverMetadata);

      const { metadata } = call;

      if (!metadata || metadata.get('protocolVersion').length === 0) {
        throw new VersionMismatchGrpcError({
          clientVersion: null,
          serverVersion: serverProtocolVersionNumber,
        });
      }

      const [clientProtocolVersionFromMeta] = metadata.get('protocolVersion');

      const clientProtocolVersionNumber = parseInt(clientProtocolVersionFromMeta, 10);
      const clientProtocolVersionString = convertInt32VersionToString(
        clientProtocolVersionNumber,
      );

      const clientVersion = semver.coerce(clientProtocolVersionString);
      const serverVersion = semver.coerce(serverProtocolVersionString);

      const majorMismatch = clientVersion.major !== serverVersion.major;
      const minorMismatch = clientVersion.minor !== serverVersion.minor;

      if (majorMismatch || minorMismatch) {
        throw new VersionMismatchGrpcError({
          clientVersion: clientProtocolVersionNumber,
          serverVersion: serverProtocolVersionNumber,
        });
      }

      return method(call, callback);
    }

    return handler;
  }

  return checkVersionWrapper;
}

module.exports = checkVersionWrapperFactory;
