/**
 * @returns {{coreChainLockedHeight: number, height: number}}
 */
function getMetadataFixture() {
  return {
    height: 10,
    coreChainLockedHeight: 42,
    // signature:
    // time:
    // protocolVersion:
  };
}

module.exports = getMetadataFixture;
