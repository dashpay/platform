import {Platform} from "../../Platform";

// @ts-ignore
import Identifier from "@dashevo/dpp/lib/Identifier";
import Metadata from "@dashevo/dpp/lib/Metadata";
const NotFoundError = require('@dashevo/dapi-client/lib/errors/response/NotFoundError');

declare type ContractIdentifier = string | Identifier;

/**
 * Get contracts from the platform
 *
 * @param {Platform} this - bound instance class
 * @param {ContractIdentifier} identifier - identifier of the contract to fetch
 * @returns contracts
 */
export async function get(this: Platform, identifier: ContractIdentifier): Promise<any> {
    await this.initialize();

    const contractId : Identifier = Identifier.from(identifier);

    // Try to get contract from the cache
    for (const appName of this.client.getApps().getNames()) {
        const appDefinition = this.client.getApps().get(appName);
        if (appDefinition.contractId.equals(contractId) && appDefinition.contract) {
            return appDefinition.contract;
        }
    }

    // Fetch contract otherwise
    let dataContractResponse;
    try {
        dataContractResponse = await this.client.getDAPIClient().platform.getDataContract(contractId);
    } catch (e) {
        if (e instanceof NotFoundError) {
            return null;
        }

        throw e;
    }

    const contract = await this.dpp.dataContract.createFromBuffer(dataContractResponse.getDataContract());

    let metadata = null;
    const responseMetadata = dataContractResponse.getMetadata();
    if (responseMetadata) {
        metadata = new Metadata({
            blockHeight: responseMetadata.getHeight(),
            coreChainLockedHeight: responseMetadata.getCoreChainLockedHeight(),
        });
    }
    contract.setMetadata(metadata);

    // Store contract to the cache

    for (const appName of this.client.getApps().getNames()) {
        const appDefinition = this.client.getApps().get(appName);
        if (appDefinition.contractId.equals(contractId)) {
            appDefinition.contract = contract;
        }
    }

    return contract;
}

export default get;
