module.exports = {
  latestVersion: 0,
  // Even if we bumping protocol version, previous versions of entity structures
  // can be still compatible, that allow to not update clients so often.
  //
  // Minimum compatible versions must be defined for all protocol versions:
  // [protocolVersion]: [minimumCompatibleProtocolVersions]
  compatibility: {
    0: 0,
  },
};
