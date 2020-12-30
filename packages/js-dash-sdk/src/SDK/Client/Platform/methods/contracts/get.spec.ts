import {expect} from 'chai';
import get from "./get";
import identitiesFixtures from "../../../../../../tests/fixtures/identities.json";
import contractsFixtures from "../../../../../../tests/fixtures/contracts.json";
import DataContractFactory from "@dashevo/dpp/lib/dataContract/DataContractFactory";
import ValidationResult from "@dashevo/dpp/lib/validation/ValidationResult";
import Identifier from "@dashevo/dpp/lib/Identifier";
import 'mocha';
import {ClientApps} from "../../../ClientApps";

const factory = new DataContractFactory(
    () => {
        return new ValidationResult();
    });
const dpp = {
    dataContract: factory
}

const apps = new ClientApps({
    ratePlatform: {
        contractId: contractsFixtures.ratePlatform.$id
    },
});
let client;
let askedFromDapi;

describe('Client - Platform - Contracts - .get()', () => {
    before(()=>{
        askedFromDapi = 0;
        const getDataContract = async (id) => {
            const fixtureIdentifier = Identifier.from(contractsFixtures.ratePlatform.$id);
            askedFromDapi+=1;

            if (id.equals(fixtureIdentifier)) {
                const contract = await dpp.dataContract.createFromObject(contractsFixtures.ratePlatform);
                return contract.toBuffer()
            }
            return null;
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
    })
    describe('get a contract from string', ()=>{
        it('should get from DAPIClient if there is none locally', async function () {

            // @ts-ignore
            const contract = await get.call({apps, dpp, client}, contractsFixtures.ratePlatform.$id);
            expect(contract.toJSON()).to.deep.equal(contractsFixtures.ratePlatform);
            expect(askedFromDapi).to.equal(1);
        });
        it('should get from local when already fetched once', async function () {
            // @ts-ignore
            const contract = await get.call({apps, dpp, client}, contractsFixtures.ratePlatform.$id);
            expect(contract.toJSON()).to.deep.equal(contractsFixtures.ratePlatform);
            expect(askedFromDapi).to.equal(1);
        });
    })

    describe('other conditions', ()=>{
        it('should deal when contract do not exist', async function () {
            // @ts-ignore
            const contract = await get.call({apps, dpp, client}, identitiesFixtures.bob.id);
            expect(contract).to.equal(null);
        });
    })
});
