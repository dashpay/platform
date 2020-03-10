const chai = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

const sinon = require('sinon');

const getIdentityHandlerFactory = require('../../../../../lib/rpcServer/commands/platform/getIdentityHandlerFactory');
const ArgumentsValidationError = require('../../../../../lib/errors/ArgumentsValidationError');
const Validator = require('../../../../../lib/utils/Validator');
const argsSchema = require('../../../../../lib/rpcServer/commands/platform/schemas/getIdentity');

chai.use(dirtyChai);
chai.use(chaiAsPromised);
const { expect } = chai;

describe('getIdentityHandlerFactory', () => {
  let tendermintRpcMock;
  let handleAbciMock;
  let validator;

  beforeEach(() => {
    tendermintRpcMock = {
      request: sinon.stub().returns({ result: { response: { value: 'identityBase64' } }, error: null }),
    };

    handleAbciMock = sinon.stub();
    validator = new Validator(argsSchema);
  });

  it('should call the right method with the correct args', async () => {
    const getIdentity = getIdentityHandlerFactory(tendermintRpcMock, handleAbciMock, validator);
    const testId = '2UErKUaV3rPBbvjbMdEkjTGNyuVKpdtHQ3KoDyoogzR7';

    const res = await getIdentity({ id: testId });

    expect(res).to.be.deep.equal({ identity: 'identityBase64' });
    expect(tendermintRpcMock.request.calledOnce).to.be.true();
    expect(tendermintRpcMock.request.calledWithExactly('abci_query', {
      path: '/identity',
      data: Buffer.from(testId).toString('hex'),
    })).to.be.true();
  });

  it("should throw an error if args don't match the arg schema", async () => {
    const getIdentity = getIdentityHandlerFactory(tendermintRpcMock, handleAbciMock, validator);

    await expect(getIdentity(1)).to.be.rejectedWith('params should be object');
    try {
      await getIdentity({ id: '123' });
    } catch (e) {
      expect(e).to.be.instanceOf(ArgumentsValidationError);
      expect(e.message).to.be.equal('params.id should NOT be shorter than 42 characters');
    }
  });

  it('should throw an error if machine returned error', async () => {
    const error = { code: -1, message: 'Machine returned an error', data: 'Some data' };
    tendermintRpcMock.request.returns({ result: null, error });
    const getIdentity = getIdentityHandlerFactory(tendermintRpcMock, handleAbciMock, validator);
    const testId = '2UErKUaV3rPBbvjbMdEkjTGNyuVKpdtHQ3KoDyoogzR7';

    try {
      await getIdentity({ id: testId });
    } catch (e) {
      expect(e).to.be.an.instanceOf(Error);
      expect(e.message).to.equal(error.message);
      expect(e.data).to.equal(error.data);
      expect(e.code).to.equal(error.code);
    }
  });
});
