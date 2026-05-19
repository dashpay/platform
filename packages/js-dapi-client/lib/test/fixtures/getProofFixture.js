import { hexToBytes, base64ToBytes } from '../../utils/bytes.js';

/**
 * @returns {{
 *   merkleProof: Uint8Array,
 *   signature: Uint8Array,
 *   quorumHash: Uint8Array
 * }}
 */
function getProofFixture() {
  return {
    quorumHash: base64ToBytes('AQEBAQEBAQEBAQEB'),
    signature: base64ToBytes('AgICAgICAgICAgIC'),
    merkleProof: hexToBytes('0100000001f0faf5f55674905a68eba1be2f946e667c1cb5010101'),
    round: 42,
  };
}

export default getProofFixture;
