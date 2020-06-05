const cbor = require('cbor');
const {
  PlatformPromiseClient,
  GetDocumentsRequest,
  GetDocumentsResponse,
} = require('@dashevo/dapi-grpc');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const getDocumentsFactory = require('../../../../lib/methods/platform/getDocumentsFactory');

describe('getDocumentsFactory', () => {
  let grpcTransportMock;
  let getDocuments;
  let options;
  let contractId;
  let type;
  let documentsFixture;
  let serializedDocuments;

  beforeEach(function beforeEach() {
    type = 'niceDocument';
    contractId = '11c70af56a763b05943888fa3719ef56b3e826615fdda2d463c63f4034cb861c';

    options = {
      limit: 10,
      orderBy: [
        ['order', 'asc'],
      ],
      startAt: 1,
      where: [['lastName', '==', 'unknown']],
      startAfter: 10,
    };

    documentsFixture = getDocumentsFixture();
    serializedDocuments = documentsFixture
      .map((document) => Buffer.from(JSON.stringify(document)));

    const response = new GetDocumentsResponse();
    response.setDocumentsList(serializedDocuments);

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    getDocuments = getDocumentsFactory(grpcTransportMock);
  });

  it('should return documents', async () => {
    const result = await getDocuments(contractId, type, options);

    const request = new GetDocumentsRequest();
    request.setDataContractId(contractId);
    request.setDocumentType(type);
    request.setLimit(options.limit);
    request.setWhere(cbor.encode(options.where));
    request.setOrderBy(cbor.encode(options.orderBy));
    request.setStartAfter(options.startAfter);
    request.setStartAt(options.startAt);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getDocuments',
      request,
      options,
    );
    expect(result).to.deep.equal(serializedDocuments);
  });
});
