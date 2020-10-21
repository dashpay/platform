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
const getDataContract = async (id) => {
    const fixtureIdentifier = Identifier.from(contractsFixtures.ratePlatform.$id);

    if (id.equals(fixtureIdentifier)) {
        const contract = await dpp.dataContract.createFromObject(contractsFixtures.ratePlatform);
        return contract.toBuffer()
    }

    return null;
};

const client = {
    getDAPIClient: () => {
        return {
            platform: {
                getDataContract
            }
        };
    },
    getApps(): ClientApps {
        return new ClientApps();
    }
};

const apps = {};

describe('Client - Platform - Contracts - .get()', () => {
    it('should get a contract by string', async function () {
        // @ts-ignore
        const contract = await get.call({apps, dpp, client}, contractsFixtures.ratePlatform.$id);
        expect(contract.toJSON()).to.deep.equal(contractsFixtures.ratePlatform);
    });

    it('should get a contract by identifier', async function () {
        // @ts-ignore
        const contract = await get.call({apps, dpp, client}, Identifier.from(contractsFixtures.ratePlatform.$id));
        expect(contract.toJSON()).to.deep.equal(contractsFixtures.ratePlatform);
    });

    it('should deal when no contract', async function () {
        // @ts-ignore
        const contract = await get.call({apps, dpp, client}, identitiesFixtures.bob.id);
        expect(contract).to.equal(null);
    });
});
