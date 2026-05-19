import { expect } from 'chai';

import loadDpp from '@dashevo/wasm-dpp';

import getDataContractFixture from '@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture.js';

import getResponseMetadataFixture from '../../../../../test/fixtures/getResponseMetadataFixture.js';
import get from './get.js';
import identitiesFixtures from '../../../../../../tests/fixtures/identities.json' with { type: 'json' };
import 'mocha';
import { ClientApps } from '../../../ClientApps.js';

import GetDataContractResponse from '@dashevo/dapi-client/lib/methods/platform/getDataContract/GetDataContractResponse.js';
import NotFoundError from '@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError.js';

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
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const load = (loadDpp as any).default ?? loadDpp;
    await load();

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
      expect(askedFromDapi).to.equal(1);
    });

    it('should get from local when already fetched once', async () => {
      const contract = await get.call({
        // @ts-ignore
        apps, dpp, client, initialize, logger, fetcher,
      }, dataContractFixture.getId());
      expect(contract.toJSON()).to.deep.equal(dataContractFixture.toJSON());
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
