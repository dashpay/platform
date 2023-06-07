import { expect } from 'chai';

import loadDpp from '@dashevo/wasm-dpp';

import getDataContractFixture from '@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture';

import getResponseMetadataFixture from '../../../../../test/fixtures/getResponseMetadataFixture';
import get from './get';
import identitiesFixtures from '../../../../../../tests/fixtures/identities.json';
import 'mocha';
import { ClientApps } from '../../../ClientApps';

const GetDataContractResponse = require('@dashevo/dapi-client/lib/methods/platform/getDataContract/GetDataContractResponse');
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

describe('Client - Platform - Contracts - .get()', () => {
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
    const fetchDataContract = async (id) => {
      const fixtureIdentifier = dataContractFixture.getId();
      askedFromDapi += 1;

      if (id.equals(fixtureIdentifier)) {
        return new GetDataContractResponse(dataContractFixture.toBuffer(), metadataFixture);
      }

      throw new NotFoundError();
    };

    fetcher = {
      fetchDataContract,
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
      const contract = await get.call({
        // @ts-ignore
        apps, dpp, client, initialize, logger, fetcher,
      }, dataContractFixture.getId());
      expect(contract.toJSON()).to.deep.equal(dataContractFixture.toJSON());
      expect(contract.getMetadata().getBlockHeight()).to.equal(10);
      expect(contract.getMetadata().getCoreChainLockedHeight()).to.equal(42);
      expect(contract.getMetadata().getTimeMs()).to.equal(metadataFixture.getTimeMs());
      expect(contract.getMetadata().getProtocolVersion())
        .to.equal(metadataFixture.getProtocolVersion());
      expect(askedFromDapi).to.equal(1);
    });

    it('should get from local when already fetched once', async () => {
      const contract = await get.call({
        // @ts-ignore
        apps, dpp, client, initialize, logger, fetcher,
      }, dataContractFixture.getId());
      expect(contract.toJSON()).to.deep.equal(dataContractFixture.toJSON());
      expect(contract.getMetadata().getBlockHeight()).to.equal(10);
      expect(contract.getMetadata().getCoreChainLockedHeight()).to.equal(42);
      expect(contract.getMetadata().getTimeMs()).to.equal(metadataFixture.getTimeMs());
      expect(contract.getMetadata().getProtocolVersion())
        .to.equal(metadataFixture.getProtocolVersion());
      expect(askedFromDapi).to.equal(1);
    });
  });

  describe('other conditions', () => {
    it('should deal when contract do not exist', async () => {
      const contract = await get.call({
        // @ts-ignore
        apps, dpp, client, initialize, logger, fetcher,
      }, identitiesFixtures.bob.id);
      expect(contract).to.equal(null);
    });
  });
});
