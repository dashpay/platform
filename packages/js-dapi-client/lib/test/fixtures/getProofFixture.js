/**
 * @returns {{
 *   merkleProof: Buffer,
 *   signature: Buffer,
 *   quorumHash: Buffer
 * }}
 */
function getProofFixture() {
  return {
    quorumHash: Buffer.from('AQEBAQEBAQEBAQEB', 'base64'),
    signature: Buffer.from('AgICAgICAgICAgIC', 'base64'),
    merkleProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
    round: 42,
  };
}

module.exports = getProofFixture;
