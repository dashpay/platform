const {
  tendermint: {
    abci: {
      ValidatorUpdate,
      ValidatorSetUpdate,
    },
    crypto: {
      PublicKey,
    },
  },
} = require('@dashevo/abci/types');

/**
 * @typedef {createValidatorSetUpdate}
 * @param {ValidatorSet} validatorSet
 * @return {ValidatorSetUpdate}
 */
function createValidatorSetUpdate(validatorSet) {
  const validatorUpdates = validatorSet.getValidators()
    .map((validator) => {
      const validatorUpdate = new ValidatorUpdate({
        power: validator.getVotingPower(),
        proTxHash: validator.getProTxHash(),
      });

      if (validator.getPublicKeyShare()) {
        validatorUpdate.pubKey = new PublicKey({
          bls12381: validator.getPublicKeyShare(),
        });
      }

      return validatorUpdate;
    });

  const { quorumPublicKey, quorumHash } = validatorSet.getQuorum();

  return new ValidatorSetUpdate({
    validatorUpdates,
    thresholdPublicKey: new PublicKey({
      bls12381: Buffer.from(quorumPublicKey, 'hex'),
    }),
    quorumHash: Buffer.from(quorumHash, 'hex'),
  });
}

module.exports = createValidatorSetUpdate;
