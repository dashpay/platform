/**
 * @returns {{
 *   merkleProof: Buffer,
 *   signature: Buffer,
 *   signatureLLMQHash: Buffer
 * }}
 */
function getProofFixture() {
  return {
    signatureLLMQHash: Buffer.from('AQEBAQEBAQEBAQEB', 'base64'),
    signature: Buffer.from('AgICAgICAgICAgIC', 'base64'),
    merkleProof: Buffer.from('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101', 'hex'),
  };
}

module.exports = getProofFixture;
