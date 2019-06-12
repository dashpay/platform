const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getLastStateTransitionHashHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/core/getLastUserStateTransitionHashHandlerFactory',
);

const InvalidArgumentError = require('../../../../../lib/grpcServer/error/InvalidArgumentError');

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);

describe('getLastStateTransitionHashHandlerFactory', () => {
  let coreAPIMock;
  let getLastStateTransitionHashHandler;
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
    userId = Buffer.alloc(256);
    subTxs = [
      '6f6e65',
      '6f6e66',
      '6f6e67',
    ];

    call = new GrpcCallMock(this.sinon, {
      userId,
    });

    coreAPIMock = {
      getUser: this.sinon.stub(),
    };

    getLastStateTransitionHashHandler = getLastStateTransitionHashHandlerFactory(
      coreAPIMock,
    );
  });

  it('should throw an error if userId is not of correct length', function it(done) {
    call = new GrpcCallMock(this.sinon, {
      userId: Buffer.from('SomeOtherId'),
    });

    const callback = (e, v) => {
      try {
        expect(v).to.equal(null);
        expect(e).to.be.an.instanceOf(InvalidArgumentError);
        expect(e.getMessage()).to.equal('Invalid argument: userId length is not 256 bytes');

        done();
      } catch (error) {
        done(error);
      }
    };

    coreAPIMock.getUser.resolves(undefined);

    getLastStateTransitionHashHandler(call, callback);
  });

  it('should throw an error if user was not found', (done) => {
    const callback = (e, v) => {
      try {
        expect(coreAPIMock.getUser).to.have.been.calledOnceWith(userId.toString('hex'));

        expect(v).to.equal(null);
        expect(e).to.be.an.instanceOf(InvalidArgumentError);
        expect(e.getMessage()).to.equal(`Invalid argument: User was not found by id ${userId.toString('hex')}`);

        done();
      } catch (error) {
        done(error);
      }
    };

    coreAPIMock.getUser.resolves(undefined);

    getLastStateTransitionHashHandler(call, callback);
  });

  it('should throw-forward an error if core API call goes wrong', function it() {
    const callback = this.sinon.stub();

    const anError = new Error('Core API goes nuts');

    coreAPIMock.getUser.throws(anError);

    try {
      getLastStateTransitionHashHandler(call, callback);
      expect.fail('An error have not been thrown');
    } catch (e) {
      expect(e.message).to.equal(anError.message);
    }

    expect(callback).to.not.have.been.called();
  });

  it('should return empty state transition hash in case no state transitions exist', (done) => {
    const callback = (e, v) => {
      try {
        expect(e).to.equal(null);
        expect(v.getStateTransitionHash()).to.equal('');
        done();
      } catch (error) {
        done(error);
      }
    };

    coreAPIMock.getUser.resolves({
      subtx: [],
    });

    getLastStateTransitionHashHandler(call, callback);
  });

  it('should return last state transitions hash', (done) => {
    const callback = (e, v) => {
      try {
        expect(e).to.equal(null);
        expect(v.getStateTransitionHash()).to.deep.equal(
          Buffer.from(subTxs[subTxs.length - 1], 'hex'),
        );
        done();
      } catch (error) {
        done(error);
      }
    };

    coreAPIMock.getUser.resolves({
      subtx: subTxs,
    });

    getLastStateTransitionHashHandler(call, callback);
  });
});
