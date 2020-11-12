const BufferWriter = require('@dashevo/dashcore-lib/lib/encoding/bufferwriter');
const { convertBitArrayToUInt8Array } = require('@dashevo/dashcore-lib/lib/util/bitarray');

/**
 *
 * @param {Array<{data: Buffer, position: string}>} rootTreeProof
 * @return {Buffer}
 */
function convertRootTreeProofToBuffer(rootTreeProof) {
  /*
   amount of subtrees to proof
   It is equal to 1 because at the moment our proof won't be a simultaneous proof for more
   than 1 tree, i.e. it's always identities OR documents OR contracts, not an AND in any case,
   so there's always only 1 leaf to prove. The libraries that perform verification need to know
   this, as this is equal to 1 in this particular case, usually it's not equal to 1.
  */
  const totalObjects = 1;

  const bufferWriter = new BufferWriter();

  bufferWriter.writeUInt32LE(totalObjects);
  bufferWriter.writeVarintNum(rootTreeProof.length);

  rootTreeProof.forEach(({ data: hash }) => {
    bufferWriter.write(hash);
  });

  // boolean array, where left position = false, right position = true
  const merkleFlagsArray = convertBitArrayToUInt8Array(
    rootTreeProof.map(({ position }) => position === 'right'),
  );

  bufferWriter.writeVarintNum(merkleFlagsArray.length);
  merkleFlagsArray.forEach((flag) => {
    bufferWriter.writeUInt8(flag);
  });

  return bufferWriter.toBuffer();
}

module.exports = convertRootTreeProofToBuffer;
