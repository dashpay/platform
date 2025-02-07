/**
 * @returns {{
 * height: bigint,
 * coreChainLockedHeight: string,
 * timeMs: string,
 * protocolVersion: number,
 * }}
 */
function getMetadataFixture() {
  return {
    height: BigInt(10),
    coreChainLockedHeight: '42',
    timeMs: new Date().getTime().toString(),
    protocolVersion: 1,
  };
}

module.exports = getMetadataFixture;
