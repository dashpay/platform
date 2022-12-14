const spvChain = require('./lib/spvchain');
const merkleProof = require('./lib/merkleproofs');
const genesis = require('./config/config');
const SPVError = require('./lib/errors/SPVError');

module.exports = {
  SpvChain: spvChain,
  MerkleProof: merkleProof,
  genesis,
  SPVError,
};
