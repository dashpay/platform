import { expect } from 'chai';

import loadDpp from '@dashevo/wasm-dpp';

import getDataContractFixture from '@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture';

import getResponseMetadataFixture from '../../../../../test/fixtures/getResponseMetadataFixture';
import history from './history';
import identitiesFixtures from '../../../../../../tests/fixtures/identities.json';
import 'mocha';
import { ClientApps } from '../../../ClientApps';

const GetDataContractHistoryResponse = require('@dashevo/dapi-client/lib/methods/platform/getDataContractHistory/GetDataContractHistoryResponse');
const NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');

let client;
let fetcher;
let askedFromDapi;
let initialize;
let metadataFixture;
let dataContractFixture;

const factory = {
  createFromBuffer: () => dataContractFixture,
};

const dpp = {
  dataContract: factory,
  getProtocolVersion: () => 42,
};

const logger = {
  debug: () => {},
  silly: () => {},
};

let apps;

describe('Client - Platform - Contracts - .history()', () => {
  before(async function before() {
    await loadDpp();

    dataContractFixture = await getDataContractFixture();
    metadataFixture = getResponseMetadataFixture();

    apps = new ClientApps({
      ratePlatform: {
        contractId: dataContractFixture.getId(),
      },
    });

    askedFromDapi = 0;
    const fetchDataContractHistory = async (id) => {
      const fixtureIdentifier = dataContractFixture.getId();
      askedFromDapi += 1;

      if (id.equals(fixtureIdentifier)) {
        return new GetDataContractHistoryResponse(
          { 1000: dataContractFixture.toBuffer() },
          metadataFixture,
        );
      }

      throw new NotFoundError();
    };

    fetcher = {
      fetchDataContractHistory,
    };

    client = {
      getApps(): ClientApps {
        return apps;
      },
    };

    initialize = this.sinon.stub();
  });

  describe('get a contract from string', () => {
    it('should get from DAPIClient if there is none locally', async () => {
      const contractHistory = await history.call({
        // @ts-ignore
        apps, dpp, client, initialize, logger, fetcher,
      }, dataContractFixture.getId(), 0, 10, 0);
      const contract = contractHistory[1000];
      expect(contract.toJSON()).to.deep.equal(dataContractFixture.toJSON());
      expect(askedFromDapi).to.equal(1);
    });

    it('should get from local when already fetched once', async () => {
      const contractHistory = await history.call({
        // @ts-ignore
        apps, dpp, client, initialize, logger, fetcher,
      }, dataContractFixture.getId(), 0, 10, 0);
      const contract = contractHistory[1000];
      expect(contract.toJSON()).to.deep.equal(dataContractFixture.toJSON());
      expect(askedFromDapi).to.equal(2);
    });
  });

  describe('other conditions', () => {
    it('should deal when contract do not exist', async () => {
      const contract = await history.call({
        // @ts-ignore
        apps, dpp, client, initialize, logger, fetcher,
      }, identitiesFixtures.bob.id, 0, 10, 0);
      expect(contract).to.equal(null);
    });
  });
});
