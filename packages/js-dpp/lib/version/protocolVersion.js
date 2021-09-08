module.exports = {
  latestVersion: 1,
  // Even if we bumping protocol version, previous versions of entity structures
  // can be still compatible, that allow to not update clients so often.
  //
  // Minimum compatible versions must be defined for all protocol versions:
  // [protocolVersion]: [minimumCompatibleProtocolVersions]
  compatibility: {
    1: 1,
  },
};
