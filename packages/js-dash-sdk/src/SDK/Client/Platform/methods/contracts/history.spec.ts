import { expect } from 'chai';

import loadDpp from '@dashevo/wasm-dpp';

import getDataContractFixture from '@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture';

import getResponseMetadataFixture from '../../../../../test/fixtures/getResponseMetadataFixture';
import history, { historyUnproved } from './history';
import identitiesFixtures from '../../../../../../tests/fixtures/identities.json';
import 'mocha';
import { ClientApps } from '../../../ClientApps';

const DataContractHistoryEntry = require('@dashevo/dapi-client/lib/methods/platform/getDataContractHistory/DataContractHistoryEntry');
const GetDataContractHistoryResponse = require('@dashevo/dapi-client/lib/methods/platform/getDataContractHistory/GetDataContractHistoryResponse');
const NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');

let client;
let fetcher;
let askedFromDapi;
let initialize;
let metadataFixture;
let dataContractFixture;
let requestedProofModes;

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
    requestedProofModes = [];
    const fetchDataContractHistory = async (id, startAtMs, limit, offset, prove) => {
      const fixtureIdentifier = dataContractFixture.getId();
      askedFromDapi += 1;
      requestedProofModes.push(prove);

      if (id.equals(fixtureIdentifier)) {
        return new GetDataContractHistoryResponse(
          prove ? null : [
            new DataContractHistoryEntry(BigInt(1000), dataContractFixture.toBuffer()),
          ],
          metadataFixture,
          prove ? {} : undefined,
        );
      }

      throw new NotFoundError();
    };

    fetcher = {
      fetchDataContractHistory,
    };

    client = {
      network: 'testnet',
      getApps(): ClientApps {
        return apps;
      },
      getPlatformProofVerifier: () => ({
        verifyDataContractHistory: async () => [{
          date: BigInt(1000),
          value: dataContractFixture.toBuffer(),
        }],
      }),
    };

    initialize = this.sinon.stub();
  });

  describe('get a contract from string', () => {
    it('should get from DAPIClient if there is none locally', async () => {
      const contractHistory = await history.call({
        // @ts-ignore
        apps, dpp, client, initialize, logger, fetcher, protocolVersion: 42,
      }, dataContractFixture.getId(), 0, 10, 0);
      const contract = contractHistory[1000];
      expect(contract.toJSON()).to.deep.equal(dataContractFixture.toJSON());
      expect(askedFromDapi).to.equal(1);
      expect(requestedProofModes).to.deep.equal([true]);
    });

    it('should get from local when already fetched once', async () => {
      const contractHistory = await history.call({
        // @ts-ignore
        apps, dpp, client, initialize, logger, fetcher, protocolVersion: 42,
      }, dataContractFixture.getId(), 0, 10, 0);
      const contract = contractHistory[1000];
      expect(contract.toJSON()).to.deep.equal(dataContractFixture.toJSON());
      expect(askedFromDapi).to.equal(2);
      expect(requestedProofModes).to.deep.equal([true, true]);
    });
  });

  describe('other conditions', () => {
    it('should deal when contract do not exist', async () => {
      const contract = await history.call({
        // @ts-ignore
        apps, dpp, client, initialize, logger, fetcher, protocolVersion: 42,
      }, identitiesFixtures.bob.id, 0, 10, 0);
      expect(contract).to.equal(null);
    });

    it('should fail before querying when no proof verifier is configured', async () => {
      const callsBefore = askedFromDapi;
      const clientWithoutVerifier = {
        ...client,
        getPlatformProofVerifier: () => undefined,
      };

      await expect(history.call({
        // @ts-ignore
        apps, dpp, client: clientWithoutVerifier, initialize, logger, fetcher, protocolVersion: 42,
      }, dataContractFixture.getId(), 0, 10, 0)).to.be.rejectedWith(
        'requires an authenticated Platform proof verifier',
      );
      expect(askedFromDapi).to.equal(callsBefore);
    });

    it('should expose endpoint-trusted history only through historyUnproved', async () => {
      const contractHistory = await historyUnproved.call({
        // @ts-ignore
        apps, dpp, client, initialize, logger, fetcher, protocolVersion: 42,
      }, dataContractFixture.getId(), 0, 10, 0);

      expect(contractHistory[1000].toJSON()).to.deep.equal(dataContractFixture.toJSON());
      expect(requestedProofModes[requestedProofModes.length - 1]).to.equal(false);
    });
  });
});
