import {Platform} from "../../Platform";

declare type ContractIdentifier = string;

/**
 * Get contracts from the platform
 *
 * @param {Platform} this - bound instance class
 * @param {ContractIdentifier} identifier - identifier
 * @returns contracts
 */
export async function get(this: Platform, identifier: ContractIdentifier): Promise<any> {
    let localContract;

    for (let appName in this.apps) {
        const app = this.apps[appName];
        if (app.contractId === identifier) {
            localContract = app;
            break;
        }
    }

    if (localContract && localContract.contract) {
        return localContract.contract;
    } else {
        try {
            // @ts-ignore
            const rawContract = await this.client.getDataContract(identifier);
            if(!rawContract){
                return null;
            }

            const contract = await this.dpp.dataContract.createFromSerialized(rawContract);
            const app = {contractId: identifier, contract};
            // If we do not have even the identifier in this.apps, we add it with timestamp as key
            if (localContract === undefined) {
                this.apps[Date.now()] = app
            }
            return app.contract;
        } catch (e) {
            console.error('Failed to get dataContract', e);
            throw e;
        }

    }
}

export default get;
