const AbstractDocumentTransition = require('../../../lib/document/stateTransition/documentTransition/AbstractDocumentTransition');

const getDataTriggersFactory = require('../../../lib/dataTrigger/getDataTriggersFactory');

const getDpnsDocumentFixture = require('../../../lib/test/fixtures/getDpnsDocumentFixture');

const DataTrigger = require('../../../lib/dataTrigger/DataTrigger');

const createDomainDataTrigger = require('../../../lib/dataTrigger/dpnsTriggers/createDomainDataTrigger');
const rejectDataTrigger = require('../../../lib/dataTrigger/dpnsTriggers/rejectDataTrigger');

const generateRandomId = require('../../../lib/test/utils/generateRandomId');

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

  beforeEach(function beforeEach() {
    createDocument = getDpnsDocumentFixture.getChildDocumentFixture();
    updateDocument = getDpnsDocumentFixture.getChildDocumentFixture();
    deleteDocument = getDpnsDocumentFixture.getChildDocumentFixture();
    deleteDocument.data = {};

    dataContractId = getDpnsDocumentFixture.dataContract.getId();
    topLevelIdentity = generateRandomId().toBuffer();

    createTrigger = new DataTrigger(
      dataContractId, 'domain', AbstractDocumentTransition.ACTIONS.CREATE, createDomainDataTrigger, topLevelIdentity,
    );
    updateTrigger = new DataTrigger(
      dataContractId, 'domain', AbstractDocumentTransition.ACTIONS.REPLACE, rejectDataTrigger, topLevelIdentity,
    );
    deleteTrigger = new DataTrigger(
      dataContractId, 'domain', AbstractDocumentTransition.ACTIONS.DELETE, rejectDataTrigger, topLevelIdentity,
    );
    updatePreorderTrigger = new DataTrigger(
      dataContractId, 'preorder', AbstractDocumentTransition.ACTIONS.REPLACE, rejectDataTrigger, topLevelIdentity,
    );
    deletePreorderTrigger = new DataTrigger(
      dataContractId, 'preorder', AbstractDocumentTransition.ACTIONS.DELETE, rejectDataTrigger, topLevelIdentity,
    );

    this.sinonSandbox.stub(process, 'env').value({
      DPNS_CONTRACT_ID: dataContractId,
      DPNS_TOP_LEVEL_IDENTITY: topLevelIdentity,
    });

    getDataTriggers = getDataTriggersFactory();
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
