import getDataContractFixture from '@dashevo/dpp/lib/test/fixtures/getDataContractFixture';
import generateRandomIdentifier from '@dashevo/dpp/lib/test/utils/generateRandomIdentifier';
import createDPPMock from '@dashevo/dpp/lib/test/mocks/createDPPMock';
import getDocumentsFixture from '@dashevo/dpp/lib/test/fixtures/getDocumentsFixture';
import getResponseMetadataFixture from '../../../../../test/fixtures/getResponseMetadataFixture';
const GetDocumentsResponse = require("@dashevo/dapi-client/lib/methods/platform/getDocuments/GetDocumentsResponse");

import get from './get';
import { expect } from 'chai';

describe('Client - Platform - Documents - .get()', () => {
  let platform;
  let dataContract;
  let appDefinition;
  let getDocumentsMock;
  let appsGetMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();

    appDefinition = {
      contractId: dataContract.getId(),
      contract: dataContract,
    };

    getDocumentsMock = this.sinon.stub().resolves(new GetDocumentsResponse([], getResponseMetadataFixture()));
    appsGetMock = this.sinon.stub().returns(appDefinition);

    platform = {
      dpp: createDPPMock(this.sinon),
      client: {
        getApps: () => ({
          has: this.sinon.stub().returns(true),
          get: appsGetMock,
        }),
        getDAPIClient: () => ({
          platform: {
            getDocuments: getDocumentsMock,
          },
        })
      },
      initialize: this.sinon.stub(),
    };
  });

  it('should convert identifier properties inside where condition', async () => {
    const id = generateRandomIdentifier();
    await get.call(platform, 'app.withByteArrays', {
      where: [
        ['identifierField', '==', id.toString()],
      ],
    });

    expect(getDocumentsMock.getCall(0).args).to.have.deep.members([
      appDefinition.contractId,
      'withByteArrays',
      {
        where: [
          ['identifierField', '==', id],
        ],
      },
    ]);
  });

  it('should convert $id and $ownerId to identifiers inside where condition', async () => {
    const id = generateRandomIdentifier();
    const ownerId = generateRandomIdentifier();

    await get.call(platform, 'app.withByteArrays', {
      where: [
        ['$id', '==', id.toString()],
        ['$ownerId', '==', ownerId.toString()],
      ],
    });

    expect(getDocumentsMock.getCall(0).args).to.have.deep.members([
      appDefinition.contractId,
      'withByteArrays',
      {
        where: [
          ['$id', '==', id],
          ['$ownerId', '==', ownerId],
        ],
      },
    ]);
  });

  it('should convert Document to identifiers inside where condition for "startAt" and "startAfter"', async () => {
    const [docA, docB] = getDocumentsFixture();

    await get.call(platform, 'app.withByteArrays', {
      startAt: docA,
      startAfter: docB,
    });

    expect(getDocumentsMock.getCall(0).args).to.have.deep.members([
      appDefinition.contractId,
      'withByteArrays',
      {
        startAt: docA.getId(),
        startAfter: docB.getId(),
      },
    ]);
  });

  it('should convert string to identifiers inside where condition for "startAt" and "startAfter"', async () => {
    const [docA, docB] = getDocumentsFixture();

    await get.call(platform, 'app.withByteArrays', {
      startAt: docA.getId().toString('base58'),
      startAfter: docB.getId().toString('base58'),
    });

    expect(getDocumentsMock.getCall(0).args).to.have.deep.members([
      appDefinition.contractId,
      'withByteArrays',
      {
        startAt: docA.getId(),
        startAfter: docB.getId(),
      },
    ]);
  });

  it('should convert nested identifier properties inside where condition if `elementMatch` is used', async () => {
    const id = generateRandomIdentifier();

    dataContract = getDataContractFixture();
    dataContract.documents.withByteArrays.properties.nestedObject = {
      type: 'object',
      properties: {
        idField: {
          type: "array",
          byteArray: true,
          contentMediaType: "application/x.dash.dpp.identifier",
          minItems: 32,
          maxItems: 32,
        },
        anotherNested: {
          type: 'object',
          properties: {
            anotherIdField: {
              type: "array",
              byteArray: true,
              contentMediaType: "application/x.dash.dpp.identifier",
              minItems: 32,
              maxItems: 32,
            },
          },
        },
      },
    };

    appDefinition = {
      contractId: dataContract.getId(),
      contract: dataContract,
    };

    appsGetMock.reset();
    appsGetMock.returns(appDefinition);

    await get.call(platform, 'app.withByteArrays', {
      where: [
        ['nestedObject', 'elementMatch', ['idField', '==', id.toString()]],
        ['nestedObject', 'elementMatch', ['anotherNested', 'elementMatch', ['anotherIdField', '==', id.toString()]]]
      ],
    });

    expect(getDocumentsMock.getCall(0).args).to.have.deep.members([
      appDefinition.contractId,
      'withByteArrays',
      {
        where: [
          ['nestedObject', 'elementMatch', ['idField', '==', id]],
          ['nestedObject', 'elementMatch', ['anotherNested', 'elementMatch', ['anotherIdField', '==', id]]]
        ],
      },
    ]);
  });
});
