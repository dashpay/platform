const Long = require('long');

const {
  tendermint: {
    abci: {
      ResponseInitChain,
      ValidatorSetUpdate,
    },
  },
} = require('@dashevo/abci/types');

const initChainHandlerFactory = require('../../../../lib/abci/handlers/initChainHandlerFactory');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');

describe('initChainHandlerFactory', () => {
  let initChainHandler;
  let updateSimplifiedMasternodeListMock;
  let initialCoreChainLockedHeight;
  let validatorSetMock;
  let createValidatorSetUpdateMock;
  let loggerMock;
  let validatorSetUpdate;

  beforeEach(function beforeEach() {
    initialCoreChainLockedHeight = 1;

    updateSimplifiedMasternodeListMock = this.sinon.stub();

    const quorumHash = Buffer.alloc(64).fill(1).toString('hex');
    validatorSetMock = {
      initialize: this.sinon.stub(),
      getQuorum: this.sinon.stub().returns({
        quorumHash,
      }),
    };

    validatorSetUpdate = new ValidatorSetUpdate();

    createValidatorSetUpdateMock = this.sinon.stub().returns(validatorSetUpdate);

    loggerMock = new LoggerMock(this.sinon);

    initChainHandler = initChainHandlerFactory(
      updateSimplifiedMasternodeListMock,
      initialCoreChainLockedHeight,
      validatorSetMock,
      createValidatorSetUpdateMock,
      loggerMock,
    );
  });

  it('should update height, start transactions and return ResponseBeginBlock', async () => {
    const request = {
      initialHeight: Long.fromInt(1),
      chainId: 'test',
    };

    const response = await initChainHandler(request);

    expect(response).to.be.an.instanceOf(ResponseInitChain);
    expect(response.validatorSetUpdate).to.be.equal(validatorSetUpdate);

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      initialCoreChainLockedHeight,
      {
        logger: loggerMock,
      },
    );

    expect(validatorSetMock.initialize).to.be.calledOnceWithExactly(
      initialCoreChainLockedHeight,
    );

    expect(createValidatorSetUpdateMock).to.be.calledOnceWithExactly(validatorSetMock);
  });
});
