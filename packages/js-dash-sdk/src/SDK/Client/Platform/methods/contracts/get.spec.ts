import { expect } from 'chai';
import DataContractFactory from '@dashevo/dpp/lib/dataContract/DataContractFactory';
import ValidationResult from '@dashevo/dpp/lib/validation/ValidationResult';
import Identifier from '@dashevo/dpp/lib/Identifier';
import getResponseMetadataFixture from '../../../../../test/fixtures/getResponseMetadataFixture';
import get from './get';
import identitiesFixtures from '../../../../../../tests/fixtures/identities.json';
import contractsFixtures from '../../../../../../tests/fixtures/contracts.json';
import 'mocha';
import { ClientApps } from '../../../ClientApps';

const GetDataContractResponse = require('@dashevo/dapi-client/lib/methods/platform/getDataContract/GetDataContractResponse');
const NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');

const factory = new DataContractFactory(
  undefined,
  () => new ValidationResult(),
  () => [42, contractsFixtures.ratePlatform],
);
const dpp = {
  dataContract: factory,
  getProtocolVersion: () => 42,
};
factory.dpp = dpp;

const apps = new ClientApps({
  ratePlatform: {
    contractId: contractsFixtures.ratePlatform.$id,
  },
});
let client;
let askedFromDapi;
let initialize;
let metadataFixture;

describe('Client - Platform - Contracts - .get()', () => {
  before(function before() {
    metadataFixture = getResponseMetadataFixture();
    askedFromDapi = 0;
    const getDataContract = async (id) => {
      const fixtureIdentifier = Identifier.from(contractsFixtures.ratePlatform.$id);
      askedFromDapi += 1;

      if (id.equals(fixtureIdentifier)) {
        const contract = await dpp.dataContract.createFromObject(contractsFixtures.ratePlatform);
        return new GetDataContractResponse(contract.toBuffer(), metadataFixture);
      }

      throw new NotFoundError();
    };

    client = {
      getDAPIClient: () => ({
        platform: {
          getDataContract,
        },
      }),
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
        apps, dpp, client, initialize,
      }, contractsFixtures.ratePlatform.$id);
      expect(contract.toJSON()).to.deep.equal(contractsFixtures.ratePlatform);
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
        apps, dpp, client, initialize,
      }, contractsFixtures.ratePlatform.$id);
      expect(contract.toJSON()).to.deep.equal(contractsFixtures.ratePlatform);
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
        apps, dpp, client, initialize,
      }, identitiesFixtures.bob.id);
      expect(contract).to.equal(null);
    });
  });
});
