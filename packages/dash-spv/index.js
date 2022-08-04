const spvChain = require('./lib/spvchain');
const merkleProof = require('./lib/merkleproofs');
const genesis = require('./config/config');

module.exports = {
  SpvChain: spvChain,
  MerkleProof: merkleProof,
  genesis,
};
