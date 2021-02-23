const cbor = require('cbor');

const { startMongoDb } = require('@dashevo/dp-services-ctl');

const {
  asValue,
} = require('awilix');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const createTestDIContainer = require('../../../../lib/test/createTestDIContainer');

const InvalidArgumentAbciError = require('../../../../lib/abci/errors/InvalidArgumentAbciError');

describe('queryHandlerFactory', function main() {
  this.timeout(25000);

  let container;
  let mongoDB;
  let queryHandler;
  let identityQueryHandlerMock;
  let dataContractQueryHandlerMock;
  let documentQueryHandlerMock;
  let dataContract;
  let documents;
  let identity;
  let proof;

  before(async () => {
    mongoDB = await startMongoDb();
  });

  after(async () => {
    await mongoDB.remove();
  });

  beforeEach(function beforeEach() {
    proof = Buffer.from('843176bc004504d6baf735cf0215e9d9a3fecf1d', 'hex');

    container = createTestDIContainer(mongoDB);

    dataContract = getDataContractFixture();
    documents = getDocumentsFixture(dataContract);
    identity = getIdentityFixture();

    identityQueryHandlerMock = this.sinon.stub();
    identityQueryHandlerMock.resolves({
      value: identity,
      proof,
    });

    dataContractQueryHandlerMock = this.sinon.stub();
    dataContractQueryHandlerMock.resolves(dataContract);

    documentQueryHandlerMock = this.sinon.stub();
    documentQueryHandlerMock.resolves(documents);

    container.register('identityQueryHandler', asValue(identityQueryHandlerMock));
    container.register('dataContractQueryHandler', asValue(dataContractQueryHandlerMock));
    container.register('documentQueryHandler', asValue(documentQueryHandlerMock));

    queryHandler = container.resolve('queryHandler');
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  describe('/identities', () => {
    it('should call identity handler and return an identity with proof', async () => {
      const result = await queryHandler({
        path: '/identities',
        data: cbor.encode({
          id: 1,
        }),
        prove: 'true',
      });

      expect(identityQueryHandlerMock).to.have.been.calledOnceWithExactly(
        {},
        { id: 1 },
        {
          path: '/identities',
          data: cbor.encode({
            id: 1,
          }),
          prove: 'true',
        },
      );

      expect(result).to.deep.equal({
        value: identity,
        proof,
      });
    });
  });

  describe('/dataContracts', () => {
    it('should call data contract handler and return data contract', async () => {
      const result = await queryHandler({
        path: '/dataContracts',
        data: cbor.encode({
          id: 1,
        }),
      });

      expect(dataContractQueryHandlerMock).to.have.been.calledOnceWithExactly(
        {},
        { id: 1 },
        {
          path: '/dataContracts',
          data: cbor.encode({
            id: 1,
          }),
        },
      );
      expect(result).to.deep.equal(dataContract);
    });
  });

  describe('/dataContracts/documents', () => {
    it('should call documents handler and return documents', async () => {
      const result = await queryHandler({
        path: '/dataContracts/documents',
        data: cbor.encode({
          contractId: 1,
          type: 'someType',
        }),
      });

      expect(documentQueryHandlerMock).to.have.been.calledOnceWithExactly(
        {},
        { contractId: 1, type: 'someType' },
        {
          path: '/dataContracts/documents',
          data: cbor.encode({
            contractId: 1,
            type: 'someType',
          }),
        },
      );
      expect(result).to.deep.equal(documents);
    });
  });

  it('should throw an error if invalid path is submitted', async () => {
    try {
      await queryHandler({
        path: '/unknownPath',
        data: Buffer.alloc(0),
      });
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidArgumentAbciError);
      expect(e.getMessage()).to.equal('Invalid path');
    }
  });
});
