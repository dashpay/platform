const BufferReader = require('@dashevo/dashcore-lib/lib/encoding/bufferreader');

module.exports = function parseRootTreeBuffer(rootTreeProofBuffer) {
  /*
   amount of subtrees to proof
   It is equal to 1 because at the moment our proof won't be a simultaneous proof for more
   than 1 tree, i.e. it's always identities OR documents OR contracts, not an AND in any case,
   so there's always only 1 leaf to prove. The libraries that perform verification need to know
   this, as this is equal to 1 in this particular case, usually it's not equal to 1.
  */
  const bufferReader = new BufferReader(rootTreeProofBuffer);

  // const totalObjectsCount = bufferReader.readUInt32LE();
  const rootTreeProofHashesCount = bufferReader.readVarintNum();

  const hashes = [];
  for (let i = 0; i < rootTreeProofHashesCount; i++) {
    hashes.push({ data: bufferReader.read(32) });
  }

  return hashes;
};
