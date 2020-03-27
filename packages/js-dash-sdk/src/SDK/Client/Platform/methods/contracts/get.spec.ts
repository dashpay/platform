import {expect} from 'chai';
import get from "./get";
import identitiesFixtures from "../../../../../../tests/fixtures/identities.json";
import contractsFixtures from "../../../../../../tests/fixtures/contracts.json";
import createDataContract from "@dashevo/dpp/lib/dataContract/createDataContract";
import DataContractFactory from "@dashevo/dpp/lib/dataContract/DataContractFactory";
import ValidationResult from "@dashevo/dpp/lib/validation/ValidationResult";
import 'mocha';

const factory = new DataContractFactory(
    createDataContract,
    () => {
        const result = new ValidationResult();
        return result;
    });
const dpp = {
    dataContract: factory
}
const client = {
    getDataContract: async (id) => {
        switch (id) {
            case identitiesFixtures.ratePhezApp.id:
                const contract = await dpp.dataContract.createFromObject(contractsFixtures.ratePhezApp);
                return contract.serialize()
            default:
                return null;
        }
    },
};
const apps = {};

describe('Client - Platform - Contracts - .get()', () => {
    it('should get a contract', async function () {
        // @ts-ignore
        const contract = await get.call({apps, dpp, client}, identitiesFixtures.ratePhezApp.id);
        // console.log(contract)
        expect(contract.toJSON()).to.deep.equal(contractsFixtures.ratePhezApp);
    });
    it('should deal when no contract', async function () {
        // @ts-ignore
        const contract = await get.call({apps, dpp, client}, identitiesFixtures.bob.id);
        expect(contract).to.equal(null);
    });
});
