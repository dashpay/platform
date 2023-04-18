/**
 * @returns {{
 * coreChainLockedHeight: number,
 * height: number,
 * signature: Buffer,
 * protocolVersion: number,
 * timeMs: number,
 * }}
 */
function getMetadataFixture() {
  return {
    height: 10,
    coreChainLockedHeight: 42,
    timeMs: new Date().getTime(),
    protocolVersion: 1,
  };
}

module.exports = getMetadataFixture;
