const {
  tendermint: {
    abci: {
      ValidatorSetUpdate,
    },
  },
} = require('@dashevo/abci/types');
const { expect } = require('chai');

const createValidatorSetUpdate = require('../../../../../lib/abci/handlers/validator/createValidatorSetUpdate');

describe('createValidatorSetUpdate', () => {
  let validatorSetMock;
  let validatorMock;
  let quorumHash;
  let quorumPublicKey;

  beforeEach(function beforeEach() {
    validatorMock = {
      getPublicKeyShare: this.sinon.stub(),
      getVotingPower: this.sinon.stub(),
      getProTxHash: this.sinon.stub(),
    };

    validatorMock.getVotingPower.returns(Buffer.alloc(2, 32));
    validatorMock.getProTxHash.returns(Buffer.alloc(3, 32));

    validatorSetMock = {
      getValidators: this.sinon.stub(),
      getQuorum: this.sinon.stub(),
    };

    quorumHash = Buffer.alloc(1, 32).toString('hex');
    quorumPublicKey = 'a7e75af9dd4d868a41ad2f5a5b021d653e31084261724fb40ae2f1b1c31c778d3b9464502d599cf6720723ec5c68b59d';

    validatorMock.getPublicKeyShare.returns(
      Buffer.from(quorumPublicKey, 'hex'),
    );

    validatorSetMock.getValidators.returns([validatorMock]);
    validatorSetMock.getQuorum.returns({
      quorumHash,
      quorumPublicKey,
    });
  });

  it('should create ValidatorSetUpdate object from specified ValidatorSet instance', () => {
    const result = createValidatorSetUpdate(validatorSetMock);

    expect(result).to.be.an.instanceOf(ValidatorSetUpdate);

    // TODO: check something else?
  });
});
