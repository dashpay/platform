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
    let localContract;

    const contractId : Identifier = Identifier.from(identifier);

    for (const appName of this.client.getApps().getNames()) {
        const appDefinition = this.client.getApps().get(appName);
        if (appDefinition.contractId.equals(contractId)) {
            localContract = appDefinition;
            break;
        }
    }

    if (localContract && localContract.contract) {
        return localContract.contract;
    } else {
        // @ts-ignore
        const rawContract = await this.client.getDAPIClient().platform.getDataContract(contractId);

        if (!rawContract) {
            return null;
        }

        const contract = await this.dpp.dataContract.createFromBuffer(rawContract);

        if (!localContract) {
            // If we do not have even the identifier in this.apps, we add it with timestamp as key
            this.client.getApps().set(
                Date.now().toString(),
                {
                    contractId: contractId,
                    contract
                }
            );
        } else {
            localContract.contract = contract;
        }

        return contract;
    }
}

export default get;
