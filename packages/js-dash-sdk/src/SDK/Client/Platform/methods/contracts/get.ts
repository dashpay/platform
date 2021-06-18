import {Platform} from "../../Platform";

// @ts-ignore
import Identifier from "@dashevo/dpp/lib/Identifier";

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

    // @ts-ignore
    const rawContract = await this.client.getDAPIClient().platform.getDataContract(contractId);

    if (!rawContract) {
        return null;
    }

    const contract = await this.dpp.dataContract.createFromBuffer(rawContract);

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
