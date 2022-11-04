/**
 * @returns {{
 * coreChainLockedHeight: number,
 * height: number,
 * signature: Buffer,
 * protocolVersion: Long,
 * blockTime: ITimestamp,
 * }}
 */
function getMetadataFixture() {
  return {
    height: 10,
    coreChainLockedHeight: 42,
    signature: Buffer.alloc(12).fill(2).toString('base64'),
    blockTime: {
      seconds: Math.ceil(new Date().getTime() / 1000),
      nanos: 0,
    },
    protocolVersion: 1,
  };
}

module.exports = getMetadataFixture;
