const chai = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

const sinon = require('sinon');

const ArgumentsValidationError = require('../../../../../lib/errors/ArgumentsValidationError');
const Validator = require('../../../../../lib/utils/Validator');
const argsSchema = require('../../../../../lib/rpcServer/commands/platform/schemas/getDataContract');

chai.use(dirtyChai);
chai.use(chaiAsPromised);
const { expect } = chai;

const getDataContractHandlerFactory = require('../../../../../lib/rpcServer/commands/platform/getDataContractHandlerFactory');

describe('getDataContractHandlerFactory', () => {
  let driveAdapterMock;
  let dppMock;
  let validator;

  beforeEach(() => {
    driveAdapterMock = {
      fetchContract: sinon.stub(),
    };
    dppMock = {
      dataContract: {
        createFromObject: sinon.stub().returns({ serialize() { return Buffer.from('ff', 'hex'); } }),
      },
    };
    validator = new Validator(argsSchema);
  });

  it('should call the right method with the correct args', async () => {
    const getDataContractHandler = getDataContractHandlerFactory(
      driveAdapterMock, dppMock, validator,
    );
    const testId = '2UErKUaV3rPBbvjbMdEkjTGNyuVKpdtHQ3KoDyoogzR7';

    const res = await getDataContractHandler({ id: testId });

    expect(res).to.be.deep.equal({ dataContract: '/w==' });
    expect(driveAdapterMock.fetchContract.calledOnce).to.be.true();
    expect(driveAdapterMock.fetchContract.calledWithExactly(testId)).to.be.true();
  });

  it("should throw an error if args don't match the arg schema", async () => {
    const getDataContractHandler = getDataContractHandlerFactory(
      driveAdapterMock, dppMock, validator,
    );

    await expect(getDataContractHandler(1)).to.be.rejectedWith('params should be object');
    try {
      await getDataContractHandler({ id: '123' });
    } catch (e) {
      expect(e).to.be.instanceOf(ArgumentsValidationError);
      expect(e.message).to.be.equal('params.id should NOT be shorter than 42 characters');
    }
  });

  it('should throw an error if drive return an error', async () => {
    const testId = '2UErKUaV3rPBbvjbMdEkjTGNyuVKpdtHQ3KoDyoogzR7';

    driveAdapterMock.fetchContract.throws(new Error('Something went wrong with drive'));
    const getDataContractHandler = getDataContractHandlerFactory(
      driveAdapterMock, dppMock, validator,
    );

    await expect(getDataContractHandler({ id: testId })).to.be.rejectedWith('Something went wrong with drive');
  });
});
