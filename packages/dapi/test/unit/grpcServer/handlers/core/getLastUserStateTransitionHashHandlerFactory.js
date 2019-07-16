const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getLastUserStateTransitionHashHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/core/getLastUserStateTransitionHashHandlerFactory',
);

const InvalidArgumentGrpcError = require('../../../../../lib/grpcServer/error/InvalidArgumentGrpcError');

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);

describe('getLastUserStateTransitionHashHandlerFactory', () => {
  let coreAPIMock;
  let getLastUserStateTransitionHashHandler;
  let userId;
  let subTxs;
  let call;

  beforeEach(function beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.createSandbox();
    } else {
      this.sinon.restore();
    }
  });

  afterEach(function afterEach() {
    this.sinon.restore();
  });

  beforeEach(function beforeEach() {
    userId = Buffer.alloc(256, 1);
    subTxs = [
      '6f6e65',
      '6f6e66',
      '6f6e67',
    ];

    call = new GrpcCallMock(this.sinon, {
      getUserId_asU8: () => new Uint8Array(userId),
    });

    coreAPIMock = {
      getUser: this.sinon.stub(),
    };

    getLastUserStateTransitionHashHandler = getLastUserStateTransitionHashHandlerFactory(
      coreAPIMock,
    );
  });

  it('should throw an error if userId is not specified', async function it() {
    call = new GrpcCallMock(this.sinon, {
      getUserId_asU8: () => new Uint8Array([]),
    });

    coreAPIMock.getUser.resolves(undefined);

    try {
      await getLastUserStateTransitionHashHandler(call);

      expect.fail('Error was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('Invalid argument: userId is not specified');
    }
  });

  it('should throw an error if user was not found', async () => {
    const userNotFoundError = new Error('no user mate');

    coreAPIMock.getUser.throws(userNotFoundError);

    try {
      await getLastUserStateTransitionHashHandler(call);

      expect.fail('Error was not thrown');
    } catch (e) {
      expect(coreAPIMock.getUser).to.have.been.calledOnceWith(userId.toString('hex'));
      expect(e).to.be.an.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal(
        `Invalid argument: Could not retrieve user by id ${userId.toString('hex')}. Reason: ${userNotFoundError.message}`,
      );
    }
  });

  it('should return empty state transition hash in case no state transitions exist', async () => {
    coreAPIMock.getUser.resolves({
      subtx: [],
    });

    const response = await getLastUserStateTransitionHashHandler(call);

    expect(response.getStateTransitionHash()).to.equal('');
  });

  it('should return last state transitions hash', async () => {
    coreAPIMock.getUser.resolves({
      subtx: subTxs,
    });

    const response = await getLastUserStateTransitionHashHandler(call);

    expect(response.getStateTransitionHash()).to.deep.equal(
      Buffer.from(subTxs[subTxs.length - 1], 'hex'),
    );
  });
});
