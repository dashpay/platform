const {
  contractId: dpnsContractId,
  ownerId: dpnsOwnerId,
} = require('@dashevo/dpns-contract/lib/systemIds');

const AbstractDocumentTransition = require('../../../lib/document/stateTransition/DocumentsBatchTransition/documentTransition/AbstractDocumentTransition');

const getDataTriggersFactory = require('../../../lib/dataTrigger/getDataTriggersFactory');

const getDpnsDocumentFixture = require('../../../lib/test/fixtures/getDpnsDocumentFixture');

const DataTrigger = require('../../../lib/dataTrigger/DataTrigger');

const createDomainDataTrigger = require('../../../lib/dataTrigger/dpnsTriggers/createDomainDataTrigger');
const rejectDataTrigger = require('../../../lib/dataTrigger/rejectDataTrigger');
const Identifier = require('../../../lib/identifier/Identifier');

describe('getDataTriggers', () => {
  let getDataTriggers;

  let createDocument;
  let updateDocument;
  let deleteDocument;

  let createTrigger;
  let updateTrigger;
  let deleteTrigger;

  let updatePreorderTrigger;
  let deletePreorderTrigger;

  let dataContractId;
  let topLevelIdentity;

  let processMock;

  beforeEach(function beforeEach() {
    createDocument = getDpnsDocumentFixture.getChildDocumentFixture();
    updateDocument = getDpnsDocumentFixture.getChildDocumentFixture();
    deleteDocument = getDpnsDocumentFixture.getChildDocumentFixture();
    deleteDocument.data = {};

    dataContractId = Identifier.from(dpnsContractId);
    topLevelIdentity = Identifier.from(dpnsOwnerId);

    createTrigger = new DataTrigger(
      dataContractId, 'domain', AbstractDocumentTransition.ACTIONS.CREATE, createDomainDataTrigger, topLevelIdentity,
    );
    updateTrigger = new DataTrigger(
      dataContractId, 'domain', AbstractDocumentTransition.ACTIONS.REPLACE, rejectDataTrigger,
    );
    deleteTrigger = new DataTrigger(
      dataContractId, 'domain', AbstractDocumentTransition.ACTIONS.DELETE, rejectDataTrigger,
    );
    updatePreorderTrigger = new DataTrigger(
      dataContractId, 'preorder', AbstractDocumentTransition.ACTIONS.REPLACE, rejectDataTrigger,
    );
    deletePreorderTrigger = new DataTrigger(
      dataContractId, 'preorder', AbstractDocumentTransition.ACTIONS.DELETE, rejectDataTrigger,
    );

    processMock = this.sinonSandbox.stub(process, 'env').value({
      DPNS_CONTRACT_ID: dataContractId,
      DPNS_TOP_LEVEL_IDENTITY: topLevelIdentity,
    });

    getDataTriggers = getDataTriggersFactory();
  });

  afterEach(() => {
    processMock.restore();
  });

  it('should return matching triggers', () => {
    let result = getDataTriggers(
      dataContractId, createDocument.getType(), AbstractDocumentTransition.ACTIONS.CREATE,
    );

    expect(result).to.deep.equal([createTrigger]);

    result = getDataTriggers(
      dataContractId, updateDocument.getType(), AbstractDocumentTransition.ACTIONS.REPLACE,
    );

    expect(result).to.deep.equal([updateTrigger]);

    result = getDataTriggers(
      dataContractId, deleteDocument.getType(), AbstractDocumentTransition.ACTIONS.DELETE,
    );

    expect(result).to.deep.equal([deleteTrigger]);

    result = getDataTriggers(
      dataContractId, 'preorder', AbstractDocumentTransition.ACTIONS.REPLACE,
    );

    expect(result).to.deep.equal([updatePreorderTrigger]);

    result = getDataTriggers(
      dataContractId, 'preorder', AbstractDocumentTransition.ACTIONS.DELETE,
    );

    expect(result).to.deep.equal([deletePreorderTrigger]);
  });

  it('should return empty trigger array for any other type except `domain`', () => {
    const result = getDataTriggers(
      dataContractId, 'otherType', AbstractDocumentTransition.ACTIONS.CREATE,
    );

    expect(result).to.deep.equal([]);
  });
});
