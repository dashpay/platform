import {expect} from 'chai';
import get from "./get";
import identitiesFixtures from "../../../../../../tests/fixtures/identities.json";
import contractsFixtures from "../../../../../../tests/fixtures/contracts.json";
import DataContractFactory from "@dashevo/dpp/lib/dataContract/DataContractFactory";
import ValidationResult from "@dashevo/dpp/lib/validation/ValidationResult";
import 'mocha';

const factory = new DataContractFactory(
    () => {
        return new ValidationResult();
    });
const dpp = {
    dataContract: factory
}
const getDataContract = async (id) => {
    switch (id) {
        // @ts-ignore
        case contractsFixtures.ratePlatform.id:
            const contract = await dpp.dataContract.createFromObject(contractsFixtures.ratePlatform);
            return contract.serialize()
        default:
            return null;
    }
};

const client = {
    getDAPIClient: () => {
        return {
            platform: {
                getDataContract
            }
        };
    }
};

const apps = {};

describe('Client - Platform - Contracts - .get()', () => {
    it('should get a contract', async function () {
        // @ts-ignore
        const contract = await get.call({apps, dpp, client}, contractsFixtures.ratePlatform.id);
        expect(contract.toJSON()).to.deep.equal(contractsFixtures.ratePlatform);
    });
    it('should deal when no contract', async function () {
        // @ts-ignore
        const contract = await get.call({apps, dpp, client}, identitiesFixtures.bob.id);
        expect(contract).to.equal(null);
    });
});
