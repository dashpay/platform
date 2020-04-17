const bs58 = require('bs58');

const createDomainDataTrigger = require('../../../../lib/dataTrigger/dpnsTriggers/createDomainDataTrigger');

const DataTriggerExecutionContext = require('../../../../lib/dataTrigger/DataTriggerExecutionContext');
const DataTriggerExecutionResult = require('../../../../lib/dataTrigger/DataTriggerExecutionResult');

const { getParentDocumentFixture, getChildDocumentFixture } = require('../../../../lib/test/fixtures/getDpnsDocumentFixture');
const getPreorderDocumentFixture = require('../../../../lib/test/fixtures/getPreorderDocumentFixture');
const getDpnsContractFixture = require('../../../../lib/test/fixtures/getDpnsContractFixture');
const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');

const multihash = require('../../../../lib/util/multihashDoubleSHA256');

const DataTriggerConditionError = require('../../../../lib/errors/DataTriggerConditionError');

describe('createDomainDataTrigger', () => {
  let parentDocument;
  let childDocument;
  let preorderDocument;
  let context;
  let stateRepositoryMock;
  let dataContract;
  let topLevelIdentity;

  beforeEach(function beforeEach() {
    dataContract = getDpnsContractFixture();

    parentDocument = getParentDocumentFixture();
    childDocument = getChildDocumentFixture();
    preorderDocument = getPreorderDocumentFixture();

    const {
      preorderSalt, nameHash, records, normalizedParentDomainName,
    } = childDocument.getData();

    const parentDomainHash = multihash.hash(
      Buffer.from(normalizedParentDomainName),
    ).toString('hex');

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDocuments.resolves([]);
    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContract.getId(),
        childDocument.getType(),
        { where: [['nameHash', '==', parentDomainHash]] },
      )
      .resolves([parentDocument.toJSON()]);

    const saltedDomainHashBuffer = Buffer.concat([
      bs58.decode(preorderSalt),
      Buffer.from(nameHash, 'hex'),
    ]);

    const saltedDomainHash = multihash.hash(
      saltedDomainHashBuffer,
    ).toString('hex');

    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContract.getId(),
        'preorder',
        { where: [['saltedDomainHash', '==', saltedDomainHash]] },
      )
      .resolves([preorderDocument.toJSON()]);

    stateRepositoryMock.fetchTransaction.resolves(null);

    stateRepositoryMock.fetchTransaction
      .withArgs(
        records.dashIdentity,
      )
      .resolves({ confirmations: 10 });

    context = new DataTriggerExecutionContext(
      stateRepositoryMock,
      records.dashIdentity,
      dataContract,
    );

    topLevelIdentity = context.getOwnerId();
  });

  it('should successfully execute if document is valid', async () => {
    const result = await createDomainDataTrigger(childDocument, context, topLevelIdentity);

    expect(result.isOk()).to.be.true();
  });

  it('should fail with invalid hash', async () => {
    childDocument = getChildDocumentFixture({
      nameHash: multihash.hash(Buffer.from('invalidHash')).toString('hex'),
    });
    stateRepositoryMock.fetchTransaction
      .withArgs(
        childDocument.getData().records.dashIdentity,
      )
      .resolves({ confirmations: 10 });


    const result = await createDomainDataTrigger(childDocument, context, topLevelIdentity);

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal('Document nameHash doesn\'t match actual hash');
  });

  it('should fail with invalid normalizedLabel', async () => {
    childDocument = getChildDocumentFixture({ normalizedLabel: childDocument.getData().label });
    stateRepositoryMock.fetchTransaction
      .withArgs(
        childDocument.getData().records.dashIdentity,
      )
      .resolves({ confirmations: 10 });

    const result = await createDomainDataTrigger(childDocument, context, topLevelIdentity);

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal('Normalized label doesn\'t match label');
  });

  it('should fail with invalid parent domain', async () => {
    childDocument = getChildDocumentFixture({
      label: 'label',
      normalizedLabel: 'label',
      nameHash: multihash.hash(Buffer.from('label.invalidname')).toString('hex'),
      normalizedParentDomainName: 'invalidname',
    });

    stateRepositoryMock.fetchTransaction
      .withArgs(
        childDocument.getData().records.dashIdentity,
      )
      .resolves({ confirmations: 10 });

    const result = await createDomainDataTrigger(childDocument, context, topLevelIdentity);

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal('Can\'t find parent domain matching parent hash');
  });

  it('should fail with invalid ownerId', async () => {
    childDocument = getChildDocumentFixture({
      records: {
        dashIdentity: 'invalidHash',
      },
    });

    const result = await createDomainDataTrigger(childDocument, context, topLevelIdentity);

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal('ownerId doesn\'t match dashIdentity');
  });

  it('should fail with preorder document was not found', async () => {
    childDocument = getChildDocumentFixture({
      preorderSalt: bs58.encode(Buffer.alloc(256, '012fd')),
    });

    const result = await createDomainDataTrigger(childDocument, context, topLevelIdentity);

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal('preorderDocument was not found');
  });

  it('should fail with hash not being a valid multihash', async () => {
    childDocument = getChildDocumentFixture({
      nameHash: '01',
    });

    const result = await createDomainDataTrigger(childDocument, context, topLevelIdentity);

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal('nameHash is not a valid multihash');
  });

  it('should fail with invalid full domain name length', async () => {
    childDocument = getChildDocumentFixture({
      normalizedParentDomainName: Buffer.alloc(512).toString('hex'),
    });

    const result = await createDomainDataTrigger(childDocument, context, topLevelIdentity);

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal(
      'Full domain name length can not be more than 253 characters long',
    );
  });

  it('should fail with normalizedParentDomainName not being lower case', async () => {
    childDocument = getChildDocumentFixture({
      nameHash: multihash.hash(Buffer.from('child.Parent.domain')).toString('hex'),
      normalizedParentDomainName: 'Parent.domain',
    });

    const result = await createDomainDataTrigger(childDocument, context, topLevelIdentity);

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal(
      'Parent domain name is not normalized (e.g. contains non-lowercase letter)',
    );
  });

  it('should fail with identity can\'t create top level domain', async () => {
    parentDocument.data.normalizedParentDomainName = '';
    parentDocument.data.nameHash = multihash.hash(Buffer.from('parent')).toString('hex');

    topLevelIdentity = 'someIdentity';

    const result = await createDomainDataTrigger(parentDocument, context, topLevelIdentity);

    expect(result).to.be.an.instanceOf(DataTriggerExecutionResult);
    expect(result.isOk()).to.be.false();

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(DataTriggerConditionError);
    expect(error.message).to.equal(
      'Can\'t create top level domain for this identity',
    );
  });
});
