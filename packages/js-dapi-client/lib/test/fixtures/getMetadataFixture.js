/**
 * @returns {{coreChainLockedHeight: number, height: number}}
 */
function getMetadataFixture() {
  return {
    height: 10,
    coreChainLockedHeight: 42,
  };
}

module.exports = getMetadataFixture;
