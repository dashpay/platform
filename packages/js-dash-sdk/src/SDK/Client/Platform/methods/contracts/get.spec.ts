import {expect} from 'chai';
import getResponseMetadataFixture from '../../../../../test/fixtures/getResponseMetadataFixture';
import get from "./get";
import identitiesFixtures from "../../../../../../tests/fixtures/identities.json";
import contractsFixtures from "../../../../../../tests/fixtures/contracts.json";
import DataContractFactory from "@dashevo/dpp/lib/dataContract/DataContractFactory";
import ValidationResult from "@dashevo/dpp/lib/validation/ValidationResult";
import Identifier from "@dashevo/dpp/lib/Identifier";
import 'mocha';
import { ClientApps } from "../../../ClientApps";
const GetDataContractResponse = require("@dashevo/dapi-client/lib/methods/platform/getDataContract/GetDataContractResponse");
const NotFoundError = require('@dashevo/dapi-client/lib/errors/response/NotFoundError');

const factory = new DataContractFactory(
    undefined,
    () => {
        return new ValidationResult();
    },
    () => [42, contractsFixtures.ratePlatform]);
const dpp = {
    dataContract: factory,
    getProtocolVersion: () => 42,
}
factory.dpp = dpp;

const apps = new ClientApps({
    ratePlatform: {
        contractId: contractsFixtures.ratePlatform.$id
    },
});
let client;
let askedFromDapi;
let initialize;

describe('Client - Platform - Contracts - .get()', () => {
    before(function before() {
        askedFromDapi = 0;
        const getDataContract = async (id) => {
            const fixtureIdentifier = Identifier.from(contractsFixtures.ratePlatform.$id);
            askedFromDapi += 1;

            if (id.equals(fixtureIdentifier)) {
                const contract = await dpp.dataContract.createFromObject(contractsFixtures.ratePlatform);
                return new GetDataContractResponse(contract.toBuffer(), getResponseMetadataFixture());
            }

            throw new NotFoundError();
        };

        client = {
            getDAPIClient: () => {
                return {
                    platform: {
                        getDataContract
                    }
                };
            },
            getApps(): ClientApps {
                return apps
            }
        };

        initialize = this.sinon.stub();
    });

    describe('get a contract from string', () => {
        it('should get from DAPIClient if there is none locally', async function () {

            // @ts-ignore
            const contract = await get.call({apps, dpp, client, initialize}, contractsFixtures.ratePlatform.$id);
            expect(contract.toJSON()).to.deep.equal(contractsFixtures.ratePlatform);
            expect(contract.getMetadata().getBlockHeight()).to.equal(10);
            expect(contract.getMetadata().getCoreChainLockedHeight()).to.equal(42);
            expect(askedFromDapi).to.equal(1);
        });

        it('should get from local when already fetched once', async function () {
            // @ts-ignore
            const contract = await get.call({apps, dpp, client, initialize}, contractsFixtures.ratePlatform.$id);
            expect(contract.toJSON()).to.deep.equal(contractsFixtures.ratePlatform);
            expect(contract.getMetadata().getBlockHeight()).to.equal(10);
            expect(contract.getMetadata().getCoreChainLockedHeight()).to.equal(42);
            expect(askedFromDapi).to.equal(1);
        });
    })

    describe('other conditions', () => {
        it('should deal when contract do not exist', async function () {
            // @ts-ignore
            const contract = await get.call({apps, dpp, client, initialize}, identitiesFixtures.bob.id);
            expect(contract).to.equal(null);
        });
    });
});
